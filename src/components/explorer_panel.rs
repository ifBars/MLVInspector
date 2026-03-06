/// Middle-left panel: hierarchical namespace → type → method tree.
use std::collections::BTreeSet;

use dioxus::prelude::*;

use crate::state::AppState;

use super::helpers::{extract_methods, group_methods_by_namespace, method_tab_id, type_tab_id};
use super::theme::{
    C_ACCENT_GREEN, C_BG_BASE, C_BG_ELEVATED, C_BG_SURFACE, C_BORDER, C_BORDER_ACCENT,
    C_TEXT_MUTED, C_TEXT_PRIMARY, C_TEXT_SECONDARY, FONT_MONO,
};
use super::view_models::{IlTab, IlTabKind};

#[component]
pub fn ExplorerPanel(
    explorer_width: f64,
    open_tabs: Signal<Vec<IlTab>>,
    active_tab_id: Signal<Option<String>>,
    highlighted_il_offset: Signal<Option<i64>>,
) -> Element {
    let state = use_context::<AppState>();

    // Collapse state is local to this panel
    let mut collapsed_assemblies = use_signal(BTreeSet::<String>::new);
    let mut collapsed_namespaces = use_signal(BTreeSet::<String>::new);
    let mut collapsed_types = use_signal(BTreeSet::<String>::new);

    let selected_id = state.selected_id.read().clone();
    let assemblies = state.assemblies.read().clone();

    let methods = if let Some(ref id) = selected_id {
        let explore_key = format!("{id}::explore");
        state
            .get_analysis_entry(&explore_key)
            .as_ref()
            .and_then(|e| e.result.as_ref())
            .map(extract_methods)
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let grouped_methods = group_methods_by_namespace(&methods);
    let methods_count = methods.len();
    let class_count = grouped_methods
        .iter()
        .map(|ns| ns.types.len())
        .sum::<usize>();
    let namespace_count = grouped_methods.len();

    let active_tab = {
        let id = active_tab_id.read().clone();
        id.and_then(|tab_id| {
            open_tabs
                .read()
                .iter()
                .find(|tab| tab.id == tab_id)
                .cloned()
        })
    };
    let selected_method_name = active_tab.as_ref().and_then(|tab| {
        tab.method_name
            .as_ref()
            .map(|method_name| format!("{}::{}", tab.type_name, method_name))
    });
    let selected_type_name = active_tab.as_ref().map(|tab| tab.type_name.clone());

    let selected_assembly = selected_id
        .as_ref()
        .and_then(|id| assemblies.iter().find(|asm| asm.id == *id))
        .cloned();

    rsx! {
        div {
            style: format!(
                "width: {explorer_width:.0}px; flex-shrink: 0; display: flex; \
                 flex-direction: column; background: {C_BG_BASE};"
            ),

            div {
                class: "panel-header",
                span { "Explorer" }
                span {
                    class: "badge",
                    "{methods_count} / {class_count} cls / {namespace_count} ns"
                }
            }

            div {
                style: "flex: 1; overflow-y: auto; padding: 8px 0;",

                if methods.is_empty() {
                    div {
                        class: "empty-state",
                        svg {
                            width: "40", height: "40", view_box: "0 0 24 24",
                            fill: "none", stroke: C_ACCENT_GREEN,
                            stroke_width: "1.5",
                            polyline { points: "16 18 22 12 16 6" }
                            polyline { points: "8 6 2 12 8 18" }
                        }
                        p { "Explore results will appear here after loading an assembly" }
                    }
                } else if let Some(asm) = selected_assembly {
                    {
                        let asm_key = asm.id.clone();
                        let asm_collapsed = !collapsed_assemblies.read().contains(&asm_key);
                        let asm_chevron = if asm_collapsed { ">" } else { "v" };
                        rsx! {
                            button {
                                key: "assembly-tree-{asm_key}",
                                style: format!(
                                    "display: flex; align-items: center; gap: 8px; width: calc(100% - 16px); \
                                     margin: 0 8px 6px; padding: 8px 10px; border-radius: 8px; cursor: pointer; \
                                     border: 1px solid {C_BORDER_ACCENT}; background: {C_BG_ELEVATED}; color: {C_TEXT_PRIMARY};"
                                ),
                                onclick: move |_| {
                                    let mut set = collapsed_assemblies.write();
                                    if set.contains(&asm_key) {
                                        set.remove(&asm_key);
                                    } else {
                                        set.insert(asm_key.clone());
                                    }
                                },
                                span {
                                    style: format!(
                                        "font-size: 10px; color: {C_TEXT_MUTED}; width: 10px; text-align: center;"
                                    ),
                                    "{asm_chevron}"
                                }
                                span {
                                    style: format!(
                                        "font-size: 11px; font-weight: 700; color: {C_TEXT_PRIMARY}; \
                                         overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                                    ),
                                    "{asm.name}"
                                }
                                span {
                                    style: format!("margin-left: auto; font-size: 10px; color: {C_TEXT_MUTED};"),
                                    "{namespace_count} ns"
                                }
                            }

                            if !asm_collapsed {
                                for namespace in grouped_methods.iter() {
                                    {
                                        let namespace_key = namespace.namespace_name.clone();
                                        let namespace_collapsed =
                                            !collapsed_namespaces.read().contains(&namespace_key);
                                        let namespace_chevron =
                                            if namespace_collapsed { ">" } else { "v" };
                                        rsx! {
                                            button {
                                                key: "namespace-{namespace_key}",
                                                style: format!(
                                                    "display: flex; align-items: center; gap: 8px; \
                                                     width: calc(100% - 16px); margin: 0 8px 4px; \
                                                     padding: 7px 10px 7px 18px; border-radius: 8px; cursor: pointer; \
                                                     border: 1px solid {C_BORDER}; background: {C_BG_SURFACE}; \
                                                     color: {C_TEXT_SECONDARY};"
                                                ),
                                                onclick: move |_| {
                                                    let mut set = collapsed_namespaces.write();
                                                    if set.contains(&namespace_key) {
                                                        set.remove(&namespace_key);
                                                    } else {
                                                        set.insert(namespace_key.clone());
                                                    }
                                                },
                                                span {
                                                    style: format!(
                                                        "font-size: 10px; color: {C_TEXT_MUTED}; width: 10px; \
                                                         text-align: center;"
                                                    ),
                                                    "{namespace_chevron}"
                                                }
                                                span {
                                                    style: format!(
                                                        "font-size: 11px; font-weight: 700; color: {C_TEXT_PRIMARY}; \
                                                         overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                                                    ),
                                                    "{namespace.namespace_name}"
                                                }
                                                span {
                                                    style: format!(
                                                        "margin-left: auto; font-size: 10px; color: {C_TEXT_MUTED};"
                                                    ),
                                                    "{namespace.types.len()}"
                                                }
                                            }

                                            if !namespace_collapsed {
                                                for group in namespace.types.iter() {
                                                    {
                                                        let type_name = group.full_type_name.clone();
                                                        let type_name_toggle = type_name.clone();
                                                        let type_name_select = type_name.clone();
                                                        let type_collapsed =
                                                            !collapsed_types.read().contains(&type_name);
                                                        let type_chevron =
                                                            if type_collapsed { ">" } else { "v" };
                                                        let is_type_selected =
                                                            selected_type_name.as_ref() == Some(&type_name)
                                                                && selected_method_name.is_none();
                                                        rsx! {
                                                            div {
                                                                key: "type-{type_name}",
                                                                style: "display: flex; gap: 4px; margin: 0 8px 3px; \
                                                                        width: calc(100% - 16px);",

                                                                // Expand/collapse toggle
                                                                button {
                                                                    style: format!(
                                                                        "width: 26px; flex-shrink: 0; \
                                                                         border-radius: 6px; cursor: pointer; \
                                                                         border: 1px solid {C_BORDER}; \
                                                                         background: {C_BG_BASE}; color: {C_TEXT_MUTED};"
                                                                    ),
                                                                    onclick: move |_| {
                                                                        let mut set = collapsed_types.write();
                                                                        if set.contains(&type_name_toggle) {
                                                                            set.remove(&type_name_toggle);
                                                                        } else {
                                                                            set.insert(type_name_toggle.clone());
                                                                        }
                                                                    },
                                                                    "{type_chevron}"
                                                                }

                                                                // Type name button (opens type tab)
                                                                button {
                                                                    style: format!(
                                                                        "display: flex; align-items: center; gap: 8px; \
                                                                         min-width: 0; flex: 1; padding: 6px 10px; \
                                                                         border-radius: 8px; cursor: pointer; text-align: left; \
                                                                         border: 1px solid {}; background: {}; \
                                                                         color: {C_TEXT_PRIMARY};",
                                                                        if is_type_selected {
                                                                            C_BORDER_ACCENT
                                                                        } else {
                                                                            C_BORDER
                                                                        },
                                                                        if is_type_selected {
                                                                            "rgba(245,245,245,0.06)"
                                                                        } else {
                                                                            C_BG_ELEVATED
                                                                        }
                                                                    ),
                                                                    onclick: move |_| {
                                                                        let tab_id = type_tab_id(&type_name_select);
                                                                        {
                                                                            let mut tabs = open_tabs.write();
                                                                            if !tabs.iter().any(|tab| tab.id == tab_id) {
                                                                                let display_name =
                                                                                    type_name_select
                                                                                        .rsplit('.')
                                                                                        .next()
                                                                                        .unwrap_or(&type_name_select)
                                                                                        .to_string();
                                                                                tabs.push(IlTab {
                                                                                    id: tab_id.clone(),
                                                                                    kind: IlTabKind::Type,
                                                                                    type_name: type_name_select.clone(),
                                                                                    method_name: None,
                                                                                    title: display_name,
                                                                                    subtitle: type_name_select.clone(),
                                                                                });
                                                                            }
                                                                        }
                                                                        active_tab_id.set(Some(tab_id));
                                                                        highlighted_il_offset.set(None);
                                                                        collapsed_types
                                                                            .write()
                                                                            .insert(type_name_select.clone());
                                                                    },
                                                                    span {
                                                                        style: format!(
                                                                            "font-size: 11px; font-weight: 700; \
                                                                             color: {C_TEXT_PRIMARY}; overflow: hidden; \
                                                                             text-overflow: ellipsis; white-space: nowrap;"
                                                                        ),
                                                                        "{group.display_name}"
                                                                    }
                                                                    span {
                                                                        style: format!(
                                                                            "margin-left: auto; font-size: 10px; \
                                                                             color: {C_TEXT_MUTED};"
                                                                        ),
                                                                        "{group.methods.len()}"
                                                                    }
                                                                }
                                                            }

                                                            if !type_collapsed {
                                                                for method in group.methods.iter() {
                                                                    {
                                                                        let key_name = format!(
                                                                            "{}::{}",
                                                                            method.type_name,
                                                                            method.method_name
                                                                        );
                                                                        let m_type = method.type_name.clone();
                                                                        let m_name = method.method_name.clone();
                                                                        let is_selected =
                                                                            selected_method_name.as_ref()
                                                                                == Some(&key_name);
                                                                        let item_class = if is_selected {
                                                                            "method-item selected"
                                                                        } else {
                                                                            "method-item"
                                                                        };
                                                                        rsx! {
                                                                            button {
                                                                                key: "{key_name}",
                                                                                class: "{item_class}",
                                                                                style: "padding-left: 44px;",
                                                                                onclick: move |_| {
                                                                                    let tab_id =
                                                                                        method_tab_id(&m_type, &m_name);
                                                                                    {
                                                                                        let mut tabs = open_tabs.write();
                                                                                        if !tabs.iter().any(|tab| {
                                                                                            tab.id == tab_id
                                                                                        }) {
                                                                                            tabs.push(IlTab {
                                                                                                id: tab_id.clone(),
                                                                                                kind: IlTabKind::Method,
                                                                                                type_name: m_type.clone(),
                                                                                                method_name: Some(
                                                                                                    m_name.clone(),
                                                                                                ),
                                                                                                title: m_name.clone(),
                                                                                                subtitle: m_type.clone(),
                                                                                            });
                                                                                        }
                                                                                    }
                                                                                    active_tab_id.set(Some(tab_id));
                                                                                    highlighted_il_offset.set(None);
                                                                                },
                                                                                div {
                                                                                    style: format!(
                                                                                        "font-size: 12px; font-weight: 600; \
                                                                                         font-family: {FONT_MONO}; \
                                                                                         color: {}; overflow: hidden; \
                                                                                         text-overflow: ellipsis; \
                                                                                         white-space: nowrap;",
                                                                                        if is_selected {
                                                                                            C_ACCENT_GREEN
                                                                                        } else {
                                                                                            C_TEXT_PRIMARY
                                                                                        }
                                                                                    ),
                                                                                    "{method.method_name}"
                                                                                }
                                                                                div {
                                                                                    style: format!(
                                                                                        "font-size: 10px; \
                                                                                         color: {C_TEXT_MUTED}; \
                                                                                         margin-top: 3px; \
                                                                                         overflow: hidden; \
                                                                                         text-overflow: ellipsis; \
                                                                                         white-space: nowrap;"
                                                                                    ),
                                                                                    "{method.type_name}"
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
                            }
                        }
                    }
                } else {
                    div {
                        class: "empty-state",
                        p { "Select an assembly to browse namespaces, classes, and methods" }
                    }
                }
            }
        }
    }
}
