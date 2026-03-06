//! WorkerClient — manages the long-lived ILInspector.Worker subprocess.
//!
//! A single worker process is spawned on first use and kept alive for the
//! session.  Each request is assigned a monotonically increasing `id`.
//! Responses are matched back by `id`.
//!
//! Wire format: newline-delimited JSON (NDJSON) over stdin / stdout.
//! The worker's stderr is forwarded to our own tracing subscriber so debug
//! output is visible without polluting the IPC channel.

use std::sync::Arc;

use serde::Serialize;
use std::collections::HashMap;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, Command},
    sync::{oneshot, Mutex},
};
use tracing::{debug, error, warn};

use crate::{
    error::AppError,
    ipc::{
        DecompileParams, DecompilePayload, ExploreParams, ExplorePayload, NoParams, RuleEntry,
        ScanParams, ScanPayload, WorkerRequest, WorkerResponse,
    },
    services::tool_paths::resolve_worker_path,
};

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// Path to the ILInspector.Worker executable.
    pub worker_path: String,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            worker_path: resolve_worker_path().to_string_lossy().into_owned(),
        }
    }
}

// ─── Client ───────────────────────────────────────────────────────────────────

type PendingMap = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<serde_json::Value, String>>>>>;

/// Handle to the running worker process.  Clone-cheap via `Arc`.
#[derive(Clone)]
pub struct WorkerClient {
    inner: Arc<Mutex<WorkerInner>>,
}

struct WorkerInner {
    config: WorkerConfig,
    state: Option<WorkerState>,
}

struct WorkerState {
    stdin: ChildStdin,
    next_id: u64,
    pending: PendingMap,
    // Keep the Child alive so it isn't killed on drop.
    _child: Child,
}

impl WorkerClient {
    pub fn new(config: WorkerConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(WorkerInner {
                config,
                state: None,
            })),
        }
    }

    /// Ensure the worker is running, (re-)spawning if necessary.
    async fn ensure_running(inner: &mut WorkerInner) -> Result<(), AppError> {
        if inner.state.is_some() {
            return Ok(());
        }

        let exe = &inner.config.worker_path;
        debug!(exe = %exe, "spawning ILInspector.Worker");

        let mut child = {
            let mut cmd = Command::new(exe);
            cmd.stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            // Hide the console window on Windows.
            #[cfg(target_os = "windows")]
            {
                const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }

            cmd.spawn()
                .map_err(|e| AppError::Process(format!("failed to spawn worker: {e}")))?
        };

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| AppError::Process("worker stdin not available".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::Process("worker stdout not available".into()))?;

        // Forward stderr to tracing so we can see [worker] log lines.
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    debug!(target: "ilworker", "{}", line);
                }
            });
        }

        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));

        // Reader task — routes incoming responses to waiting callers.
        {
            let pending_clone = Arc::clone(&pending);
            tokio::spawn(async move {
                let mut lines = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    match serde_json::from_str::<WorkerResponse>(&line) {
                        Ok(resp) => {
                            let tx = pending_clone.lock().await.remove(&resp.id);
                            if let Some(tx) = tx {
                                let result = if resp.ok {
                                    Ok(resp.payload.unwrap_or(serde_json::Value::Null))
                                } else {
                                    Err(resp.error.unwrap_or_else(|| "unknown worker error".into()))
                                };
                                let _ = tx.send(result);
                            } else {
                                warn!(id = resp.id, "received response for unknown request id");
                            }
                        }
                        Err(e) => {
                            error!(err = %e, line = %line, "failed to parse worker response");
                        }
                    }
                }
                // If the reader loop exits the worker process has closed its stdout.
                // Callers with pending requests will see their oneshot senders dropped.
                debug!("worker stdout reader exited");
            });
        }

        inner.state = Some(WorkerState {
            stdin,
            next_id: 1,
            pending,
            _child: child,
        });

        Ok(())
    }

    /// Send a typed request and wait for the parsed response payload.
    async fn call<P: Serialize, R: serde::de::DeserializeOwned>(
        &self,
        method: &'static str,
        params: P,
    ) -> Result<R, AppError> {
        let (tx, rx) = oneshot::channel();
        let line = {
            let mut guard = self.inner.lock().await;
            Self::ensure_running(&mut guard).await?;
            let state = guard.state.as_mut().unwrap();

            let id = state.next_id;
            state.next_id += 1;

            let req = WorkerRequest { id, method, params };
            let mut line = serde_json::to_string(&req)
                .map_err(|e| AppError::Process(format!("request serialize error: {e}")))?;
            line.push('\n');

            state.pending.lock().await.insert(id, tx);
            line
        };

        // Write to stdin outside the inner lock so we don't hold it while awaiting.
        {
            let mut guard = self.inner.lock().await;
            if let Some(state) = guard.state.as_mut() {
                state.stdin.write_all(line.as_bytes()).await.map_err(|e| {
                    AppError::Process(format!("failed to write to worker stdin: {e}"))
                })?;
            }
        }

        let raw = rx
            .await
            .map_err(|_| AppError::Process("worker dropped response channel".into()))?
            .map_err(|e| AppError::Process(format!("worker error: {e}")))?;

        serde_json::from_value(raw)
            .map_err(|e| AppError::Parse(format!("failed to deserialize worker payload: {e}")))
    }

    // ── Public API ────────────────────────────────────────────────────────────

    pub async fn explore(&self, params: ExploreParams) -> Result<ExplorePayload, AppError> {
        self.call("explore", params).await
    }

    pub async fn scan(&self, params: ScanParams) -> Result<ScanPayload, AppError> {
        self.call("scan", params).await
    }

    pub async fn list_rules(&self) -> Result<Vec<RuleEntry>, AppError> {
        self.call("list-rules", NoParams {}).await
    }

    pub async fn decompile(&self, params: DecompileParams) -> Result<DecompilePayload, AppError> {
        self.call("decompile", params).await
    }

    /// Ask the worker to evict a cached assembly (e.g. after a file is modified).
    pub async fn evict(&self, assembly_path: String) -> Result<(), AppError> {
        let _: serde_json::Value = self.call("evict", assembly_path).await?;
        Ok(())
    }

    /// Gracefully shut down the worker process.
    pub async fn shutdown(&self) {
        let mut guard = self.inner.lock().await;
        if let Some(state) = guard.state.as_mut() {
            let req = serde_json::json!({ "id": 0u64, "method": "shutdown", "params": {} });
            let mut line = req.to_string();
            line.push('\n');
            let _ = state.stdin.write_all(line.as_bytes()).await;
        }
        guard.state = None;
    }
}
