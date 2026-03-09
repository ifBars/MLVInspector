use dioxus::prelude::*;

use crate::services::export_project::{export_project_bundle, open_in_file_explorer};
use crate::shortcuts::{ShortcutBinding, ShortcutKey, ShortcutSettings};
use crate::state::AppState;

use super::analysis::run_analysis;
use super::overlay::OverlayKind;
use super::view_models::IlTab;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandId {
    OpenCommandPalette,
    OpenSettings,
    CloseOverlay,
    ToggleFindings,
    ClearWorkspace,
    OpenAssembly,
    ExportProject,
    OpenExportFolder,
    RunAnalysis,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandDefinition {
    pub id: CommandId,
    pub title: &'static str,
    pub description: &'static str,
    pub default_shortcut: ShortcutBinding,
    pub show_in_palette: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteCommandItem {
    pub command_id: CommandId,
    pub title: &'static str,
    pub description: &'static str,
    pub shortcut_label: Option<String>,
}

#[derive(Clone, Copy)]
pub struct CommandContext {
    pub state: AppState,
    pub active_overlay: Signal<Option<OverlayKind>>,
    pub show_scan_panel: Signal<bool>,
    pub last_error: Signal<String>,
    pub open_tabs: Signal<Vec<IlTab>>,
    pub active_tab_id: Signal<Option<String>>,
    pub selected_finding: Signal<Option<usize>>,
}

pub fn command_definitions() -> &'static [CommandDefinition] {
    use CommandId::*;

    static DEFINITIONS: std::sync::OnceLock<Vec<CommandDefinition>> = std::sync::OnceLock::new();

    DEFINITIONS.get_or_init(|| {
        vec![
            CommandDefinition {
                id: OpenCommandPalette,
                title: "Open command palette",
                description: "Jump to assemblies, types, methods, and app actions",
                default_shortcut: ShortcutBinding::new(ShortcutKey::Character("k".to_string()))
                    .with_ctrl(),
                show_in_palette: false,
            },
            CommandDefinition {
                id: OpenSettings,
                title: "Open settings",
                description: "Review and customize keyboard shortcuts",
                default_shortcut: ShortcutBinding::new(ShortcutKey::Character(",".to_string()))
                    .with_ctrl(),
                show_in_palette: true,
            },
            CommandDefinition {
                id: CloseOverlay,
                title: "Close overlay",
                description: "Dismiss the active command palette or settings dialog",
                default_shortcut: ShortcutBinding::new(ShortcutKey::Escape),
                show_in_palette: false,
            },
            CommandDefinition {
                id: ToggleFindings,
                title: "Toggle findings panel",
                description: "Show or hide the scan findings sidebar",
                default_shortcut: ShortcutBinding::new(ShortcutKey::Character("f".to_string()))
                    .with_ctrl()
                    .with_shift(),
                show_in_palette: true,
            },
            CommandDefinition {
                id: ClearWorkspace,
                title: "Clear workspace",
                description: "Remove all open assemblies and analysis results",
                default_shortcut: ShortcutBinding::new(ShortcutKey::Character("w".to_string()))
                    .with_ctrl()
                    .with_shift(),
                show_in_palette: true,
            },
            CommandDefinition {
                id: OpenAssembly,
                title: "Open assembly",
                description: "Choose a DLL or EXE file and analyze it",
                default_shortcut: ShortcutBinding::new(ShortcutKey::Character("o".to_string()))
                    .with_ctrl(),
                show_in_palette: true,
            },
            CommandDefinition {
                id: ExportProject,
                title: "Export project",
                description: "Write a portable bundle with decompiled source and analysis JSON",
                default_shortcut: ShortcutBinding::new(ShortcutKey::Character("e".to_string()))
                    .with_ctrl(),
                show_in_palette: true,
            },
            CommandDefinition {
                id: OpenExportFolder,
                title: "Open export folder",
                description: "Reveal the most recently exported decompiled project",
                default_shortcut: ShortcutBinding::new(ShortcutKey::Character("o".to_string()))
                    .with_ctrl()
                    .with_shift(),
                show_in_palette: true,
            },
            CommandDefinition {
                id: RunAnalysis,
                title: "Run analysis",
                description: "Re-run explore and scan for the selected assembly",
                default_shortcut: ShortcutBinding::new(ShortcutKey::Character("r".to_string()))
                    .with_ctrl()
                    .with_shift(),
                show_in_palette: true,
            },
        ]
    })
}

pub fn palette_command_items(
    state: AppState,
    active_overlay: Option<OverlayKind>,
    shortcuts: &ShortcutSettings,
) -> Vec<PaletteCommandItem> {
    command_definitions()
        .iter()
        .filter(|definition| definition.show_in_palette)
        .filter(|definition| is_command_enabled(state, active_overlay, definition.id))
        .map(|definition| PaletteCommandItem {
            command_id: definition.id,
            title: definition.title,
            description: definition.description,
            shortcut_label: shortcuts
                .binding_for(definition.id)
                .map(ShortcutBinding::display_label),
        })
        .collect()
}

pub fn command_definition(command_id: CommandId) -> &'static CommandDefinition {
    command_definitions()
        .iter()
        .find(|definition| definition.id == command_id)
        .expect("command definition should exist")
}

