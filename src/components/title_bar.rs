/// Custom title bar with drag region, toolbar, and window controls.
use dioxus::prelude::*;
use dioxus_desktop::window;

use crate::state::AppState;

use super::analysis::run_analysis;
use super::theme::{C_ACCENT_GREEN, C_BORDER, C_TEXT_MUTED, C_TEXT_PRIMARY, FONT_MONO};
use super::view_models::IlTab;

#[component]
pub fn TitleBar(
    show_scan_panel: Signal<bool>,
    last_error: Signal<String>,
    open_tabs: Signal<Vec<IlTab>>,
    active_tab_id: Signal<Option<String>>,
    highlighted_il_offset: Signal<Option<i64>>,
) -> Element {
    let state = use_context::<AppState>();
    let desktop_window = window();
    let mut is_fullscreen = use_signal(|| false);
    let is_running = *state.is_running.read();

    let desktop_window_min = desktop_window.clone();
    let desktop_window_full = desktop_window.clone();
    let desktop_window_close = desktop_window.clone();

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

                // Traffic-light dot
                div {
                    style: format!(
                        "width: 9px; height: 9px; border-radius: 50%; \
                         background: {C_ACCENT_GREEN}; \
                         border: 1px solid {C_BORDER};"
                    )
                }
                span {
                    style: format!(
                        "font-size: 12px; font-weight: 600; letter-spacing: 0.3px; \
                         color: {C_TEXT_PRIMARY};"
                    ),
                    "MLVInspector"
                }
                span {
                    style: format!(
                        "font-size: 11px; color: {C_TEXT_MUTED}; \
                         font-family: {FONT_MONO};"
                    ),
                    ".Dioxus"
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
                        class: if show_scan_panel() { "tool-btn active" } else { "tool-btn" },
                        title: if show_scan_panel() { "Hide Findings" } else { "Show Findings" },
                        "aria-label": if show_scan_panel() { "Hide Findings" } else { "Show Findings" },
                        onclick: move |_| show_scan_panel.toggle(),
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
                        onclick: move |_| {
                            state.clear_all();
                            open_tabs.write().clear();
                            active_tab_id.set(None);
                            highlighted_il_offset.set(None);
                        },
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
                        onclick: move |_| {
                            if let Some(path) = rfd::FileDialog::new().pick_file() {
                                let file_path = path.display().to_string();
                                state.open_assembly(file_path.clone());
                                open_tabs.write().clear();
                                active_tab_id.set(None);
                                highlighted_il_offset.set(None);
                                let id = state.selected_id.read().clone();
                                if let Some(assembly_id) = id {
                                    run_analysis(state, last_error, assembly_id, file_path);
                                }
                            }
                        },
                        svg {
                            width: "13", height: "13", view_box: "0 0 24 24", fill: "none",
                            stroke: "currentColor", stroke_width: "2",
                            path { d: "M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" }
                            polyline { points: "17 8 12 3 7 8" }
                            line { x1: "12", y1: "3", x2: "12", y2: "15" }
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
