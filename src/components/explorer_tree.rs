use std::collections::BTreeSet;

use dioxus::prelude::*;

use crate::types::OpenAssembly;

use super::helpers::{assembly_metadata_tab_id, method_tab_id, type_tab_id};
use super::theme::{
    C_ACCENT_GREEN, C_BG_ELEVATED, C_BG_SURFACE, C_BORDER, C_TEXT_MUTED, C_TEXT_PRIMARY,
    C_TEXT_SECONDARY, FONT_MONO,
};
use super::view_models::{IlTab, IlTabKind, UiNamespaceGroup};

#[component]
pub(crate) fn ExplorerTree(
    selected_assembly: OpenAssembly,
    has_metadata: bool,
    grouped_types: Vec<UiNamespaceGroup>,
    namespace_count: usize,
    type_count: usize,
    methods_count: usize,
    selected_type_name: Option<String>,
    selected_method_name: Option<String>,
    expanded_assemblies: Signal<BTreeSet<String>>,
    expanded_namespaces: Signal<BTreeSet<String>>,
    expanded_types: Signal<BTreeSet<String>>,
    open_tabs: Signal<Vec<IlTab>>,
    active_tab_id: Signal<Option<String>>,
    selected_finding: Signal<Option<usize>>,
) -> Element {
    let assembly_key = selected_assembly.id.clone();
    let selected_assembly_name = selected_assembly.name.clone();
    let selected_assembly_path = selected_assembly.path.clone();
    let assembly_expanded = expanded_assemblies.read().contains(&assembly_key);
    let metadata_tab_id = assembly_metadata_tab_id(&assembly_key);
    let is_metadata_selected = active_tab_id.read().as_ref() == Some(&metadata_tab_id);

    rsx! {
        div {
            style: "display: flex; flex-direction: column; padding-bottom: 10px;",

            div {
                style: "display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 8px 12px 6px;",
                span {
                    style: format!(
                        "font-size: 10px; font-weight: 700; letter-spacing: 0.08em; color: {C_TEXT_MUTED}; text-transform: uppercase;"
                    ),
                    "Browser"
                }
                span {
                    style: format!(
                        "font-size: 9px; color: {C_TEXT_MUTED}; font-family: {FONT_MONO};"
                    ),
                    "{type_count} types / {methods_count} methods"
                }
            }

            button {
                style: format!(
                    "width: calc(100% - 16px); margin: 0 8px; display: flex; align-items: center; gap: 8px; \
                     padding: 8px 10px; border: 1px solid {C_BORDER}; border-radius: 8px; background: {C_BG_ELEVATED}; \
                     color: {C_TEXT_PRIMARY}; text-align: left; cursor: pointer;"
                ),
                onclick: move |_| {
                    let mut set = expanded_assemblies.write();
                    if set.contains(&assembly_key) {
                        set.remove(&assembly_key);
                    } else {
                        set.insert(assembly_key.clone());
                    }
                },

                ChevronIcon { expanded: assembly_expanded }

                div {
                    style: "display: flex; align-items: center; gap: 8px; min-width: 0; flex: 1;",
                    AssemblyGlyph {}
                    div {
                        style: "min-width: 0; display: flex; flex-direction: column; gap: 2px; flex: 1;",
                        span {
                            style: format!(
                                "font-size: 11px; font-weight: 700; color: {C_TEXT_PRIMARY}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                            ),
                            "{selected_assembly_name}"
                        }
                        span {
                            style: format!(
                                "font-size: 9px; color: {C_TEXT_MUTED}; font-family: {FONT_MONO}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                            ),
                            "{selected_assembly_path}"
                        }
                    }
                }

                span {
                    style: format!(
                        "flex-shrink: 0; font-size: 9px; color: {C_TEXT_SECONDARY}; font-family: {FONT_MONO};"
                    ),
                    "{namespace_count} ns"
                }
            }

            if assembly_expanded {
                BranchContainer {
                    if has_metadata {
                        button {
                            style: format!(
                                "width: calc(100% - 12px); margin: 6px 0 0 12px; display: flex; align-items: center; gap: 8px; \
                                 padding: 6px 8px; border: none; border-radius: 6px; background: {}; text-align: left; cursor: pointer;",
                                if is_metadata_selected {
                                    "rgba(255,255,255,0.08)"
                                } else {
                                    "transparent"
                                }
                            ),
                            onclick: move |_| {
                                {
                                    let mut tabs = open_tabs.write();
                                    if !tabs.iter().any(|tab| tab.id == metadata_tab_id) {
                                        tabs.push(IlTab {
                                            id: metadata_tab_id.clone(),
                                            kind: IlTabKind::AssemblyMetadata,
                                            type_name: String::new(),
                                            method_name: None,
                                            title: "Metadata".to_string(),
                                            subtitle: selected_assembly_path.clone(),
                                        });
                                    }
                                }
                                active_tab_id.set(Some(metadata_tab_id.clone()));
                                selected_finding.set(None);
                            },

                            MetadataGlyph {}

                            span {
                                style: format!(
                                    "font-size: 10px; font-weight: 600; color: {C_TEXT_PRIMARY};"
                                ),
                                "Metadata"
                            }

                            span {
                                style: format!(
                                    "margin-left: auto; font-size: 9px; color: {C_TEXT_MUTED}; font-family: {FONT_MONO};"
                                ),
                                "details"
                            }
                        }
                    }

                    if grouped_types.is_empty() {
                        div {
                            style: format!(
                                "margin: 6px 0 0 12px; padding: 8px 10px; color: {C_TEXT_MUTED}; font-size: 10px; line-height: 1.45;"
                            ),
                            "Explore results will appear here after loading an assembly."
                        }
                    } else {
                        for namespace in grouped_types.iter() {
                            {
                                let namespace_key = namespace.namespace_name.clone();
                                let namespace_expanded =
                                    expanded_namespaces.read().contains(&namespace_key);
                                rsx! {
                                    button {
                                        key: "namespace-{namespace_key}",
                                        style: format!(
                                            "width: calc(100% - 12px); margin: 2px 0 0 12px; display: flex; align-items: center; gap: 8px; \
                                             padding: 6px 8px; border: none; border-radius: 6px; background: {}; \
                                             color: {C_TEXT_PRIMARY}; text-align: left; cursor: pointer;",
                                            if namespace_expanded {
                                                "rgba(255,255,255,0.035)"
                                            } else {
                                                "transparent"
                                            }
                                        ),
                                        onclick: move |_| {
                                            let mut set = expanded_namespaces.write();
                                            if set.contains(&namespace_key) {
                                                set.remove(&namespace_key);
                                            } else {
                                                set.insert(namespace_key.clone());
                                            }
                                        },

                                        ChevronIcon { expanded: namespace_expanded }

                                        NamespaceGlyph {}

                                        span {
                                            style: format!(
                                                "min-width: 0; flex: 1; font-size: 10px; font-weight: 600; color: {C_TEXT_PRIMARY}; \
                                                 overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                                            ),
                                            "{namespace.namespace_name}"
                                        }

                                        span {
                                            style: format!(
                                                "flex-shrink: 0; font-size: 9px; color: {C_TEXT_MUTED}; font-family: {FONT_MONO};"
                                            ),
                                            "{namespace.types.len()}"
                                        }
                                    }

                                    if namespace_expanded {
                                        BranchContainer {
                                            for group in namespace.types.iter() {
                                                {
                                                    let type_name = group.full_type_name.clone();
                                                    let type_name_toggle = type_name.clone();
                                                    let type_name_select = type_name.clone();
                                                    let type_expanded =
                                                        expanded_types.read().contains(&type_name);
                                                    let is_type_selected =
                                                        selected_type_name.as_ref() == Some(&type_name)
                                                            && selected_method_name.is_none();
                                                    rsx! {
                                                        div {
                                                            key: "type-{type_name}",
                                                            style: "display: flex; align-items: stretch; gap: 4px; margin: 2px 0 0 12px;",

                                                            button {
                                                                aria_label: "Toggle type methods",
                                                                style: format!(
                                                                    "width: 18px; flex-shrink: 0; border: none; border-radius: 6px; background: transparent; \
                                                                     color: {C_TEXT_MUTED}; cursor: pointer; display: flex; align-items: center; justify-content: center;"
                                                                ),
                                                                onclick: move |_| {
                                                                    let mut set = expanded_types.write();
                                                                    if set.contains(&type_name_toggle) {
                                                                        set.remove(&type_name_toggle);
                                                                    } else {
                                                                        set.insert(type_name_toggle.clone());
                                                                    }
                                                                },
                                                                ChevronIcon { expanded: type_expanded }
                                                            }

                                                            button {
                                                                style: format!(
                                                                    "min-width: 0; flex: 1; display: flex; align-items: center; gap: 8px; padding: 5px 8px; \
                                                                     border: none; border-radius: 6px; background: {}; color: {C_TEXT_PRIMARY}; \
                                                                     text-align: left; cursor: pointer;",
                                                                    if is_type_selected {
                                                                        "rgba(255,255,255,0.08)"
                                                                    } else {
                                                                        "transparent"
                                                                    }
                                                                ),
                                                                onclick: move |_| {
                                                                    let tab_id = type_tab_id(&type_name_select);
                                                                    {
                                                                        let mut tabs = open_tabs.write();
                                                                        if !tabs.iter().any(|tab| tab.id == tab_id) {
                                                                            let display_name = type_name_select
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
                                                                    selected_finding.set(None);
                                                                    expanded_types.write().insert(type_name_select.clone());
                                                                },

                                                                KindBadge {
                                                                    label: type_kind_short_label(&group.kind).to_string()
                                                                }

                                                                span {
                                                                    style: format!(
                                                                        "min-width: 0; flex: 1; font-size: 10px; font-weight: 600; color: {C_TEXT_PRIMARY}; \
                                                                         overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                                                                    ),
                                                                    "{group.display_name}"
                                                                }

                                                                span {
                                                                    style: format!(
                                                                        "flex-shrink: 0; font-size: 9px; color: {C_TEXT_MUTED}; font-family: {FONT_MONO};"
                                                                    ),
                                                                    "{type_method_count_label(group.methods.len())}"
                                                                }
                                                            }
                                                        }

                                                        if type_expanded {
                                                            BranchContainer {
                                                                if group.methods.is_empty() {
                                                                    div {
                                                                        style: format!(
                                                                            "margin: 2px 0 0 12px; padding: 6px 10px; font-size: 10px; line-height: 1.45; color: {C_TEXT_MUTED};"
                                                                        ),
                                                                        "No methods exposed for this {group.kind}."
                                                                    }
                                                                } else {
                                                                    for method in group.methods.iter() {
                                                                        {
                                                                            let method_key = format!(
                                                                                "{}::{}",
                                                                                method.type_name,
                                                                                method.method_name
                                                                            );
                                                                            let method_type = method.type_name.clone();
                                                                            let method_name = method.method_name.clone();
                                                                            let is_selected =
                                                                                selected_method_name.as_ref()
                                                                                    == Some(&method_key);
                                                                            rsx! {
                                                                                button {
                                                                                    key: "{method_key}",
                                                                                    style: format!(
                                                                                        "width: calc(100% - 12px); margin: 2px 0 0 12px; display: flex; flex-direction: column; gap: 2px; \
                                                                                         padding: 5px 8px; border: none; border-radius: 6px; background: {}; \
                                                                                         text-align: left; cursor: pointer;",
                                                                                        if is_selected {
                                                                                            "rgba(255,255,255,0.08)"
                                                                                        } else {
                                                                                            "transparent"
                                                                                        }
                                                                                    ),
                                                                                    onclick: move |_| {
                                                                                        let tab_id =
                                                                                            method_tab_id(&method_type, &method_name);
                                                                                        {
                                                                                            let mut tabs = open_tabs.write();
                                                                                            if !tabs.iter().any(|tab| tab.id == tab_id) {
                                                                                                tabs.push(IlTab {
                                                                                                    id: tab_id.clone(),
                                                                                                    kind: IlTabKind::Method,
                                                                                                    type_name: method_type.clone(),
                                                                                                    method_name: Some(method_name.clone()),
                                                                                                    title: method_name.clone(),
                                                                                                    subtitle: method_type.clone(),
                                                                                                });
                                                                                            }
                                                                                        }
                                                                                        active_tab_id.set(Some(tab_id));
                                                                                        selected_finding.set(None);
                                                                                    },

                                                                                    span {
                                                                                        style: format!(
                                                                                            "font-size: 10px; font-weight: 600; color: {}; font-family: {FONT_MONO}; \
                                                                                             overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                                                                            if is_selected {
                                                                                                C_ACCENT_GREEN
                                                                                            } else {
                                                                                                C_TEXT_PRIMARY
                                                                                            }
                                                                                        ),
                                                                                        "{method.method_name}"
                                                                                    }
                                                                                    span {
                                                                                        style: format!(
                                                                                            "font-size: 9px; color: {C_TEXT_MUTED}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                                                                                        ),
                                                                                        "{method.signature}"
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
                    }
                }
            }
        }
    }
}

#[component]
fn BranchContainer(children: Element) -> Element {
    rsx! {
        div {
            style: format!(
                "display: flex; flex-direction: column; margin-left: 18px; padding-left: 6px; border-left: 1px solid rgba(255,255,255,0.06);"
            ),
            {children}
        }
    }
}

#[component]
fn ChevronIcon(expanded: bool) -> Element {
    let rotation = if expanded { "90deg" } else { "0deg" };

    rsx! {
        div {
            style: "width: 10px; flex-shrink: 0; display: flex; align-items: center; justify-content: center;",
            svg {
                width: "7",
                height: "7",
                view_box: "0 0 8 8",
                fill: "none",
                stroke: C_TEXT_MUTED,
                stroke_width: "1.5",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                style: format!("transform: rotate({rotation}); transition: transform 120ms ease;"),
                polyline { points: "2,1.5 5.5,4 2,6.5" }
            }
        }
    }
}

#[component]
fn AssemblyGlyph() -> Element {
    rsx! {
        svg {
            width: "12",
            height: "12",
            view_box: "0 0 12 12",
            fill: "none",
            stroke: C_TEXT_MUTED,
            stroke_width: "1",
            rect { x: "1.5", y: "1.5", width: "9", height: "9", rx: "1.5" }
            path { d: "M4 1.5v9" }
            path { d: "M7.5 1.5v9" }
        }
    }
}

#[component]
fn NamespaceGlyph() -> Element {
    rsx! {
        svg {
            width: "12",
            height: "12",
            view_box: "0 0 12 12",
            fill: "none",
            stroke: C_TEXT_MUTED,
            stroke_width: "1",
            path { d: "M1.5 3.5h9" }
            path { d: "M1.5 6h9" }
            path { d: "M1.5 8.5h6" }
        }
    }
}

#[component]
fn MetadataGlyph() -> Element {
    rsx! {
        svg {
            width: "12",
            height: "12",
            view_box: "0 0 12 12",
            fill: "none",
            stroke: C_TEXT_MUTED,
            stroke_width: "1",
            rect { x: "2", y: "1.5", width: "8", height: "9", rx: "1" }
            path { d: "M4 4h4" }
            path { d: "M4 6h4" }
            path { d: "M4 8h3" }
        }
    }
}

#[component]
fn KindBadge(label: String) -> Element {
    rsx! {
        span {
            style: format!(
                "flex-shrink: 0; min-width: 40px; padding: 1px 5px; border-radius: 999px; \
                 background: {C_BG_SURFACE}; color: {C_TEXT_MUTED}; font-size: 8px; font-weight: 700; \
                 letter-spacing: 0.06em; text-transform: uppercase; text-align: center;"
            ),
            "{label}"
        }
    }
}

fn type_kind_short_label(kind: &str) -> &'static str {
    match kind {
        "struct" => "struct",
        "interface" => "iface",
        "enum" => "enum",
        "delegate" => "deleg",
        _ => "class",
    }
}

fn type_method_count_label(count: usize) -> String {
    if count == 1 {
        "1".to_string()
    } else {
        count.to_string()
    }
}