pub fn is_command_enabled(
    state: AppState,
    active_overlay: Option<OverlayKind>,
    command_id: CommandId,
) -> bool {
    match command_id {
        CommandId::OpenCommandPalette | CommandId::OpenSettings => true,
        CommandId::CloseOverlay => active_overlay.is_some(),
        CommandId::ToggleFindings => true,
        CommandId::ClearWorkspace => !state.assemblies.read().is_empty(),
        CommandId::OpenAssembly => true,
        CommandId::ExportProject | CommandId::RunAnalysis => selected_assembly(state).is_some(),
        CommandId::OpenExportFolder => state.last_export_path.read().is_some(),
    }
}

pub fn dispatch_shortcut_binding(
    context: CommandContext,
    shortcuts: &ShortcutSettings,
    binding: &ShortcutBinding,
) -> bool {
    let active_overlay = (context.active_overlay)();
    let Some(command_id) = shortcuts.command_for_binding(binding) else {
        return false;
    };

    if !is_command_enabled(context.state, active_overlay, command_id) {
        return false;
    }

    if !shortcut_is_allowed_in_overlay(active_overlay, command_id) {
        return false;
    }

    execute_command(context, command_id);
    true
}

fn shortcut_is_allowed_in_overlay(active_overlay: Option<OverlayKind>, command_id: CommandId) -> bool {
    !matches!(active_overlay, Some(OverlayKind::Settings)) || command_id == CommandId::CloseOverlay
}

pub fn execute_command(mut context: CommandContext, command_id: CommandId) {
    if !is_command_enabled(context.state, (context.active_overlay)(), command_id) {
        return;
    }

    match command_id {
        CommandId::OpenCommandPalette => {
            context
                .active_overlay
                .set(Some(OverlayKind::CommandPalette));
        }
        CommandId::OpenSettings => {
            context.active_overlay.set(Some(OverlayKind::Settings));
        }
        CommandId::CloseOverlay => {
            context.active_overlay.set(None);
        }
        CommandId::ToggleFindings => {
            context.show_scan_panel.toggle();
        }
        CommandId::ClearWorkspace => {
            context.state.clear_all();
            context.open_tabs.write().clear();
            context.active_tab_id.set(None);
            context.selected_finding.set(None);
            context.last_error.set(String::new());
        }
        CommandId::OpenAssembly => {
            if let Some(path) = rfd::FileDialog::new().pick_file() {
                let file_path = path.display().to_string();
                context.state.open_assembly(file_path.clone());
                context.open_tabs.write().clear();
                context.active_tab_id.set(None);
                context.selected_finding.set(None);

                if let Some(assembly_id) = context.state.selected_id.read().clone() {
                    run_analysis(context.state, context.last_error, assembly_id, file_path);
                }
            }
        }
        CommandId::ExportProject => {
            let Some(assembly) = selected_assembly(context.state) else {
                context
                    .last_error
                    .set("Select an assembly before exporting".to_string());
                return;
            };

            let Some(folder) = rfd::FileDialog::new()
                .set_title("Export Project Bundle")
                .pick_folder()
            else {
                return;
            };

            let worker = context.state.worker.read().clone();
            let analysis = context
                .state
                .get_analysis_entry(&format!("{}::explore", assembly.id))
                .and_then(|entry| entry.result);
            let mut last_error = context.last_error;
            let state = context.state;

            spawn(async move {
                match export_project_bundle(worker, assembly, analysis, folder).await {
                    Ok(path) => {
                        state.set_last_export_path(Some(path.display().to_string()));
                        last_error.set(String::new());
                        tracing::info!(path = %path.display(), "exported project bundle");
                    }
                    Err(err) => last_error.set(err.to_string()),
                }
            });
        }
        CommandId::OpenExportFolder => {
            let Some(path) = context.state.last_export_path.read().clone() else {
                context
                    .last_error
                    .set("No export folder is available yet".to_string());
                return;
            };

            if let Err(err) = open_in_file_explorer(&path) {
                context.last_error.set(err.to_string());
            }
        }
        CommandId::RunAnalysis => {
            if let Some(assembly) = selected_assembly(context.state) {
                run_analysis(
                    context.state,
                    context.last_error,
                    assembly.id,
                    assembly.path,
                );
            } else {
                context
                    .last_error
                    .set("Select an assembly before running analysis".to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_findings_default_shortcut_resolves_to_the_command() {
        let binding = ShortcutBinding::new(ShortcutKey::Character("f".to_string()))
            .with_ctrl()
            .with_shift();
        let shortcuts = ShortcutSettings::with_defaults();

        assert_eq!(
            shortcuts.command_for_binding(&binding),
            Some(CommandId::ToggleFindings)
        );
    }

    #[test]
    fn settings_overlay_blocks_non_close_shortcuts() {
        assert!(!shortcut_is_allowed_in_overlay(
            Some(OverlayKind::Settings),
            CommandId::ToggleFindings
        ));
    }

    #[test]
    fn settings_overlay_allows_close_shortcut() {
        assert!(shortcut_is_allowed_in_overlay(
            Some(OverlayKind::Settings),
            CommandId::CloseOverlay
        ));
    }
}

fn selected_assembly(state: AppState) -> Option<crate::types::OpenAssembly> {
    let selected_id = state.selected_id.read().clone()?;
    state
        .assemblies
        .read()
        .iter()
        .find(|assembly| assembly.id == selected_id)
        .cloned()
}
