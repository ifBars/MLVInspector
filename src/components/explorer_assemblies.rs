use dioxus::prelude::*;

use crate::state::AppState;
use crate::types::OpenAssembly;

use super::theme::{
    C_ACCENT_GREEN, C_BORDER, C_TEXT_MUTED, C_TEXT_PRIMARY, C_TEXT_SECONDARY, FONT_MONO,
};
use super::view_models::IlTab;

#[component]
pub(crate) fn ExplorerAssemblySwitcher(
    assemblies: Vec<OpenAssembly>,
    selected_id: Option<String>,
    open_tabs: Signal<Vec<IlTab>>,
    active_tab_id: Signal<Option<String>>,
    selected_finding: Signal<Option<usize>>,
) -> Element {
    let state = use_context::<AppState>();
    let assemblies_count = assemblies.len();

    rsx! {
        div {
            style: format!("display: flex; flex-direction: column; border-bottom: 1px solid {C_BORDER};"),

            div {
                style: "display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 8px 12px 7px;",
                span {
                    style: format!(
                        "font-size: 10px; font-weight: 700; letter-spacing: 0.08em; color: {C_TEXT_MUTED}; text-transform: uppercase;"
                    ),
                    "Assemblies"
                }
                span {
                    style: format!(
                        "font-size: 9px; color: {C_TEXT_MUTED}; font-family: {FONT_MONO};"
                    ),
                    "{assemblies_count}"
                }
            }

            if assemblies.is_empty() {
                div {
                    style: "padding: 0 12px 12px;",
                    p {
                        style: format!("font-size: 11px; line-height: 1.5; color: {C_TEXT_MUTED};"),
                        "Open a .NET assembly to begin analysis."
                    }
                }
            } else {
                div {
                    style: "display: flex; flex-direction: column; max-height: 136px; overflow-y: auto;",
                    for asm in assemblies.iter() {
                        {
                            let asm_id = asm.id.clone();
                            let asm_id_select = asm_id.clone();
                            let asm_id_close = asm_id.clone();
                            let is_selected = selected_id.as_ref() == Some(&asm_id);
                            let assembly_name = asm.name.clone();
                            let assembly_path = asm.path.clone();
                            rsx! {
                                div {
                                    key: "assembly-row-{asm.id}",
                                    style: format!(
                                        "width: 100%; display: flex; align-items: flex-start; gap: 8px; padding: 9px 12px 10px; \
                                         border: none; border-top: 1px solid rgba(255,255,255,0.04); background: {}; \
                                         box-shadow: inset {} 0 0 {};",
                                        if is_selected { "rgba(255,255,255,0.05)" } else { "transparent" },
                                        if is_selected { "2px" } else { "0" },
                                        if is_selected { C_ACCENT_GREEN } else { "transparent" },
                                    ),

                                    button {
                                        style: "min-width: 0; flex: 1; display: flex; flex-direction: column; gap: 4px; border: none; background: transparent; text-align: left; cursor: pointer; padding: 0;",
                                        onclick: move |_| {
                                            state.select_assembly(asm_id_select.clone());
                                            open_tabs.write().clear();
                                            active_tab_id.set(None);
                                            selected_finding.set(None);
                                        },

                                        div {
                                            style: "display: flex; align-items: center; gap: 8px; min-width: 0;",
                                            div {
                                                style: format!(
                                                    "width: 6px; height: 6px; border-radius: 50%; flex-shrink: 0; background: {};",
                                                    if is_selected { C_ACCENT_GREEN } else { C_TEXT_MUTED }
                                                )
                                            }
                                            span {
                                                style: format!(
                                                    "min-width: 0; flex: 1; font-size: 12px; font-weight: 600; \
                                                     color: {}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                                    if is_selected { C_TEXT_PRIMARY } else { C_TEXT_SECONDARY }
                                                ),
                                                "{assembly_name}"
                                            }
                                        }

                                        div {
                                            style: format!(
                                                "padding-left: 14px; font-size: 9px; line-height: 1.4; color: {C_TEXT_MUTED}; \
                                                 font-family: {FONT_MONO}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                                            ),
                                            "{assembly_path}"
                                        }
                                    }

                                    button {
                                        aria_label: "Close assembly",
                                        style: format!(
                                            "flex-shrink: 0; width: 18px; height: 18px; border-radius: 4px; border: none; \
                                             background: transparent; color: {C_TEXT_MUTED}; cursor: pointer; display: flex; \
                                             align-items: center; justify-content: center; padding: 0; margin-top: 1px;"
                                        ),
                                        onclick: move |_| {
                                            state.close_assembly(asm_id_close.clone());
                                            if is_selected {
                                                open_tabs.write().clear();
                                                active_tab_id.set(None);
                                                selected_finding.set(None);
                                            }
                                        },
                                        svg {
                                            width: "10",
                                            height: "10",
                                            view_box: "0 0 24 24",
                                            fill: "none",
                                            stroke: "currentColor",
                                            stroke_width: "2.2",
                                            line { x1: "18", y1: "6", x2: "6", y2: "18" }
                                            line { x1: "6", y1: "6", x2: "18", y2: "18" }
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
