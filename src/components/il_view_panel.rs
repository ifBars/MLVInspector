/// Center panel: tabbed IL instruction viewer with optional C# decompiled source.
use std::collections::HashMap;

use dioxus::prelude::*;

use crate::ipc::{AnalyzeSymbolParams, AnalyzeSymbolPayload, DecompileParams, DecompilePayload};
use crate::state::AppState;

use super::csharp_highlight::highlight_csharp;
use super::explorer_metadata::{
    default_collapsed_metadata_sections, has_metadata, AssemblyMetadataView,
};
use super::helpers::{
    extract_findings, extract_methods, extract_scan_neighborhoods, extract_type_details,
    highlighted_csharp_lines, highlighted_csharp_lines_from_source_spans,
    is_compiler_generated_type_name, method_tab_id, resolve_method_reference, severity_color,
    should_retry_decompile_source, type_tab_id,
};
use super::theme::{
    C_ACCENT_BLUE, C_ACCENT_GREEN, C_BG_BASE, C_BG_ELEVATED, C_BG_SURFACE, C_BORDER, C_TEXT_MUTED,
    C_TEXT_PRIMARY, C_TEXT_SECONDARY, FONT_MONO,
};
use super::view_models::{
    format_symbol_evidence_location, format_symbol_reference_location,
    symbol_evidence_display_value, symbol_reference_display_name, IlTab, IlTabKind,
    UiAttributeMetadata, UiMemberMetadata, UiScanNeighborhood, UiTypeDetails, ViewMode,
};

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
    let mut analyze_cache: Signal<HashMap<String, Result<AnalyzeSymbolPayload, String>>> =
        use_signal(HashMap::new);
    let mut analyze_loading = use_signal(|| false);
    let analyze_depth = use_signal(|| 1_i32);
    let metadata_collapsed_sections = use_signal(default_collapsed_metadata_sections);

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
        if tab.kind == IlTabKind::AssemblyMetadata {
            return;
        }
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

    // Trigger static usage analysis when switching to Analyze view or changing active tab.
    use_effect(move || {
        let mode = *view_mode.read();
        if mode != ViewMode::Analyze {
            return;
        }
        let max_depth = analyze_depth().clamp(1, 3);

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
        if tab.kind == IlTabKind::AssemblyMetadata {
            return;
        }
        let Some(asm_id) = sel_id else {
            return;
        };
        let Some(asm) = assemblies.into_iter().find(|a| a.id == asm_id) else {
            return;
        };

        let assembly_path = asm.path.clone();
        let type_name = tab.type_name.clone();
        let method_name = tab.method_name.clone();
        let metadata_token = tab.metadata_token.clone();
        let cache_key = format!(
            "{}::{}::{}::{}::{}",
            assembly_path,
            type_name,
            method_name.as_deref().unwrap_or(""),
            metadata_token.as_deref().unwrap_or(""),
            max_depth
        );

        if analyze_cache.read().contains_key(&cache_key) {
            return;
        }

        let worker = state.worker.read().clone();
        let cache_key_for_insert = cache_key.clone();
        analyze_loading.set(true);

        spawn(async move {
            let result = worker
                .analyze_symbol(AnalyzeSymbolParams {
                    assembly: assembly_path,
                    type_name,
                    method_name,
                    metadata_token,
                    max_depth: Some(max_depth),
                })
                .await
                .map_err(|err| err.to_string());

            analyze_cache.write().insert(cache_key_for_insert, result);
            analyze_loading.set(false);
        });
    });

    // Derive display data
    let selected_id = state.selected_id.read().clone();
    let selected_assembly = selected_id.as_ref().and_then(|id| {
        state
            .assemblies
            .read()
            .iter()
            .find(|assembly| assembly.id == *id)
            .cloned()
    });
    let assembly_metadata = if let Some(ref id) = selected_id {
        let explore_key = format!("{id}::explore");
        state
            .with_analysis_result(&explore_key, |result| {
                result
                    .explore
                    .as_ref()
                    .map(|payload| payload.assembly_metadata.clone())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    } else {
        Default::default()
    };
    let metadata_available = has_metadata(&assembly_metadata);
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
    let scan_neighborhoods =
        if let (Some(id), Some(tab)) = (selected_id.as_ref(), active_tab.as_ref()) {
            let scan_key = format!("{id}::scan");
            state
                .with_analysis_result(&scan_key, |result| {
                    extract_scan_neighborhoods(result, &tab.type_name, tab.method_name.as_deref())
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
    let selected_type_name = active_tab.as_ref().map(|tab| tab.type_name.clone());
    let selected_type_details =
        if let (Some(id), Some(type_name)) = (selected_id.as_ref(), selected_type_name.as_ref()) {
            let explore_key = format!("{id}::explore");
            state
                .with_analysis_result(&explore_key, |result| {
                    extract_type_details(result, type_name)
                })
                .flatten()
        } else {
            None
        };
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
    let show_metadata_view = active_tab
        .as_ref()
        .map(|tab| tab.kind == IlTabKind::AssemblyMetadata)
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
        if tab.kind == IlTabKind::AssemblyMetadata {
            return None;
        }
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
    let analyze_cache_key = {
        let active_id_for_analyze = active_tab_id.read().clone();
        let tabs_for_analyze = open_tabs.read().clone();
        let assemblies_for_analyze = state.assemblies.read().clone();
        let max_depth = analyze_depth().clamp(1, 3);
        active_id_for_analyze.and_then(|tab_id| {
            let tab = tabs_for_analyze.into_iter().find(|t| t.id == tab_id)?;
            if tab.kind == IlTabKind::AssemblyMetadata {
                return None;
            }
            let asm_id = selected_id.clone()?;
            let asm = assemblies_for_analyze
                .into_iter()
                .find(|a| a.id == asm_id)?;
            Some(format!(
                "{}::{}::{}::{}::{}",
                asm.path,
                tab.type_name,
                tab.method_name.as_deref().unwrap_or(""),
                tab.metadata_token.as_deref().unwrap_or(""),
                max_depth
            ))
        })
    };
    let analyze_result = analyze_cache_key
        .as_ref()
        .and_then(|key| analyze_cache.read().get(key).cloned());
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
            ViewMode::Analyze => None,
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
                    if show_metadata_view {
                        "Assembly Details"
                    } else if view_mode() == ViewMode::Analyze {
                        "Analyze"
                    } else if view_mode() == ViewMode::CSharp {
                        "C# View"
                    } else {
                        "IL View"
                    }
                }
                if !show_metadata_view {
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
                        button {
                            style: format!(
                                "font-size: 10px; font-weight: 600; padding: 3px 8px; \
                                 border-radius: 4px; cursor: pointer; border: none; \
                                 transition: all 120ms; background: {}; color: {};",
                                if view_mode() == ViewMode::Analyze { C_BG_ELEVATED } else { "transparent" },
                                if view_mode() == ViewMode::Analyze { C_TEXT_PRIMARY } else { C_TEXT_MUTED }
                            ),
                            onclick: move |_| view_mode.set(ViewMode::Analyze),
                            "Analyze"
                        }
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
                                let tab_subtitle = match tab.metadata_token.as_ref() {
                                    Some(token) if !token.is_empty() => {
                                        format!("{} - {}", tab.subtitle, token)
                                    }
                                    _ => tab.subtitle.clone(),
                                };
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
                                                "{tab_subtitle}"
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

                    if show_metadata_view {
                        if let Some(assembly) = selected_assembly.clone() {
                            if metadata_available {
                                AssemblyMetadataView {
                                    assembly_name: assembly.name.clone(),
                                    assembly_path: assembly.path.clone(),
                                    metadata: assembly_metadata.clone(),
                                    collapsed_sections: metadata_collapsed_sections,
                                    active_metadata_token: active_tab
                                        .as_ref()
                                        .and_then(|tab| tab.metadata_token.clone()),
                                    framed: false,
                                }
                            } else {
                                div {
                                    class: "empty-state",
                                    p { "No assembly metadata is available for the selected assembly." }
                                }
                            }
                        } else {
                            div {
                                class: "empty-state",
                                p { "Select an assembly to inspect metadata." }
                            }
                        }
                    } else if view_mode() == ViewMode::Il {
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

                                if let Some(details) = selected_type_details.clone() {
                                    TypeMemberOverview {
                                        details,
                                        active_metadata_token: active_tab
                                            .as_ref()
                                            .and_then(|tab| tab.metadata_token.clone()),
                                        open_tabs,
                                        active_tab_id,
                                        selected_finding,
                                        view_mode,
                                    }
                                }

                                {
                                    selected_type_visible_methods.clone().into_iter().map(|method| {
                                        let click_type = method.type_name.clone();
                                        let click_method = method.method_name.clone();
                                        let key_method = method.method_name.clone();
                                        let token_label = method.metadata_token.clone();
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
                                                                metadata_token: method.metadata_token.clone(),
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
                                                if let Some(token) = token_label.as_ref() {
                                                    div {
                                                        style: format!(
                                                            "font-size: 10px; font-family: {FONT_MONO}; color: {C_TEXT_MUTED}; \
                                                             margin-bottom: 4px;"
                                                        ),
                                                        "{token}"
                                                    }
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
                                if let Some(token) = method.metadata_token.as_ref() {
                                    div {
                                        style: format!(
                                            "font-size: 10px; font-family: {FONT_MONO}; color: {C_TEXT_MUTED}; \
                                             margin-bottom: 4px;"
                                        ),
                                        "{token}"
                                    }
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

                    } else if view_mode() == ViewMode::Analyze {
                        if analyze_loading() && analyze_result.is_none() {
                            div {
                                class: "empty-state",
                                span {
                                    class: "pulse",
                                    style: format!("font-size: 12px; color: {C_TEXT_MUTED};"),
                                    "Analyzing symbol references..."
                                }
                            }
                        } else if let Some(Err(error)) = analyze_result.clone() {
                            div {
                                class: "empty-state",
                                p { "Symbol analysis failed" }
                                span {
                                    style: format!(
                                        "max-width: 360px; font-size: 10px; line-height: 1.5; \
                                         color: {C_TEXT_SECONDARY}; font-family: {FONT_MONO};"
                                    ),
                                    "{error}"
                                }
                            }
                        } else if let Some(Ok(payload)) = analyze_result.clone() {
                            div {
                                style: "display: grid; gap: 12px;",
                                div {
                                    style: format!(
                                        "padding: 10px 12px; background: {C_BG_ELEVATED}; \
                                         border: 1px solid {C_BORDER}; border-radius: 8px;"
                                    ),
                                    div {
                                        style: format!(
                                            "font-size: 12px; font-weight: 700; color: {C_TEXT_PRIMARY}; \
                                             font-family: {FONT_MONO}; margin-bottom: 5px; overflow: hidden; \
                                             text-overflow: ellipsis; white-space: nowrap;"
                                        ),
                                        if let Some(method_name) = payload.method_name.as_ref() {
                                            "{payload.type_name}.{method_name}"
                                        } else {
                                            "{payload.type_name}"
                                        }
                                    }
                                    if let Some(signature) = payload.target_signature.as_ref() {
                                        div {
                                            style: format!(
                                                "font-size: 10px; color: {C_TEXT_SECONDARY}; \
                                                 font-family: {FONT_MONO}; line-height: 1.45; overflow-wrap: anywhere;"
                                            ),
                                            "{signature}"
                                        }
                                    }
                                    AnalyzeDepthControl {
                                        current_depth: analyze_depth(),
                                        analyze_depth,
                                    }
                                    div {
                                        style: format!(
                                            "display: flex; gap: 8px; margin-top: 9px; color: {C_TEXT_MUTED}; \
                                             font-size: 10px; font-family: {FONT_MONO};"
                                        ),
                                        span { class: "badge", "{payload.callers.len()} callers" }
                                        span { class: "badge", "{payload.callees.len()} callees" }
                                        span { class: "badge", "depth {payload.max_depth}" }
                                        span { class: "badge", "{payload.evidence.len()} evidence" }
                                    }
                                }

                                div {
                                    style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 12px;",
                                    SymbolReferenceList {
                                        title: "Callers".to_string(),
                                        empty_text: "No static callers found in this assembly.".to_string(),
                                        references: payload.callers.clone(),
                                        open_tabs,
                                        active_tab_id,
                                        selected_finding,
                                    }
                                    SymbolReferenceList {
                                        title: "Callees".to_string(),
                                        empty_text: "No direct callees found for this method.".to_string(),
                                        references: payload.callees.clone(),
                                        open_tabs,
                                        active_tab_id,
                                        selected_finding,
                                    }
                                }

                                SymbolEvidencePanel {
                                    evidence: payload.evidence.clone(),
                                }
                                ScanNeighborhoodPanel {
                                    neighborhoods: scan_neighborhoods.clone(),
                                    methods: methods.clone(),
                                    open_tabs,
                                    active_tab_id,
                                    selected_finding,
                                }
                            }
                        } else {
                            div {
                                class: "empty-state",
                                p { "Select a method or type tab to analyze symbol references" }
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

#[component]
fn TypeMemberOverview(
    details: UiTypeDetails,
    active_metadata_token: Option<String>,
    open_tabs: Signal<Vec<IlTab>>,
    active_tab_id: Signal<Option<String>>,
    selected_finding: Signal<Option<usize>>,
    view_mode: Signal<ViewMode>,
) -> Element {
    let member_count = details.fields.len()
        + details.properties.len()
        + details.events.len()
        + details.nested_types.len()
        + details.custom_attributes.len();
    let active_member_token = active_member_metadata_token(
        active_metadata_token.as_deref(),
        details.metadata_token.as_deref(),
    );

    use_effect({
        let active_member_token = active_member_token.clone();
        move || {
            let Some(token) = active_member_token.clone() else {
                return;
            };

            spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                let target_id = member_token_dom_id(&token);
                let js = format!(
                    "const el = document.getElementById('{target_id}'); if (el) el.scrollIntoView({{behavior:'smooth',block:'center'}});"
                );
                let _ = document::eval(&js).await;
            });
        }
    });

    if member_count == 0 {
        return rsx! {};
    }

    rsx! {
        div {
            style: format!(
                "margin-bottom: 14px; display: grid; gap: 8px; padding: 10px 12px; \
                 background: {C_BG_SURFACE}; border: 1px solid {C_BORDER}; border-radius: 8px;"
            ),
            div {
                style: "display: flex; align-items: center; justify-content: space-between; gap: 8px;",
                span {
                    style: format!(
                        "font-size: 10px; font-weight: 700; letter-spacing: 0.08em; \
                         text-transform: uppercase; color: {C_TEXT_MUTED};"
                    ),
                    "Members"
                }
                span { class: "badge", "{details.kind}" }
            }
            div {
                style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 8px;",
                MemberList {
                    title: "Fields".to_string(),
                    owner_type_name: details.full_type_name.clone(),
                    members: details.fields.clone(),
                    active_metadata_token: active_member_token.clone(),
                    open_tabs,
                    active_tab_id,
                    selected_finding,
                    view_mode,
                }
                MemberList {
                    title: "Properties".to_string(),
                    owner_type_name: details.full_type_name.clone(),
                    members: details.properties.clone(),
                    active_metadata_token: active_member_token.clone(),
                    open_tabs,
                    active_tab_id,
                    selected_finding,
                    view_mode,
                }
                MemberList {
                    title: "Events".to_string(),
                    owner_type_name: details.full_type_name.clone(),
                    members: details.events.clone(),
                    active_metadata_token: active_member_token.clone(),
                    open_tabs,
                    active_tab_id,
                    selected_finding,
                    view_mode,
                }
                MemberList {
                    title: "Nested Types".to_string(),
                    owner_type_name: details.full_type_name.clone(),
                    members: details.nested_types.clone(),
                    active_metadata_token: active_member_token.clone(),
                    open_tabs,
                    active_tab_id,
                    selected_finding,
                    view_mode,
                }
                AttributeList {
                    attributes: details.custom_attributes.clone(),
                }
            }
        }
    }
}

#[component]
fn MemberList(
    title: String,
    owner_type_name: String,
    members: Vec<UiMemberMetadata>,
    active_metadata_token: Option<String>,
    open_tabs: Signal<Vec<IlTab>>,
    active_tab_id: Signal<Option<String>>,
    selected_finding: Signal<Option<usize>>,
    view_mode: Signal<ViewMode>,
) -> Element {
    if members.is_empty() {
        return rsx! {};
    }

    rsx! {
        div {
            style: format!(
                "min-width: 0; display: grid; gap: 5px; padding: 8px; \
                 background: {C_BG_ELEVATED}; border: 1px solid {C_BORDER}; border-radius: 7px;"
            ),
            div {
                style: "display: flex; align-items: center; justify-content: space-between; gap: 8px;",
                span {
                    style: format!(
                        "font-size: 10px; font-weight: 700; color: {C_TEXT_PRIMARY};"
                    ),
                    "{title}"
                }
                span { class: "badge", "{members.len()}" }
            }
            for member in members.iter().take(8) {
                {
                    let is_active = member.metadata_token.as_deref().is_some_and(|token| {
                        metadata_tokens_match(Some(token), active_metadata_token.as_deref())
                    });
                    let member_dom_id = member
                        .metadata_token
                        .as_deref()
                        .map(member_token_dom_id)
                        .unwrap_or_default();
                    let can_analyze = member_supports_usage_analysis(member);
                    let analyze_member_token = member.metadata_token.clone();
                    let analyze_type_name = owner_type_name.clone();
                    rsx! {
                        div {
                            id: "{member_dom_id}",
                            key: "member-{title}-{member.name}-{member.metadata_token.clone().unwrap_or_default()}",
                            style: format!(
                                "display: grid; gap: 2px; min-width: 0; padding: 5px 6px; \
                                 border-radius: 6px; border: 1px solid {}; background: {};",
                                if is_active { C_ACCENT_BLUE } else { "transparent" },
                                if is_active { "rgba(104,160,214,0.12)" } else { "transparent" }
                            ),
                            div {
                                style: format!(
                                    "display: flex; align-items: center; gap: 6px; min-width: 0; \
                                     font-family: {FONT_MONO};"
                                ),
                                span {
                                    style: format!(
                                        "min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; \
                                         color: {}; font-size: 10px;",
                                        if is_active { C_ACCENT_BLUE } else { C_TEXT_PRIMARY }
                                    ),
                                    "{member.name}"
                                }
                                if let Some(token) = member.metadata_token.as_ref() {
                                    span {
                                        style: format!(
                                            "flex-shrink: 0; color: {C_TEXT_MUTED}; font-size: 9px;"
                                        ),
                                        "{token}"
                                    }
                                }
                                if can_analyze {
                                    button {
                                        style: format!(
                                            "margin-left: auto; flex-shrink: 0; padding: 2px 7px; \
                                             border-radius: 999px; border: 1px solid {C_BORDER}; \
                                             background: {C_BG_SURFACE}; color: {C_TEXT_SECONDARY}; \
                                             font-size: 9px; font-weight: 700; cursor: pointer;"
                                        ),
                                        title: "Analyze member usage",
                                        onclick: move |evt| {
                                            evt.stop_propagation();
                                            open_member_analyze_tab(
                                                &analyze_type_name,
                                                analyze_member_token.clone(),
                                                open_tabs,
                                                active_tab_id,
                                                selected_finding,
                                                view_mode,
                                            );
                                        },
                                        "Analyze"
                                    }
                                }
                            }
                            div {
                                style: format!(
                                    "font-size: 9px; color: {C_TEXT_SECONDARY}; font-family: {FONT_MONO}; \
                                     overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                                ),
                                "{member.signature}"
                            }
                        }
                    }
                }
            }
            if members.len() > 8 {
                div {
                    style: format!("font-size: 9px; color: {C_TEXT_MUTED};"),
                    "+{members.len() - 8} more"
                }
            }
        }
    }
}

fn member_supports_usage_analysis(member: &UiMemberMetadata) -> bool {
    let Some(token) = member.metadata_token.as_deref() else {
        return false;
    };
    if token.trim().is_empty() {
        return false;
    }

    matches!(
        member.kind.as_str(),
        "field" | "property" | "event" | "Field" | "Property" | "Event"
    )
}

fn open_member_analyze_tab(
    type_name: &str,
    metadata_token: Option<String>,
    mut open_tabs: Signal<Vec<IlTab>>,
    mut active_tab_id: Signal<Option<String>>,
    mut selected_finding: Signal<Option<usize>>,
    mut view_mode: Signal<ViewMode>,
) {
    let tab_id = type_tab_id(type_name);
    let title = type_name
        .rsplit(['.', '/'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(type_name)
        .to_string();

    {
        let mut tabs = open_tabs.write();
        if let Some(tab) = tabs.iter_mut().find(|tab| tab.id == tab_id) {
            tab.metadata_token = metadata_token.clone();
        } else {
            tabs.push(IlTab {
                id: tab_id.clone(),
                kind: IlTabKind::Type,
                type_name: type_name.to_string(),
                method_name: None,
                metadata_token: metadata_token.clone(),
                title,
                subtitle: type_name.to_string(),
            });
        }
    }

    active_tab_id.set(Some(tab_id));
    selected_finding.set(None);
    view_mode.set(ViewMode::Analyze);
}

fn active_member_metadata_token(active: Option<&str>, type_token: Option<&str>) -> Option<String> {
    let active = active?;
    if metadata_tokens_match(Some(active), type_token) {
        None
    } else {
        Some(active.to_string())
    }
}

fn metadata_tokens_match(left: Option<&str>, right: Option<&str>) -> bool {
    normalize_metadata_token(left) == normalize_metadata_token(right)
}

fn normalize_metadata_token(token: Option<&str>) -> Option<String> {
    let mut token = token?.trim();
    token = token.strip_prefix("token:").unwrap_or(token).trim();
    token = token
        .strip_prefix("0x")
        .or_else(|| token.strip_prefix("0X"))
        .unwrap_or(token)
        .trim();

    if token.is_empty() {
        None
    } else {
        Some(token.to_ascii_lowercase())
    }
}

fn member_token_dom_id(token: &str) -> String {
    let normalized = normalize_metadata_token(Some(token)).unwrap_or_else(|| "unknown".to_string());
    format!("member-token-{normalized}")
}

#[component]
fn AttributeList(attributes: Vec<UiAttributeMetadata>) -> Element {
    if attributes.is_empty() {
        return rsx! {};
    }

    rsx! {
        div {
            style: format!(
                "min-width: 0; display: grid; gap: 5px; padding: 8px; \
                 background: {C_BG_ELEVATED}; border: 1px solid {C_BORDER}; border-radius: 7px;"
            ),
            div {
                style: "display: flex; align-items: center; justify-content: space-between; gap: 8px;",
                span {
                    style: format!(
                        "font-size: 10px; font-weight: 700; color: {C_TEXT_PRIMARY};"
                    ),
                    "Attributes"
                }
                span { class: "badge", "{attributes.len()}" }
            }
            for attribute in attributes.iter().take(8) {
                div {
                    key: "attribute-{attribute.attribute_type}-{attribute.summary.clone().unwrap_or_default()}",
                    style: "display: grid; gap: 2px; min-width: 0;",
                    div {
                        style: format!(
                            "font-size: 10px; color: {C_TEXT_PRIMARY}; font-family: {FONT_MONO}; \
                             overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                        ),
                        "{attribute.attribute_type}"
                    }
                    if let Some(summary) = attribute.summary.as_ref() {
                        div {
                            style: format!(
                                "font-size: 9px; color: {C_TEXT_MUTED}; font-family: {FONT_MONO}; \
                                 overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                            ),
                            "{summary}"
                        }
                    }
                }
            }
            if attributes.len() > 8 {
                div {
                    style: format!("font-size: 9px; color: {C_TEXT_MUTED};"),
                    "+{attributes.len() - 8} more"
                }
            }
        }
    }
}

#[component]
fn ScanNeighborhoodPanel(
    neighborhoods: Vec<UiScanNeighborhood>,
    methods: Vec<super::view_models::UiMethod>,
    open_tabs: Signal<Vec<IlTab>>,
    active_tab_id: Signal<Option<String>>,
    selected_finding: Signal<Option<usize>>,
) -> Element {
    rsx! {
        div {
            style: format!(
                "display: grid; gap: 8px; padding: 10px; background: {C_BG_SURFACE}; \
                 border: 1px solid {C_BORDER}; border-radius: 8px;"
            ),
            div {
                style: "display: flex; align-items: center; justify-content: space-between; gap: 8px;",
                span {
                    style: format!(
                        "font-size: 10px; font-weight: 700; letter-spacing: 0.08em; \
                         text-transform: uppercase; color: {C_TEXT_MUTED};"
                    ),
                    "MLVScan Neighborhoods"
                }
                span { class: "badge", "{neighborhoods.len()}" }
            }

            if neighborhoods.is_empty() {
                div {
                    style: format!(
                        "padding: 4px 0 2px; font-size: 10px; line-height: 1.45; color: {C_TEXT_MUTED};"
                    ),
                    "No MLVScan call-chain or data-flow evidence touches this symbol."
                }
            } else {
                div {
                    style: "display: grid; gap: 6px;",
                    for neighborhood in neighborhoods.iter() {
                        {
                            let sev_color = severity_color(&neighborhood.severity);
                            let click_neighborhood = neighborhood.clone();
                            let click_methods = methods.clone();
                            let node_count = neighborhood.nodes.len();
                            let first_nodes = neighborhood.nodes.iter().take(4).cloned().collect::<Vec<_>>();
                            rsx! {
                                button {
                                    key: "scan-neighborhood-{neighborhood.finding_index}-{neighborhood.rule_id}-{neighborhood.location}",
                                    style: format!(
                                        "width: 100%; min-width: 0; display: grid; gap: 6px; padding: 9px 10px; \
                                         border: 1px solid {C_BORDER}; border-left: 3px solid {sev_color}; \
                                         border-radius: 7px; background: {C_BG_ELEVATED}; color: {C_TEXT_PRIMARY}; \
                                         text-align: left; cursor: pointer;"
                                    ),
                                    onclick: move |_| {
                                        selected_finding.set(Some(click_neighborhood.finding_index));
                                        if let (Some(type_name), Some(method_name)) = (
                                            click_neighborhood.primary_type_name.as_ref(),
                                            click_neighborhood.primary_method_name.as_ref(),
                                        ) {
                                            if let Some((resolved_type, resolved_method)) =
                                                resolve_method_reference(&click_methods, type_name, method_name)
                                            {
                                                let tab_id = method_tab_id(&resolved_type, &resolved_method);
                                                {
                                                    let mut tabs = open_tabs.write();
                                                    if !tabs.iter().any(|tab| tab.id == tab_id) {
                                                        tabs.push(IlTab {
                                                            id: tab_id.clone(),
                                                            kind: IlTabKind::Method,
                                                            type_name: resolved_type.clone(),
                                                            method_name: Some(resolved_method.clone()),
                                                            metadata_token: None,
                                                            title: resolved_method.clone(),
                                                            subtitle: resolved_type.clone(),
                                                        });
                                                    }
                                                }
                                                active_tab_id.set(Some(tab_id));
                                            }
                                        }
                                    },
                                    div {
                                        style: "display: flex; align-items: center; justify-content: space-between; gap: 8px; min-width: 0;",
                                        span {
                                            style: format!(
                                                "min-width: 0; font-size: 10px; font-weight: 700; color: {C_TEXT_PRIMARY}; \
                                                 font-family: {FONT_MONO}; overflow: hidden; text-overflow: ellipsis; \
                                                 white-space: nowrap;"
                                            ),
                                            "{neighborhood.rule_id}"
                                        }
                                        div {
                                            style: "display: flex; gap: 6px; flex-shrink: 0;",
                                            span {
                                                class: "badge",
                                                style: format!("border-color: {sev_color}; color: {sev_color};"),
                                                "{neighborhood.severity}"
                                            }
                                            span { class: "badge", "{neighborhood.chain_kind}" }
                                            span { class: "badge", "{node_count} nodes" }
                                        }
                                    }
                                    div {
                                        style: format!(
                                            "font-size: 10px; color: {C_TEXT_SECONDARY}; font-family: {FONT_MONO}; \
                                             overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                                        ),
                                        "{neighborhood.title}"
                                    }
                                    div {
                                        style: format!(
                                            "font-size: 10px; color: {C_TEXT_MUTED}; line-height: 1.45; \
                                             overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                                        ),
                                        "{neighborhood.description}"
                                    }
                                    div {
                                        style: "display: grid; gap: 4px;",
                                        for node in first_nodes.iter() {
                                            {
                                                let offset = node
                                                    .instruction_offset
                                                    .map(|offset| format!("IL_{offset:04X}"))
                                                    .unwrap_or_default();
                                                rsx! {
                                                    div {
                                                        key: "scan-node-{neighborhood.finding_index}-{node.location}-{node.node_type}-{offset}",
                                                        style: format!(
                                                            "display: grid; grid-template-columns: minmax(62px, 0.35fr) minmax(0, 1fr); \
                                                             gap: 7px; font-size: 9px; color: {C_TEXT_MUTED}; font-family: {FONT_MONO};"
                                                        ),
                                                        span {
                                                            style: "overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                                            if offset.is_empty() {
                                                                "{node.node_type}"
                                                            } else {
                                                                "{offset}"
                                                            }
                                                        }
                                                        span {
                                                            style: "min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                                            if node.operation.is_empty() {
                                                                "{node.location} - {node.description}"
                                                            } else {
                                                                "{node.location} - {node.operation} - {node.description}"
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
fn AnalyzeDepthControl(current_depth: i32, mut analyze_depth: Signal<i32>) -> Element {
    rsx! {
        div {
            style: format!(
                "display: flex; align-items: center; gap: 6px; margin-top: 9px; \
                 color: {C_TEXT_MUTED}; font-size: 10px;"
            ),
            span {
                style: format!(
                    "font-size: 9px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; \
                     color: {C_TEXT_MUTED};"
                ),
                "Depth"
            }
            for depth in [1_i32, 2, 3] {
                {
                    let selected = current_depth == depth;
                    rsx! {
                        button {
                            key: "analyze-depth-{depth}",
                            style: format!(
                                "min-width: 24px; height: 20px; padding: 0 7px; border-radius: 5px; \
                                 border: 1px solid {}; background: {}; color: {}; font-size: 10px; \
                                 font-weight: 700; cursor: pointer;",
                                if selected { C_ACCENT_GREEN } else { C_BORDER },
                                if selected { "rgba(122,162,120,0.14)" } else { C_BG_SURFACE },
                                if selected { C_TEXT_PRIMARY } else { C_TEXT_SECONDARY }
                            ),
                            onclick: move |_| analyze_depth.set(depth),
                            "{depth}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SymbolReferenceList(
    title: String,
    empty_text: String,
    references: Vec<crate::ipc::SymbolReferenceEntry>,
    open_tabs: Signal<Vec<IlTab>>,
    active_tab_id: Signal<Option<String>>,
    selected_finding: Signal<Option<usize>>,
) -> Element {
    rsx! {
        div {
            style: format!(
                "min-width: 0; display: flex; flex-direction: column; gap: 7px; \
                 padding: 10px; background: {C_BG_SURFACE}; border: 1px solid {C_BORDER}; \
                 border-radius: 8px;"
            ),
            div {
                style: "display: flex; align-items: center; justify-content: space-between; gap: 8px;",
                span {
                    style: format!(
                        "font-size: 10px; font-weight: 700; letter-spacing: 0.08em; \
                         text-transform: uppercase; color: {C_TEXT_MUTED};"
                    ),
                    "{title}"
                }
                span { class: "badge", "{references.len()}" }
            }

            if references.is_empty() {
                div {
                    style: format!(
                        "padding: 8px 0 2px; font-size: 10px; line-height: 1.45; color: {C_TEXT_MUTED};"
                    ),
                    "{empty_text}"
                }
            } else {
                div {
                    style: "display: grid; gap: 6px;",
                    for reference in references.iter() {
                        {
                            let type_name = reference.type_name.clone();
                            let method_name = reference.method_name.clone();
                            let tab_title = method_name.clone();
                            let tab_subtitle = type_name.clone();
                            let location = format_symbol_reference_location(reference);
                            let display_name = symbol_reference_display_name(reference);
                            let operand = reference.operand.clone().unwrap_or_default();
                            rsx! {
                                button {
                                    key: "{display_name}-{location}-{operand}",
                                    style: format!(
                                        "width: 100%; min-width: 0; display: grid; gap: 3px; padding: 8px 9px; \
                                         border: 1px solid {C_BORDER}; border-radius: 7px; background: {C_BG_ELEVATED}; \
                                         color: {C_TEXT_PRIMARY}; text-align: left; cursor: pointer;"
                                    ),
                                    onclick: move |_| {
                                        let tab_id = method_tab_id(&type_name, &method_name);
                                        {
                                            let mut tabs = open_tabs.write();
                                            if !tabs.iter().any(|tab| tab.id == tab_id) {
                                                tabs.push(IlTab {
                                                    id: tab_id.clone(),
                                                    kind: IlTabKind::Method,
                                                    type_name: type_name.clone(),
                                                    method_name: Some(method_name.clone()),
                                                    metadata_token: None,
                                                    title: tab_title.clone(),
                                                    subtitle: tab_subtitle.clone(),
                                                });
                                            }
                                        }
                                        active_tab_id.set(Some(tab_id));
                                        selected_finding.set(None);
                                    },
                                    div {
                                        style: format!(
                                            "min-width: 0; font-size: 10px; font-weight: 700; \
                                             color: {C_TEXT_PRIMARY}; font-family: {FONT_MONO}; \
                                             overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                                        ),
                                        span { "{display_name}" }
                                        span {
                                            style: format!(
                                                "margin-left: 7px; color: {C_TEXT_MUTED}; font-size: 9px;"
                                            ),
                                            "d{reference.depth}"
                                        }
                                    }
                                    div {
                                        style: format!(
                                            "font-size: 9px; color: {C_TEXT_SECONDARY}; font-family: {FONT_MONO}; \
                                             overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                                        ),
                                        "{reference.signature}"
                                    }
                                    div {
                                        style: format!(
                                            "display: flex; align-items: center; gap: 8px; min-width: 0; \
                                             color: {C_TEXT_MUTED}; font-size: 9px; font-family: {FONT_MONO};"
                                        ),
                                        span { style: "flex-shrink: 0;", "{location}" }
                                        if !operand.is_empty() {
                                            span {
                                                style: "min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                                "{operand}"
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
fn SymbolEvidencePanel(evidence: Vec<crate::ipc::SymbolEvidenceEntry>) -> Element {
    let groups = group_symbol_evidence_by_category(&evidence);

    rsx! {
        div {
            style: format!(
                "display: grid; gap: 8px; padding: 10px; background: {C_BG_SURFACE}; \
                 border: 1px solid {C_BORDER}; border-radius: 8px;"
            ),
            div {
                style: "display: flex; align-items: center; justify-content: space-between; gap: 8px;",
                span {
                    style: format!(
                        "font-size: 10px; font-weight: 700; letter-spacing: 0.08em; \
                         text-transform: uppercase; color: {C_TEXT_MUTED};"
                    ),
                    "Static Evidence"
                }
                span { class: "badge", "{evidence.len()}" }
            }

            if evidence.is_empty() {
                div {
                    style: format!(
                        "padding: 4px 0 2px; font-size: 10px; line-height: 1.45; color: {C_TEXT_MUTED};"
                    ),
                    "No strings, allocations, field access, P/Invoke, or malware-relevant API hints were found in this scope."
                }
            } else {
                for (category, entries) in groups {
                    div {
                        key: "evidence-group-{category}",
                        style: "display: grid; gap: 6px;",
                        div {
                            style: "display: flex; align-items: center; gap: 8px;",
                            span {
                                style: format!(
                                    "font-size: 10px; font-weight: 700; color: {C_TEXT_PRIMARY};"
                                ),
                                "{evidence_category_label(&category)}"
                            }
                            span { class: "badge", "{entries.len()}" }
                        }
                        div {
                            style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 6px;",
                            for entry in entries.iter() {
                                {
                                    let location = format_symbol_evidence_location(entry);
                                    let value = symbol_evidence_display_value(entry);
                                    rsx! {
                                        div {
                                            key: "evidence-{entry.category}-{entry.type_name}-{entry.method_name}-{location}-{value}",
                                            style: format!(
                                                "min-width: 0; display: grid; gap: 3px; padding: 8px 9px; \
                                                 border: 1px solid {C_BORDER}; border-radius: 7px; background: {C_BG_ELEVATED};"
                                            ),
                                            div {
                                                style: format!(
                                                    "display: flex; align-items: center; justify-content: space-between; gap: 8px;"
                                                ),
                                                span {
                                                    style: format!(
                                                        "min-width: 0; font-size: 10px; font-weight: 700; \
                                                         color: {C_TEXT_PRIMARY}; overflow: hidden; text-overflow: ellipsis; \
                                                         white-space: nowrap;"
                                                    ),
                                                    "{entry.label}"
                                                }
                                                span {
                                                    style: format!(
                                                        "flex-shrink: 0; font-size: 9px; color: {C_TEXT_MUTED}; \
                                                         font-family: {FONT_MONO};"
                                                    ),
                                                    "{location}"
                                                }
                                            }
                                            div {
                                                style: format!(
                                                    "font-size: 10px; color: {C_TEXT_SECONDARY}; font-family: {FONT_MONO}; \
                                                     overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                                                ),
                                                "{value}"
                                            }
                                            div {
                                                style: format!(
                                                    "font-size: 9px; color: {C_TEXT_MUTED}; font-family: {FONT_MONO}; \
                                                     overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                                                ),
                                                "{entry.type_name}.{entry.method_name}"
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

fn group_symbol_evidence_by_category(
    evidence: &[crate::ipc::SymbolEvidenceEntry],
) -> Vec<(String, Vec<crate::ipc::SymbolEvidenceEntry>)> {
    let mut groups =
        std::collections::BTreeMap::<String, Vec<crate::ipc::SymbolEvidenceEntry>>::new();
    for entry in evidence {
        groups
            .entry(entry.category.clone())
            .or_default()
            .push(entry.clone());
    }
    groups.into_iter().collect()
}

fn evidence_category_label(category: &str) -> &'static str {
    match category {
        "allocation" => "Allocations",
        "field-read" => "Field Reads",
        "field-write" => "Field Writes",
        "field-reference" => "Field References",
        "property-get" => "Property Gets",
        "property-set" => "Property Sets",
        "event-add" => "Event Subscriptions",
        "event-remove" => "Event Unsubscriptions",
        "pinvoke" => "P/Invoke",
        "resource-read" => "Resource Reads",
        "string" => "Strings",
        "suspicious-call" => "Review Hints",
        _ => "Other",
    }
}

fn is_redundant_type_member_method(method: &super::view_models::UiMethod) -> bool {
    method.method_name.starts_with("get_")
        || method.method_name.starts_with("set_")
        || method.method_name.starts_with("add_")
        || method.method_name.starts_with("remove_")
}

#[cfg(test)]
mod tests {
    use crate::components::view_models::UiMemberMetadata;

    #[test]
    fn active_member_metadata_token_ignores_type_token_and_normalizes_member_id() {
        assert_eq!(
            super::active_member_metadata_token(Some("0x02000004"), Some("02000004")),
            None
        );
        assert_eq!(
            super::active_member_metadata_token(Some("token: 0X04000001"), Some("0x02000004"))
                .as_deref(),
            Some("token: 0X04000001")
        );
        assert_eq!(
            super::member_token_dom_id("token: 0X04000001"),
            "member-token-04000001"
        );
    }

    #[test]
    fn evidence_category_label_names_member_usage_groups() {
        assert_eq!(
            super::evidence_category_label("property-get"),
            "Property Gets"
        );
        assert_eq!(
            super::evidence_category_label("event-remove"),
            "Event Unsubscriptions"
        );
        assert_eq!(
            super::evidence_category_label("field-reference"),
            "Field References"
        );
    }

    #[test]
    fn member_usage_analysis_is_limited_to_supported_member_tokens() {
        let field = UiMemberMetadata {
            name: "_config".to_string(),
            metadata_token: Some("0x04000001".to_string()),
            kind: "field".to_string(),
            signature: "string _config".to_string(),
            attributes: None,
        };
        let property = UiMemberMetadata {
            name: "Config".to_string(),
            metadata_token: Some("0x17000001".to_string()),
            kind: "property".to_string(),
            signature: "string Config".to_string(),
            attributes: None,
        };
        let event = UiMemberMetadata {
            name: "Changed".to_string(),
            metadata_token: Some("0x14000001".to_string()),
            kind: "event".to_string(),
            signature: "event EventHandler Changed".to_string(),
            attributes: None,
        };
        let nested_type = UiMemberMetadata {
            name: "Nested".to_string(),
            metadata_token: Some("0x02000002".to_string()),
            kind: "nested-type".to_string(),
            signature: "class Nested".to_string(),
            attributes: None,
        };
        let missing_token = UiMemberMetadata {
            metadata_token: None,
            ..field.clone()
        };

        assert!(super::member_supports_usage_analysis(&field));
        assert!(super::member_supports_usage_analysis(&property));
        assert!(super::member_supports_usage_analysis(&event));
        assert!(!super::member_supports_usage_analysis(&nested_type));
        assert!(!super::member_supports_usage_analysis(&missing_token));
    }
}
