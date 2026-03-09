/// Pure helper functions for data extraction and UI utilities.
use std::collections::BTreeMap;

use crate::ipc::{DecompileSourceSpan, FindingEntry};
use crate::types::AnalysisResult;

use super::view_models::{
    UiFinding, UiFindingMethodSpan, UiFindingNavigation, UiInstruction, UiMethod, UiNamespaceGroup,
    UiTypeGroup,
};

// Data extraction

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
        .map(|finding| {
            let snippet = finding.code_snippet.as_deref().unwrap_or("");
            UiFinding {
                rule_id: finding
                    .rule_id
                    .as_deref()
                    .unwrap_or("UnknownRule")
                    .to_string(),
                severity: finding.severity.clone(),
                location: finding.location.clone(),
                description: finding.description.clone(),
                code_snippet: snippet.to_string(),
                il_offset: parse_il_offset_from_snippet(snippet),
                navigation: build_finding_navigation(finding),
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

// Tab ID helpers

pub fn type_tab_id(type_name: &str) -> String {
    format!("type::{type_name}")
}

pub fn method_tab_id(type_name: &str, method_name: &str) -> String {
    format!("method::{type_name}::{method_name}")
}

// Misc utilities

pub fn parse_il_offset_from_snippet(snippet: &str) -> Option<i64> {
    parse_il_offsets_from_snippet(snippet).into_iter().next()
}

pub fn parse_il_offsets_from_snippet(snippet: &str) -> Vec<i64> {
    let mut offsets = Vec::new();
    let mut search_start = 0usize;

    while let Some(relative_pos) = snippet[search_start..].find("IL_") {
        let start = search_start + relative_pos + 3;
        let hex_len = snippet[start..]
            .chars()
            .take_while(|ch| ch.is_ascii_hexdigit())
            .count();

        if hex_len < 4 {
            search_start = start;
            continue;
        }

        let end = start + hex_len;
        if let Some(hex) = snippet.get(start..end) {
            if let Ok(offset) = i64::from_str_radix(hex, 16) {
                if !offsets.contains(&offset) {
                    offsets.push(offset);
                }
            }
        }
        search_start = end;
    }

    offsets.sort_unstable();
    offsets
}

pub fn parse_method_location(location: &str) -> Option<(String, String)> {
    let location = strip_trailing_location_suffix(location.trim());
    let (type_name, method_name) = location
        .rsplit_once("::")
        .or_else(|| location.rsplit_once('.'))?;
    let type_name = type_name.trim();
    let method_name = method_name.trim();

    if type_name.is_empty() || method_name.is_empty() {
        return None;
    }

    Some((type_name.to_string(), method_name.to_string()))
}

pub fn resolve_method_reference(
    methods: &[UiMethod],
    type_name: &str,
    method_name: &str,
) -> Option<(String, String)> {
    let target_type = normalize_type_name(type_name);
    let target_method = normalize_method_name(method_name);

    let resolved = methods
        .iter()
        .find(|method| {
            normalize_type_name(&method.type_name) == target_type
                && normalize_method_name(&method.method_name) == target_method
        })
        .or_else(|| {
            methods.iter().find(|method| {
                normalize_type_name(&method.type_name) == target_type
                    && method_name_mentions_candidate(method_name, &method.method_name)
            })
        })
        .or_else(|| {
            methods.iter().find(|method| {
                type_name_mentions_candidate(type_name, &method.type_name)
                    && method_name_mentions_candidate(method_name, &method.method_name)
            })
        })
        .or_else(|| resolve_compiler_generated_owner_reference(methods, type_name, method_name))
        .map(|method| (method.type_name.clone(), method.method_name.clone()));

    tracing::debug!(
        requested_type = %type_name,
        requested_method = %method_name,
        normalized_type = %target_type,
        normalized_method = %target_method,
        method_count = methods.len(),
        resolved = ?resolved,
        "resolved finding method reference"
    );

    resolved
}

pub fn is_compiler_generated_type_name(type_name: &str) -> bool {
    let trimmed = type_name.trim();
    let simple_name = trimmed
        .rsplit_once('/')
        .map(|(_, name)| name)
        .or_else(|| trimmed.rsplit_once('.').map(|(_, name)| name))
        .unwrap_or(trimmed);

    looks_like_compiler_generated_name(simple_name)
}

pub fn highlighted_csharp_lines(source: &str, snippets: &[String]) -> Vec<usize> {
    let normalized_source_lines = source
        .lines()
        .map(normalize_search_line)
        .collect::<Vec<_>>();
    let mut matched_lines = Vec::new();

    for snippet in snippets {
        let snippet_lines = snippet
            .lines()
            .map(normalize_search_line)
            .filter(|line| should_match_csharp_line(line))
            .collect::<Vec<_>>();

        if snippet_lines.is_empty() {
            continue;
        }

        let mut found_block = false;
        for start in 0..normalized_source_lines.len() {
            if start + snippet_lines.len() > normalized_source_lines.len() {
                break;
            }

            let is_match = snippet_lines
                .iter()
                .enumerate()
                .all(|(offset, snippet_line)| {
                    search_lines_match(&normalized_source_lines[start + offset], snippet_line)
                });

            if is_match {
                found_block = true;
                for line_number in start + 1..=start + snippet_lines.len() {
                    if !matched_lines.contains(&line_number) {
                        matched_lines.push(line_number);
                    }
                }
            }
        }

        if found_block {
            continue;
        }

        for snippet_line in snippet_lines {
            for (index, source_line) in normalized_source_lines.iter().enumerate() {
                if search_lines_match(source_line, &snippet_line) {
                    let line_number = index + 1;
                    if !matched_lines.contains(&line_number) {
                        matched_lines.push(line_number);
                    }
                }
            }
        }
    }

    matched_lines.sort_unstable();
    matched_lines
}

pub fn highlighted_csharp_lines_from_source_spans(
    source_spans: &[DecompileSourceSpan],
    finding_span: &UiFindingMethodSpan,
) -> Vec<usize> {
    let mut matched_lines = Vec::new();

    for source_span in source_spans {
        if !source_span_matches_method(source_span, finding_span) {
            continue;
        }

        let matches_offset = finding_span
            .il_offsets
            .iter()
            .any(|offset| source_span_contains_offset(source_span, *offset));

        if !matches_offset {
            continue;
        }

        for line_number in source_span.start_line..=source_span.end_line {
            if !matched_lines.contains(&line_number) {
                matched_lines.push(line_number);
            }
        }
    }

    matched_lines.sort_unstable();
    matched_lines
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

fn build_finding_navigation(finding: &FindingEntry) -> Option<UiFindingNavigation> {
    let mut method_spans = Vec::new();

    push_method_span(
        &mut method_spans,
        parse_method_location(&finding.location),
        parse_il_offsets_from_snippet(finding.code_snippet.as_deref().unwrap_or("")),
        snippets_for_csharp_matching(finding.code_snippet.as_deref().unwrap_or("")),
    );

    if let Some(call_chain) = finding.call_chain.as_ref() {
        for node in &call_chain.nodes {
            push_method_span(
                &mut method_spans,
                parse_method_location(&node.location),
                parse_il_offsets_from_snippet(node.code_snippet.as_deref().unwrap_or("")),
                snippets_for_csharp_matching(node.code_snippet.as_deref().unwrap_or("")),
            );
        }
    }

    if let Some(data_flow_chain) = finding.data_flow_chain.as_ref() {
        push_method_span(
            &mut method_spans,
            parse_method_location(&data_flow_chain.method_location),
            Vec::new(),
            Vec::new(),
        );

        for node in &data_flow_chain.nodes {
            let method_location = parse_method_location(&node.location)
                .or_else(|| node.method_key.as_deref().and_then(parse_method_location))
                .or_else(|| {
                    node.target_method_key
                        .as_deref()
                        .and_then(parse_method_location)
                });

            let mut offsets =
                parse_il_offsets_from_snippet(node.code_snippet.as_deref().unwrap_or(""));
            let node_offset = i64::from(node.instruction_offset);
            if !offsets.contains(&node_offset) {
                offsets.push(node_offset);
                offsets.sort_unstable();
            }

            push_method_span(
                &mut method_spans,
                method_location,
                offsets,
                snippets_for_csharp_matching(node.code_snippet.as_deref().unwrap_or("")),
            );
        }
    }

    let primary_method = parse_method_location(&finding.location).or_else(|| {
        method_spans
            .first()
            .map(|span| (span.type_name.clone(), span.method_name.clone()))
    })?;

    Some(UiFindingNavigation {
        primary_type_name: primary_method.0,
        primary_method_name: primary_method.1,
        method_spans,
    })
}

fn push_method_span(
    method_spans: &mut Vec<UiFindingMethodSpan>,
    method_location: Option<(String, String)>,
    il_offsets: Vec<i64>,
    csharp_snippets: Vec<String>,
) {
    let Some((type_name, method_name)) = method_location else {
        return;
    };

    if let Some(existing) = method_spans
        .iter_mut()
        .find(|span| span.type_name == type_name && span.method_name == method_name)
    {
        for offset in il_offsets {
            if !existing.il_offsets.contains(&offset) {
                existing.il_offsets.push(offset);
            }
        }
        existing.il_offsets.sort_unstable();

        for snippet in csharp_snippets {
            if !existing.csharp_snippets.contains(&snippet) {
                existing.csharp_snippets.push(snippet);
            }
        }
        return;
    }

    let mut span = UiFindingMethodSpan {
        type_name,
        method_name,
        il_offsets,
        csharp_snippets,
    };
    span.il_offsets.sort_unstable();
    method_spans.push(span);
}

fn snippets_for_csharp_matching(snippet: &str) -> Vec<String> {
    let trimmed = snippet.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    vec![trimmed.to_string()]
}

fn normalize_search_line(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn resolve_compiler_generated_owner_reference<'a>(
    methods: &'a [UiMethod],
    type_name: &str,
    method_name: &str,
) -> Option<&'a UiMethod> {
    let owner_method_name = extract_compiler_generated_owner_name(type_name)
        .or_else(|| extract_compiler_generated_owner_name(method_name))?;
    let owner_type_name = compiler_generated_owner_type_name(type_name)?;

    methods.iter().find(|method| {
        normalize_type_name(&method.type_name) == normalize_type_name(&owner_type_name)
            && normalize_method_name(&method.method_name)
                == normalize_method_name(&owner_method_name)
    })
}

fn source_span_matches_method(
    source_span: &DecompileSourceSpan,
    finding_span: &UiFindingMethodSpan,
) -> bool {
    let type_matches = match source_span.type_name.as_deref() {
        Some(type_name) => type_name == finding_span.type_name,
        None => true,
    };
    let method_matches = match source_span.method_name.as_deref() {
        Some(method_name) => method_name == finding_span.method_name,
        None => true,
    };

    type_matches && method_matches
}

fn source_span_contains_offset(source_span: &DecompileSourceSpan, offset: i64) -> bool {
    let start = i64::from(source_span.il_start_offset);
    let end = i64::from(source_span.il_end_offset);

    if end > start {
        offset >= start && offset < end
    } else {
        offset == start
    }
}

fn strip_trailing_location_suffix(location: &str) -> &str {
    let Some((head, tail)) = location.rsplit_once(':') else {
        return location;
    };

    if !tail.is_empty() && tail.chars().all(|ch| ch.is_ascii_digit()) {
        head
    } else {
        location
    }
}

fn normalize_type_name(type_name: &str) -> String {
    type_name.trim().replace('+', ".")
}

fn compiler_generated_owner_type_name(type_name: &str) -> Option<String> {
    let normalized = type_name.trim();
    let (owner_type, generated_type) = normalized.rsplit_once('/')?;

    if !looks_like_compiler_generated_name(generated_type) {
        return None;
    }

    Some(owner_type.to_string())
}

fn extract_compiler_generated_owner_name(value: &str) -> Option<String> {
    let start = value.find('<')?;
    let rest = value.get(start + 1..)?;
    let end = rest.find('>')?;
    let candidate = rest.get(..end)?.trim();

    if candidate.is_empty() {
        None
    } else {
        Some(candidate.to_string())
    }
}

fn looks_like_compiler_generated_name(name: &str) -> bool {
    name.starts_with('<')
}

fn normalize_method_name(method_name: &str) -> String {
    let trimmed = method_name.trim();
    let without_params = trimmed.split('(').next().unwrap_or(trimmed);
    let without_generics = without_params.split('<').next().unwrap_or(without_params);
    without_generics
        .rsplit([' ', ':'])
        .next()
        .unwrap_or(without_generics)
        .trim_matches(':')
        .trim()
        .to_string()
}

fn type_name_mentions_candidate(location_type: &str, candidate_type: &str) -> bool {
    let location_type = normalize_type_name(location_type);
    let candidate_type = normalize_type_name(candidate_type);

    location_type == candidate_type
        || location_type.ends_with(&candidate_type)
        || candidate_type.ends_with(&location_type)
}

fn method_name_mentions_candidate(location_method: &str, candidate_method: &str) -> bool {
    let location_method = normalize_method_name(location_method);
    let candidate_method = normalize_method_name(candidate_method);

    location_method == candidate_method
        || location_method.contains(&candidate_method)
        || candidate_method.contains(&location_method)
}

fn should_match_csharp_line(line: &str) -> bool {
    !line.is_empty() && !line.starts_with("IL_") && line != "{" && line != "}"
}

fn search_lines_match(source_line: &str, snippet_line: &str) -> bool {
    source_line == snippet_line
        || (snippet_line.len() >= 6
            && (source_line.contains(snippet_line) || snippet_line.contains(source_line)))
}

#[cfg(test)]
mod tests {
    use super::{
        highlighted_csharp_lines, highlighted_csharp_lines_from_source_spans,
        parse_il_offset_from_snippet, parse_il_offsets_from_snippet, resolve_method_reference,
    };
    use crate::{
        components::view_models::{UiFindingMethodSpan, UiMethod},
        ipc::DecompileSourceSpan,
    };

    #[test]
    fn parse_il_offset_from_snippet_handles_hex_offsets() {
        assert_eq!(
            parse_il_offset_from_snippet("... IL_002A: call ..."),
            Some(0x2A)
        );
    }

    #[test]
    fn parse_il_offsets_from_snippet_collects_all_unique_offsets() {
        assert_eq!(
            parse_il_offsets_from_snippet("IL_0001: ldarg.0\nIL_000A: call\nIL_0001: ret"),
            vec![1, 10]
        );
    }

    #[test]
    fn highlighted_csharp_lines_matches_multiline_snippets() {
        let source = "public void Run()\n{\n    var sql = input;\n    Execute(sql);\n}";
        let snippets = vec!["var sql = input;\nExecute(sql);".to_string()];

        assert_eq!(highlighted_csharp_lines(source, &snippets), vec![3, 4]);
    }

    #[test]
    fn highlighted_csharp_lines_from_source_spans_matches_il_ranges() {
        let source_spans = vec![
            DecompileSourceSpan {
                type_name: Some("Demo.Service".to_string()),
                method_name: Some("Run".to_string()),
                il_start_offset: 0,
                il_end_offset: 8,
                start_line: 3,
                end_line: 3,
            },
            DecompileSourceSpan {
                type_name: Some("Demo.Service".to_string()),
                method_name: Some("Run".to_string()),
                il_start_offset: 8,
                il_end_offset: 18,
                start_line: 4,
                end_line: 5,
            },
        ];
        let finding_span = UiFindingMethodSpan {
            type_name: "Demo.Service".to_string(),
            method_name: "Run".to_string(),
            il_offsets: vec![8, 12],
            csharp_snippets: Vec::new(),
        };

        assert_eq!(
            highlighted_csharp_lines_from_source_spans(&source_spans, &finding_span),
            vec![4, 5]
        );
    }

    #[test]
    fn parse_method_location_handles_nested_type_with_offset_suffix() {
        assert_eq!(
            super::parse_method_location(
                "CustomTV.Utils.YoutubeUtils.Youtube/<>c__DisplayClass1_1.<DownloadYoutubeVideo>b__1:400"
            ),
            Some((
                "CustomTV.Utils.YoutubeUtils.Youtube/<>c__DisplayClass1_1".to_string(),
                "<DownloadYoutubeVideo>b__1".to_string(),
            ))
        );
    }

    #[test]
    fn resolve_method_reference_matches_signature_style_location() {
        let methods = vec![UiMethod {
            type_name: "Demo.Service".to_string(),
            method_name: "Run".to_string(),
            signature: String::new(),
            instructions: Vec::new(),
        }];

        assert_eq!(
            resolve_method_reference(&methods, "Demo.Service", "System.Void Run(System.String)"),
            Some(("Demo.Service".to_string(), "Run".to_string()))
        );
    }

    #[test]
    fn resolve_method_reference_redirects_async_state_machine_type_to_owner_method() {
        let methods = vec![UiMethod {
            type_name: "CustomerSearcher.Core".to_string(),
            method_name: "DownloadRun".to_string(),
            signature: String::new(),
            instructions: Vec::new(),
        }];

        assert_eq!(
            resolve_method_reference(
                &methods,
                "CustomerSearcher.Core/<DownloadRun>d__2",
                "MoveNext"
            ),
            Some((
                "CustomerSearcher.Core".to_string(),
                "DownloadRun".to_string()
            ))
        );
    }

    #[test]
    fn is_compiler_generated_type_name_detects_nested_state_machine_type() {
        assert!(super::is_compiler_generated_type_name(
            "CustomerSearcher.Core/<DownloadRun>d__2"
        ));
        assert!(!super::is_compiler_generated_type_name(
            "CustomerSearcher.Core"
        ));
    }
}
