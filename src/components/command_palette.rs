use dioxus::prelude::*;

use crate::shortcuts::ShortcutSettings;
use crate::state::AppState;

use super::commands::{execute_command, palette_command_items, CommandContext};
use super::explorer_tools::{build_palette_entries, PaletteEntry, PaletteEntryKind};
use super::helpers::{method_tab_id, type_tab_id};
use super::overlay::OverlayKind;
use super::theme::{C_TEXT_MUTED, C_TEXT_PRIMARY, FONT_MONO};
use super::view_models::{IlTab, IlTabKind};

#[component]
pub fn CommandPalette(
    active_overlay: Signal<Option<OverlayKind>>,
    shortcut_settings: Signal<ShortcutSettings>,
    show_scan_panel: Signal<bool>,
    last_error: Signal<String>,
    open_tabs: Signal<Vec<IlTab>>,
    active_tab_id: Signal<Option<String>>,
    selected_finding: Signal<Option<usize>>,
) -> Element {
    let state = use_context::<AppState>();
    let mut query = use_signal(String::new);
    let is_open = active_overlay() == Some(OverlayKind::CommandPalette);

    use_effect(move || {
        if active_overlay() != Some(OverlayKind::CommandPalette) {
            if !query().is_empty() {
                query.set(String::new());
            }
            return;
        }

        spawn(async move {
            let _ = document::eval(
                "setTimeout(() => document.getElementById('command-palette-input')?.focus(), 0);",
            )
            .await;
        });
    });

    let assemblies = state.assemblies.read().clone();
    let analysis_entries = state.analysis_entries.read().clone();
    let action_commands = palette_command_items(state, active_overlay(), &shortcut_settings());
    let entries = if is_open {
        build_palette_entries(&assemblies, &analysis_entries, &action_commands, &query())
    } else {
        Vec::new()
    };

    if !is_open {
        return rsx! {};
    }

    rsx! {
        div {
            class: "command-palette-overlay",
            onclick: move |_| {
                active_overlay.set(None);
                query.set(String::new());
            },

            div {
                class: "command-palette no-drag",
                onclick: move |evt| evt.stop_propagation(),

                div {
                    class: "command-palette-search",
                    svg {
                        width: "14", height: "14", view_box: "0 0 24 24",
                        fill: "none", stroke: C_TEXT_MUTED, stroke_width: "2",
                        circle { cx: "11", cy: "11", r: "7" }
                        line { x1: "20", y1: "20", x2: "16.65", y2: "16.65" }
                    }
                    input {
                        id: "command-palette-input",
                        value: "{query()}",
                        placeholder: "Search assemblies, types, methods, and actions",
                        style: format!(
                            "flex: 1; min-width: 0; border: none; outline: none; background: transparent; color: {C_TEXT_PRIMARY}; font-size: 13px;"
                        ),
                        oninput: move |evt| query.set(evt.value()),
                    }
                    button {
                        class: "command-palette-close",
                        onclick: move |_| {
                            active_overlay.set(None);
                            query.set(String::new());
                        },
                        "Close"
                    }
                }

                div {
                    class: "command-palette-results",
                    if entries.is_empty() {
                        div {
                            class: "empty-state",
                            style: "height: auto; min-height: 240px;",
                            p { "No results match the current search" }
                        }
                    } else {
                        for group_label in ["Actions", "Assemblies", "Types", "Methods"] {
                            {
                                let group_entries = entries
                                    .iter()
                                    .filter(|entry| entry.group_label == group_label)
                                    .cloned()
                                    .collect::<Vec<_>>();
                                rsx! {
                                    if !group_entries.is_empty() {
                                        div {
                                            key: "palette-group-{group_label}",
                                            class: "command-palette-group",
                                            div {
                                                class: "command-palette-group-label",
                                                "{group_label}"
                                            }
                                            for entry in group_entries {
                                                {
                                                    let palette_entry = entry.clone();
                                                    let click_entry = palette_entry.clone();
                                                    rsx! {
                                                        button {
                                                            key: "{palette_entry.id}",
                                                            class: "command-palette-item",
                                                            onclick: move |_| {
                                                                handle_palette_entry(
                                                                    state,
                                                                    active_overlay,
                                                                    query,
                                                                    show_scan_panel,
                                                                    last_error,
                                                                    open_tabs,
                                                                    active_tab_id,
                                                                    selected_finding,
                                                                    click_entry.clone(),
                                                                );
                                                            },
                                                            div {
                                                                style: "display: grid; gap: 3px; min-width: 0;",
                                                                div {
                                                                    style: format!(
                                                                        "font-size: 12px; font-weight: 600; color: {C_TEXT_PRIMARY}; font-family: {}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                                                        if palette_entry.kind == PaletteEntryKind::Action {
                                                                            "inherit"
                                                                        } else {
                                                                            FONT_MONO
                                                                        }
                                                                    ),
                                                                    "{palette_entry.title}"
                                                                }
                                                                div {
                                                                    style: format!(
                                                                        "font-size: 10px; color: {C_TEXT_MUTED}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                                                                    ),
                                                                    "{palette_entry.subtitle}"
                                                                }
                                                            }
                                                            if let Some(shortcut_label) = palette_entry.shortcut_label.clone() {
                                                                div {
                                                                    class: "shortcut-badge",
                                                                    "{shortcut_label}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div {
                    class: "command-palette-footer",
                    span { "Toolbar search" }
                    span { "Open an assembly result to switch context quickly" }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_palette_entry(
    state: AppState,
    mut active_overlay: Signal<Option<OverlayKind>>,
    mut query: Signal<String>,
    show_scan_panel: Signal<bool>,
    last_error: Signal<String>,
    mut open_tabs: Signal<Vec<IlTab>>,
    mut active_tab_id: Signal<Option<String>>,
    mut selected_finding: Signal<Option<usize>>,
    entry: PaletteEntry,
) {
    match entry.command_id {
        Some(command_id) => {
            execute_command(
                CommandContext {
                    state,
                    active_overlay,
                    show_scan_panel,
                    last_error,
                    open_tabs,
                    active_tab_id,
                    selected_finding,
                },
                command_id,
            );

            if active_overlay() == Some(OverlayKind::CommandPalette) {
                active_overlay.set(None);
            }
            query.set(String::new());
            return;
        }
        None => {
            if let Some(assembly_id) = entry.assembly_id.clone() {
                state.select_assembly(assembly_id);
            }

            match entry.kind {
                PaletteEntryKind::Assembly => {
                    open_tabs.write().clear();
                    active_tab_id.set(None);
                    selected_finding.set(None);
                }
                PaletteEntryKind::Type => {
                    let Some(type_name) = entry.type_name.clone() else {
                        return;
                    };
                    let tab_id = type_tab_id(&type_name);
                    let display_name = type_name
                        .rsplit('.')
                        .next()
                        .unwrap_or(&type_name)
                        .to_string();
                    {
                        let mut tabs = open_tabs.write();
                        if !tabs.iter().any(|tab| tab.id == tab_id) {
                            tabs.push(IlTab {
                                id: tab_id.clone(),
                                kind: IlTabKind::Type,
                                type_name: type_name.clone(),
                                method_name: None,
                                title: display_name,
                                subtitle: type_name.clone(),
                            });
                        }
                    }
                    active_tab_id.set(Some(tab_id));
                    selected_finding.set(None);
                }
                PaletteEntryKind::Method => {
                    let (Some(type_name), Some(method_name)) =
                        (entry.type_name.clone(), entry.method_name.clone())
                    else {
                        return;
                    };
                    let tab_id = method_tab_id(&type_name, &method_name);
                    {
                        let mut tabs = open_tabs.write();
                        if !tabs.iter().any(|tab| tab.id == tab_id) {
                            tabs.push(IlTab {
                                id: tab_id.clone(),
                                kind: IlTabKind::Method,
                                type_name: type_name.clone(),
                                method_name: Some(method_name.clone()),
                                title: method_name,
                                subtitle: type_name,
                            });
                        }
                    }
                    active_tab_id.set(Some(tab_id));
                    selected_finding.set(None);
                }
                PaletteEntryKind::Action => {}
            }
        }
    }

    active_overlay.set(None);
    query.set(String::new());
}
