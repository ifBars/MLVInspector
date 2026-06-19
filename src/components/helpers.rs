/// Pure helper functions for data extraction and UI utilities.
use std::collections::BTreeMap;

use crate::ipc::{
    AttributeMetadataEntry, CallChainEntry, DataFlowChainEntry, DecompileSourceSpan, FindingEntry,
    MemberMetadataEntry, TypeEntry,
};
use crate::types::AnalysisResult;

use super::view_models::{
    UiAttributeMetadata, UiFinding, UiFindingMethodSpan, UiFindingNavigation, UiInstruction,
    UiMemberMetadata, UiMethod, UiNamespaceGroup, UiScanNeighborhood, UiScanNeighborhoodNode,
    UiTypeDetails, UiTypeGroup,
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
                metadata_token: m.metadata_token.clone(),
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

pub fn extract_scan_neighborhoods(
    result: &AnalysisResult,
    type_name: &str,
    method_name: Option<&str>,
) -> Vec<UiScanNeighborhood> {
    let Some(scan) = result.scan.as_ref() else {
        return Vec::new();
    };

    scan.findings
        .iter()
        .enumerate()
        .filter(|(_, finding)| finding_touches_symbol(finding, type_name, method_name))
        .map(|(finding_index, finding)| build_scan_neighborhood(finding_index, finding))
        .collect()
}

pub fn extract_type_details(result: &AnalysisResult, type_name: &str) -> Option<UiTypeDetails> {
    let explore = result.explore.as_ref()?;
    let ty = explore.types.iter().find(|ty| ty.type_name == type_name)?;

    Some(UiTypeDetails {
        full_type_name: ty.type_name.clone(),
        metadata_token: ty.metadata_token.clone(),
        kind: normalize_type_kind(&ty.kind),
        fields: ty.fields.iter().map(map_member_metadata).collect(),
        properties: ty.properties.iter().map(map_member_metadata).collect(),
        events: ty.events.iter().map(map_member_metadata).collect(),
        nested_types: ty.nested_types.iter().map(map_member_metadata).collect(),
        custom_attributes: ty
            .custom_attributes
            .iter()
            .map(map_attribute_metadata)
            .collect(),
    })
}

pub fn group_types_by_namespace(
    types: &[TypeEntry],
    methods: &[UiMethod],
) -> Vec<UiNamespaceGroup> {
    let mut namespaces: BTreeMap<String, BTreeMap<String, UiTypeGroup>> = BTreeMap::new();

    for ty in types {
        let full_type_name = ty.type_name.clone();
        let namespace = namespace_for_type(&full_type_name);

        namespaces.entry(namespace).or_default().insert(
            full_type_name.clone(),
            UiTypeGroup {
                full_type_name: full_type_name.clone(),
                display_name: display_name_for_type(&full_type_name),
                metadata_token: ty.metadata_token.clone(),
                kind: normalize_type_kind(&ty.kind),
                methods: Vec::new(),
            },
        );
    }

    for method in methods {
        let full_type_name = method.type_name.clone();
        let namespace = namespace_for_type(&full_type_name);
        let entry = namespaces
            .entry(namespace)
            .or_default()
            .entry(full_type_name.clone())
            .or_insert_with(|| UiTypeGroup {
                full_type_name: full_type_name.clone(),
                display_name: display_name_for_type(&full_type_name),
                metadata_token: None,
                kind: "class".to_string(),
                methods: Vec::new(),
            });
        entry.methods.push(method.clone());
    }

    namespaces
        .into_iter()
        .map(|(namespace_name, type_map)| {
            let mut types = type_map
                .into_values()
                .map(|mut group| {
                    group
                        .methods
                        .sort_by(|a, b| a.method_name.cmp(&b.method_name));
                    group
                })
                .collect::<Vec<_>>();

            types.sort_by(|a, b| a.display_name.cmp(&b.display_name));

            UiNamespaceGroup {
                namespace_name,
                types,
            }
        })
        .collect()
}

fn map_member_metadata(member: &MemberMetadataEntry) -> UiMemberMetadata {
    UiMemberMetadata {
        name: member.name.clone(),
        metadata_token: member.metadata_token.clone(),
        kind: member.kind.clone(),
        signature: member.signature.clone(),
        attributes: member.attributes.clone(),
    }
}

fn map_attribute_metadata(attribute: &AttributeMetadataEntry) -> UiAttributeMetadata {
    UiAttributeMetadata {
        attribute_type: attribute.attribute_type.clone(),
        summary: attribute.summary.clone(),
    }
}

fn build_scan_neighborhood(finding_index: usize, finding: &FindingEntry) -> UiScanNeighborhood {
    let navigation = build_finding_navigation(finding);
    let (chain_kind, title, mut nodes) = if let Some(data_flow) = finding.data_flow_chain.as_ref() {
        (
            "Data Flow".to_string(),
            data_flow_title(data_flow),
            data_flow
                .nodes
                .iter()
                .map(|node| UiScanNeighborhoodNode {
                    node_type: node.node_type.clone(),
                    location: node.location.clone(),
                    operation: node.operation.clone(),
                    description: node.data_description.clone(),
                    instruction_offset: Some(node.instruction_offset),
                })
                .collect::<Vec<_>>(),
        )
    } else if let Some(call_chain) = finding.call_chain.as_ref() {
        (
            "Call Chain".to_string(),
            call_chain_title(call_chain),
            call_chain
                .nodes
                .iter()
                .map(|node| UiScanNeighborhoodNode {
                    node_type: node.node_type.clone(),
                    location: node.location.clone(),
                    operation: String::new(),
                    description: node.description.clone(),
                    instruction_offset: None,
                })
                .collect::<Vec<_>>(),
        )
    } else {
        (
            "Finding".to_string(),
            finding
                .rule_id
                .clone()
                .unwrap_or_else(|| "UnknownRule".to_string()),
            Vec::new(),
        )
    };

    if nodes.is_empty() {
        nodes.push(UiScanNeighborhoodNode {
            node_type: "finding".to_string(),
            location: finding.location.clone(),
            operation: String::new(),
            description: finding.description.clone(),
            instruction_offset: parse_il_offset_from_snippet(
                finding.code_snippet.as_deref().unwrap_or(""),
            )
            .map(|offset| offset as i32),
        });
    }

    UiScanNeighborhood {
        finding_index,
        rule_id: finding
            .rule_id
            .clone()
            .unwrap_or_else(|| "UnknownRule".to_string()),
        severity: finding.severity.clone(),
        chain_kind,
        title,
        description: finding.description.clone(),
        location: finding.location.clone(),
        primary_type_name: navigation
            .as_ref()
            .map(|navigation| navigation.primary_type_name.clone()),
        primary_method_name: navigation
            .as_ref()
            .map(|navigation| navigation.primary_method_name.clone()),
        nodes,
    }
}

fn data_flow_title(data_flow: &DataFlowChainEntry) -> String {
    if !data_flow.pattern.is_empty() {
        data_flow.pattern.clone()
    } else {
        data_flow.description.clone()
    }
}

fn call_chain_title(call_chain: &CallChainEntry) -> String {
    if !call_chain.description.is_empty() {
        call_chain.description.clone()
    } else {
        call_chain.rule_id.clone()
    }
}

fn finding_touches_symbol(
    finding: &FindingEntry,
    type_name: &str,
    method_name: Option<&str>,
) -> bool {
    if location_matches_symbol(&finding.location, type_name, method_name) {
        return true;
    }

    if let Some(navigation) = build_finding_navigation(finding) {
        if navigation
            .method_spans
            .iter()
            .any(|span| symbol_matches(&span.type_name, &span.method_name, type_name, method_name))
        {
            return true;
        }
    }

    finding
        .call_chain
        .as_ref()
        .is_some_and(|chain| call_chain_touches_symbol(chain, type_name, method_name))
        || finding
            .data_flow_chain
            .as_ref()
            .is_some_and(|chain| data_flow_touches_symbol(chain, type_name, method_name))
}

fn call_chain_touches_symbol(
    chain: &CallChainEntry,
    type_name: &str,
    method_name: Option<&str>,
) -> bool {
    chain
        .nodes
        .iter()
        .any(|node| location_matches_symbol(&node.location, type_name, method_name))
}

fn data_flow_touches_symbol(
    chain: &DataFlowChainEntry,
    type_name: &str,
    method_name: Option<&str>,
) -> bool {
    location_matches_symbol(&chain.method_location, type_name, method_name)
        || chain.involved_methods.as_ref().is_some_and(|methods| {
            methods
                .iter()
                .any(|method| location_matches_symbol(method, type_name, method_name))
        })
        || chain.nodes.iter().any(|node| {
            location_matches_symbol(&node.location, type_name, method_name)
                || node
                    .method_key
                    .as_deref()
                    .is_some_and(|key| location_matches_symbol(key, type_name, method_name))
                || node
                    .target_method_key
                    .as_deref()
                    .is_some_and(|key| location_matches_symbol(key, type_name, method_name))
        })
}

fn location_matches_symbol(location: &str, type_name: &str, method_name: Option<&str>) -> bool {
    parse_method_location(location)
        .map(|(location_type, location_method)| {
            symbol_matches(&location_type, &location_method, type_name, method_name)
        })
        .unwrap_or_else(|| {
            method_name.is_none() && type_name_mentions_candidate(location, type_name)
        })
}

fn symbol_matches(
    location_type: &str,
    location_method: &str,
    type_name: &str,
    method_name: Option<&str>,
) -> bool {
    if !type_name_mentions_candidate(location_type, type_name) {
        return false;
    }

    method_name
        .map(|method_name| method_name_mentions_candidate(location_method, method_name))
        .unwrap_or(true)
}

fn namespace_for_type(full_type_name: &str) -> String {
    full_type_name
        .rsplit_once('.')
        .map(|(ns, _)| ns.to_string())
        .unwrap_or_else(|| "(global)".to_string())
}

fn display_name_for_type(full_type_name: &str) -> String {
    full_type_name
        .rsplit('.')
        .next()
        .unwrap_or(full_type_name)
        .to_string()
}

fn normalize_type_kind(kind: &str) -> String {
    match kind.trim().to_ascii_lowercase().as_str() {
        "struct" | "interface" | "enum" | "delegate" | "class" => kind.trim().to_ascii_lowercase(),
        _ => "class".to_string(),
    }
}

pub fn resolve_finding_target(
    methods: &[UiMethod],
    finding: &UiFinding,
) -> Option<(String, String)> {
    if let Some(navigation) = finding.navigation.as_ref() {
        for span in &navigation.method_spans {
            if let Some(resolved) =
                resolve_method_reference(methods, &span.type_name, &span.method_name)
            {
                return Some(resolved);
            }
        }

        if let Some(resolved) = resolve_method_reference(
            methods,
            &navigation.primary_type_name,
            &navigation.primary_method_name,
        ) {
            return Some(resolved);
        }
    }

    parse_method_location(&finding.location).and_then(|(type_name, method_name)| {
        resolve_method_reference(methods, &type_name, &method_name)
    })
}

// Tab ID helpers

pub fn assembly_metadata_tab_id(assembly_id: &str) -> String {
    format!("assembly::{assembly_id}::metadata")
}

pub fn type_tab_id(type_name: &str) -> String {
    format!("type::{type_name}")
}

pub fn should_retry_decompile_source(source: &str) -> bool {
    source.contains("Could not find type definition System.Net.WebClient")
        || source.contains("Handle with invalid row number")
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
    if let Some(type_name) = location.strip_suffix("..cctor") {
        let type_name = type_name.trim();
        if !type_name.is_empty() {
            return Some((type_name.to_string(), ".cctor".to_string()));
        }
    }
    if let Some(type_name) = location.strip_suffix("..ctor") {
        let type_name = type_name.trim();
        if !type_name.is_empty() {
            return Some((type_name.to_string(), ".ctor".to_string()));
        }
    }

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

fn parse_labeled_method_locations(snippet: &str) -> Vec<((String, String), String)> {
    let mut locations = Vec::new();

    for line in snippet.lines() {
        let line = line.trim();
        let Some((_, value)) = line.split_once(':') else {
            continue;
        };

        for token in method_location_tokens(value) {
            let Some(method_location) = parse_method_location(&token) else {
                continue;
            };

            if !locations
                .iter()
                .any(|(existing, _)| existing == &method_location)
            {
                locations.push((method_location, line.to_string()));
            }
        }
    }

    locations
}

fn method_location_tokens(value: &str) -> Vec<String> {
    value
        .split([' ', '\t', ',', ';'])
        .map(|token| {
            token
                .trim()
                .trim_matches(|ch: char| matches!(ch, '(' | ')' | '[' | ']' | '{' | '}' | '`'))
                .trim_matches('.')
        })
        .filter(|token| {
            !token.is_empty()
                && !token.contains("://")
                && token.contains('.')
                && token.chars().all(|ch| {
                    ch.is_ascii_alphanumeric()
                        || matches!(ch, '_' | '.' | '/' | '<' | '>' | '`' | ':' | '-')
                })
        })
        .map(ToOwned::to_owned)
        .collect()
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
    let code_snippet = finding.code_snippet.as_deref().unwrap_or("");

    push_method_span(
        &mut method_spans,
        parse_method_location(&finding.location),
        parse_il_offsets_from_snippet(code_snippet),
        snippets_for_csharp_matching(code_snippet),
    );

    for (method_location, snippet_line) in parse_labeled_method_locations(code_snippet) {
        push_method_span(
            &mut method_spans,
            Some(method_location),
            Vec::new(),
            vec![snippet_line],
        );
    }

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
        assembly_metadata_tab_id, extract_scan_neighborhoods, group_types_by_namespace,
        highlighted_csharp_lines, highlighted_csharp_lines_from_source_spans,
        parse_il_offset_from_snippet, parse_il_offsets_from_snippet, resolve_finding_target,
        resolve_method_reference, should_retry_decompile_source,
    };
    use crate::{
        components::view_models::{UiFinding, UiFindingMethodSpan, UiFindingNavigation, UiMethod},
        ipc::{
            AttributeMetadataEntry, DataFlowChainEntry, DataFlowNodeEntry, DecompileSourceSpan,
            ExplorePayload, FindingEntry, MemberMetadataEntry, ScanInputEntry, ScanMetaEntry,
            ScanPayload, ScanSummaryEntry, TypeEntry,
        },
        types::AnalysisResult,
    };
    use std::collections::HashMap;

    #[test]
    fn group_types_by_namespace_includes_structs_without_methods() {
        let types = vec![TypeEntry {
            type_name: "Demo.Models.Point".to_string(),
            metadata_token: Some("0x02000004".to_string()),
            kind: "struct".to_string(),
            fields: Vec::new(),
            properties: Vec::new(),
            events: Vec::new(),
            nested_types: Vec::new(),
            custom_attributes: Vec::new(),
            methods: Vec::new(),
        }];

        let groups = group_types_by_namespace(&types, &[]);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].namespace_name, "Demo.Models");
        assert_eq!(groups[0].types.len(), 1);
        assert_eq!(groups[0].types[0].display_name, "Point");
        assert_eq!(
            groups[0].types[0].metadata_token.as_deref(),
            Some("0x02000004")
        );
        assert_eq!(groups[0].types[0].kind, "struct");
        assert!(groups[0].types[0].methods.is_empty());
    }

    #[test]
    fn extract_type_details_projects_member_metadata() {
        let result = AnalysisResult {
            assembly_path: "C:/sample.dll".to_string(),
            mode: "combined".to_string(),
            explore: Some(ExplorePayload {
                assembly_path: "C:/sample.dll".to_string(),
                assembly_metadata: Default::default(),
                methods: Vec::new(),
                types: vec![TypeEntry {
                    type_name: "Demo.Widget".to_string(),
                    metadata_token: Some("0x02000004".to_string()),
                    kind: "class".to_string(),
                    fields: vec![MemberMetadataEntry {
                        name: "counter".to_string(),
                        metadata_token: Some("0x04000001".to_string()),
                        kind: "field".to_string(),
                        signature: "private Int32 counter".to_string(),
                        attributes: Some("Private".to_string()),
                    }],
                    properties: vec![MemberMetadataEntry {
                        name: "Name".to_string(),
                        metadata_token: Some("0x17000001".to_string()),
                        kind: "property".to_string(),
                        signature: "String Name {get; set}".to_string(),
                        attributes: None,
                    }],
                    events: vec![MemberMetadataEntry {
                        name: "Changed".to_string(),
                        metadata_token: Some("0x14000001".to_string()),
                        kind: "event".to_string(),
                        signature: "EventHandler Changed".to_string(),
                        attributes: None,
                    }],
                    nested_types: vec![MemberMetadataEntry {
                        name: "Demo.Widget/Child".to_string(),
                        metadata_token: Some("0x02000005".to_string()),
                        kind: "class".to_string(),
                        signature: "Demo.Widget/Child".to_string(),
                        attributes: None,
                    }],
                    custom_attributes: vec![AttributeMetadataEntry {
                        attribute_type: "System.ObsoleteAttribute".to_string(),
                        summary: Some("ctor(\"probe\")".to_string()),
                    }],
                    methods: Vec::new(),
                }],
            }),
            scan: None,
            stderr: String::new(),
        };

        let details = super::extract_type_details(&result, "Demo.Widget")
            .expect("type details should be projected");

        assert_eq!(details.full_type_name, "Demo.Widget");
        assert_eq!(details.fields[0].name, "counter");
        assert_eq!(
            details.properties[0].metadata_token.as_deref(),
            Some("0x17000001")
        );
        assert_eq!(details.events[0].signature, "EventHandler Changed");
        assert_eq!(details.nested_types[0].kind, "class");
        assert_eq!(
            details.custom_attributes[0].attribute_type,
            "System.ObsoleteAttribute"
        );
        assert!(super::extract_type_details(&result, "Demo.Missing").is_none());
    }

    #[test]
    fn assembly_metadata_tab_id_includes_assembly_identity() {
        assert_eq!(
            assembly_metadata_tab_id("sample-assembly"),
            "assembly::sample-assembly::metadata"
        );
    }

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
    fn parse_method_location_handles_static_constructor_anchor() {
        assert_eq!(
            super::parse_method_location("Unity.UnityCalifornia..cctor"),
            Some(("Unity.UnityCalifornia".to_string(), ".cctor".to_string()))
        );
    }

    #[test]
    fn resolve_method_reference_matches_signature_style_location() {
        let methods = vec![UiMethod {
            type_name: "Demo.Service".to_string(),
            method_name: "Run".to_string(),
            metadata_token: None,
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
            metadata_token: None,
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

    #[test]
    fn should_retry_decompile_source_detects_known_stale_errors() {
        assert!(should_retry_decompile_source(
            "// Decompilation error:\n// Handle with invalid row number."
        ));
        assert!(should_retry_decompile_source(
            "// Decompilation error:\n// Could not find type definition System.Net.WebClient in type system."
        ));
        assert!(!should_retry_decompile_source("public class Core {}"));
    }

    #[test]
    fn resolve_finding_target_prefers_in_assembly_method_over_external_navigation_target() {
        let methods = vec![UiMethod {
            type_name: "CustomerSearcher.Core".to_string(),
            method_name: "DownloadRun".to_string(),
            metadata_token: None,
            signature: String::new(),
            instructions: Vec::new(),
        }];
        let finding = UiFinding {
            rule_id: "MLV-TEST".to_string(),
            severity: "High".to_string(),
            location: "System.Net.WebClient::DownloadFileTaskAsync".to_string(),
            description: String::new(),
            code_snippet: String::new(),
            il_offset: None,
            navigation: Some(UiFindingNavigation {
                primary_type_name: "System.Net.WebClient".to_string(),
                primary_method_name: "DownloadFileTaskAsync".to_string(),
                method_spans: vec![UiFindingMethodSpan {
                    type_name: "CustomerSearcher.Core/<DownloadRun>d__2".to_string(),
                    method_name: "MoveNext".to_string(),
                    il_offsets: vec![],
                    csharp_snippets: vec![],
                }],
            }),
        };

        assert_eq!(
            resolve_finding_target(&methods, &finding),
            Some((
                "CustomerSearcher.Core".to_string(),
                "DownloadRun".to_string()
            ))
        );
    }

    #[test]
    fn resolve_finding_target_uses_obfuscated_rule_labeled_snippet_locations() {
        let finding = FindingEntry {
            id: None,
            rule_id: Some("ObfuscatedReflectiveExecutionRule".to_string()),
            severity: "Critical".to_string(),
            location: "Unity".to_string(),
            description: String::new(),
            code_snippet: Some(
                "remote config: Unity.UnityCalifornia..cctor\n\
                 hex decode: Unity.UnityOhio.Doral\n\
                 byte string decode: Unity.UnityOhio.Doral\n\
                 property assignment: Unity.UnityMichigan.Kool\n\
                 reflection invoke: Unity.UnityMichigan.Kool\n\
                 staging: WebClient.DownloadString -> GetTempFileName + .cmd -> File.WriteAllText\n\
                 execution: ProcessStartInfo FileName=cmd.exe Arguments=/c WindowStyle=Hidden UseShellExecute=True"
                    .to_string(),
            ),
            call_chain: None,
            data_flow_chain: None,
        };
        let navigation = super::build_finding_navigation(&finding)
            .expect("snippet anchors should produce navigation");
        let ui_finding = UiFinding {
            rule_id: "ObfuscatedReflectiveExecutionRule".to_string(),
            severity: "Critical".to_string(),
            location: "Unity".to_string(),
            description: String::new(),
            code_snippet: finding.code_snippet.unwrap_or_default(),
            il_offset: None,
            navigation: Some(navigation),
        };
        let methods = vec![UiMethod {
            type_name: "Unity.UnityMichigan".to_string(),
            method_name: "Kool".to_string(),
            metadata_token: Some("0x06000009".to_string()),
            signature: String::new(),
            instructions: Vec::new(),
        }];

        assert_eq!(
            resolve_finding_target(&methods, &ui_finding),
            Some(("Unity.UnityMichigan".to_string(), "Kool".to_string()))
        );
    }

    #[test]
    fn extract_scan_neighborhoods_matches_data_flow_nodes_for_method() {
        let result = AnalysisResult {
            assembly_path: r"C:\samples\First.dll".to_string(),
            mode: "scan".to_string(),
            explore: None,
            scan: Some(ScanPayload {
                assembly_path: r"C:\samples\First.dll".to_string(),
                schema_version: "1.2.0".to_string(),
                metadata: ScanMetaEntry {
                    scanner_version: "test".to_string(),
                    timestamp: "2026-06-17T00:00:00Z".to_string(),
                    scan_mode: "static".to_string(),
                    platform: "test".to_string(),
                },
                input: ScanInputEntry {
                    file_name: "First.dll".to_string(),
                    size_bytes: 10,
                    sha256_hash: None,
                },
                summary: ScanSummaryEntry {
                    total_findings: 1,
                    count_by_severity: HashMap::new(),
                    triggered_rules: vec!["DataFlowAnalysis".to_string()],
                },
                findings: vec![FindingEntry {
                    id: Some("finding-1".to_string()),
                    rule_id: Some("DataFlowAnalysis".to_string()),
                    severity: "High".to_string(),
                    location: "Example.Loader.Run".to_string(),
                    description: "Network data reaches process execution".to_string(),
                    code_snippet: None,
                    call_chain: None,
                    data_flow_chain: Some(DataFlowChainEntry {
                        id: "flow-1".to_string(),
                        description: "download and execute".to_string(),
                        severity: "High".to_string(),
                        pattern: "DownloadAndExecute".to_string(),
                        source_variable: Some("payload".to_string()),
                        method_location: "Example.Loader.Run".to_string(),
                        is_cross_method: Some(true),
                        involved_methods: Some(vec!["Example.Loader.Download".to_string()]),
                        nodes: vec![DataFlowNodeEntry {
                            node_type: "sink".to_string(),
                            location: "Example.Loader.Run".to_string(),
                            operation: "Process.Start".to_string(),
                            data_description: "payload execution".to_string(),
                            instruction_offset: 42,
                            method_key: Some("Example.Loader.Run".to_string()),
                            is_method_boundary: Some(false),
                            target_method_key: None,
                            code_snippet: Some("IL_002A call Process.Start".to_string()),
                        }],
                    }),
                }],
                call_chains: None,
                data_flows: None,
            }),
            stderr: String::new(),
        };

        let neighborhoods = extract_scan_neighborhoods(&result, "Example.Loader", Some("Run"));

        assert_eq!(neighborhoods.len(), 1);
        assert_eq!(neighborhoods[0].finding_index, 0);
        assert_eq!(neighborhoods[0].rule_id, "DataFlowAnalysis");
        assert_eq!(neighborhoods[0].chain_kind, "Data Flow");
        assert_eq!(neighborhoods[0].title, "DownloadAndExecute");
        assert_eq!(neighborhoods[0].nodes[0].instruction_offset, Some(42));

        let type_neighborhoods = extract_scan_neighborhoods(&result, "Example.Loader", None);
        assert_eq!(type_neighborhoods.len(), 1);

        let unrelated = extract_scan_neighborhoods(&result, "Example.Other", Some("Run"));
        assert!(unrelated.is_empty());
    }
}
