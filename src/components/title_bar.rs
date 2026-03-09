/// Custom title bar with drag region, toolbar, and window controls.
use dioxus::prelude::*;
use dioxus_desktop::window;

use crate::state::AppState;

use super::commands::{execute_command, CommandContext, CommandId};
use super::overlay::OverlayKind;
use super::theme::{C_ACCENT_GREEN, C_BORDER, C_TEXT_PRIMARY};
use super::view_models::IlTab;

const APP_ICON: Asset = asset!("/assets/icon.png");

#[component]
pub fn TitleBar(
    active_overlay: Signal<Option<OverlayKind>>,
    show_scan_panel: Signal<bool>,
    last_error: Signal<String>,
    open_tabs: Signal<Vec<IlTab>>,
    active_tab_id: Signal<Option<String>>,
    selected_finding: Signal<Option<usize>>,
) -> Element {
    let state = use_context::<AppState>();
    let desktop_window = window();
    let mut is_fullscreen = use_signal(|| false);
    let is_running = *state.is_running.read();

    let desktop_window_min = desktop_window.clone();
    let desktop_window_full = desktop_window.clone();
    let desktop_window_close = desktop_window.clone();
    let can_export = state.selected_id.read().is_some();
    let last_export_path = state.last_export_path.read().clone();
    let can_open_export = last_export_path.is_some();

    let command_context = CommandContext {
        state,
        active_overlay,
        show_scan_panel,
        last_error,
        open_tabs,
        active_tab_id,
        selected_finding,
    };

    rsx! {
        div {
            class: "drag-region",
            style: format!(
                "height: 42px; flex-shrink: 0; display: flex; align-items: center; \
                 justify-content: space-between; padding: 0 14px; \
                 background: #17191c; \
                 border-bottom: 1px solid {C_BORDER};"
            ),

            // Left: identity + toolbar
            div {
                style: "display: flex; align-items: center; gap: 10px;",

                img {
                    src: APP_ICON,
                    alt: "MLVInspector icon",
                    width: "28",
                    height: "28",
                    style: "width: 28px; height: 28px; object-fit: contain; display: block;"
                }
                span {
                    style: format!(
                        "font-size: 12px; font-weight: 600; letter-spacing: 0.3px; \
                         color: {C_TEXT_PRIMARY};"
                    ),
                    "MLVInspector"
                }
                if is_running {
                    span {
                        class: "pulse",
                        style: format!(
                            "margin-left: 6px; font-size: 10px; font-weight: 600; \
                             color: {C_ACCENT_GREEN}; letter-spacing: 0.5px;"
                        ),
                        "ANALYZING"
                    }
                }

                div {
                    style: format!(
                        "width: 1px; height: 18px; background: {C_BORDER}; margin: 0 2px;"
                    )
                }

                div {
                    class: "no-drag toolbar",

                    // Toggle findings panel
                    button {
                        class: if active_overlay() == Some(OverlayKind::CommandPalette) {
                            "tool-btn active"
                        } else {
                            "tool-btn"
                        },
                        title: "Search Tools",
                        "aria-label": "Search Tools",
                        onclick: move |_| execute_command(command_context, CommandId::OpenCommandPalette),
                        svg {
                            width: "13", height: "13", view_box: "0 0 24 24", fill: "none",
                            stroke: "currentColor", stroke_width: "2",
                            circle { cx: "11", cy: "11", r: "7" }
                            line { x1: "20", y1: "20", x2: "16.65", y2: "16.65" }
                        }
                    }

                    button {
                        class: if active_overlay() == Some(OverlayKind::Settings) {
                            "tool-btn active"
                        } else {
                            "tool-btn"
                        },
                        title: "Settings",
                        "aria-label": "Settings",
                        onclick: move |_| execute_command(command_context, CommandId::OpenSettings),
                        svg {
                            width: "13", height: "13", view_box: "0 0 24 24", fill: "none",
                            stroke: "currentColor", stroke_width: "2",
                            circle { cx: "12", cy: "12", r: "3" }
                            path { d: "M19.4 15a1.7 1.7 0 0 0 .34 1.87l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.7 1.7 0 0 0-1.87-.34 1.7 1.7 0 0 0-1 1.55V21a2 2 0 1 1-4 0v-.09a1.7 1.7 0 0 0-1-1.55 1.7 1.7 0 0 0-1.87.34l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.7 1.7 0 0 0 .34-1.87 1.7 1.7 0 0 0-1.55-1H3a2 2 0 1 1 0-4h.09a1.7 1.7 0 0 0 1.55-1 1.7 1.7 0 0 0-.34-1.87l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.7 1.7 0 0 0 1.87.34h.1A1.7 1.7 0 0 0 10.09 3H10a2 2 0 1 1 4 0v.09a1.7 1.7 0 0 0 1 1.55 1.7 1.7 0 0 0 1.87-.34l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.7 1.7 0 0 0-.34 1.87v.1A1.7 1.7 0 0 0 21 10.09V10a2 2 0 1 1 0 4h-.09a1.7 1.7 0 0 0-1.55 1z" }
                        }
                    }

                    button {
                        class: if show_scan_panel() { "tool-btn active" } else { "tool-btn" },
                        title: if show_scan_panel() { "Hide Findings" } else { "Show Findings" },
                        "aria-label": if show_scan_panel() { "Hide Findings" } else { "Show Findings" },
                        onclick: move |_| execute_command(command_context, CommandId::ToggleFindings),
                        svg {
                            width: "13", height: "13", view_box: "0 0 24 24", fill: "none",
                            stroke: "currentColor", stroke_width: "2",
                            rect { x: "3", y: "4", width: "18", height: "16", rx: "2", ry: "2" }
                            line { x1: "15", y1: "4", x2: "15", y2: "20" }
                        }
                    }

                    // Clear workspace
                    button {
                        class: "tool-btn",
                        title: "Clear Workspace",
                        "aria-label": "Clear Workspace",
                        onclick: move |_| execute_command(command_context, CommandId::ClearWorkspace),
                        svg {
                            width: "13", height: "13", view_box: "0 0 24 24", fill: "none",
                            stroke: "currentColor", stroke_width: "2",
                            polyline { points: "3 6 5 6 21 6" }
                            path { d: "M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6" }
                        }
                    }

                    // Open assembly file picker
                    button {
                        class: "tool-btn",
                        title: "Open Assembly",
                        "aria-label": "Open Assembly",
                        onclick: move |_| execute_command(command_context, CommandId::OpenAssembly),
                        svg {
                            width: "13", height: "13", view_box: "0 0 24 24", fill: "none",
                            stroke: "currentColor", stroke_width: "2",
                            path { d: "M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" }
                            polyline { points: "17 8 12 3 7 8" }
                            line { x1: "12", y1: "3", x2: "12", y2: "15" }
                        }
                    }

                    button {
                        class: if can_export { "tool-btn" } else { "tool-btn disabled" },
                        title: "Export Project",
                        "aria-label": "Export Project",
                        disabled: !can_export,
                        onclick: move |_| execute_command(command_context, CommandId::ExportProject),
                        svg {
                            width: "13", height: "13", view_box: "0 0 24 24", fill: "none",
                            stroke: "currentColor", stroke_width: "2",
                            path { d: "M12 3v12" }
                            polyline { points: "8 11 12 15 16 11" }
                            path { d: "M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" }
                        }
                    }

                    button {
                        class: if can_open_export { "tool-btn" } else { "tool-btn disabled" },
                        title: "Open Export Folder",
                        "aria-label": "Open Export Folder",
                        disabled: !can_open_export,
                        onclick: move |_| execute_command(command_context, CommandId::OpenExportFolder),
                        svg {
                            width: "13", height: "13", view_box: "0 0 24 24", fill: "none",
                            stroke: "currentColor", stroke_width: "2",
                            path { d: "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" }
                            polyline { points: "12 11 15 14 21 8" }
                        }
                    }
                }
            }

            // Right: window controls
            div {
                class: "no-drag",
                style: "display: flex; align-items: center; gap: 6px;",

                button {
                    class: "btn btn-ghost",
                    style: "width: 30px; height: 26px; padding: 0; justify-content: center;",
                    title: "Minimize",
                    onclick: move |_| desktop_window_min.set_minimized(true),
                    svg {
                        width: "10", height: "10", view_box: "0 0 24 24", fill: "none",
                        stroke: "currentColor", stroke_width: "2",
                        line { x1: "5", y1: "12", x2: "19", y2: "12" }
                    }
                }

                button {
                    class: "btn btn-ghost",
                    style: "width: 30px; height: 26px; padding: 0; justify-content: center;",
                    title: "Toggle Fullscreen",
                    onclick: move |_| {
                        let next = !is_fullscreen();
                        is_fullscreen.set(next);
                        desktop_window_full.set_fullscreen(next);
                    },
                    if is_fullscreen() {
                        svg {
                            width: "10", height: "10", view_box: "0 0 24 24", fill: "none",
                            stroke: "currentColor", stroke_width: "2",
                            rect { x: "6", y: "6", width: "12", height: "12", rx: "1", ry: "1" }
                        }
                    } else {
                        svg {
                            width: "10", height: "10", view_box: "0 0 24 24", fill: "none",
                            stroke: "currentColor", stroke_width: "2",
                            rect { x: "4", y: "4", width: "16", height: "16", rx: "1", ry: "1" }
                        }
                    }
                }

                button {
                    class: "btn btn-danger",
                    style: "width: 30px; height: 26px; padding: 0; justify-content: center;",
                    title: "Close",
                    onclick: move |_| desktop_window_close.close(),
                    svg {
                        width: "10", height: "10", view_box: "0 0 24 24", fill: "none",
                        stroke: "currentColor", stroke_width: "2",
                        line { x1: "18", y1: "6", x2: "6", y2: "18" }
                        line { x1: "6", y1: "6", x2: "18", y2: "18" }
                    }
                }
            }
        }
    }
}
