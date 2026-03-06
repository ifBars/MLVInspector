//! InspectorService — legacy one-shot CLI wrapper. Superseded by WorkerClient.
//! Kept for reference; not used by the application.
#![allow(dead_code)]
//!
//! Each analysis command shells out to `ILInspector` (the compiled .NET tool)
//! with `--format json` and returns the parsed JSON output.
//!
//! The CLI binary path is configurable and stored in the service's config.
//! It defaults to looking for `ILInspector` on PATH.
//!
//! CRITICAL: Assemblies are NEVER loaded into this process — they are only
//! passed as file paths to the ILInspector subprocess. This preserves the
//! security boundary between the analyzer and potentially malicious assemblies.

use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::{debug, error, warn};

use crate::error::AppError;
use crate::services::tool_paths::resolve_inspector_path;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Persistent configuration for the InspectorService.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectorConfig {
    /// Path to the ILInspector executable. Defaults to a debug path.
    pub inspector_path: String,
}

impl Default for InspectorConfig {
    fn default() -> Self {
        Self {
            inspector_path: resolve_inspector_path().to_string_lossy().into_owned(),
        }
    }
}

// ---------------------------------------------------------------------------
// Input option types (mirror ILInspector CLI flags)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExploreOptions {
    pub type_filter: Option<String>,
    pub method_filter: Option<String>,
    pub namespace_filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScanOptions {
    pub type_filter: Option<String>,
    pub method_filter: Option<String>,
    pub namespace_filter: Option<String>,
    pub include_rules: Option<Vec<String>>,
    pub exclude_rules: Option<Vec<String>>,
    pub show_clean: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct CompareOptions {
    pub type_filter: Option<String>,
    pub method_filter: Option<String>,
    pub namespace_filter: Option<String>,
    pub expected_rule: Option<String>,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Generic analysis result.
/// The `raw_json` field contains the full JSON payload from ILInspector,
/// which the frontend parses into its own typed models.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResult {
    pub assembly_path: String,
    pub mode: String,
    pub raw_json: serde_json::Value,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleInfo {
    pub rule_id: String,
    pub description: String,
    pub severity: String,
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

pub struct InspectorService;

impl InspectorService {
    // -----------------------------------------------------------------------
    // Async methods — these take config by value for async compatibility
    // -----------------------------------------------------------------------

    pub async fn list_rules(config: InspectorConfig) -> Result<Vec<RuleInfo>, AppError> {
        let output = Command::new(&config.inspector_path)
            .args(["list-rules", "--format", "json"])
            .output()
            .await
            .map_err(|e| AppError::Process(format!("failed to launch ILInspector: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(AppError::Process(format!(
                "ILInspector exited with {}: {stderr}",
                output.status
            )));
        }

        let json: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| AppError::Parse(format!("failed to parse rule list: {e}")))?;

        let rules = json
            .as_array()
            .ok_or_else(|| AppError::Parse("expected array of rules".into()))?
            .iter()
            .filter_map(|v| {
                Some(RuleInfo {
                    rule_id: v["ruleId"].as_str()?.to_string(),
                    description: v["description"].as_str().unwrap_or("").to_string(),
                    severity: v["severity"].as_str().unwrap_or("Unknown").to_string(),
                })
            })
            .collect();

        Ok(rules)
    }

    pub async fn explore(
        config: InspectorConfig,
        assembly: String,
        opts: ExploreOptions,
    ) -> Result<AnalysisResult, AppError> {
        let mut args = vec!["explore".to_string(), assembly.clone(), "--format".to_string(), "json".to_string()];
        push_filter_args(&mut args, opts.type_filter, opts.method_filter, opts.namespace_filter);
        run_inspector(&config.inspector_path, &args, &assembly, "explore").await
    }

    pub async fn scan(
        config: InspectorConfig,
        assembly: String,
        opts: ScanOptions,
    ) -> Result<AnalysisResult, AppError> {
        debug!(
            assembly = %assembly,
            type_filter = ?opts.type_filter,
            method_filter = ?opts.method_filter,
            namespace_filter = ?opts.namespace_filter,
            include_rules = ?opts.include_rules,
            exclude_rules = ?opts.exclude_rules,
            show_clean = ?opts.show_clean,
            "[scan] received options"
        );

        let mut args = vec!["scan".to_string(), assembly.clone(), "--format".to_string(), "json".to_string()];
        push_filter_args(&mut args, opts.type_filter, opts.method_filter, opts.namespace_filter);
        if let Some(rules) = opts.include_rules {
            for r in rules { args.push("--rules".to_string()); args.push(r); }
        }
        if let Some(rules) = opts.exclude_rules {
            for r in rules { args.push("--exclude-rules".to_string()); args.push(r); }
        }
        if opts.show_clean.unwrap_or(false) {
            args.push("--show-clean".to_string());
        }

        debug!(args = ?args, "[scan] final CLI args");
        run_inspector(&config.inspector_path, &args, &assembly, "scan").await
    }

    #[allow(dead_code)]
    pub async fn compare(
        config: InspectorConfig,
        assembly: String,
        opts: CompareOptions,
    ) -> Result<AnalysisResult, AppError> {
        let mut args = vec!["compare".to_string(), assembly.clone(), "--format".to_string(), "json".to_string()];
        push_filter_args(&mut args, opts.type_filter, opts.method_filter, opts.namespace_filter);
        if let Some(rule) = opts.expected_rule {
            args.push("--expected-rule".to_string());
            args.push(rule);
        }
        run_inspector(&config.inspector_path, &args, &assembly, "compare").await
    }

    #[allow(dead_code)]
    pub async fn analyze_reflect(
        config: InspectorConfig,
        assembly: String,
    ) -> Result<AnalysisResult, AppError> {
        let args = vec![
            "analyze-reflect".to_string(),
            assembly.clone(),
            "--format".to_string(),
            "json".to_string(),
        ];
        run_inspector(&config.inspector_path, &args, &assembly, "analyze-reflect").await
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn push_filter_args(
    args: &mut Vec<String>,
    type_filter: Option<String>,
    method_filter: Option<String>,
    namespace_filter: Option<String>,
) {
    if let Some(t) = type_filter {
        args.push("--type".to_string());
        args.push(t);
    }
    if let Some(m) = method_filter {
        args.push("--method".to_string());
        args.push(m);
    }
    if let Some(n) = namespace_filter {
        args.push("--namespace".to_string());
        args.push(n);
    }
}

async fn run_inspector(
    exe: &str,
    args: &[String],
    assembly_path: &str,
    mode: &str,
) -> Result<AnalysisResult, AppError> {
    debug!(
        mode = mode,
        exe = exe,
        args = ?args,
        assembly = assembly_path,
        "[run_inspector] launching subprocess"
    );

    let output = Command::new(exe)
        .args(args)
        .output()
        .await
        .map_err(|e| {
            error!(exe = exe, err = %e, "[run_inspector] failed to launch ILInspector");
            AppError::Process(format!("failed to launch ILInspector: {e}"))
        })?;

    let exit_code = output.status.code();
    let stdout_len = output.stdout.len();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    debug!(
        mode = mode,
        exit_code = ?exit_code,
        stdout_bytes = stdout_len,
        stderr = %stderr,
        "[run_inspector] subprocess finished"
    );

    if !stderr.is_empty() {
        debug!(mode = mode, stderr = %stderr, "[run_inspector] stderr output");
    }

    if !output.status.success() {
        error!(
            mode = mode,
            exit_code = ?exit_code,
            stderr = %stderr,
            "[run_inspector] non-zero exit — returning error"
        );
        return Err(AppError::Process(format!(
            "ILInspector exited with {}: {stderr}",
            output.status
        )));
    }

    // Log the raw stdout so we can see what came back before JSON parsing.
    // Truncate to 4 KB to avoid flooding logs on large assemblies.
    let stdout_preview = String::from_utf8_lossy(
        &output.stdout[..output.stdout.len().min(4096)],
    );
    debug!(
        mode = mode,
        stdout_bytes = stdout_len,
        stdout_preview = %stdout_preview,
        "[run_inspector] raw stdout"
    );

    let raw_json: serde_json::Value = match serde_json::from_slice::<serde_json::Value>(&output.stdout) {
        Ok(v) => {
            // Log the top-level keys present so we can see if "findings" is there.
            if let Some(obj) = v.as_object() {
                let keys: Vec<&str> = obj.keys().map(String::as_str).collect();
                debug!(
                    mode = mode,
                    json_keys = ?keys,
                    "[run_inspector] JSON parsed OK — top-level keys"
                );
                // Log finding count if present.
                if let Some(findings) = obj.get("findings") {
                    let count = findings.as_array().map(|a| a.len()).unwrap_or(0);
                    debug!(
                        mode = mode,
                        findings_count = count,
                        findings_is_null = findings.is_null(),
                        "[run_inspector] findings field"
                    );
                } else {
                    warn!(mode = mode, "[run_inspector] JSON has no 'findings' key");
                }
            } else {
                warn!(mode = mode, json_type = ?v, "[run_inspector] JSON root is not an object");
            }
            v
        }
        Err(e) => {
            error!(
                mode = mode,
                err = %e,
                stdout_preview = %stdout_preview,
                "[run_inspector] JSON parse failed — returning Null"
            );
            serde_json::Value::Null
        }
    };

    // Log the serialized AnalysisResult fields so we can confirm what gets
    // sent to the frontend (especially whether raw_json is non-null).
    debug!(
        mode = mode,
        raw_json_is_null = raw_json.is_null(),
        "[run_inspector] returning AnalysisResult"
    );

    Ok(AnalysisResult {
        assembly_path: assembly_path.to_string(),
        mode: mode.to_string(),
        raw_json,
        stderr,
    })
}
