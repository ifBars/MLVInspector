/// Bottom status bar showing workspace metrics and error messages.
use dioxus::prelude::*;

use crate::state::AppState;

use super::theme::{C_ACCENT_AMBER, C_ACCENT_BLUE, C_TEXT_MUTED, C_TEXT_SECONDARY, FONT_MONO};

#[component]
pub fn StatusBar(
    show_scan_panel: Signal<bool>,
    last_error: Signal<String>,
    findings_count: usize,
) -> Element {
    let state = use_context::<AppState>();
    let assemblies_count = state.assemblies.read().len();
    let rules_count = state.rules.read().len();

    rsx! {
        div {
            style: format!(
                "height: 26px; flex-shrink: 0; display: flex; align-items: center; \
                 justify-content: space-between; padding: 0 14px; \
                 background: #17191c; border-top: 1px solid #2d3138; \
                 font-size: 10px; font-family: {FONT_MONO};"
            ),

            // Left: workspace metrics
            div {
                style: "display: flex; align-items: center; gap: 16px;",

                span {
                    style: format!("color: {C_TEXT_MUTED};"),
                    "findings panel: "
                    span {
                        style: format!("color: {C_ACCENT_BLUE}; font-weight: 600;"),
                        if show_scan_panel() { "visible" } else { "hidden" }
                    }
                }
                span {
                    style: format!("color: {C_TEXT_MUTED};"),
                    "assemblies: "
                    span {
                        style: format!("color: {C_TEXT_SECONDARY};"),
                        "{assemblies_count}"
                    }
                }
                span {
                    style: format!("color: {C_TEXT_MUTED};"),
                    "rules: "
                    span {
                        style: format!("color: {C_TEXT_SECONDARY};"),
                        "{rules_count}"
                    }
                }
                span {
                    style: format!("color: {C_TEXT_MUTED};"),
                    "findings: "
                    span {
                        style: if findings_count > 0 {
                            format!("color: {C_ACCENT_AMBER}; font-weight: 600;")
                        } else {
                            format!("color: {C_TEXT_SECONDARY};")
                        },
                        "{findings_count}"
                    }
                }
            }

            // Right: error display or version
            if !last_error.read().is_empty() {
                div {
                    style: "display: flex; align-items: center; gap: 6px; color: #caa0a6;",
                    svg {
                        width: "10", height: "10", view_box: "0 0 24 24",
                        fill: "none", stroke: "currentColor", stroke_width: "2.5",
                        circle { cx: "12", cy: "12", r: "10" }
                        line { x1: "12", y1: "8", x2: "12", y2: "12" }
                        line { x1: "12", y1: "16", x2: "12.01", y2: "16" }
                    }
                    span {
                        style: "max-width: 400px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                        "{last_error.read()}"
                    }
                }
            } else {
                span {
                    style: format!("color: {C_TEXT_MUTED};"),
                    "MLVInspector"
                }
            }
        }
    }
}
