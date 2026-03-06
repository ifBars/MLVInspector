use dioxus::prelude::*;
use dioxus::html::HasFileData;
use dioxus_desktop::tao::event::{Event, WindowEvent};
use dioxus_desktop::{use_wry_event_handler, window};
use std::collections::{BTreeMap, BTreeSet};

use crate::ipc::{ExploreParams, ScanParams};
use crate::state::AppState;
use crate::types::{ActiveMode, AnalysisEntry, AnalysisResult, AnalysisStatus};

// ─── Domain view models ──────────────────────────────────────────────────────

#[derive(Clone)]
struct UiMethod {
    type_name: String,
    method_name: String,
    signature: String,
    instructions: Vec<UiInstruction>,
}

#[derive(Clone)]
struct UiInstruction {
    offset: i64,
    op_code: String,
    operand: String,
}

#[derive(Clone)]
struct UiFinding {
    rule_id: String,
    severity: String,
    location: String,
    description: String,
    code_snippet: String,
    il_offset: Option<i64>,
}

#[derive(Clone)]
struct UiTypeGroup {
    full_type_name: String,
    display_name: String,
    methods: Vec<UiMethod>,
}

#[derive(Clone)]
struct UiNamespaceGroup {
    namespace_name: String,
    types: Vec<UiTypeGroup>,
}

#[derive(Clone, PartialEq, Eq)]
enum IlTabKind {
    Type,
    Method,
}

