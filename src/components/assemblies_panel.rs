/// Left panel listing open assemblies with select and close actions.
use dioxus::prelude::*;

use crate::state::AppState;

use super::theme::{
    C_ACCENT_BLUE, C_BG_SURFACE, C_TEXT_MUTED, C_TEXT_PRIMARY, C_TEXT_SECONDARY, FONT_MONO,
};
use super::view_models::IlTab;

#[component]
pub fn AssembliesPanel(
    assemblies_width: f64,
    open_tabs: Signal<Vec<IlTab>>,
    active_tab_id: Signal<Option<String>>,
    highlighted_il_offset: Signal<Option<i64>>,
) -> Element {
    let state = use_context::<AppState>();
    let assemblies = state.assemblies.read().clone();
    let selected_id = state.selected_id.read().clone();

    rsx! {
        div {
            style: format!(
                "width: {assemblies_width:.0}px; flex-shrink: 0; display: flex; \
                 flex-direction: column; background: {C_BG_SURFACE};"
            ),

            div {
                class: "panel-header",
                span { "Assemblies" }
                span { class: "badge", "{assemblies.len()}" }
            }

            div {
                style: "flex: 1; overflow-y: auto; padding: 8px 0;",

                if assemblies.is_empty() {
                    div {
                        class: "empty-state",
                        svg {
                            width: "40", height: "40", view_box: "0 0 24 24",
                            fill: "none", stroke: C_ACCENT_BLUE,
                            stroke_width: "1.5",
                            path { d: "M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" }
                        }
                        p { "Open a .NET assembly to begin analysis" }
                    }
                } else {
                    for asm in assemblies.iter() {
                        {
                            let asm_id = asm.id.clone();
                            let asm_id_select = asm_id.clone();
                            let asm_id_close = asm_id.clone();
                            let is_selected = selected_id.as_ref() == Some(&asm_id);
                            let item_class = if is_selected { "asm-item selected" } else { "asm-item" };
                            rsx! {
                                button {
                                    key: "{asm.id}",
                                    class: "{item_class}",
                                    onclick: move |_| {
                                        state.select_assembly(asm_id_select.clone());
                                        open_tabs.write().clear();
                                        active_tab_id.set(None);
                                        highlighted_il_offset.set(None);
                                    },

                                    div {
                                        style: "display: flex; align-items: center; justify-content: space-between; gap: 6px;",

                                        div {
                                            style: "display: flex; align-items: center; gap: 6px; min-width: 0;",
                                            div {
                                                style: format!(
                                                    "width: 6px; height: 6px; border-radius: 50%; flex-shrink: 0; \
                                                     background: {};",
                                                    if is_selected { C_ACCENT_BLUE } else { C_TEXT_MUTED }
                                                )
                                            }
                                            span {
                                                style: format!(
                                                    "font-size: 12px; font-weight: 600; color: {}; \
                                                     overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                                    if is_selected { C_TEXT_PRIMARY } else { C_TEXT_SECONDARY }
                                                ),
                                                "{asm.name}"
                                            }
                                        }

                                        button {
                                            style: format!(
                                                "flex-shrink: 0; width: 18px; height: 18px; border-radius: 4px; \
                                                 border: none; background: transparent; color: {C_TEXT_MUTED}; \
                                                 cursor: pointer; display: flex; align-items: center; \
                                                 justify-content: center; transition: all 120ms; padding: 0;"
                                            ),
                                            onclick: move |evt| {
                                                evt.stop_propagation();
                                                state.close_assembly(asm_id_close.clone());
                                                open_tabs.write().clear();
                                                active_tab_id.set(None);
                                                highlighted_il_offset.set(None);
                                            },
                                            svg {
                                                width: "10", height: "10", view_box: "0 0 24 24",
                                                fill: "none", stroke: "currentColor", stroke_width: "2.5",
                                                line { x1: "18", y1: "6", x2: "6", y2: "18" }
                                                line { x1: "6", y1: "6", x2: "18", y2: "18" }
                                            }
                                        }
                                    }

                                    div {
                                        style: format!(
                                            "font-size: 10px; color: {C_TEXT_MUTED}; margin-top: 4px; \
                                             font-family: {FONT_MONO}; \
                                             overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                                        ),
                                        "{asm.path}"
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
