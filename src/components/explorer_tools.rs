use std::collections::HashMap;

use crate::types::{AnalysisEntry, OpenAssembly};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PaletteEntryKind {
    Action,
    Assembly,
    Type,
    Method,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PaletteAction {
    RunAnalysis,
    ToggleFindings,
    ExportProject,
    OpenExportFolder,
}

#[derive(Clone, PartialEq, Eq)]
pub struct PaletteEntry {
    pub id: String,
    pub kind: PaletteEntryKind,
    pub title: String,
    pub subtitle: String,
    pub group_label: &'static str,
    pub assembly_id: Option<String>,
    pub type_name: Option<String>,
    pub method_name: Option<String>,
    pub action: Option<PaletteAction>,
}

pub fn build_palette_entries(
    assemblies: &[OpenAssembly],
    analysis_entries: &HashMap<String, AnalysisEntry>,
    has_exported_project: bool,
    query: &str,
) -> Vec<PaletteEntry> {
    let normalized = normalize_query(query);
    let mut entries = Vec::new();

    let mut action_entries = vec![
        (
            PaletteAction::RunAnalysis,
            "Run analysis",
            "Re-run explore and scan for the selected assembly",
        ),
        (
            PaletteAction::ToggleFindings,
            "Toggle findings panel",
            "Show or hide the scan findings sidebar",
        ),
        (
            PaletteAction::ExportProject,
            "Export project",
            "Write a portable bundle with decompiled source and analysis JSON",
        ),
    ];

    if has_exported_project {
        action_entries.push((
            PaletteAction::OpenExportFolder,
            "Open export folder",
            "Reveal the most recently exported decompiled project",
        ));
    }

    for (action, title, subtitle) in action_entries {
        if normalized.is_empty() || matches_query(&normalized, &[title, subtitle]) {
            entries.push(PaletteEntry {
                id: format!("action::{title}"),
                kind: PaletteEntryKind::Action,
                title: title.to_string(),
                subtitle: subtitle.to_string(),
                group_label: "Actions",
                assembly_id: None,
                type_name: None,
                method_name: None,
                action: Some(action),
            });
        }
    }

    for assembly in assemblies.iter().take(8) {
        if normalized.is_empty() || matches_query(&normalized, &[&assembly.name, &assembly.path]) {
            entries.push(PaletteEntry {
                id: format!("assembly::{}", assembly.id),
                kind: PaletteEntryKind::Assembly,
                title: assembly.name.clone(),
                subtitle: assembly.path.clone(),
                group_label: "Assemblies",
                assembly_id: Some(assembly.id.clone()),
                type_name: None,
                method_name: None,
                action: None,
            });
        }
    }

    for assembly in assemblies {
        let Some(entry) = analysis_entries.get(&format!("{}::explore", assembly.id)) else {
            continue;
        };
        let Some(result) = entry.result.as_ref() else {
            continue;
        };
        let Some(explore) = result.explore.as_ref() else {
            continue;
        };

        for type_name in explore
            .methods
            .iter()
            .map(|method| method.type_name.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .take(10)
        {
            let display_name = type_name
                .rsplit('.')
                .next()
                .unwrap_or(&type_name)
                .to_string();
            let subtitle = format!("{} - {}", assembly.name, type_name);
            if normalized.is_empty()
                || matches_query(&normalized, &[&display_name, &type_name, &subtitle])
            {
                entries.push(PaletteEntry {
                    id: format!("type::{}::{}", assembly.id, type_name),
                    kind: PaletteEntryKind::Type,
                    title: display_name,
                    subtitle,
                    group_label: "Types",
                    assembly_id: Some(assembly.id.clone()),
                    type_name: Some(type_name),
                    method_name: None,
                    action: None,
                });
            }
        }

        for method in explore.methods.iter().take(30) {
            let subtitle = format!("{} - {}", assembly.name, method.type_name);
            if normalized.is_empty()
                || matches_query(
                    &normalized,
                    &[
                        &method.method_name,
                        &method.signature,
                        &method.type_name,
                        &subtitle,
                    ],
                )
            {
                entries.push(PaletteEntry {
                    id: format!(
                        "method::{}::{}::{}",
                        assembly.id, method.type_name, method.method_name
                    ),
                    kind: PaletteEntryKind::Method,
                    title: method.method_name.clone(),
                    subtitle,
                    group_label: "Methods",
                    assembly_id: Some(assembly.id.clone()),
                    type_name: Some(method.type_name.clone()),
                    method_name: Some(method.method_name.clone()),
                    action: None,
                });
            }
        }
    }

    entries
}

fn normalize_query(query: &str) -> String {
    query.trim().to_ascii_lowercase()
}

fn matches_query(query: &str, fields: &[&str]) -> bool {
    fields
        .iter()
        .any(|field| field.to_ascii_lowercase().contains(query))
}

#[cfg(test)]
mod tests {
    use crate::types::{ActiveMode, AnalysisEntry, AnalysisResult, AnalysisStatus};

    use super::*;

    #[test]
    fn build_palette_entries_includes_actions_and_method_matches() {
        let assemblies = vec![OpenAssembly {
            id: "asm-1".to_string(),
            path: r"C:\samples\First.dll".to_string(),
            name: "First.dll".to_string(),
            loaded_at: 1,
        }];
        let mut analysis_entries = HashMap::new();
        analysis_entries.insert(
            "asm-1::explore".to_string(),
            AnalysisEntry {
                assembly_id: "asm-1".to_string(),
                assembly_path: r"C:\samples\First.dll".to_string(),
                mode: ActiveMode::Explore,
                status: AnalysisStatus::Done,
                result: Some(AnalysisResult {
                    assembly_path: r"C:\samples\First.dll".to_string(),
                    mode: "combined".to_string(),
                    explore: Some(crate::ipc::ExplorePayload {
                        assembly_path: r"C:\samples\First.dll".to_string(),
                        methods: vec![crate::ipc::MethodEntry {
                            type_name: "Example.Loader".to_string(),
                            method_name: "DownloadPayload".to_string(),
                            signature: "void DownloadPayload()".to_string(),
                            has_body: Some(true),
                            instructions: Vec::new(),
                            p_invoke: None,
                        }],
                        types: Vec::new(),
                    }),
                    scan: None,
                    stderr: String::new(),
                }),
                error: None,
                started_at: None,
                finished_at: None,
            },
        );

        let entries = build_palette_entries(&assemblies, &analysis_entries, false, "download");

        assert!(entries
            .iter()
            .any(|entry| entry.kind == PaletteEntryKind::Method));
        assert!(!entries
            .iter()
            .any(|entry| entry.kind == PaletteEntryKind::Action));

        let default_entries = build_palette_entries(&assemblies, &analysis_entries, true, "");
        assert!(default_entries
            .iter()
            .any(|entry| entry.kind == PaletteEntryKind::Action));
        assert!(default_entries
            .iter()
            .any(|entry| entry.action == Some(PaletteAction::OpenExportFolder)));
    }
}