#[derive(Clone)]
struct IlTab {
    id: String,
    kind: IlTabKind,
    type_name: String,
    method_name: Option<String>,
    title: String,
    subtitle: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ResizeTarget {
    Assemblies,
    Explorer,
    Findings,
}

#[derive(Clone, Copy)]
struct ActiveResize {
    target: ResizeTarget,
    start_x: f64,
    start_width: f64,
}

fn clamp_panel_width(target: ResizeTarget, width: f64) -> f64 {
    let (min_width, max_width) = match target {
        ResizeTarget::Assemblies => (180.0, 420.0),
        ResizeTarget::Explorer => (220.0, 520.0),
        ResizeTarget::Findings => (240.0, 520.0),
    };

    width.clamp(min_width, max_width)
}

// ─── Inline CSS constants ─────────────────────────────────────────────────────

// Design tokens
const C_BG_BASE: &str = "#101113";
const C_BG_SURFACE: &str = "#17191c";
const C_BG_ELEVATED: &str = "#1e2126";
const C_BORDER: &str = "#2d3138";
const C_BORDER_ACCENT: &str = "#3b4048";
const C_ACCENT_GREEN: &str = "#d4d4d8";
const C_ACCENT_BLUE: &str = "#a1a1aa";
const C_ACCENT_AMBER: &str = "#b8b8b0";
const C_TEXT_PRIMARY: &str = "#f5f5f5";
const C_TEXT_SECONDARY: &str = "#b4b8c0";
const C_TEXT_MUTED: &str = "#7d828d";
const FONT_SANS: &str = "'IBM Plex Sans', 'Segoe UI', system-ui, sans-serif";
const FONT_MONO: &str = "'JetBrains Mono', 'Cascadia Code', 'Consolas', monospace";

// ─── Root component ───────────────────────────────────────────────────────────

#[component]
pub fn App() -> Element {
    let state = use_context_provider(AppState::default);
    let mut open_tabs = use_signal(Vec::<IlTab>::new);
    let mut active_tab_id = use_signal(|| None::<String>);
    let mut selected_finding = use_signal(|| None::<usize>);
    let mut last_error = use_signal(String::new);
    let mut show_scan_panel = use_signal(|| true);
    let mut collapsed_assemblies = use_signal(BTreeSet::<String>::new);
    let mut collapsed_namespaces = use_signal(BTreeSet::<String>::new);
    let mut collapsed_types = use_signal(BTreeSet::<String>::new);
    let desktop_window = window();
    let mut is_fullscreen = use_signal(|| false);
    let mut highlighted_il_offset = use_signal(|| None::<i64>);
    let mut drag_counter = use_signal(|| 0);
    let mut is_dragging_over = use_signal(|| false);
    let mut assemblies_width = use_signal(|| 220.0);
    let mut explorer_width = use_signal(|| 260.0);
    let mut findings_width = use_signal(|| 300.0);
    let mut active_resize = use_signal(|| None::<ActiveResize>);

    // Scroll to highlighted IL offset when method changes
    use_effect(move || {
        let offset = *highlighted_il_offset.read();
        if let Some(off) = offset {
            spawn(async move {
                // Give time for the DOM to render the method
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                let js = format!("const el = document.getElementById('il-{off}'); if (el) el.scrollIntoView({{behavior:'smooth',block:'center'}});");
                let _ = document::eval(&js).await;
            });
        }
    });

    // Load rules on startup
    use_hook(move || {
        let worker = state.worker.read().clone();
        spawn(async move {
            match worker.list_rules().await {
                Ok(rules) => {
                    let converted = rules
                        .into_iter()
                        .map(|r| crate::types::RuleInfo {
                            rule_id:     r.rule_id,
                            description: r.description,
                            severity:    match r.severity.as_str() {
                                "Critical" => crate::types::RuleSeverity::Critical,
                                "High"     => crate::types::RuleSeverity::High,
                                "Medium"   => crate::types::RuleSeverity::Medium,
                                "Low"      => crate::types::RuleSeverity::Low,
                                "Info"     => crate::types::RuleSeverity::Info,
                                _          => crate::types::RuleSeverity::Unknown,
                            },
                        })
                        .collect();
                    state.set_rules(converted);
                }
                Err(e) => last_error.set(e.to_string()),
            }
        });
    });

    let _drag_drop_handler = use_wry_event_handler(move |event, _| match event {
        Event::WindowEvent {
            event: WindowEvent::HoveredFile(_),
            ..
        } => {
            is_dragging_over.set(true);
        }
        Event::WindowEvent {
            event: WindowEvent::HoveredFileCancelled,
            ..
        } => {
            drag_counter.set(0);
            is_dragging_over.set(false);
        }
        Event::WindowEvent {
            event: WindowEvent::DroppedFile(path),
            ..
        } => {
            drag_counter.set(0);
            is_dragging_over.set(false);

            let file_path = path.display().to_string();
            state.open_assembly(file_path.clone());
            open_tabs.write().clear();
            active_tab_id.set(None);
            highlighted_il_offset.set(None);

            if let Some(assembly_id) = state.selected_id.read().clone() {
                run_analysis(state, last_error, assembly_id, file_path);
            }
        }
        Event::WindowEvent { .. } => {}
        _ => {}
    });

    let assemblies = state.assemblies.read().clone();
    let selected_id = state.selected_id.read().clone();
    let rules_count = state.rules.read().len();
    let is_running = *state.is_running.read();

    let (methods, findings) = if let Some(id) = selected_id.clone() {
        let explore_key = format!("{id}::explore");
        let scan_key = format!("{id}::scan");
        let m = state
            .get_analysis_entry(&explore_key)
            .as_ref()
            .and_then(|e| e.result.as_ref())
            .map(extract_methods)
            .unwrap_or_default();
        let f = state
            .get_analysis_entry(&scan_key)
            .as_ref()
            .and_then(|e| e.result.as_ref())
            .map(extract_findings)
            .unwrap_or_default();
        (m, f)
    } else {
        (Vec::new(), Vec::new())
    };

    let active_tab = {
        let id = active_tab_id.read().clone();
        id.and_then(|tab_id| open_tabs.read().iter().find(|tab| tab.id == tab_id).cloned())
    };
    let selected_method_name = active_tab.as_ref().and_then(|tab| {
        tab.method_name
            .as_ref()
            .map(|method_name| format!("{}::{}", tab.type_name, method_name))
    });
    let selected_type_name = active_tab.as_ref().map(|tab| tab.type_name.clone());
    let active_method = active_tab.as_ref().and_then(|tab| {
        tab.method_name.as_ref().and_then(|method_name| {
            methods
                .iter()
                .find(|m| m.type_name == tab.type_name && m.method_name == *method_name)
                .cloned()
        })
    });

    let selected_assembly = selected_id
        .as_ref()
        .and_then(|id| assemblies.iter().find(|asm| asm.id == *id));
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

    let selected_finding_index = selected_finding.read().unwrap_or(0);
    let active_finding = findings.get(selected_finding_index).cloned();

    let methods_count = methods.len();
    let grouped_methods = group_methods_by_namespace(&methods);
    let class_count = grouped_methods.iter().map(|ns| ns.types.len()).sum::<usize>();
    let namespace_count = grouped_methods.len();
    let findings_count = findings.len();
    let tabs = open_tabs.read().clone();
    let active_tab_id_value = active_tab_id.read().clone();
    let resize_state = *active_resize.read();
    let is_resizing = resize_state.is_some();
    let is_resizing_assemblies = matches!(
        resize_state,
        Some(ActiveResize {
            target: ResizeTarget::Assemblies,
            ..
        })
    );
    let is_resizing_explorer = matches!(
        resize_state,
        Some(ActiveResize {
            target: ResizeTarget::Explorer,
            ..
        })
    );
    let is_resizing_findings = matches!(
        resize_state,
        Some(ActiveResize {
            target: ResizeTarget::Findings,
            ..
        })
    );
    let desktop_window_min = desktop_window.clone();
    let desktop_window_full = desktop_window.clone();
    let desktop_window_close = desktop_window.clone();

    rsx! {
        // Inject Google Fonts
        style {
            r#"
            @import url('https://fonts.googleapis.com/css2?family=IBM+Plex+Sans:wght@300;400;500;600;700&family=JetBrains+Mono:wght@400;500;600&display=swap');

            * {{ box-sizing: border-box; margin: 0; padding: 0; }}

            ::-webkit-scrollbar {{ width: 5px; height: 5px; }}
            ::-webkit-scrollbar-track {{ background: transparent; }}
            ::-webkit-scrollbar-thumb {{ background: #3b4048; border-radius: 3px; }}
            ::-webkit-scrollbar-thumb:hover {{ background: #5a606a; }}

            .btn {{
                display: inline-flex;
                align-items: center;
                gap: 6px;
                border-radius: 8px;
                padding: 6px 14px;
                font-size: 12px;
                font-weight: 500;
                font-family: inherit;
                cursor: pointer;
                transition: all 150ms ease;
                outline: none;
                text-decoration: none;
                white-space: nowrap;
            }}
            .btn-ghost {{
                border: 1px solid #2d3138;
                background: transparent;
                color: #b4b8c0;
            }}
            .btn-ghost:hover {{
                border-color: #5a606a;
                background: rgba(245,245,245,0.05);
                color: #f5f5f5;
            }}
            .btn-primary {{
                border: 1px solid #5a606a;
                background: rgba(245,245,245,0.08);
                color: #eceef2;
            }}
            .btn-primary:hover {{
                background: rgba(245,245,245,0.14);
                color: #ffffff;
            }}
            .btn-danger {{
                border: 1px solid #5a3f43;
                background: transparent;
                color: #caa0a6;
            }}
            .btn-danger:hover {{
                background: rgba(168,97,107,0.2);
                color: #e5c4c8;
            }}

            .toolbar {{
                display: inline-flex;
                align-items: center;
                gap: 4px;
                padding: 3px;
                border: 1px solid #2d3138;
                border-radius: 8px;
                background: #101113;
            }}
            .tool-btn {{
                width: 28px;
                height: 24px;
                border: 1px solid transparent;
                border-radius: 6px;
                background: transparent;
                color: #b4b8c0;
                display: inline-flex;
                align-items: center;
                justify-content: center;
                cursor: pointer;
                transition: all 120ms ease;
            }}
            .tool-btn:hover {{
                border-color: #3b4048;
                background: #1e2126;
                color: #f5f5f5;
            }}
            .tool-btn.active {{
                border-color: #4f5b6c;
                background: rgba(245,245,245,0.08);
                color: #f5f5f5;
            }}

            .il-tabs {{
                display: flex;
                align-items: stretch;
                gap: 4px;
                overflow-x: auto;
                padding: 8px 10px 7px;
                background: #141619;
            }}
            .il-tab {{
                min-width: 140px;
                max-width: 220px;
                display: inline-flex;
                align-items: center;
                gap: 8px;
                padding: 6px 8px 6px 10px;
                border-radius: 8px;
                border: 1px solid #2d3138;
                background: #17191c;
                color: #b4b8c0;
                cursor: pointer;
                transition: all 120ms ease;
            }}
            .il-tab:hover {{
                border-color: #3b4048;
                background: #1e2126;
            }}
            .il-tab.active {{
                border-color: #5a606a;
                background: rgba(245,245,245,0.09);
                color: #f5f5f5;
            }}
            .tab-close {{
                width: 16px;
                height: 16px;
                border-radius: 4px;
                border: 1px solid transparent;
                background: transparent;
                color: #7d828d;
                display: inline-flex;
                align-items: center;
                justify-content: center;
                cursor: pointer;
                transition: all 120ms ease;
                flex-shrink: 0;
            }}
            .tab-close:hover {{
                border-color: #5a606a;
                background: #101113;
                color: #e2e5eb;
            }}

            .panel-header {{
                font-size: 10px;
                font-weight: 700;
                line-height: 1;
                letter-spacing: 1.2px;
                text-transform: uppercase;
                color: #7d828d;
                padding: 14px 14px 10px;
                min-height: 39px;
                box-sizing: border-box;
                border-bottom: 1px solid #2d3138;
                display: flex;
                align-items: center;
                justify-content: space-between;
                flex-shrink: 0;
            }}
            .badge {{
                font-size: 10px;
                font-weight: 600;
                padding: 1px 7px;
                border-radius: 999px;
                background: #1e2126;
                color: #8b919d;
                border: 1px solid #2d3138;
            }}
            .panel-header-detail {{
                display: inline-flex;
                align-items: center;
                min-width: 0;
                max-width: 320px;
                font-family: {FONT_MONO};
                font-weight: 600;
                letter-spacing: 0;
                text-transform: none;
                overflow: hidden;
                text-overflow: ellipsis;
                white-space: nowrap;
            }}
            .resize-handle {{
                width: 8px;
                flex-shrink: 0;
                position: relative;
                margin-left: -4px;
                margin-right: -4px;
                z-index: 2;
                cursor: col-resize;
                background: transparent;
            }}
            .resize-handle::after {{
                content: "";
                position: absolute;
                top: 0;
                bottom: 0;
                left: 50%;
                transform: translateX(-50%);
                width: 1px;
                background: #2d3138;
                transition: background 120ms ease, width 120ms ease;
            }}
            .resize-handle:hover::after,
            .resize-handle.active::after {{
                background: #5a606a;
                width: 2px;
            }}

            .asm-item {{
                margin: 0 8px 6px;
                border-radius: 10px;
                border: 1px solid #2d3138;
                background: #17191c;
                padding: 9px 10px;
                cursor: pointer;
                transition: all 150ms ease;
                text-align: left;
                width: calc(100% - 16px);
            }}
            .asm-item:hover {{
                border-color: #3b4048;
                background: #1e2126;
            }}
            .asm-item.selected {{
                border-color: #8f96a2;
                background: rgba(245,245,245,0.06);
            }}

            .method-item {{
                margin: 0 8px 5px;
                border-radius: 8px;
                border: 1px solid transparent;
                background: transparent;
                padding: 8px 10px;
                cursor: pointer;
                transition: all 120ms ease;
                text-align: left;
                width: calc(100% - 16px);
            }}
            .method-item:hover {{
                border-color: #2d3138;
                background: #1e2126;
            }}
            .method-item.selected {{
                border-color: #8f96a266;
                background: rgba(245,245,245,0.05);
            }}

            .finding-item {{
                margin: 0 8px 6px;
                border-radius: 8px;
                border: 1px solid #2d3138;
                background: #17191c;
                padding: 9px 10px;
                cursor: pointer;
                transition: all 120ms ease;
                text-align: left;
                width: calc(100% - 16px);
            }}
            .finding-item:hover {{
                border-color: #3b4048;
                background: #1e2126;
            }}
            .finding-item.selected {{
                border-color: #90908a99;
                background: rgba(245,245,245,0.05);
            }}

            .sev-badge {{
                font-size: 9px;
                font-weight: 700;
                letter-spacing: 0.5px;
                text-transform: uppercase;
                padding: 2px 7px;
                border-radius: 999px;
                border: 1px solid currentColor;
            }}

            .il-row {{
                display: grid;
                grid-template-columns: 72px 120px 1fr;
                gap: 12px;
                padding: 3px 6px;
                border-radius: 4px;
                font-size: 12px;
                line-height: 1.7;
                transition: background 80ms;
            }}
            .il-row:hover {{
                background: rgba(245,245,245,0.04);
            }}
            .il-row.highlighted {{
                background: rgba(100, 180, 255, 0.12);
            }}

            .empty-state {{
                display: flex;
                flex-direction: column;
                align-items: center;
                justify-content: center;
                height: 100%;
                gap: 10px;
                padding: 32px 16px;
                color: #6f7580;
                text-align: center;
            }}
            .empty-state svg {{
                opacity: 0.35;
            }}
            .empty-state p {{
                font-size: 12px;
                line-height: 1.5;
                color: #7d828d;
                max-width: 160px;
            }}

            .pulse {{
                animation: pulse 1.5s cubic-bezier(0.4, 0, 0.6, 1) infinite;
            }}
            @keyframes pulse {{
                0%, 100% {{ opacity: 1; }}
                50% {{ opacity: 0.4; }}
            }}

            .drag-region {{
                -webkit-app-region: drag;
            }}
            .no-drag {{
                -webkit-app-region: no-drag;
            }}

            .drop-overlay {{
                position: absolute;
                inset: 0;
                background: rgba(30, 33, 38, 0.85);
                backdrop-filter: blur(2px);
                display: flex;
                flex-direction: column;
                align-items: center;
                justify-content: center;
                z-index: 1000;
                pointer-events: none;
                opacity: 0;
                transition: opacity 200ms ease;
            }}
            .drop-overlay.visible {{
                opacity: 1;
            }}
            .drop-zone {{
                border: 2px dashed #3b4048;
                border-radius: 16px;
                padding: 60px 80px;
                background: rgba(23, 25, 28, 0.9);
                display: flex;
                flex-direction: column;
                align-items: center;
                gap: 16px;
            }}
            .drop-zone.drag-over {{
                border-color: #5a606a;
                background: rgba(45, 49, 56, 0.95);
            }}
            "#
        }

        div {
            style: format!(
                "width: 100vw; height: 100vh; display: flex; flex-direction: column; \
                 background: {C_BG_BASE}; color: {C_TEXT_PRIMARY}; font-family: {FONT_SANS}; \
                 overflow: hidden; position: relative;"
            ),
            ondragenter: move |evt| {
                evt.stop_propagation();
                let count = *drag_counter.read();
                drag_counter.set(count + 1);
                is_dragging_over.set(true);
            },
            ondragleave: move |evt| {
                evt.stop_propagation();
                let count = *drag_counter.read();
                if count > 0 {
                    let new_count = count - 1;
                    drag_counter.set(new_count);
                    if new_count == 0 {
                        is_dragging_over.set(false);
                    }
                }
            },
            ondragover: move |evt| {
                evt.prevent_default();
                evt.stop_propagation();
            },
            ondrop: move |evt| {
                evt.prevent_default();
                evt.stop_propagation();
                drag_counter.set(0);
                is_dragging_over.set(false);

                let dropped_files = evt.files();

                let mut opened_any = false;
                for file in dropped_files.iter() {
                    let path = file.path();

                    let file_path = path.display().to_string();
                    state.open_assembly(file_path.clone());
                    open_tabs.write().clear();
                    active_tab_id.set(None);
                    highlighted_il_offset.set(None);

                    if let Some(assembly_id) = state.selected_id.read().clone() {
                        run_analysis(state, last_error, assembly_id, file_path);
                        opened_any = true;
                        break;
                    }
                }

                if !opened_any {
                    last_error.set("No file could be opened from drop".to_string());
                }
            },

            // ── Drop overlay ───────────────────────────────────────────────────
            if is_dragging_over() {
                div {
                    class: "drop-overlay visible",
                    div {
                        class: "drop-zone",
                        svg {
                            width: "64",
                            height: "64",
                            view_box: "0 0 24 24",
                            fill: "none",
                            stroke: C_ACCENT_BLUE,
                            stroke_width: "1.5",
                            path {
                                d: "M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"
                            }
                            polyline {
                                points: "17 8 12 3 7 8"
                            }
                            line {
                                x1: "12",
                                y1: "3",
                                x2: "12",
                                y2: "15"
                            }
                        }
                        span {
                            style: format!(
                                "font-size: 16px; font-weight: 600; color: {C_TEXT_PRIMARY};"
                            ),
                            "Drop assembly to open"
                        }
                        span {
                            style: format!(
                                "font-size: 12px; color: {C_TEXT_MUTED};"
                            ),
                            "Supports .dll and .exe files"
                        }
                    }
                }
            }

            // ── Title bar ──────────────────────────────────────────────────────
            div {
                class: "drag-region",
                style: format!(
                    "height: 42px; flex-shrink: 0; display: flex; align-items: center; \
                     justify-content: space-between; padding: 0 14px; \
                     background: {C_BG_SURFACE}; \
                     border-bottom: 1px solid {C_BORDER};"
                ),

                // Left: identity
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

                    div { style: format!("width: 1px; height: 18px; background: {C_BORDER}; margin: 0 2px;") }

                    div {
                        class: "no-drag toolbar",
                        button {
                            class: if show_scan_panel() { "tool-btn active" } else { "tool-btn" },
                            title: if show_scan_panel() { "Hide Findings" } else { "Show Findings" },
                            "aria-label": if show_scan_panel() { "Hide Findings" } else { "Show Findings" },
                            onclick: move |_| show_scan_panel.toggle(),
                            svg { width: "13", height: "13", view_box: "0 0 24 24", fill: "none",
                                stroke: "currentColor", stroke_width: "2",
                                rect { x: "3", y: "4", width: "18", height: "16", rx: "2", ry: "2" }
                                line { x1: "15", y1: "4", x2: "15", y2: "20" }
                            }
                        }

                        button {
                            class: "tool-btn",
                            title: "Clear Workspace",
                            "aria-label": "Clear Workspace",
                            onclick: move |_| {
                                state.clear_all();
                                open_tabs.write().clear();
                                active_tab_id.set(None);
                            },
                            svg { width: "13", height: "13", view_box: "0 0 24 24", fill: "none",
                                stroke: "currentColor", stroke_width: "2",
                                polyline { points: "3 6 5 6 21 6" }
                                path { d: "M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6" }
                            }
                        }

                        button {
                            class: "tool-btn",
                            title: "Open Assembly",
                            "aria-label": "Open Assembly",
                            onclick: move |_| {
                                if let Some(path) = rfd::FileDialog::new().pick_file()
                                {
                                    let file_path = path.display().to_string();
                                    state.open_assembly(file_path.clone());
                                    open_tabs.write().clear();
                                    active_tab_id.set(None);
                                    let id = state.selected_id.read().clone();
                                    if let Some(assembly_id) = id {
                                        run_analysis(state, last_error, assembly_id, file_path);
                                    }
                                }
                            },
                            svg { width: "13", height: "13", view_box: "0 0 24 24", fill: "none",
                                stroke: "currentColor", stroke_width: "2",
                                path { d: "M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" }
                                polyline { points: "17 8 12 3 7 8" }
                                line { x1: "12", y1: "3", x2: "12", y2: "15" }
                            }
                        }
                    }
                }

                // Right: window buttons
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

            // ── Four-panel workspace ───────────────────────────────────────────
            div {
                style: format!(
                    "flex: 1; display: flex; min-height: 0; overflow: hidden; cursor: {}; user-select: {};",
                    if is_resizing { "col-resize" } else { "default" },
                    if is_resizing { "none" } else { "auto" }
                ),
                onmousemove: move |evt| {
                    if let Some(active) = *active_resize.read() {
                        let cursor_x = evt.data().coordinates().client().x;
                        let delta = cursor_x - active.start_x;
                        let next_width = match active.target {
                            ResizeTarget::Assemblies | ResizeTarget::Explorer => {
                                active.start_width + delta
                            }
                            ResizeTarget::Findings => active.start_width - delta,
                        };
                        let clamped_width = clamp_panel_width(active.target, next_width);

                        match active.target {
                            ResizeTarget::Assemblies => assemblies_width.set(clamped_width),
                            ResizeTarget::Explorer => explorer_width.set(clamped_width),
                            ResizeTarget::Findings => findings_width.set(clamped_width),
                        }
                    }
                },
                onmouseup: move |_| active_resize.set(None),
                onmouseleave: move |_| active_resize.set(None),

                // ── Panel 1: Assemblies ────────────────────────────────────────
                div {
                    style: format!(
                        "width: {:.0}px; flex-shrink: 0; display: flex; flex-direction: column; \
                         background: {C_BG_SURFACE};"
                    , assemblies_width()),
                    // Header
                    div {
                        class: "panel-header",
                        span { "Assemblies" }
                        span { class: "badge", "{assemblies.len()}" }
                    }
                    // Content
                    div {
                        style: "flex: 1; overflow-y: auto; padding: 8px 0;",
                        if assemblies.is_empty() {
                            div {
                                class: "empty-state",
                                // Folder icon
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
                                    let asm_id_for_select = asm_id.clone();
                                    let asm_id_for_close = asm_id.clone();
                                    let is_selected = selected_id.as_ref() == Some(&asm_id);
                                    let item_class = if is_selected { "asm-item selected" } else { "asm-item" };
                                    rsx! {
                                        button {
                                            key: "{asm.id}",
                                            class: "{item_class}",
                                            onclick: move |_| {
                                                state.select_assembly(asm_id_for_select.clone());
                                                open_tabs.write().clear();
                                                active_tab_id.set(None);
                                                highlighted_il_offset.set(None);
                                            },
                                            div {
                                                style: "display: flex; align-items: center; justify-content: space-between; gap: 6px;",
                                                div {
                                                    style: "display: flex; align-items: center; gap: 6px; min-width: 0;",
                                                    // DLL icon dot
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
                                                // Close button
                                                button {
                                                    style: format!(
                                                        "flex-shrink: 0; width: 18px; height: 18px; border-radius: 4px; \
                                                         border: none; background: transparent; color: {C_TEXT_MUTED}; \
                                                         cursor: pointer; display: flex; align-items: center; \
                                                         justify-content: center; transition: all 120ms; padding: 0;"
                                                    ),
                                                    onclick: move |evt| {
                                                        evt.stop_propagation();
                                                        state.close_assembly(asm_id_for_close.clone());
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
                div {
                    class: if is_resizing_assemblies {
                        "resize-handle active"
                    } else {
                        "resize-handle"
                    },
                    onmousedown: move |evt| {
                        evt.prevent_default();
                        active_resize.set(Some(ActiveResize {
                            target: ResizeTarget::Assemblies,
                            start_x: evt.data().coordinates().client().x,
                            start_width: assemblies_width(),
                        }));
                    },
                }

                // ── Panel 2: Methods ───────────────────────────────────────────
                div {
                    style: format!(
                        "width: {:.0}px; flex-shrink: 0; display: flex; flex-direction: column; \
                         background: {C_BG_BASE};"
                    , explorer_width()),
                    div {
                        class: "panel-header",
                        span { "Explorer" }
                        span { class: "badge", "{methods_count} / {class_count} cls / {namespace_count} ns" }
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
                        } else {
                            if let Some(asm) = selected_assembly {
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
                                                style: format!("font-size: 10px; color: {C_TEXT_MUTED}; width: 10px; text-align: center;"),
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
                                                    let namespace_collapsed = !collapsed_namespaces.read().contains(&namespace_key);
                                                    let namespace_chevron = if namespace_collapsed { ">" } else { "v" };
                                                    rsx! {
                                                        button {
                                                            key: "namespace-{namespace_key}",
                                                            style: format!(
                                                                "display: flex; align-items: center; gap: 8px; width: calc(100% - 16px); \
                                                                 margin: 0 8px 4px; padding: 7px 10px 7px 18px; border-radius: 8px; cursor: pointer; \
                                                                 border: 1px solid {C_BORDER}; background: {C_BG_SURFACE}; color: {C_TEXT_SECONDARY};"
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
                                                                style: format!("font-size: 10px; color: {C_TEXT_MUTED}; width: 10px; text-align: center;"),
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
                                                                style: format!("margin-left: auto; font-size: 10px; color: {C_TEXT_MUTED};"),
                                                                "{namespace.types.len()}"
                                                            }
                                                        }

                                                        if !namespace_collapsed {
                                                            for group in namespace.types.iter() {
                                                                {
                                                                    let type_name = group.full_type_name.clone();
                                                                    let type_name_for_toggle = type_name.clone();
                                                                    let type_name_for_select = type_name.clone();
                                                                    let type_collapsed = !collapsed_types.read().contains(&type_name);
                                                                    let type_chevron = if type_collapsed { ">" } else { "v" };
                                                                    let is_type_selected = selected_type_name.as_ref() == Some(&type_name)
                                                                        && selected_method_name.is_none();
                                                                    rsx! {
                                                                        div {
                                                                            key: "type-{type_name}",
                                                                            style: "display: flex; gap: 4px; margin: 0 8px 3px; width: calc(100% - 16px);",
                                                                            button {
                                                                                style: format!(
                                                                                    "width: 26px; flex-shrink: 0; border-radius: 6px; cursor: pointer; \
                                                                                     border: 1px solid {C_BORDER}; background: {C_BG_BASE}; color: {C_TEXT_MUTED};"
                                                                                ),
                                                                                onclick: move |_| {
                                                                                    let mut set = collapsed_types.write();
                                                                                    if set.contains(&type_name_for_toggle) {
                                                                                        set.remove(&type_name_for_toggle);
                                                                                    } else {
                                                                                        set.insert(type_name_for_toggle.clone());
                                                                                    }
                                                                                },
                                                                                "{type_chevron}"
                                                                            }
                                                                            button {
                                                                                style: format!(
                                                                                    "display: flex; align-items: center; gap: 8px; min-width: 0; flex: 1; \
                                                                                     padding: 6px 10px; border-radius: 8px; cursor: pointer; text-align: left; \
                                                                                     border: 1px solid {}; background: {}; color: {C_TEXT_PRIMARY};",
                                                                                    if is_type_selected { C_BORDER_ACCENT } else { C_BORDER },
                                                                                    if is_type_selected { "rgba(245,245,245,0.06)" } else { C_BG_ELEVATED }
                                                                                ),
                                                                                onclick: move |_| {
                                                                                    let tab_id = type_tab_id(&type_name_for_select);
                                                                                    {
                                                                                        let mut tabs = open_tabs.write();
                                                                                        if !tabs.iter().any(|tab| tab.id == tab_id) {
                                                                                            let display_name = type_name_for_select
                                                                                                .rsplit('.')
                                                                                                .next()
                                                                                                .unwrap_or(&type_name_for_select)
                                                                                                .to_string();
                                                                                            tabs.push(IlTab {
                                                                                                id: tab_id.clone(),
                                                                                                kind: IlTabKind::Type,
                                                                                                type_name: type_name_for_select.clone(),
                                                                                                method_name: None,
                                                                                                title: display_name,
                                                                                                subtitle: type_name_for_select.clone(),
                                                                                            });
                                                                                        }
                                                                                    }
                                                                                    active_tab_id.set(Some(tab_id));
                                                                                    highlighted_il_offset.set(None);
                                                                                    let mut set = collapsed_types.write();
                                                                                    set.insert(type_name_for_select.clone());
                                                                                },
                                                                                span {
                                                                                    style: format!(
                                                                                        "font-size: 11px; font-weight: 700; color: {C_TEXT_PRIMARY}; \
                                                                                         overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                                                                                    ),
                                                                                    "{group.display_name}"
                                                                                }
                                                                                span {
                                                                                    style: format!("margin-left: auto; font-size: 10px; color: {C_TEXT_MUTED};"),
                                                                                    "{group.methods.len()}"
                                                                                }
                                                                            }
                                                                        }

                                                                        if !type_collapsed {
                                                                            for method in group.methods.iter() {
                                                                                {
                                                                                    let key_name = format!("{}::{}", method.type_name, method.method_name);
                                                                                    let method_type_name = method.type_name.clone();
                                                                                    let method_name = method.method_name.clone();
                                                                                    let is_selected = selected_method_name.as_ref() == Some(&key_name);
                                                                                    let item_class = if is_selected { "method-item selected" } else { "method-item" };
                                                                                    rsx! {
                                                                                        button {
                                                                                            key: "{key_name}",
                                                                                            class: "{item_class}",
                                                                                            style: "padding-left: 44px;",
                                                                                        onclick: move |_| {
                                                                                            let tab_id = method_tab_id(&method_type_name, &method_name);
                                                                                            {
                                                                                                let mut tabs = open_tabs.write();
                                                                                                if !tabs.iter().any(|tab| tab.id == tab_id) {
                                                                                                    tabs.push(IlTab {
                                                                                                        id: tab_id.clone(),
                                                                                                        kind: IlTabKind::Method,
                                                                                                        type_name: method_type_name.clone(),
                                                                                                        method_name: Some(method_name.clone()),
                                                                                                        title: method_name.clone(),
                                                                                                        subtitle: method_type_name.clone(),
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
                                                                                                     color: {}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                                                                                    if is_selected { C_ACCENT_GREEN } else { C_TEXT_PRIMARY }
                                                                                                ),
                                                                                                "{method.method_name}"
                                                                                            }
                                                                                            div {
                                                                                                style: format!(
                                                                                                    "font-size: 10px; color: {C_TEXT_MUTED}; margin-top: 3px; \
                                                                                                     overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
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

                div {
                    class: if is_resizing_explorer {
                        "resize-handle active"
                    } else {
                        "resize-handle"
                    },
                    onmousedown: move |evt| {
                        evt.prevent_default();
                        active_resize.set(Some(ActiveResize {
                            target: ResizeTarget::Explorer,
                            start_x: evt.data().coordinates().client().x,
                            start_width: explorer_width(),
                        }));
                    },
                }

                // ── Panel 3: IL View ───────────────────────────────────────────
                div {
                    style: format!(
                        "flex: 1; min-width: 0; display: flex; flex-direction: column; \
                         background: {C_BG_BASE};"
                    ),
                    div {
                        class: "panel-header",
                        span { "IL View" }
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
                    }
                    div {
                        style: "flex: 1; min-height: 0; display: flex; flex-direction: column;",
                        div {
                        class: "il-tabs",
                        onwheel: move |evt| {
                            // Prevent default vertical scroll when wheeling over tabs
                            let _delta_y = evt.data().delta().strip_units().y;
                            evt.prevent_default();
                        },
                        if tabs.is_empty() {
                            span {
                                style: format!(
                                    "font-size: 10px; color: {C_TEXT_MUTED}; padding: 5px 6px;"
                                ),
                                "No open tabs"
                            }
                        } else {
                            for tab in tabs.iter() {
                                {
                                    let tab_id = tab.id.clone();
                                    let tab_id_for_select = tab_id.clone();
                                    let tab_id_for_close = tab_id.clone();
                                    let is_active = active_tab_id_value.as_ref() == Some(&tab_id);
                                    let tab_class = if is_active { "il-tab active" } else { "il-tab" };
                                    rsx! {
                                        div {
                                            key: "il-tab-{tab.id}",
                                            class: "{tab_class}",
                                            onclick: move |_| {
                                                active_tab_id.set(Some(tab_id_for_select.clone()));
                                                highlighted_il_offset.set(None);
                                            },
                                            div {
                                                style: "min-width: 0; display: grid; gap: 1px;",
                                                div {
                                                    style: format!(
                                                        "font-size: 11px; font-weight: 700; color: {}; \
                                                         font-family: {FONT_MONO}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                                        if is_active { C_TEXT_PRIMARY } else { C_TEXT_SECONDARY }
                                                    ),
                                                    "{tab.title}"
                                                }
                                                div {
                                                    style: format!(
                                                        "font-size: 9px; color: {C_TEXT_MUTED}; \
                                                         overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
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
                                                    if let Some(index) = tabs_mut.iter().position(|open_tab| open_tab.id == tab_id_for_close) {
                                                        let current_active = active_tab_id.read().clone();
                                                        let closed_was_active = current_active.as_ref() == Some(&tab_id_for_close);
                                                        tabs_mut.remove(index);
                                                        let next_id = if closed_was_active {
                                                            if tabs_mut.is_empty() {
                                                                None
                                                            } else {
                                                                let next_index = index.saturating_sub(1);
                                                                tabs_mut.get(next_index).map(|open_tab| open_tab.id.clone())
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
                        div {
                        style: "flex: 1; overflow-y: auto; padding: 10px 14px;",
                        if show_class_overview {
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
                                        style: format!(
                                            "font-size: 10px; color: {C_TEXT_SECONDARY};"
                                        ),
                                        "{selected_type_methods.len()} methods"
                                    }
                                }
                                // Full class IL view - show all methods with their IL
                                {
                                    selected_type_methods.clone().into_iter().map(|method| {
                                        let click_type_name = method.type_name.clone();
                                        let click_method_name = method.method_name.clone();
                                        let key_method_name = method.method_name.clone();
                                        rsx! {
                                        div {
                                            key: "class-method-{method.method_name}",
                                                style: format!(
                                                    "margin-bottom: 14px; padding: 10px 12px; \
                                                     background: {C_BG_SURFACE}; border-radius: 8px; \
                                                     border: 1px solid {C_BORDER}; cursor: pointer;"
                                                ),
                                                onclick: move |_| {
                                                    let tab_id = method_tab_id(&click_type_name, &click_method_name);
                                                    {
                                                        let mut tabs = open_tabs.write();
                                                        if !tabs.iter().any(|tab| tab.id == tab_id) {
                                                            tabs.push(IlTab {
                                                                id: tab_id.clone(),
                                                                kind: IlTabKind::Method,
                                                                type_name: click_type_name.clone(),
                                                                method_name: Some(click_method_name.clone()),
                                                                title: click_method_name.clone(),
                                                                subtitle: click_type_name.clone(),
                                                            });
                                                        }
                                                    }
                                                    active_tab_id.set(Some(tab_id));
                                                    highlighted_il_offset.set(None);
                                                },
                                                // Method header
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
                                                         color: {C_TEXT_SECONDARY}; line-height: 1.4; margin-bottom: 8px;"
                                                    ),
                                                    "{method.signature}"
                                                }
                                                // IL instructions
                                                div {
                                                    style: format!("font-family: {FONT_MONO};"),
                                                    {
                                                        let highlighted_val = *highlighted_il_offset.read();
                                                        method.instructions.iter().map(move |ins| {
                                                            let is_highlighted = highlighted_val == Some(ins.offset);
                                                            let row_class = if is_highlighted { "il-row highlighted" } else { "il-row" };
                                                            rsx! {
                                                                div {
                                                                    key: "{key_method_name}-{ins.offset}-{ins.op_code}",
                                                                id: "il-{ins.offset}",
                                                                class: "{row_class}",
                                                                span {
                                                                    style: format!("color: {C_ACCENT_BLUE}; font-size: 11px;"),
                                                                    "IL_{ins.offset:04X}"
                                                                }
                                                                span {
                                                                    style: format!("color: {C_ACCENT_GREEN}; font-size: 11px; font-weight: 500;"),
                                                                    "{ins.op_code}"
                                                                }
                                                                span {
                                                                    style: format!("color: {C_TEXT_SECONDARY}; font-size: 11px;"),
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
                                        let row_class = if is_highlighted { "il-row highlighted" } else { "il-row" };
                                        rsx! {
                                            div {
                                                key: "{method.method_name}-{ins.offset}-{ins.op_code}",
                                                id: "il-{ins.offset}",
                                                class: "{row_class}",
                                                span {
                                                    style: format!("color: {C_ACCENT_BLUE}; font-size: 11px;"),
                                                    "IL_{ins.offset:04X}"
                                                }
                                                span {
                                                    style: format!("color: {C_ACCENT_GREEN}; font-size: 11px; font-weight: 500;"),
                                                    "{ins.op_code}"
                                                }
                                                span {
                                                    style: format!("color: {C_TEXT_SECONDARY}; font-size: 11px;"),
                                                    "{ins.operand}"
                                                }
                                            }
                                        }
                                    })
                                }
                            }
                        } else if !show_class_overview {
                            div {
                                class: "empty-state",
                                svg {
                                    width: "44", height: "44", view_box: "0 0 24 24",
                                    fill: "none", stroke: C_ACCENT_BLUE,
                                    stroke_width: "1.5",
                                    rect { x: "2", y: "3", width: "20", height: "14", rx: "2", ry: "2" }
                                    line { x1: "8", y1: "21", x2: "16", y2: "21" }
                                    line { x1: "12", y1: "17", x2: "12", y2: "21" }
                                }
                                p { "Select a method to view its IL instructions" }
                            }
                        }
                        }
                    }
                }

                // ── Panel 4: Findings ──────────────────────────────────────────
                if show_scan_panel() {
                    div {
                        class: if is_resizing_findings {
                            "resize-handle active"
                        } else {
                            "resize-handle"
                        },
                        onmousedown: move |evt| {
                            evt.prevent_default();
                            active_resize.set(Some(ActiveResize {
                                target: ResizeTarget::Findings,
                                start_x: evt.data().coordinates().client().x,
                                start_width: findings_width(),
                            }));
                        },
                    }
                    div {
                        style: format!(
                            "width: {:.0}px; flex-shrink: 0; display: flex; flex-direction: column; \
                             background: {C_BG_SURFACE};"
                        , findings_width()),
                        div {
                            class: "panel-header",
                            span { "Findings" }
                            span {
                                class: "badge",
                                style: if findings_count > 0 {
                                    format!("color: {C_ACCENT_AMBER}; border-color: {C_ACCENT_AMBER}40; background: rgba(245,245,245,0.06);")
                                } else {
                                    String::new()
                                },
                                "{findings_count}"
                            }
                        }
                        div {
                            style: "flex: 1; overflow-y: auto; padding: 8px 0; display: flex; flex-direction: column;",

                        if findings.is_empty() {
                            div {
                                class: "empty-state",
                                svg {
                                    width: "40", height: "40", view_box: "0 0 24 24",
                                    fill: "none", stroke: C_ACCENT_AMBER,
                                    stroke_width: "1.5",
                                    path { d: "M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" }
                                    line { x1: "12", y1: "9", x2: "12", y2: "13" }
                                    line { x1: "12", y1: "17", x2: "12.01", y2: "17" }
                                }
                                p { "No findings — scan results will appear here" }
                            }
                        } else {
                            // Finding list
                            div {
                                for (index, finding) in findings.iter().enumerate() {
                                    {
                                        let sev_color = severity_color(&finding.severity);
                                        let is_selected = selected_finding_index == index;
                                        let item_class = if is_selected { "finding-item selected" } else { "finding-item" };
                                        let finding_location = finding.location.clone();
                                        let finding_offset = finding.il_offset;
                                        rsx! {
                                            button {
                                                key: "{index}-{finding.rule_id}",
                                                class: "{item_class}",
                                                onclick: move |_| {
                                                    selected_finding.set(Some(index));
                                                    if let Some((type_part, method_part)) = finding_location.split_once("::") {
                                                        let tab_id = method_tab_id(type_part, method_part);
                                                        {
                                                            let mut tabs = open_tabs.write();
                                                            if !tabs.iter().any(|tab| tab.id == tab_id) {
                                                                tabs.push(IlTab {
                                                                    id: tab_id.clone(),
                                                                    kind: IlTabKind::Method,
                                                                    type_name: type_part.to_string(),
                                                                    method_name: Some(method_part.to_string()),
                                                                    title: method_part.to_string(),
                                                                    subtitle: type_part.to_string(),
                                                                });
                                                            }
                                                        }
                                                        active_tab_id.set(Some(tab_id));
                                                    }
                                                    highlighted_il_offset.set(finding_offset);
                                                },
                                                div {
                                                    style: "display: flex; align-items: center; justify-content: space-between; gap: 6px; margin-bottom: 4px;",
                                                    span {
                                                        style: format!(
                                                            "font-size: 11px; font-weight: 700; font-family: {FONT_MONO}; \
                                                             color: {};",
                                                            if is_selected { C_TEXT_PRIMARY } else { C_TEXT_SECONDARY }
                                                        ),
                                                        "{finding.rule_id}"
                                                    }
                                                    span {
                                                        class: "sev-badge",
                                                        style: format!("color: {sev_color};"),
                                                        "{finding.severity}"
                                                    }
                                                }
                                                div {
                                                    style: format!(
                                                        "font-size: 10px; color: {C_TEXT_MUTED}; \
                                                         overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                                                    ),
                                                    "{finding.location}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Finding detail pane
                            if let Some(detail) = active_finding {
                                div {
                                    style: format!(
                                        "margin: 4px 8px 8px; padding: 12px; \
                                         background: {C_BG_ELEVATED}; \
                                         border: 1px solid {C_BORDER_ACCENT}; \
                                         border-radius: 10px;"
                                    ),
                                    p {
                                        style: format!(
                                            "font-size: 10px; font-weight: 700; letter-spacing: 0.8px; \
                                             text-transform: uppercase; color: {C_TEXT_MUTED}; \
                                             margin-bottom: 8px;"
                                        ),
                                        "Detail"
                                    }
                                    p {
                                        style: format!(
                                            "font-size: 12px; color: {C_TEXT_SECONDARY}; \
                                             line-height: 1.55; margin-bottom: 10px;"
                                        ),
                                        "{detail.description}"
                                    }
                                    if !detail.code_snippet.is_empty() {
                                        pre {
                                            style: format!(
                                                "font-family: {FONT_MONO}; font-size: 10px; \
                                                 line-height: 1.6; color: {C_TEXT_SECONDARY}; \
                                                 background: {C_BG_BASE}; \
                                                 border: 1px solid {C_BORDER}; \
                                                 border-radius: 6px; padding: 8px 10px; \
                                                 overflow-x: auto; white-space: pre-wrap; \
                                                 word-break: break-all;"
                                            ),
                                            "{detail.code_snippet}"
                                        }
                                    }
                                }
                            }
                        }
                        }
                    }
                }
            }

            // ── Status bar ─────────────────────────────────────────────────────
            div {
                style: format!(
                    "height: 26px; flex-shrink: 0; display: flex; align-items: center; \
                     justify-content: space-between; padding: 0 14px; \
                     background: {C_BG_SURFACE}; border-top: 1px solid {C_BORDER}; \
                     font-size: 10px; font-family: {FONT_MONO};"
                ),

                // Left metrics
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
                            "{assemblies.len()}"
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

                // Right: error / status
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
                        "MLVInspector v0.1"
                    }
                }
            }
        }
    }
}

// ─── Analysis runner ──────────────────────────────────────────────────────────

fn run_analysis(
    mut state: AppState,
    mut last_error: Signal<String>,
    assembly_id: String,
    assembly_path: String,
) {
    spawn(async move {
        state.is_running.set(true);
        last_error.set(String::new());

        let started = now_ts();
        let worker  = state.worker.read().clone();

        // Single combined entry that will hold both explore + scan payloads.
        let mut combined_entry = AnalysisEntry {
            assembly_id:   assembly_id.clone(),
            assembly_path: assembly_path.clone(),
            mode:          ActiveMode::Explore,
            status:        AnalysisStatus::Running,
            result:        None,
            error:         None,
            started_at:    Some(started),
            finished_at:   None,
        };

        // Mark running immediately so the UI shows the spinner.
        state.set_analysis_result(format!("{}::explore", assembly_id), combined_entry.clone());
        state.set_analysis_result(format!("{}::scan",    assembly_id), combined_entry.clone());

        let explore_future = worker.explore(ExploreParams {
            assembly: assembly_path.clone(),
            ..Default::default()
        });
        let scan_future = worker.scan(ScanParams {
            assembly: assembly_path.clone(),
            ..Default::default()
        });

        let (explore_result, scan_result) = tokio::join!(explore_future, scan_future);
        let finished = now_ts();

        // Build a unified AnalysisResult from both payloads.
        let mut result = AnalysisResult {
            assembly_path: assembly_path.clone(),
            mode:          "combined".to_string(),
            explore:       None,
            scan:          None,
            stderr:        String::new(),
        };

        match explore_result {
            Ok(payload) => {
                tracing::debug!(methods = payload.methods.len(), "explore done");
                result.explore = Some(payload);
            }
            Err(e) => {
                tracing::error!(err = %e, "explore failed");
                last_error.set(e.to_string());
                combined_entry.status = AnalysisStatus::Error;
                combined_entry.error  = Some(e.to_string());
            }
        }

        match scan_result {
            Ok(payload) => {
                tracing::debug!(findings = payload.findings.len(), "scan done");
                result.scan = Some(payload);
            }
            Err(e) => {
                tracing::error!(err = %e, "scan failed");
                last_error.set(e.to_string());
                if combined_entry.status != AnalysisStatus::Error {
                    combined_entry.status = AnalysisStatus::Error;
                    combined_entry.error  = Some(e.to_string());
                }
            }
        }

        // Only mark Done if we got at least one successful payload.
        if result.explore.is_some() || result.scan.is_some() {
            combined_entry.status      = AnalysisStatus::Done;
            combined_entry.result      = Some(result);
            combined_entry.finished_at = Some(finished);
        }

        // Store under both keys so existing lookup code (::explore / ::scan) still works.
        state.set_analysis_result(
            format!("{}::explore", assembly_id),
            combined_entry.clone(),
        );
        state.set_analysis_result(
            format!("{}::scan", assembly_id),
            combined_entry,
        );

        state.is_running.set(false);
    });
}

// ─── JSON extraction helpers ──────────────────────────────────────────────────

fn group_methods_by_namespace(methods: &[UiMethod]) -> Vec<UiNamespaceGroup> {
    let mut namespaces: BTreeMap<String, BTreeMap<String, Vec<UiMethod>>> = BTreeMap::new();

    for method in methods {
        let full_type_name = method.type_name.clone();
        let (namespace, _class) = full_type_name
            .rsplit_once('.')
            .map(|(ns, cls)| (ns.to_string(), cls.to_string()))
            .unwrap_or_else(|| ("(global)".to_string(), full_type_name.clone()));

        namespaces
            .entry(namespace)
            .or_default()
            .entry(full_type_name)
            .or_default()
            .push(method.clone());
    }

    namespaces
        .into_iter()
        .map(|(namespace_name, type_map)| {
            let types = type_map
                .into_iter()
                .map(|(full_type_name, mut methods)| {
                    methods.sort_by(|a, b| a.method_name.cmp(&b.method_name));
                    let display_name = full_type_name
                        .rsplit('.')
                        .next()
                        .unwrap_or(&full_type_name)
                        .to_string();

                    UiTypeGroup {
                        full_type_name,
                        display_name,
                        methods,
                    }
                })
                .collect();

            UiNamespaceGroup {
                namespace_name,
                types,
            }
        })
        .collect()
}

fn extract_methods(result: &AnalysisResult) -> Vec<UiMethod> {
    let Some(explore) = result.explore.as_ref() else {
        return Vec::new();
    };

    explore
        .methods
        .iter()
        .map(|m| {
            let instructions = m
                .instructions
                .iter()
                .map(|ins| UiInstruction {
                    offset:  ins.offset as i64,
                    op_code: ins.op_code.clone(),
                    operand: ins.operand.clone().unwrap_or_default(),
                })
                .collect();

            UiMethod {
                type_name:   m.type_name.clone(),
                method_name: m.method_name.clone(),
                signature:   m.signature.clone(),
                instructions,
            }
        })
        .collect()
}

fn extract_findings(result: &AnalysisResult) -> Vec<UiFinding> {
    let Some(scan) = result.scan.as_ref() else {
        return Vec::new();
    };

    scan.findings
        .iter()
        .map(|f| {
            let snippet = f.code_snippet.as_deref().unwrap_or("");
            UiFinding {
                rule_id:      f.rule_id.as_deref().unwrap_or("UnknownRule").to_string(),
                severity:     f.severity.clone(),
                location:     f.location.clone(),
                description:  f.description.clone(),
                code_snippet: snippet.to_string(),
                il_offset:    parse_il_offset_from_snippet(snippet),
            }
        })
        .collect()
}

// ─── Utilities ─────────────────────────────────────────────────────────────────

fn type_tab_id(type_name: &str) -> String {
    format!("type::{type_name}")
}

fn method_tab_id(type_name: &str, method_name: &str) -> String {
    format!("method::{type_name}::{method_name}")
}

fn parse_il_offset_from_snippet(snippet: &str) -> Option<i64> {
    let pos = snippet.find("IL_")?;
    let hex = snippet.get(pos + 3..pos + 7)?;
    i64::from_str_radix(hex, 16).ok()
}

fn severity_color(severity: &str) -> &'static str {
    match severity {
        "Critical" => "#c08b91",
        "High" => "#b59a86",
        "Medium" => "#b8ae96",
        "Low" => "#98a893",
        "Info" => "#8f9dac",
        _ => "#8b919d",
    }
}

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
