//! Global application state using Dioxus signals.
//!
//! Replaces Zustand stores from the Tauri frontend.
//! In Dioxus, signals provide interior mutability, but methods need `mut self`.

use std::collections::HashMap;

use dioxus::prelude::*;

use crate::services::worker_client::{WorkerClient, WorkerConfig};
use crate::types::{AnalysisEntry, AssemblyId, OpenAssembly, RuleInfo};

/// Global application state.
///
/// All fields are signals that can be read/written from any component.
/// Use `use_context::<AppState>()` to access.
#[derive(Clone, Copy)]
pub struct AppState {
    pub assemblies: Signal<Vec<OpenAssembly>>,
    pub selected_id: Signal<Option<AssemblyId>>,
    pub analysis_entries: Signal<HashMap<String, AnalysisEntry>>,
    pub rules: Signal<Vec<RuleInfo>>,
    pub last_export_path: Signal<Option<String>>,
    /// The long-lived worker client — shared across all async tasks.
    pub worker: Signal<WorkerClient>,
    pub is_running: Signal<bool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            assemblies: Signal::new(Vec::new()),
            selected_id: Signal::new(None),
            analysis_entries: Signal::new(HashMap::new()),
            rules: Signal::new(Vec::new()),
            last_export_path: Signal::new(None),
            worker: Signal::new(WorkerClient::new(WorkerConfig::default())),
            is_running: Signal::new(false),
        }
    }
}

impl AppState {
    /// Add a new assembly to the list.
    pub fn open_assembly(mut self, path: String) {
        let name = std::path::Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let id = format!(
            "{}-{}",
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );

        let assembly = OpenAssembly {
            id: id.clone(),
            path,
            name: name.clone(),
            loaded_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        self.assemblies.write().push(assembly);
        self.selected_id.set(Some(id));
        tracing::info!("Opened assembly: {}", name);
    }

    /// Close an assembly by ID.
    pub fn close_assembly(mut self, id: AssemblyId) {
        let mut assemblies = self.assemblies.write();
        if let Some(pos) = assemblies.iter().position(|a| a.id == id) {
            let removed = assemblies.remove(pos);
            tracing::info!("Closed assembly: {}", removed.name);

            // Update selection if this was the selected assembly
            if self.selected_id.read().as_ref() == Some(&id) {
                drop(assemblies);
                self.selected_id
                    .set(self.assemblies.read().first().map(|a| a.id.clone()));
            }
        }
    }

    /// Select an assembly by ID.
    pub fn select_assembly(mut self, id: AssemblyId) {
        if self.assemblies.read().iter().any(|a| a.id == id) {
            self.selected_id.set(Some(id));
        }
    }

    /// Store an analysis result.
    pub fn set_analysis_result(mut self, key: String, entry: AnalysisEntry) {
        self.analysis_entries.write().insert(key, entry);
    }

    /// Get an analysis entry by key.
    pub fn get_analysis_entry(self, key: &str) -> Option<AnalysisEntry> {
        self.analysis_entries.read().get(key).cloned()
    }

    /// Clear all assemblies and analysis state.
    pub fn clear_all(mut self) {
        self.assemblies.write().clear();
        self.selected_id.set(None);
        self.analysis_entries.write().clear();
        self.last_export_path.set(None);
    }

    /// Set rules loaded from the worker.
    pub fn set_rules(mut self, rules: Vec<RuleInfo>) {
        tracing::info!("Setting {} rules", rules.len());
        self.rules.set(rules);
    }

    pub fn set_last_export_path(mut self, path: Option<String>) {
        self.last_export_path.set(path);
    }
}
