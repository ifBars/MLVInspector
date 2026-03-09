use dioxus::html::HasFileData;
use dioxus::prelude::*;
use dioxus_desktop::tao::event::{Event, WindowEvent};
use dioxus_desktop::use_wry_event_handler;

use crate::components::{
    clamp_panel_width, dispatch_shortcut_binding, extract_findings, global_css, run_analysis,
    ActiveResize, CommandContext, CommandId, CommandPalette, ExplorerPanel, FindingsPanel, IlTab,
    IlViewPanel, OverlayKind, ResizeTarget, SettingsOverlay, StatusBar, TitleBar, C_ACCENT_BLUE,
    C_BG_BASE, C_TEXT_PRIMARY, FONT_SANS,
};
use crate::services::settings::{load_shortcut_settings, save_shortcut_settings};
use crate::shortcuts::{
    binding_from_shortcut_event_payload, ShortcutKey, ShortcutKeyEventPayload, ShortcutSettings,
    APP_SHORTCUT_LISTENER_SCRIPT,
};
use crate::state::AppState;

// ─── Root component ───────────────────────────────────────────────────────────

#[component]
pub fn App() -> Element {
    let state = use_context_provider(AppState::default);

    // Shared cross-panel signals
    let mut open_tabs = use_signal(Vec::<IlTab>::new);
    let mut active_tab_id = use_signal(|| None::<String>);
    let mut selected_finding = use_signal(|| None::<usize>);
    let mut last_error = use_signal(String::new);
    let active_overlay = use_signal(|| None::<OverlayKind>);
    let show_scan_panel = use_signal(|| true);
    let shortcut_settings = use_signal(|| {
        load_shortcut_settings()
            .map(ShortcutSettings::merged_with_defaults)
            .unwrap_or_else(|err| {
                tracing::warn!(error = %err, "failed to load shortcut settings; using defaults");
                ShortcutSettings::with_defaults()
            })
    });
    let mut editing_command = use_signal(|| None::<CommandId>);

    // Drag-and-drop state (App-local)
    let mut drag_counter = use_signal(|| 0i32);
    let mut is_dragging_over = use_signal(|| false);

    // Panel resize state (App-local)
    let mut explorer_width = use_signal(|| 320.0f64);
    let mut findings_width = use_signal(|| 300.0f64);
    let mut active_resize = use_signal(|| None::<ActiveResize>);

    use_effect(move || {
        let settings = shortcut_settings();
        if let Err(err) = save_shortcut_settings(&settings) {
            tracing::warn!(error = %err, "failed to save shortcut settings");
        }
    });

    use_hook(move || {
        spawn(async move {
            let mut eval = document::eval(APP_SHORTCUT_LISTENER_SCRIPT);

            loop {
                let payload: ShortcutKeyEventPayload = match eval.recv().await {
                    Ok(payload) => payload,
                    Err(err) => {
                        tracing::warn!(error = %err, "shortcut listener stopped");
                        break;
                    }
                };

                if payload.repeat {
                    continue;
                }

                if active_overlay() == Some(OverlayKind::Settings) && editing_command().is_some() {
                    continue;
                }

                let Some(binding) = binding_from_shortcut_event_payload(&payload) else {
                    continue;
                };

                if payload.is_editable_target() && !matches!(binding.key, ShortcutKey::Escape) {
                    continue;
                }

                dispatch_shortcut_binding(
                    CommandContext {
                        state,
                        active_overlay,
                        show_scan_panel,
                        last_error,
                        open_tabs,
                        active_tab_id,
                        selected_finding,
                    },
                    &shortcut_settings(),
                    &binding,
                );
            }
        });
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
                            rule_id: r.rule_id,
                            description: r.description,
                            severity: match r.severity.as_str() {
                                "Critical" => crate::types::RuleSeverity::Critical,
                                "High" => crate::types::RuleSeverity::High,
                                "Medium" => crate::types::RuleSeverity::Medium,
                                "Low" => crate::types::RuleSeverity::Low,
                                "Info" => crate::types::RuleSeverity::Info,
                                _ => crate::types::RuleSeverity::Unknown,
                            },
                        })
                        .collect();
                    state.set_rules(converted);
                }
                Err(e) => last_error.set(e.to_string()),
            }
        });
    });

    // Native file-drop via wry event handler (supplements HTML drag-drop)
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
            selected_finding.set(None);

            if let Some(assembly_id) = state.selected_id.read().clone() {
                run_analysis(state, last_error, assembly_id, file_path);
            }
        }
        Event::WindowEvent { .. } => {}
        _ => {}
    });

    let _keyboard_handler = use_wry_event_handler(move |event, _| {
        if let Event::WindowEvent {
            event: WindowEvent::Focused(false),
            ..
        } = event
        {
            editing_command.set(None);
        }
    });

    // Derive findings count for status bar
    let selected_id = state.selected_id.read().clone();
    let findings_count = if let Some(ref id) = selected_id {
        let scan_key = format!("{id}::scan");
        state
            .get_analysis_entry(&scan_key)
            .as_ref()
            .and_then(|e| e.result.as_ref())
            .map(extract_findings)
            .map(|f| f.len())
            .unwrap_or(0)
    } else {
        0
    };

    let is_resizing = active_resize.read().is_some();
    let is_resizing_explorer = matches!(
        *active_resize.read(),
        Some(ActiveResize {
            target: ResizeTarget::Explorer,
            ..
        })
    );
    let is_resizing_findings = matches!(
        *active_resize.read(),
        Some(ActiveResize {
            target: ResizeTarget::Findings,
            ..
        })
    );

    rsx! {
        // Inject global CSS once at the root
        style { "{global_css()}" }

        div {
            style: format!(
                "width: 100vw; height: 100vh; display: flex; flex-direction: column; \
                 background: {C_BG_BASE}; color: {C_TEXT_PRIMARY}; font-family: {FONT_SANS}; \
                 overflow: hidden; position: relative;"
            ),

            // HTML drag-and-drop overlay handling
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
                    let file_path = file.path().display().to_string();
                    state.open_assembly(file_path.clone());
                    open_tabs.write().clear();
                    active_tab_id.set(None);
                    selected_finding.set(None);

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

            // Drop overlay
            if is_dragging_over() {
                div {
                    class: "drop-overlay visible",
                    div {
                        class: "drop-zone",
                        svg {
                            width: "64", height: "64", view_box: "0 0 24 24",
                            fill: "none", stroke: C_ACCENT_BLUE, stroke_width: "1.5",
                            path { d: "M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" }
                            polyline { points: "17 8 12 3 7 8" }
                            line { x1: "12", y1: "3", x2: "12", y2: "15" }
                        }
                        span {
                            style: format!(
                                "font-size: 16px; font-weight: 600; color: {C_TEXT_PRIMARY};"
                            ),
                            "Drop assembly to open"
                        }
                        span {
                            style: "font-size: 12px; color: #7d828d;",
                            "Supports .dll and .exe files"
                        }
                    }
                }
            }

            // ── Title bar ──────────────────────────────────────────────────────
            TitleBar {
                active_overlay,
                show_scan_panel,
                last_error,
                open_tabs,
                active_tab_id,
                selected_finding,
            }

            CommandPalette {
                active_overlay,
                shortcut_settings,
                show_scan_panel,
                last_error,
                open_tabs,
                active_tab_id,
                selected_finding,
            }

            SettingsOverlay {
                active_overlay,
                shortcut_settings,
                editing_command,
            }

            // ── Three-panel workspace ──────────────────────────────────────────
            div {
                style: format!(
                    "flex: 1; display: flex; min-height: 0; overflow: hidden; \
                     cursor: {}; user-select: {};",
                    if is_resizing { "col-resize" } else { "default" },
                    if is_resizing { "none" } else { "auto" }
                ),

                // Global resize mouse tracking
                onmousemove: move |evt| {
                    if let Some(active) = *active_resize.read() {
                        let cursor_x = evt.data().coordinates().client().x;
                        let delta = cursor_x - active.start_x;
                        let next_width = match active.target {
                            ResizeTarget::Explorer => active.start_width + delta,
                            ResizeTarget::Findings => active.start_width - delta,
                        };
                        let clamped = clamp_panel_width(active.target, next_width);
                        match active.target {
                            ResizeTarget::Explorer => explorer_width.set(clamped),
                            ResizeTarget::Findings => findings_width.set(clamped),
                        }
                    }
                },
                onmouseup: move |_| active_resize.set(None),
                onmouseleave: move |_| active_resize.set(None),

                // Panel 1: Explorer
                ExplorerPanel {
                    sidebar_width: explorer_width(),
                    open_tabs,
                    active_tab_id,
                    selected_finding,
                }

                // Resize handle: explorer ↔ IL view
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

                // Panel 2: IL / C# view
                IlViewPanel {
                    open_tabs,
                    active_tab_id,
                    selected_finding,
                }

                // Panel 3: Findings (optional)
                if show_scan_panel() {
                    // Resize handle: IL view ↔ findings
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

                    FindingsPanel {
                        findings_width: findings_width(),
                        open_tabs,
                        active_tab_id,
                        selected_finding,
                    }
                }
            }

            // ── Status bar ─────────────────────────────────────────────────────
            StatusBar {
                show_scan_panel,
                last_error,
                findings_count,
            }
        }
    }
}
