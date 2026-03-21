/// Center panel: tabbed IL instruction viewer with optional C# decompiled source.
use std::collections::HashMap;

use dioxus::prelude::*;

use crate::ipc::{DecompileParams, DecompilePayload};
use crate::state::AppState;

use super::csharp_highlight::highlight_csharp;
use super::helpers::{
    extract_findings, extract_methods, highlighted_csharp_lines,
    highlighted_csharp_lines_from_source_spans, is_compiler_generated_type_name, method_tab_id,
    resolve_method_reference, should_retry_decompile_source,
};
use super::theme::{
    C_ACCENT_BLUE, C_ACCENT_GREEN, C_BG_BASE, C_BG_ELEVATED, C_BG_SURFACE, C_BORDER, C_TEXT_MUTED,
    C_TEXT_PRIMARY, C_TEXT_SECONDARY, FONT_MONO,
};
use super::view_models::{IlTab, IlTabKind, ViewMode};

const DECOMPILE_PROFILE: &str = "readable";

#[component]
pub fn IlViewPanel(
    open_tabs: Signal<Vec<IlTab>>,
    active_tab_id: Signal<Option<String>>,
    selected_finding: Signal<Option<usize>>,
) -> Element {
    let state = use_context::<AppState>();

    // View mode and C# cache are local to this panel
    let mut view_mode = use_signal(|| ViewMode::Il);
    let mut csharp_cache: Signal<HashMap<String, DecompilePayload>> = use_signal(HashMap::new);
    let mut csharp_loading = use_signal(|| false);

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
            "{}::{}::{}::{}",
            assembly_path,
            type_name,
            method_name.as_deref().unwrap_or(""),
            DECOMPILE_PROFILE
        );

        let should_fetch = csharp_cache
            .read()
            .get(&cache_key)
            .map(|payload| should_retry_decompile_source(&payload.csharp_source))
            .unwrap_or(true);

        if !should_fetch {
            return;
        }

        let worker = state.worker.read().clone();
        let cache_key_for_insert = cache_key.clone();
        let assembly_path_for_error = assembly_path.clone();
        let type_name_for_error = type_name.clone();
        let method_name_for_error = method_name.clone();
        csharp_loading.set(true);

        spawn(async move {
            let result = worker
                .decompile(DecompileParams {
                    assembly: assembly_path,
                    type_name: Some(type_name),
                    method_name,
                    profile: Some(DECOMPILE_PROFILE.to_string()),
                })
                .await;

            let payload = match result {
                Ok(payload) => payload,
                Err(e) => DecompilePayload {
                    assembly_path: assembly_path_for_error,
                    type_name: Some(type_name_for_error),
                    method_name: method_name_for_error,
                    csharp_source: format!("// Decompilation error:\n// {e}"),
                    profile: DECOMPILE_PROFILE.to_string(),
                    source_spans: Vec::new(),
                },
            };

            csharp_cache.write().insert(cache_key_for_insert, payload);
            csharp_loading.set(false);
        });
    });

    // Derive display data
    let selected_id = state.selected_id.read().clone();
    let methods = if let Some(ref id) = selected_id {
        let explore_key = format!("{id}::explore");
        state
            .with_analysis_result(&explore_key, extract_methods)
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let findings = if let Some(ref id) = selected_id {
        let scan_key = format!("{id}::scan");
        state
            .with_analysis_result(&scan_key, extract_findings)
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
    let active_method = active_tab.as_ref().and_then(|tab| {
        tab.method_name.as_ref().and_then(|method_name| {
            methods
                .iter()
                .find(|m| m.type_name == tab.type_name && m.method_name == *method_name)
                .cloned()
        })
    });
    let selected_type_name = active_tab.as_ref().map(|tab| tab.type_name.clone());
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
    let selected_type_visible_methods = selected_type_methods
        .iter()
        .filter(|method| !is_redundant_type_member_method(method))
        .cloned()
        .collect::<Vec<_>>();
    let hidden_type_method_count = selected_type_methods
        .len()
        .saturating_sub(selected_type_visible_methods.len());
    let show_type_overview = active_tab
        .as_ref()
        .map(|tab| tab.kind == IlTabKind::Type)
        .unwrap_or(false);

    let active_finding = selected_finding().and_then(|index| findings.get(index).cloned());
    let active_csharp_finding_span = active_finding.as_ref().and_then(|finding| {
        let active_tab = active_tab.as_ref()?;
        let method_name = active_tab.method_name.as_ref()?;
        let navigation = finding.navigation.as_ref()?;

        navigation
            .method_spans
            .iter()
            .find(|span| {
                resolve_method_reference(&methods, &span.type_name, &span.method_name).is_some_and(
                    |(resolved_type, resolved_method)| {
                        resolved_type == active_tab.type_name && resolved_method == *method_name
                    },
                )
            })
            .cloned()
    });
    let active_il_finding_span = active_finding.as_ref().and_then(|finding| {
        let active_tab = active_tab.as_ref()?;
        let method_name = active_tab.method_name.as_ref()?;
        let navigation = finding.navigation.as_ref()?;

        navigation
            .method_spans
            .iter()
            .find(|span| span.type_name == active_tab.type_name && span.method_name == *method_name)
            .cloned()
    });
    let active_generated_il_methods = active_finding
        .as_ref()
        .and_then(|finding| finding.navigation.as_ref())
        .and_then(|navigation| {
            let active_tab = active_tab.as_ref()?;
            let method_name = active_tab.method_name.as_ref()?;

            Some(
                navigation
                    .method_spans
                    .iter()
                    .filter(|span| {
                        (span.type_name != active_tab.type_name || span.method_name != *method_name)
                            && is_compiler_generated_type_name(&span.type_name)
                            && resolve_method_reference(
                                &methods,
                                &span.type_name,
                                &span.method_name,
                            )
                            .is_some_and(
                                |(resolved_type, resolved_method)| {
                                    resolved_type == active_tab.type_name
                                        && resolved_method == *method_name
                                },
                            )
                    })
                    .filter_map(|span| {
                        methods
                            .iter()
                            .find(|method| {
                                method.type_name == span.type_name
                                    && method.method_name == span.method_name
                            })
                            .cloned()
                            .map(|method| (span.clone(), method))
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .unwrap_or_default();

    let active_id_for_source = active_tab_id.read().clone();
    let tabs_for_source = open_tabs.read().clone();
    let assemblies_for_source = state.assemblies.read().clone();
    let csharp_cache_key = active_id_for_source.and_then(|tab_id| {
        let tab = tabs_for_source.into_iter().find(|t| t.id == tab_id)?;
        let asm_id = selected_id.clone()?;
        let asm = assemblies_for_source.into_iter().find(|a| a.id == asm_id)?;
        Some(format!(
            "{}::{}::{}::{}",
            asm.path,
            tab.type_name,
            tab.method_name.as_deref().unwrap_or(""),
            DECOMPILE_PROFILE
        ))
    });
    let csharp_payload = csharp_cache_key
        .as_ref()
        .and_then(|key| csharp_cache.read().get(key).cloned());
    let csharp_source = csharp_payload
        .as_ref()
        .map(|payload| payload.csharp_source.clone());
    let highlighted_csharp_line_numbers =
        match (csharp_payload.as_ref(), active_csharp_finding_span.as_ref()) {
            (Some(payload), Some(span)) => {
                let mut lines =
                    highlighted_csharp_lines_from_source_spans(&payload.source_spans, span);
                if lines.is_empty() {
                    lines = highlighted_csharp_lines(&payload.csharp_source, &span.csharp_snippets);
                }
                if lines.is_empty() && !payload.csharp_source.is_empty() {
                    vec![1]
                } else {
                    lines
                }
            }
            _ => Vec::new(),
        };
    let effect_active_il_finding_span = active_il_finding_span.clone();
    let effect_active_generated_il_methods = active_generated_il_methods.clone();
    let effect_active_method = active_method.clone();
    let effect_highlighted_csharp_line_numbers = highlighted_csharp_line_numbers.clone();

    use_effect(move || {
        let il_scroll_target = effect_active_il_finding_span
            .clone()
            .and_then(|span| {
                span.il_offsets
                    .first()
                    .copied()
                    .map(|offset| format!("il-{offset}"))
            })
            .or_else(|| {
                effect_active_generated_il_methods
                    .first()
                    .and_then(|(span, _)| span.il_offsets.first().copied())
                    .map(|offset| format!("generated-il-0-{offset}"))
            })
            .or_else(|| {
                effect_active_method.as_ref().and_then(|method| {
                    method
                        .instructions
                        .first()
                        .map(|ins| format!("il-{}", ins.offset))
                })
            });

        if il_scroll_target.is_none() && effect_highlighted_csharp_line_numbers.is_empty() {
            return;
        }

        let scroll_target = match *view_mode.read() {
            ViewMode::Il => il_scroll_target,
            ViewMode::CSharp => effect_highlighted_csharp_line_numbers
                .first()
                .map(|line_number| format!("csharp-line-{line_number}")),
        };

        let Some(target_id) = scroll_target else {
            return;
        };

        spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            let js = format!(
                "const el = document.getElementById('{target_id}'); if (el) el.scrollIntoView({{behavior:'smooth',block:'center'}});"
            );
            let _ = document::eval(&js).await;
        });
    });

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
                                            selected_finding.set(None);
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
                                                    selected_finding.set(None);
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

                        if show_type_overview {
                            if let Some(type_name) = selected_type_name.as_ref() {
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
                                        style: format!("font-size: 10px; color: {C_TEXT_SECONDARY}; line-height: 1.5;"),
                                        "{selected_type_visible_methods.len()} visible methods"
                                        if hidden_type_method_count > 0 {
                                            " / {hidden_type_method_count} hidden accessors"
                                        }
                                    }
                                }

                                if selected_type_visible_methods.is_empty() {
                                    div {
                                        style: format!(
                                            "margin-bottom: 14px; padding: 10px 12px; background: {C_BG_SURFACE}; \
                                             border-radius: 8px; border: 1px solid {C_BORDER}; color: {C_TEXT_SECONDARY};"
                                        ),
                                        div {
                                            style: format!("font-size: 11px; color: {C_TEXT_PRIMARY}; margin-bottom: 4px;"),
                                            "No non-accessor method bodies available"
                                        }
                                        div {
                                            style: format!("font-size: 10px; line-height: 1.5; color: {C_TEXT_SECONDARY};"),
                                            "This type can still be inspected in the C# tab. Property/event accessors are hidden here by default because they are usually redundant."
                                        }
                                    }
                                }

                                {
                                    selected_type_visible_methods.clone().into_iter().map(|method| {
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
                                                    selected_finding.set(None);
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
                                                    if method.instructions.is_empty() {
                                                        div {
                                                            style: format!(
                                                                "padding: 6px 0; font-size: 10px; color: {C_TEXT_MUTED};"
                                                            ),
                                                            "<no IL body>"
                                                        }
                                                    } else {
                                                        {
                                                            let highlighted_offsets = active_finding
                                                                .as_ref()
                                                                .and_then(|finding| finding.navigation.as_ref())
                                                                .and_then(|navigation| {
                                                                    navigation.method_spans.iter().find(|span| {
                                                                        resolve_method_reference(
                                                                            &methods,
                                                                            &span.type_name,
                                                                            &span.method_name,
                                                                        )
                                                                        .is_some_and(|(resolved_type, resolved_method)| {
                                                                            resolved_type == method.type_name
                                                                                && resolved_method == method.method_name
                                                                        })
                                                                    })
                                                                })
                                                                .map(|span| span.il_offsets.clone())
                                                                .unwrap_or_default();
                                                            method.instructions.iter().map(move |ins| {
                                                                let is_highlighted =
                                                                    highlighted_offsets.contains(&ins.offset);
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
                                    let highlighted_offsets = active_il_finding_span
                                        .as_ref()
                                        .map(|span| span.il_offsets.clone())
                                        .unwrap_or_default();
                                    method.instructions.iter().map(move |ins| {
                                        let is_highlighted = highlighted_offsets.contains(&ins.offset);
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
                            if !active_generated_il_methods.is_empty() {
                                div {
                                    style: format!(
                                        "margin-top: 16px; padding: 10px 12px; background: {C_BG_SURFACE}; \
                                         border-radius: 8px; border: 1px solid {C_BORDER};"
                                    ),
                                    div {
                                        style: format!(
                                            "font-size: 11px; font-weight: 700; color: {C_TEXT_PRIMARY}; \
                                             margin-bottom: 4px;"
                                        ),
                                        "Async/Generated Execution Body"
                                    }
                                    div {
                                        style: format!(
                                            "font-size: 10px; color: {C_TEXT_SECONDARY}; line-height: 1.5; \
                                             margin-bottom: 10px;"
                                        ),
                                        "The selected method starts a compiler-generated async/iterator body. \
                                         The finding's executable IL is highlighted below."
                                    }
                                    for (generated_index, (span, generated_method)) in active_generated_il_methods.iter().enumerate() {
                                        div {
                                            key: "generated-method-{generated_index}-{generated_method.type_name}-{generated_method.method_name}",
                                            style: format!(
                                                "margin-top: 10px; padding-top: 10px; border-top: 1px solid {C_BORDER};"
                                            ),
                                            div {
                                                style: format!(
                                                    "font-size: 11px; font-weight: 700; font-family: {FONT_MONO}; \
                                                     color: {C_TEXT_PRIMARY}; margin-bottom: 3px;"
                                                ),
                                                "{generated_method.method_name}"
                                            }
                                            div {
                                                style: format!(
                                                    "font-size: 10px; font-family: {FONT_MONO}; color: {C_TEXT_MUTED}; \
                                                     line-height: 1.4; margin-bottom: 8px;"
                                                ),
                                                "{generated_method.type_name}"
                                            }
                                            div {
                                                style: format!("font-family: {FONT_MONO};"),
                                                {
                                                    let highlighted_offsets = span.il_offsets.clone();
                                                    generated_method.instructions.iter().map(move |ins| {
                                                        let is_highlighted = highlighted_offsets.contains(&ins.offset);
                                                        let row_class = if is_highlighted {
                                                            "il-row highlighted"
                                                        } else {
                                                            "il-row"
                                                        };
                                                        rsx! {
                                                            div {
                                                                key: "generated-{generated_index}-{ins.offset}-{ins.op_code}",
                                                                id: "generated-il-{generated_index}-{ins.offset}",
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
                            if let Some(src) = csharp_source.clone() {
                                {
                                    let highlighted = highlight_csharp(&src);

                                    rsx! {
                                        pre {
                                            class: "csharp-source",
                                            style: format!(
                                                "font-family: {FONT_MONO}; font-size: 11px; \
                                                 line-height: 1.7; color: {C_TEXT_SECONDARY};"
                                            ),
                                            for (line_index, line) in highlighted.into_iter().enumerate() {
                                                {
                                                    let line_number = line_index + 1;
                                                    let is_empty_line = line.is_empty();
                                                    let is_highlighted = highlighted_csharp_line_numbers
                                                        .contains(&line_number);
                                                    let line_class = if is_highlighted {
                                                        "csharp-line highlighted"
                                                    } else {
                                                        "csharp-line"
                                                    };
                                                    rsx! {
                                                        span {
                                                            key: "csharp-line-{line_number}",
                                                            id: "csharp-line-{line_number}",
                                                            class: "{line_class}",
                                                            for (segment_index, segment) in line.into_iter().enumerate() {
                                                                span {
                                                                    key: "csharp-segment-{line_index}-{segment_index}",
                                                                    class: segment.kind.class_name(),
                                                                    "{segment.text}"
                                                                }
                                                            }
                                                            if is_empty_line {
                                                                " "
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

fn is_redundant_type_member_method(method: &super::view_models::UiMethod) -> bool {
    method.method_name.starts_with("get_")
        || method.method_name.starts_with("set_")
        || method.method_name.starts_with("add_")
        || method.method_name.starts_with("remove_")
}
