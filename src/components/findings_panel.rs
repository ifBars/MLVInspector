/// Right panel: scan findings list with detail pane.
use dioxus::prelude::*;

use crate::state::AppState;

use super::helpers::{extract_findings, method_tab_id, severity_color};
use super::theme::{
    C_ACCENT_AMBER, C_BG_ELEVATED, C_BG_SURFACE, C_BORDER, C_BORDER_ACCENT, C_TEXT_MUTED,
    C_TEXT_PRIMARY, C_TEXT_SECONDARY, FONT_MONO,
};
use super::view_models::{IlTab, IlTabKind};

#[component]
pub fn FindingsPanel(
    findings_width: f64,
    open_tabs: Signal<Vec<IlTab>>,
    active_tab_id: Signal<Option<String>>,
    highlighted_il_offset: Signal<Option<i64>>,
    selected_finding: Signal<Option<usize>>,
) -> Element {
    let state = use_context::<AppState>();
    let selected_id = state.selected_id.read().clone();

    let findings = if let Some(ref id) = selected_id {
        let scan_key = format!("{id}::scan");
        state
            .get_analysis_entry(&scan_key)
            .as_ref()
            .and_then(|e| e.result.as_ref())
            .map(extract_findings)
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let findings_count = findings.len();
    let selected_finding_index = selected_finding.read().unwrap_or(0);
    let active_finding = findings.get(selected_finding_index).cloned();

    rsx! {
        div {
            style: format!(
                "width: {findings_width:.0}px; flex-shrink: 0; display: flex; \
                 flex-direction: column; background: {C_BG_SURFACE};"
            ),

            div {
                class: "panel-header",
                span { "Findings" }
                span {
                    class: "badge",
                    style: if findings_count > 0 {
                        format!(
                            "color: {C_ACCENT_AMBER}; border-color: {C_ACCENT_AMBER}40; \
                             background: rgba(245,245,245,0.06);"
                        )
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
                            path {
                                d: "M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"
                            }
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
                                let item_class = if is_selected {
                                    "finding-item selected"
                                } else {
                                    "finding-item"
                                };
                                let finding_location = finding.location.clone();
                                let finding_offset = finding.il_offset;
                                rsx! {
                                    button {
                                        key: "{index}-{finding.rule_id}",
                                        class: "{item_class}",
                                        onclick: move |_| {
                                            selected_finding.set(Some(index));
                                            if let Some((type_part, method_part)) =
                                                finding_location.split_once("::")
                                            {
                                                let tab_id =
                                                    method_tab_id(type_part, method_part);
                                                {
                                                    let mut tabs = open_tabs.write();
                                                    if !tabs.iter().any(|tab| tab.id == tab_id) {
                                                        tabs.push(IlTab {
                                                            id: tab_id.clone(),
                                                            kind: IlTabKind::Method,
                                                            type_name: type_part.to_string(),
                                                            method_name: Some(
                                                                method_part.to_string(),
                                                            ),
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
                                            style: "display: flex; align-items: center; \
                                                    justify-content: space-between; gap: 6px; \
                                                    margin-bottom: 4px;",
                                            span {
                                                style: format!(
                                                    "font-size: 11px; font-weight: 700; \
                                                     font-family: {FONT_MONO}; color: {};",
                                                    if is_selected {
                                                        C_TEXT_PRIMARY
                                                    } else {
                                                        C_TEXT_SECONDARY
                                                    }
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
                                                 overflow: hidden; text-overflow: ellipsis; \
                                                 white-space: nowrap;"
                                            ),
                                            "{finding.location}"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Detail pane for selected finding
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
                                         background: #101113; \
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
