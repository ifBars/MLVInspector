/// Right panel: scan findings list with detail pane.
use dioxus::prelude::*;

use crate::state::AppState;
use crate::types::AnalysisStatus;

use super::helpers::{
    extract_findings, extract_methods, extract_scan_overview, method_tab_id,
    resolve_finding_target, resolve_method_reference, severity_color,
};
use super::theme::{
    C_ACCENT_AMBER, C_BG_ELEVATED, C_BG_SURFACE, C_BORDER, C_BORDER_ACCENT, C_TEXT_MUTED,
    C_TEXT_PRIMARY, C_TEXT_SECONDARY, FONT_MONO,
};
use super::view_models::{IlTab, IlTabKind, UiFinding, UiMethod, UiScanOverview};

#[component]
pub fn FindingsPanel(
    findings_width: f64,
    open_tabs: Signal<Vec<IlTab>>,
    active_tab_id: Signal<Option<String>>,
    selected_finding: Signal<Option<usize>>,
) -> Element {
    let state = use_context::<AppState>();
    let selected_id = state.selected_id.read().clone();

    let findings = if let Some(ref id) = selected_id {
        let scan_key = format!("{id}::scan");
        state
            .with_analysis_result(&scan_key, extract_findings)
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let scan_overview = if let Some(ref id) = selected_id {
        let scan_key = format!("{id}::scan");
        state
            .with_analysis_result(&scan_key, extract_scan_overview)
            .flatten()
    } else {
        None
    };
    let methods = if let Some(ref id) = selected_id {
        let explore_key = format!("{id}::explore");
        state
            .with_analysis_result(&explore_key, extract_methods)
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let findings_count = findings.len();
    let (scan_status, scan_error) = selected_id
        .as_ref()
        .and_then(|id| {
            state
                .analysis_entries
                .read()
                .get(&format!("{id}::scan"))
                .map(|entry| (Some(entry.status), entry.error.clone()))
        })
        .unwrap_or((None, None));
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

                if let Some(overview) = scan_overview.as_ref() {
                    ScanOverviewCard { overview: overview.clone() }
                }

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
                        p {
                            match scan_status {
                                Some(AnalysisStatus::Idle) => "Scan queued after metadata exploration",
                                Some(AnalysisStatus::Running) => "MLVScan analysis is running",
                                Some(AnalysisStatus::Error) => scan_error.as_deref().unwrap_or("MLVScan analysis failed"),
                                Some(AnalysisStatus::Done) => "No findings from the current scan",
                                None => "Open an assembly to run MLVScan analysis",
                            }
                        }
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
                                let finding_rule_id = finding.rule_id.clone();
                                let finding_location = finding.location.clone();
                                let navigation = finding.navigation.clone();
                                let resolved_method = resolve_finding_target(&methods, finding);
                                rsx! {
                                    button {
                                        key: "{index}-{finding.rule_id}",
                                        class: "{item_class}",
                                        onclick: move |_| {
                                            tracing::info!(
                                                finding_index = index,
                                                rule_id = %finding_rule_id,
                                                location = %finding_location,
                                                navigation = ?navigation,
                                                resolved_method = ?resolved_method,
                                                "finding clicked"
                                            );
                                            selected_finding.set(Some(index));
                                            if let Some((type_name, method_name)) = resolved_method.as_ref() {
                                                let tab_id = method_tab_id(type_name, method_name);
                                                {
                                                    let mut tabs = open_tabs.write();
                                                    tracing::debug!(
                                                        tab_id = %tab_id,
                                                        existing_tabs = ?tabs.iter().map(|tab| (&tab.id, &tab.type_name, &tab.method_name)).collect::<Vec<_>>(),
                                                        "opening finding method tab"
                                                    );
                                                    if !tabs.iter().any(|tab| tab.id == tab_id) {
                                                        tabs.push(IlTab {
                                                            id: tab_id.clone(),
                                                            kind: IlTabKind::Method,
                                                            type_name: type_name.clone(),
                                                            method_name: Some(method_name.clone()),
                                                            metadata_token: None,
                                                            title: method_name.clone(),
                                                            subtitle: type_name.clone(),
                                                        });
                                                        tracing::info!(tab_id = %tab_id, "added new finding tab");
                                                    } else {
                                                        tracing::info!(tab_id = %tab_id, "finding tab already open");
                                                    }
                                                }
                                                active_tab_id.set(Some(tab_id));
                                                tracing::info!("set active tab from finding click");
                                            } else {
                                                tracing::warn!(
                                                    finding_index = index,
                                                    rule_id = %finding_rule_id,
                                                    location = %finding_location,
                                                    "finding click could not resolve a method to open"
                                                );
                                            }
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
                        {
                            let jump_targets = resolved_finding_navigation_targets(&methods, &detail);
                            rsx! {
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
                                    div {
                                        style: "display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 10px;",
                                        if let Some(score) = detail.risk_score {
                                            span {
                                                class: "badge",
                                                style: format!("font-family: {FONT_MONO};"),
                                                "Risk {score}"
                                            }
                                        }
                                        if let Some(visibility) = detail.visibility.as_ref() {
                                            span {
                                                class: "badge",
                                                style: format!("font-family: {FONT_MONO};"),
                                                "{visibility}"
                                            }
                                        }
                                    }
                                    if let Some(guidance) = detail.developer_guidance.as_ref() {
                                        div {
                                            style: format!(
                                                "display: grid; gap: 6px; margin-bottom: 10px; padding: 8px; \
                                                 border: 1px solid {C_BORDER}; border-radius: 6px; background: #101113;"
                                            ),
                                            p {
                                                style: format!(
                                                    "font-size: 10px; font-weight: 700; letter-spacing: 0.8px; \
                                                     text-transform: uppercase; color: {C_TEXT_MUTED};"
                                                ),
                                                "Guidance"
                                            }
                                            p {
                                                style: format!(
                                                    "font-size: 11px; color: {C_TEXT_SECONDARY}; line-height: 1.5;"
                                                ),
                                                "{guidance.remediation}"
                                            }
                                            if !guidance.alternative_apis.is_empty() {
                                                p {
                                                    style: format!(
                                                        "font-size: 10px; color: {C_TEXT_MUTED}; font-family: {FONT_MONO}; \
                                                         overflow-wrap: anywhere;"
                                                    ),
                                                    "{guidance.alternative_apis.join(\", \")}"
                                                }
                                            }
                                        }
                                    }
                            if !jump_targets.is_empty() {
                                div {
                                    style: "display: grid; gap: 6px; margin-bottom: 10px;",
                                    p {
                                        style: format!(
                                            "font-size: 10px; font-weight: 700; letter-spacing: 0.8px; \
                                             text-transform: uppercase; color: {C_TEXT_MUTED};"
                                        ),
                                        "Jump Targets"
                                    }
                                    for (target_index, (target_type, target_method)) in jump_targets.iter().enumerate() {
                                        {
                                            let click_type = target_type.clone();
                                            let click_method = target_method.clone();
                                            rsx! {
                                                button {
                                                    key: "finding-target-{target_index}-{target_type}-{target_method}",
                                                    style: format!(
                                                        "width: 100%; min-width: 0; display: grid; gap: 2px; padding: 7px 8px; \
                                                         border-radius: 7px; border: 1px solid {C_BORDER}; background: #101113; \
                                                         color: {C_TEXT_SECONDARY}; text-align: left; cursor: pointer;"
                                                    ),
                                                    onclick: move |_| {
                                                        open_method_tab(
                                                            &click_type,
                                                            &click_method,
                                                            open_tabs,
                                                            active_tab_id,
                                                            selected_finding,
                                                            Some(selected_finding_index),
                                                        );
                                                    },
                                                    span {
                                                        style: format!(
                                                            "font-size: 10px; color: {C_TEXT_PRIMARY}; font-family: {FONT_MONO}; \
                                                             overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                                                        ),
                                                        "{target_method}"
                                                    }
                                                    span {
                                                        style: format!(
                                                            "font-size: 9px; color: {C_TEXT_MUTED}; font-family: {FONT_MONO}; \
                                                             overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                                                        ),
                                                        "{target_type}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
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
    }
}

#[component]
fn ScanOverviewCard(overview: UiScanOverview) -> Element {
    let classification_color =
        disposition_color(&overview.classification, overview.blocking_recommended);
    let completeness_color = if overview.review_recommended {
        C_ACCENT_AMBER
    } else {
        C_TEXT_MUTED
    };

    rsx! {
        div {
            style: format!(
                "margin: 0 8px 8px; padding: 10px; display: grid; gap: 8px; \
                 border: 1px solid {C_BORDER}; border-radius: 8px; background: {C_BG_ELEVATED};"
            ),
            div {
                style: "display: flex; align-items: flex-start; justify-content: space-between; gap: 8px;",
                div {
                    style: "min-width: 0; display: grid; gap: 3px;",
                    span {
                        style: format!(
                            "font-size: 10px; font-weight: 700; letter-spacing: 0.8px; \
                             text-transform: uppercase; color: {C_TEXT_MUTED};"
                        ),
                        "Disposition"
                    }
                    span {
                        style: format!(
                            "font-size: 12px; font-weight: 700; color: {C_TEXT_PRIMARY}; \
                             overflow-wrap: anywhere;"
                        ),
                        "{overview.headline}"
                    }
                }
                span {
                    class: "sev-badge",
                    style: format!("color: {classification_color}; flex-shrink: 0;"),
                    "{overview.classification}"
                }
            }
            p {
                style: format!(
                    "font-size: 11px; color: {C_TEXT_SECONDARY}; line-height: 1.5;"
                ),
                "{overview.summary}"
            }
            div {
                style: "display: flex; flex-wrap: wrap; gap: 6px;",
                if overview.blocking_recommended {
                    span {
                        class: "badge",
                        style: format!("color: {classification_color}; border-color: {classification_color}66;"),
                        "Block"
                    }
                }
                span {
                    class: "badge",
                    style: format!("color: {completeness_color}; border-color: {completeness_color}66;"),
                    "{overview.completeness_status}"
                }
                if let Some(primary_family) = overview.primary_threat_family_id.as_ref() {
                    span {
                        class: "badge",
                        style: format!("font-family: {FONT_MONO};"),
                        "{primary_family}"
                    }
                }
            }
            if overview.review_recommended || !overview.completeness_reasons.is_empty() {
                div {
                    style: format!(
                        "display: grid; gap: 4px; padding-top: 2px; color: {C_TEXT_MUTED}; \
                         font-size: 10px; line-height: 1.45;"
                    ),
                    if overview.completeness_reasons.is_empty() {
                        p { "Manual review recommended for this scan result." }
                    } else {
                        for (reason_index, reason) in overview.completeness_reasons.iter().enumerate() {
                            p { key: "reason-{reason_index}", "{reason}" }
                        }
                    }
                }
            }
            if !overview.threat_families.is_empty() {
                div {
                    style: "display: grid; gap: 6px;",
                    for family in overview.threat_families.iter() {
                        {
                            let confidence = format!("{:.0}%", family.confidence * 100.0);
                            rsx! {
                                div {
                                    key: "{family.family_id}-{family.match_kind}",
                                    style: format!(
                                        "display: grid; gap: 3px; padding: 7px 8px; border-radius: 6px; \
                                         border: 1px solid {C_BORDER}; background: #101113;"
                                    ),
                                    div {
                                        style: "display: flex; align-items: center; justify-content: space-between; gap: 8px;",
                                        span {
                                            style: format!(
                                                "min-width: 0; font-size: 11px; font-weight: 700; color: {C_TEXT_PRIMARY}; \
                                                 overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                                            ),
                                            if family.display_name.is_empty() {
                                                "{family.family_id}"
                                            } else {
                                                "{family.display_name}"
                                            }
                                        }
                                        span {
                                            class: "badge",
                                            style: format!("font-family: {FONT_MONO};"),
                                            "{confidence}"
                                        }
                                    }
                                    p {
                                        style: format!(
                                            "font-size: 10px; color: {C_TEXT_MUTED}; line-height: 1.45;"
                                        ),
                                        "{family.match_kind}"
                                        if family.exact_hash_match {
                                            " exact hash"
                                        }
                                    }
                                    if !family.summary.is_empty() {
                                        p {
                                            style: format!(
                                                "font-size: 10px; color: {C_TEXT_SECONDARY}; line-height: 1.45;"
                                            ),
                                            "{family.summary}"
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

fn disposition_color(classification: &str, blocking_recommended: bool) -> &'static str {
    if blocking_recommended {
        return C_ACCENT_AMBER;
    }

    match classification {
        "KnownThreat" | "Suspicious" => C_ACCENT_AMBER,
        "ManualReview" | "Incomplete" => C_TEXT_SECONDARY,
        "Clean" => C_TEXT_MUTED,
        _ => C_TEXT_SECONDARY,
    }
}

fn open_method_tab(
    type_name: &str,
    method_name: &str,
    mut open_tabs: Signal<Vec<IlTab>>,
    mut active_tab_id: Signal<Option<String>>,
    mut selected_finding: Signal<Option<usize>>,
    finding_index: Option<usize>,
) {
    let tab_id = method_tab_id(type_name, method_name);
    {
        let mut tabs = open_tabs.write();
        if !tabs.iter().any(|tab| tab.id == tab_id) {
            tabs.push(IlTab {
                id: tab_id.clone(),
                kind: IlTabKind::Method,
                type_name: type_name.to_string(),
                method_name: Some(method_name.to_string()),
                metadata_token: None,
                title: method_name.to_string(),
                subtitle: type_name.to_string(),
            });
        }
    }

    active_tab_id.set(Some(tab_id));
    selected_finding.set(finding_index);
}

fn resolved_finding_navigation_targets(
    methods: &[UiMethod],
    finding: &UiFinding,
) -> Vec<(String, String)> {
    let mut targets = Vec::new();

    if let Some(navigation) = finding.navigation.as_ref() {
        for span in &navigation.method_spans {
            if let Some(target) =
                resolve_method_reference(methods, &span.type_name, &span.method_name)
            {
                if !targets.contains(&target) {
                    targets.push(target);
                }
            }
        }

        if let Some(target) = resolve_method_reference(
            methods,
            &navigation.primary_type_name,
            &navigation.primary_method_name,
        ) {
            if !targets.contains(&target) {
                targets.push(target);
            }
        }
    }

    if targets.is_empty() {
        if let Some(target) = resolve_finding_target(methods, finding) {
            targets.push(target);
        }
    }

    targets
}

#[cfg(test)]
mod tests {
    use super::resolved_finding_navigation_targets;
    use crate::components::view_models::{
        UiFinding, UiFindingMethodSpan, UiFindingNavigation, UiMethod,
    };

    #[test]
    fn finding_detail_targets_include_all_resolved_cross_method_spans() {
        let methods = vec![
            UiMethod {
                type_name: "Unity.UnityCalifornia".to_string(),
                method_name: ".cctor".to_string(),
                metadata_token: None,
                signature: String::new(),
                instructions: Vec::new(),
            },
            UiMethod {
                type_name: "Unity.UnityOhio".to_string(),
                method_name: "Doral".to_string(),
                metadata_token: None,
                signature: String::new(),
                instructions: Vec::new(),
            },
            UiMethod {
                type_name: "Unity.UnityMichigan".to_string(),
                method_name: "Kool".to_string(),
                metadata_token: None,
                signature: String::new(),
                instructions: Vec::new(),
            },
        ];
        let finding = UiFinding {
            rule_id: "ObfuscatedReflectiveExecutionRule".to_string(),
            severity: "Critical".to_string(),
            location: "Unity".to_string(),
            description: String::new(),
            code_snippet: String::new(),
            visibility: None,
            risk_score: None,
            developer_guidance: None,
            il_offset: None,
            navigation: Some(UiFindingNavigation {
                primary_type_name: "Unity.UnityCalifornia".to_string(),
                primary_method_name: ".cctor".to_string(),
                method_spans: vec![
                    UiFindingMethodSpan {
                        type_name: "Unity.UnityCalifornia".to_string(),
                        method_name: ".cctor".to_string(),
                        il_offsets: Vec::new(),
                        csharp_snippets: Vec::new(),
                    },
                    UiFindingMethodSpan {
                        type_name: "Unity.UnityOhio".to_string(),
                        method_name: "Doral".to_string(),
                        il_offsets: Vec::new(),
                        csharp_snippets: Vec::new(),
                    },
                    UiFindingMethodSpan {
                        type_name: "Unity.UnityMichigan".to_string(),
                        method_name: "Kool".to_string(),
                        il_offsets: Vec::new(),
                        csharp_snippets: Vec::new(),
                    },
                ],
            }),
        };

        assert_eq!(
            resolved_finding_navigation_targets(&methods, &finding),
            vec![
                ("Unity.UnityCalifornia".to_string(), ".cctor".to_string()),
                ("Unity.UnityOhio".to_string(), "Doral".to_string()),
                ("Unity.UnityMichigan".to_string(), "Kool".to_string()),
            ]
        );
    }
}
