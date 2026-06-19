/// Background analysis task: runs explore first, then scan.
///
/// Explore is intentionally committed as soon as it finishes so the assembly
/// browser and IL navigation become usable before the malware scan completes.
use dioxus::prelude::*;

use crate::ipc::{ExploreParams, ScanParams};
use crate::services::worker_client::{WorkerClient, WorkerConfig};
use crate::state::AppState;
use crate::types::{ActiveMode, AnalysisEntry, AnalysisResult, AnalysisStatus};

use super::helpers::now_ts;

pub fn run_analysis(
    mut state: AppState,
    mut last_error: Signal<String>,
    assembly_id: String,
    assembly_path: String,
) {
    spawn(async move {
        state.is_running.set(true);
        last_error.set(String::new());

        let started = now_ts();
        let worker = state.worker.read().clone();
        let scan_worker = WorkerClient::new(WorkerConfig::default());

        let explore_key = format!("{}::explore", assembly_id);
        let scan_key = format!("{}::scan", assembly_id);

        let mut explore_entry = AnalysisEntry {
            assembly_id: assembly_id.clone(),
            assembly_path: assembly_path.clone(),
            mode: ActiveMode::Explore,
            status: AnalysisStatus::Running,
            result: None,
            error: None,
            started_at: Some(started),
            finished_at: None,
        };
        let mut scan_entry = AnalysisEntry {
            assembly_id: assembly_id.clone(),
            assembly_path: assembly_path.clone(),
            mode: ActiveMode::Scan,
            status: AnalysisStatus::Idle,
            result: None,
            error: None,
            started_at: None,
            finished_at: None,
        };

        state.set_analysis_result(explore_key.clone(), explore_entry.clone());
        state.set_analysis_result(scan_key.clone(), scan_entry.clone());

        let explore_result = worker
            .explore(ExploreParams {
                assembly: assembly_path.clone(),
                ..Default::default()
            })
            .await;

        match explore_result {
            Ok(payload) => {
                tracing::debug!(methods = payload.methods.len(), "explore done");
                explore_entry.status = AnalysisStatus::Done;
                explore_entry.result = Some(AnalysisResult {
                    assembly_path: assembly_path.clone(),
                    mode: "explore".to_string(),
                    explore: Some(payload),
                    scan: None,
                    stderr: String::new(),
                });
                explore_entry.finished_at = Some(now_ts());
            }
            Err(e) => {
                tracing::error!(err = %e, "explore failed");
                last_error.set(e.to_string());
                explore_entry.status = AnalysisStatus::Error;
                explore_entry.error = Some(e.to_string());
                explore_entry.finished_at = Some(now_ts());
            }
        }

        state.set_analysis_result(explore_key, explore_entry);

        scan_entry.status = AnalysisStatus::Running;
        scan_entry.started_at = Some(now_ts());
        state.set_analysis_result(scan_key.clone(), scan_entry.clone());

        let scan_result = scan_worker
            .scan(ScanParams {
                assembly: assembly_path.clone(),
                ..Default::default()
            })
            .await;
        scan_worker.shutdown().await;

        match scan_result {
            Ok(payload) => {
                tracing::debug!(findings = payload.findings.len(), "scan done");
                scan_entry.status = AnalysisStatus::Done;
                scan_entry.result = Some(AnalysisResult {
                    assembly_path,
                    mode: "scan".to_string(),
                    explore: None,
                    scan: Some(payload),
                    stderr: String::new(),
                });
                scan_entry.finished_at = Some(now_ts());
            }
            Err(e) => {
                tracing::error!(err = %e, "scan failed");
                last_error.set(e.to_string());
                scan_entry.status = AnalysisStatus::Error;
                scan_entry.error = Some(e.to_string());
                scan_entry.finished_at = Some(now_ts());
            }
        }

        state.set_analysis_result(scan_key, scan_entry);
        state.is_running.set(false);
    });
}
