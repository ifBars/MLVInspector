/// Pure helper functions for data extraction and UI utilities.
use std::collections::BTreeMap;

use crate::types::AnalysisResult;

use super::view_models::{UiFinding, UiInstruction, UiMethod, UiNamespaceGroup, UiTypeGroup};

// ─── Data extraction ──────────────────────────────────────────────────────────

pub fn extract_methods(result: &AnalysisResult) -> Vec<UiMethod> {
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
                    offset: ins.offset as i64,
                    op_code: ins.op_code.clone(),
                    operand: ins.operand.clone().unwrap_or_default(),
                })
                .collect();

            UiMethod {
                type_name: m.type_name.clone(),
                method_name: m.method_name.clone(),
                signature: m.signature.clone(),
                instructions,
            }
        })
        .collect()
}

pub fn extract_findings(result: &AnalysisResult) -> Vec<UiFinding> {
    let Some(scan) = result.scan.as_ref() else {
        return Vec::new();
    };

    scan.findings
        .iter()
        .map(|f| {
            let snippet = f.code_snippet.as_deref().unwrap_or("");
            UiFinding {
                rule_id: f.rule_id.as_deref().unwrap_or("UnknownRule").to_string(),
                severity: f.severity.clone(),
                location: f.location.clone(),
                description: f.description.clone(),
                code_snippet: snippet.to_string(),
                il_offset: parse_il_offset_from_snippet(snippet),
            }
        })
        .collect()
}

pub fn group_methods_by_namespace(methods: &[UiMethod]) -> Vec<UiNamespaceGroup> {
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

// ─── Tab ID helpers ───────────────────────────────────────────────────────────

pub fn type_tab_id(type_name: &str) -> String {
    format!("type::{type_name}")
}

pub fn method_tab_id(type_name: &str, method_name: &str) -> String {
    format!("method::{type_name}::{method_name}")
}

// ─── Misc utilities ───────────────────────────────────────────────────────────

pub fn parse_il_offset_from_snippet(snippet: &str) -> Option<i64> {
    let pos = snippet.find("IL_")?;
    let hex = snippet.get(pos + 3..pos + 7)?;
    i64::from_str_radix(hex, 16).ok()
}

pub fn severity_color(severity: &str) -> &'static str {
    match severity {
        "Critical" => "#c08b91",
        "High" => "#b59a86",
        "Medium" => "#b8ae96",
        "Low" => "#98a893",
        "Info" => "#8f9dac",
        _ => "#8b919d",
    }
}

pub fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
