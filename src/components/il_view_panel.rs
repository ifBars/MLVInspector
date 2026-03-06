/// Center panel: tabbed IL instruction viewer with optional C# decompiled source.
use std::collections::HashMap;

use dioxus::prelude::*;

use crate::ipc::DecompileParams;
use crate::state::AppState;

use super::helpers::{extract_methods, method_tab_id};
use super::theme::{
    C_ACCENT_BLUE, C_ACCENT_GREEN, C_BG_BASE, C_BG_ELEVATED, C_BG_SURFACE, C_BORDER, C_TEXT_MUTED,
    C_TEXT_PRIMARY, C_TEXT_SECONDARY, FONT_MONO,
};
use super::view_models::{IlTab, IlTabKind, ViewMode};

#[component]
pub fn IlViewPanel(
    open_tabs: Signal<Vec<IlTab>>,
    active_tab_id: Signal<Option<String>>,
    highlighted_il_offset: Signal<Option<i64>>,
) -> Element {
    let state = use_context::<AppState>();

    // View mode and C# cache are local to this panel
    let mut view_mode = use_signal(|| ViewMode::Il);
    let mut csharp_cache: Signal<HashMap<String, String>> = use_signal(HashMap::new);
    let mut csharp_loading = use_signal(|| false);

    // Scroll to highlighted IL offset
    use_effect(move || {
        let offset = *highlighted_il_offset.read();
        if let Some(off) = offset {
            spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                let js = format!(
                    "const el = document.getElementById('il-{off}'); \
                     if (el) el.scrollIntoView({{behavior:'smooth',block:'center'}});"
                );
                let _ = document::eval(&js).await;
            });
        }
    });

    // Trigger C# decompilation when switching to C# view or changing active tab
    use_effect(move || {
        let mode = *view_mode.read();
        if mode != ViewMode::CSharp {
            return;
        }

        let active_id = active_tab_id.read().clone();
        let tabs = open_tabs.read().clone();
        let sel_id = state.selected_id.read().clone();
        let assemblies = state.assemblies.read().clone();

        let Some(tab_id) = active_id else {
            return;
        };
        let Some(tab) = tabs.into_iter().find(|t| t.id == tab_id) else {
            return;
        };
        let Some(asm_id) = sel_id else {
            return;
        };
        let Some(asm) = assemblies.into_iter().find(|a| a.id == asm_id) else {
            return;
        };

        let assembly_path = asm.path.clone();
        let type_name = tab.type_name.clone();
        let method_name = tab.method_name.clone();
        let cache_key = format!(
            "{}::{}::{}",
            assembly_path,
            type_name,
            method_name.as_deref().unwrap_or("")
        );

        if csharp_cache.read().contains_key(&cache_key) {
            return;
        }

        let worker = state.worker.read().clone();
        csharp_loading.set(true);

        spawn(async move {
            let result = worker
                .decompile(DecompileParams {
                    assembly: assembly_path,
                    type_name: Some(type_name),
                    method_name,
                })
                .await;

            let source = match result {
                Ok(payload) => payload.csharp_source,
                Err(e) => format!("// Decompilation error:\n// {e}"),
            };

            csharp_cache.write().insert(cache_key, source);
            csharp_loading.set(false);
        });
    });

    // Derive display data
    let selected_id = state.selected_id.read().clone();
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
    let selected_type_name = active_tab.as_ref().map(|tab| tab.type_name.clone());
    let active_method = active_tab.as_ref().and_then(|tab| {
        tab.method_name.as_ref().and_then(|method_name| {
            methods
                .iter()
                .find(|m| m.type_name == tab.type_name && m.method_name == *method_name)
                .cloned()
        })
    });
    let selected_type_methods = selected_type_name
        .as_ref()
        .map(|type_name| {
            methods
                .iter()
                .filter(|m| m.type_name == *type_name)
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let show_class_overview = active_tab
        .as_ref()
        .map(|tab| tab.kind == IlTabKind::Type)
        .unwrap_or(false)
        && !selected_type_methods.is_empty();

    let tabs = open_tabs.read().clone();
    let active_tab_id_value = active_tab_id.read().clone();

    rsx! {
        div {
            style: format!(
                "flex: 1; min-width: 0; display: flex; flex-direction: column; \
                 background: {C_BG_BASE};"
            ),

            // Panel header
            div {
                class: "panel-header",
                span {
                    if view_mode() == ViewMode::CSharp { "C# View" } else { "IL View" }
                }
                if show_class_overview {
                    if let Some(type_name) = selected_type_name.as_ref() {
                        span {
                            class: "badge panel-header-detail",
                            style: format!(
                                "color: {C_ACCENT_BLUE}; border-color: {C_ACCENT_BLUE}33; \
                                 background: rgba(91, 155, 255, 0.08);"
                            ),
                            "{type_name}"
                        }
                    }
                } else if let Some(ref m) = active_method {
                    span {
                        class: "badge panel-header-detail",
                        style: format!(
                            "color: {C_ACCENT_GREEN}; border-color: {C_ACCENT_GREEN}33; \
                             background: rgba(110, 231, 183, 0.08);"
                        ),
                        "{m.type_name}.{m.method_name}"
                    }
                }

                // IL / C# toggle
                div {
                    style: format!(
                        "margin-left: auto; display: flex; align-items: center; gap: 2px; \
                         background: {C_BG_BASE}; border: 1px solid {C_BORDER}; \
                         border-radius: 6px; padding: 2px;"
                    ),
                    button {
                        style: format!(
                            "font-size: 10px; font-weight: 600; padding: 3px 8px; \
                             border-radius: 4px; cursor: pointer; border: none; \
                             transition: all 120ms; background: {}; color: {};",
                            if view_mode() == ViewMode::Il { C_BG_ELEVATED } else { "transparent" },
                            if view_mode() == ViewMode::Il { C_TEXT_PRIMARY } else { C_TEXT_MUTED }
                        ),
                        onclick: move |_| view_mode.set(ViewMode::Il),
                        "IL"
                    }
                    button {
                        style: format!(
                            "font-size: 10px; font-weight: 600; padding: 3px 8px; \
                             border-radius: 4px; cursor: pointer; border: none; \
                             transition: all 120ms; background: {}; color: {};",
                            if view_mode() == ViewMode::CSharp { C_BG_ELEVATED } else { "transparent" },
                            if view_mode() == ViewMode::CSharp { C_TEXT_PRIMARY } else { C_TEXT_MUTED }
                        ),
                        onclick: move |_| view_mode.set(ViewMode::CSharp),
                        "C#"
                    }
                }
            }

            // Tab bar + content
            div {
                style: "flex: 1; min-height: 0; display: flex; flex-direction: column;",

                // Tab bar
                div {
                    class: "il-tabs",
                    onwheel: move |evt| {
                        evt.prevent_default();
                    },
                    if tabs.is_empty() {
                        span {
                            style: format!("font-size: 10px; color: {C_TEXT_MUTED}; padding: 5px 6px;"),
                            "No open tabs"
                        }
                    } else {
                        for tab in tabs.iter() {
                            {
                                let tab_id = tab.id.clone();
                                let tab_id_select = tab_id.clone();
                                let tab_id_close = tab_id.clone();
                                let is_active = active_tab_id_value.as_ref() == Some(&tab_id);
                                let tab_class = if is_active { "il-tab active" } else { "il-tab" };
                                rsx! {
                                    div {
                                        key: "il-tab-{tab.id}",
                                        class: "{tab_class}",
                                        onclick: move |_| {
                                            active_tab_id.set(Some(tab_id_select.clone()));
                                            highlighted_il_offset.set(None);
                                        },

                                        div {
                                            style: "min-width: 0; display: grid; gap: 1px;",
                                            div {
                                                style: format!(
                                                    "font-size: 11px; font-weight: 700; color: {}; \
                                                     font-family: {FONT_MONO}; overflow: hidden; \
                                                     text-overflow: ellipsis; white-space: nowrap;",
                                                    if is_active { C_TEXT_PRIMARY } else { C_TEXT_SECONDARY }
                                                ),
                                                "{tab.title}"
                                            }
                                            div {
                                                style: format!(
                                                    "font-size: 9px; color: {C_TEXT_MUTED}; \
                                                     overflow: hidden; text-overflow: ellipsis; \
                                                     white-space: nowrap;"
                                                ),
                                                "{tab.subtitle}"
                                            }
                                        }

                                        button {
                                            class: "tab-close",
                                            title: "Close tab",
                                            "aria-label": "Close tab",
                                            onclick: move |evt| {
                                                evt.stop_propagation();
                                                let mut tabs_mut = open_tabs.write();
                                                if let Some(index) = tabs_mut
                                                    .iter()
                                                    .position(|open_tab| open_tab.id == tab_id_close)
                                                {
                                                    let current_active = active_tab_id.read().clone();
                                                    let closed_was_active =
                                                        current_active.as_ref() == Some(&tab_id_close);
                                                    tabs_mut.remove(index);
                                                    let next_id = if closed_was_active {
                                                        if tabs_mut.is_empty() {
                                                            None
                                                        } else {
                                                            let next_index = index.saturating_sub(1);
                                                            tabs_mut
                                                                .get(next_index)
                                                                .map(|open_tab| open_tab.id.clone())
                                                        }
                                                    } else {
                                                        current_active
                                                    };
                                                    drop(tabs_mut);
                                                    active_tab_id.set(next_id);
                                                    highlighted_il_offset.set(None);
                                                }
                                            },
                                            svg {
                                                width: "10", height: "10", view_box: "0 0 24 24",
                                                fill: "none", stroke: "currentColor", stroke_width: "2",
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

                // Content area
                div {
                    style: "flex: 1; overflow-y: auto; padding: 10px 14px;",

                    if view_mode() == ViewMode::Il {
                        // ── IL view ───────────────────────────────────────────────

                        if show_class_overview {
                            if let Some(type_name) = selected_type_name.as_ref() {
                                // Class overview header
                                div {
                                    style: format!(
                                        "margin-bottom: 14px; padding: 10px 12px; \
                                         background: {C_BG_ELEVATED}; border-radius: 8px; \
                                         border: 1px solid {C_BORDER};"
                                    ),
                                    div {
                                        style: format!(
                                            "font-size: 12px; font-weight: 700; \
                                             font-family: {FONT_MONO}; color: {C_TEXT_PRIMARY}; \
                                             margin-bottom: 5px;"
                                        ),
                                        "{type_name}"
                                    }
                                    div {
                                        style: format!("font-size: 10px; color: {C_TEXT_SECONDARY};"),
                                        "{selected_type_methods.len()} methods"
                                    }
                                }

                                // All methods in the class
                                {
                                    selected_type_methods.clone().into_iter().map(|method| {
                                        let click_type = method.type_name.clone();
                                        let click_method = method.method_name.clone();
                                        let key_method = method.method_name.clone();
                                        rsx! {
                                            div {
                                                key: "class-method-{method.method_name}",
                                                style: format!(
                                                    "margin-bottom: 14px; padding: 10px 12px; \
                                                     background: {C_BG_SURFACE}; border-radius: 8px; \
                                                     border: 1px solid {C_BORDER}; cursor: pointer;"
                                                ),
                                                onclick: move |_| {
                                                    let tab_id = method_tab_id(&click_type, &click_method);
                                                    {
                                                        let mut tabs = open_tabs.write();
                                                        if !tabs.iter().any(|tab| tab.id == tab_id) {
                                                            tabs.push(IlTab {
                                                                id: tab_id.clone(),
                                                                kind: IlTabKind::Method,
                                                                type_name: click_type.clone(),
                                                                method_name: Some(click_method.clone()),
                                                                title: click_method.clone(),
                                                                subtitle: click_type.clone(),
                                                            });
                                                        }
                                                    }
                                                    active_tab_id.set(Some(tab_id));
                                                    highlighted_il_offset.set(None);
                                                },

                                                div {
                                                    style: format!(
                                                        "font-size: 13px; font-weight: 700; \
                                                         font-family: {FONT_MONO}; color: {C_ACCENT_GREEN}; \
                                                         margin-bottom: 4px;"
                                                    ),
                                                    "{method.method_name}"
                                                }
                                                div {
                                                    style: format!(
                                                        "font-size: 10px; font-family: {FONT_MONO}; \
                                                         color: {C_TEXT_SECONDARY}; line-height: 1.4; \
                                                         margin-bottom: 8px;"
                                                    ),
                                                    "{method.signature}"
                                                }
                                                div {
                                                    style: format!("font-family: {FONT_MONO};"),
                                                    {
                                                        let highlighted_val = *highlighted_il_offset.read();
                                                        method.instructions.iter().map(move |ins| {
                                                            let is_highlighted =
                                                                highlighted_val == Some(ins.offset);
                                                            let row_class = if is_highlighted {
                                                                "il-row highlighted"
                                                            } else {
                                                                "il-row"
                                                            };
                                                            rsx! {
                                                                div {
                                                                    key: "{key_method}-{ins.offset}-{ins.op_code}",
                                                                    id: "il-{ins.offset}",
                                                                    class: "{row_class}",
                                                                    span {
                                                                        style: format!(
                                                                            "color: {C_ACCENT_BLUE}; font-size: 11px;"
                                                                        ),
                                                                        "IL_{ins.offset:04X}"
                                                                    }
                                                                    span {
                                                                        style: format!(
                                                                            "color: {C_ACCENT_GREEN}; font-size: 11px; \
                                                                             font-weight: 500;"
                                                                        ),
                                                                        "{ins.op_code}"
                                                                    }
                                                                    span {
                                                                        style: format!(
                                                                            "color: {C_TEXT_SECONDARY}; font-size: 11px;"
                                                                        ),
                                                                        "{ins.operand}"
                                                                    }
                                                                }
                                                            }
                                                        })
                                                    }
                                                }
                                            }
                                        }
                                    })
                                }
                            }
                        }

                        // Single method view
                        if let Some(method) = active_method {
                            div {
                                style: format!(
                                    "margin-bottom: 14px; padding: 10px 12px; \
                                     background: {C_BG_ELEVATED}; border-radius: 8px; \
                                     border: 1px solid {C_BORDER};"
                                ),
                                div {
                                    style: format!(
                                        "font-size: 13px; font-weight: 700; \
                                         font-family: {FONT_MONO}; color: {C_ACCENT_GREEN}; \
                                         margin-bottom: 4px;"
                                    ),
                                    "{method.method_name}"
                                }
                                div {
                                    style: format!(
                                        "font-size: 10px; font-family: {FONT_MONO}; \
                                         color: {C_TEXT_SECONDARY}; line-height: 1.4;"
                                    ),
                                    "{method.signature}"
                                }
                            }
                            div {
                                style: format!("font-family: {FONT_MONO};"),
                                {
                                    let highlighted_val = *highlighted_il_offset.read();
                                    method.instructions.iter().map(move |ins| {
                                        let is_highlighted = highlighted_val == Some(ins.offset);
                                        let row_class = if is_highlighted {
                                            "il-row highlighted"
                                        } else {
                                            "il-row"
                                        };
                                        rsx! {
                                            div {
                                                key: "{ins.offset}-{ins.op_code}",
                                                id: "il-{ins.offset}",
                                                class: "{row_class}",
                                                span {
                                                    style: format!(
                                                        "color: {C_ACCENT_BLUE}; font-size: 11px;"
                                                    ),
                                                    "IL_{ins.offset:04X}"
                                                }
                                                span {
                                                    style: format!(
                                                        "color: {C_ACCENT_GREEN}; font-size: 11px; \
                                                         font-weight: 500;"
                                                    ),
                                                    "{ins.op_code}"
                                                }
                                                span {
                                                    style: format!(
                                                        "color: {C_TEXT_SECONDARY}; font-size: 11px;"
                                                    ),
                                                    "{ins.operand}"
                                                }
                                            }
                                        }
                                    })
                                }
                            }
                        }

                    } else {
                        // ── C# decompiled source ──────────────────────────────────

                        if csharp_loading() {
                            div {
                                class: "empty-state",
                                span {
                                    class: "pulse",
                                    style: format!("font-size: 12px; color: {C_TEXT_MUTED};"),
                                    "Decompiling…"
                                }
                            }
                        } else {
                            {
                                let active_id = active_tab_id.read().clone();
                                let tabs_snap = open_tabs.read().clone();
                                let sel_id = state.selected_id.read().clone();
                                let assemblies_snap = state.assemblies.read().clone();

                                let cache_key = active_id.and_then(|tab_id| {
                                    let tab = tabs_snap.into_iter().find(|t| t.id == tab_id)?;
                                    let asm_id = sel_id?;
                                    let asm = assemblies_snap.into_iter().find(|a| a.id == asm_id)?;
                                    Some(format!(
                                        "{}::{}::{}",
                                        asm.path,
                                        tab.type_name,
                                        tab.method_name.as_deref().unwrap_or("")
                                    ))
                                });

                                let source = cache_key
                                    .as_ref()
                                    .and_then(|k| csharp_cache.read().get(k).cloned());

                                rsx! {
                                    if let Some(src) = source {
                                        pre {
                                            style: format!(
                                                "font-family: {FONT_MONO}; font-size: 11px; \
                                                 line-height: 1.7; color: {C_TEXT_SECONDARY}; \
                                                 white-space: pre-wrap; word-break: break-all;"
                                            ),
                                            "{src}"
                                        }
                                    } else {
                                        div {
                                            class: "empty-state",
                                            p { "Select a method or type tab to view C# source" }
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
