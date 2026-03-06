/// Background analysis task: runs explore + scan in parallel and stores results.
use dioxus::prelude::*;

use crate::ipc::{ExploreParams, ScanParams};
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

        let mut combined_entry = AnalysisEntry {
            assembly_id: assembly_id.clone(),
            assembly_path: assembly_path.clone(),
            mode: ActiveMode::Explore,
            status: AnalysisStatus::Running,
            result: None,
            error: None,
            started_at: Some(started),
            finished_at: None,
        };

        // Mark running immediately so the UI shows the spinner.
        state.set_analysis_result(format!("{}::explore", assembly_id), combined_entry.clone());
        state.set_analysis_result(format!("{}::scan", assembly_id), combined_entry.clone());

        let explore_future = worker.explore(ExploreParams {
            assembly: assembly_path.clone(),
            ..Default::default()
        });
        let scan_future = worker.scan(ScanParams {
            assembly: assembly_path.clone(),
            ..Default::default()
        });

        let (explore_result, scan_result) = tokio::join!(explore_future, scan_future);
        let finished = now_ts();

        let mut result = AnalysisResult {
            assembly_path: assembly_path.clone(),
            mode: "combined".to_string(),
            explore: None,
            scan: None,
            stderr: String::new(),
        };

        match explore_result {
            Ok(payload) => {
                tracing::debug!(methods = payload.methods.len(), "explore done");
                result.explore = Some(payload);
            }
            Err(e) => {
                tracing::error!(err = %e, "explore failed");
                last_error.set(e.to_string());
                combined_entry.status = AnalysisStatus::Error;
                combined_entry.error = Some(e.to_string());
            }
        }

        match scan_result {
            Ok(payload) => {
                tracing::debug!(findings = payload.findings.len(), "scan done");
                result.scan = Some(payload);
            }
            Err(e) => {
                tracing::error!(err = %e, "scan failed");
                last_error.set(e.to_string());
                if combined_entry.status != AnalysisStatus::Error {
                    combined_entry.status = AnalysisStatus::Error;
                    combined_entry.error = Some(e.to_string());
                }
            }
        }

        // Only mark Done if we got at least one successful payload.
        if result.explore.is_some() || result.scan.is_some() {
            combined_entry.status = AnalysisStatus::Done;
            combined_entry.result = Some(result);
            combined_entry.finished_at = Some(finished);
        }

        // Store under both keys so existing lookup code (::explore / ::scan) still works.
        state.set_analysis_result(format!("{}::explore", assembly_id), combined_entry.clone());
        state.set_analysis_result(format!("{}::scan", assembly_id), combined_entry);

        state.is_running.set(false);
    });
}
