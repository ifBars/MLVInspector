use std::collections::HashMap;

use super::commands::{CommandId, PaletteCommandItem};
use crate::types::{AnalysisEntry, OpenAssembly};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PaletteEntryKind {
    Action,
    Assembly,
    Type,
    Method,
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
    pub command_id: Option<CommandId>,
    pub shortcut_label: Option<String>,
}

pub fn build_palette_entries(
    assemblies: &[OpenAssembly],
    analysis_entries: &HashMap<String, AnalysisEntry>,
    action_commands: &[PaletteCommandItem],
    query: &str,
) -> Vec<PaletteEntry> {
    let normalized = normalize_query(query);
    let mut entries = Vec::new();

    for command in action_commands {
        if normalized.is_empty()
            || matches_query(&normalized, &[command.title, command.description])
        {
            entries.push(PaletteEntry {
                id: format!("action::{:?}", command.command_id),
                kind: PaletteEntryKind::Action,
                title: command.title.to_string(),
                subtitle: command.description.to_string(),
                group_label: "Actions",
                assembly_id: None,
                type_name: None,
                method_name: None,
                command_id: Some(command.command_id),
                shortcut_label: command.shortcut_label.clone(),
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
                command_id: None,
                shortcut_label: None,
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
                    command_id: None,
                    shortcut_label: None,
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
                    command_id: None,
                    shortcut_label: None,
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

        let entries = build_palette_entries(&assemblies, &analysis_entries, &[], "download");

        assert!(entries
            .iter()
            .any(|entry| entry.kind == PaletteEntryKind::Method));
        assert!(!entries
            .iter()
            .any(|entry| entry.kind == PaletteEntryKind::Action));

        let default_entries = build_palette_entries(
            &assemblies,
            &analysis_entries,
            &[PaletteCommandItem {
                command_id: CommandId::OpenExportFolder,
                title: "Open export folder",
                description: "Reveal the most recently exported decompiled project",
                shortcut_label: Some("Ctrl+Shift+O".to_string()),
            }],
            "",
        );
        assert!(default_entries
            .iter()
            .any(|entry| entry.kind == PaletteEntryKind::Action));
        assert!(default_entries
            .iter()
            .any(|entry| entry.command_id == Some(CommandId::OpenExportFolder)));
    }
}
