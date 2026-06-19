use std::collections::HashMap;

use super::commands::{CommandId, PaletteCommandItem};
use crate::types::{AnalysisEntry, OpenAssembly};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteEntryKind {
    Action,
    Assembly,
    Type,
    Member,
    Method,
    Resource,
}

#[derive(Clone, PartialEq, Eq)]
pub struct PaletteEntry {
    pub id: String,
    pub kind: PaletteEntryKind,
    pub title: String,
    pub subtitle: String,
    pub group_label: &'static str,
    pub assembly_id: Option<String>,
    pub assembly_path: Option<String>,
    pub type_name: Option<String>,
    pub method_name: Option<String>,
    pub metadata_token: Option<String>,
    pub command_id: Option<CommandId>,
    pub shortcut_label: Option<String>,
}

pub fn build_palette_entries(
    assemblies: &[OpenAssembly],
    analysis_entries: &HashMap<String, AnalysisEntry>,
    action_commands: &[PaletteCommandItem],
    query: &str,
) -> Vec<PaletteEntry> {
    let normalized = normalize_query(query);
    let token_query = normalize_metadata_token_query(query);
    let mut entries = Vec::new();

    for command in action_commands {
        if normalized.is_empty()
            || matches_query(&normalized, &[command.title, command.description])
        {
            entries.push(PaletteEntry {
                id: format!("action::{:?}", command.command_id),
                kind: PaletteEntryKind::Action,
                title: command.title.to_string(),
                subtitle: command.description.to_string(),
                group_label: "Actions",
                assembly_id: None,
                assembly_path: None,
                type_name: None,
                method_name: None,
                metadata_token: None,
                command_id: Some(command.command_id),
                shortcut_label: command.shortcut_label.clone(),
            });
        }
    }

    for assembly in assemblies.iter().take(8) {
        if normalized.is_empty() || matches_query(&normalized, &[&assembly.name, &assembly.path]) {
            entries.push(PaletteEntry {
                id: format!("assembly::{}", assembly.id),
                kind: PaletteEntryKind::Assembly,
                title: assembly.name.clone(),
                subtitle: assembly.path.clone(),
                group_label: "Assemblies",
                assembly_id: Some(assembly.id.clone()),
                assembly_path: Some(assembly.path.clone()),
                type_name: None,
                method_name: None,
                metadata_token: None,
                command_id: None,
                shortcut_label: None,
            });
        }
    }

    for assembly in assemblies {
        let Some(entry) = analysis_entries.get(&format!("{}::explore", assembly.id)) else {
            continue;
        };
        let Some(result) = entry.result.as_ref() else {
            continue;
        };
        let Some(explore) = result.explore.as_ref() else {
            continue;
        };

        if let Some(entry_point) = explore
            .assembly_metadata
            .entry_point
            .as_deref()
            .and_then(|entry_point| resolve_entry_point_method(&explore.methods, entry_point))
        {
            let subtitle = entry_subtitle_with_token(
                &assembly.name,
                &entry_point.type_name,
                entry_point.metadata_token.as_deref(),
            );
            if normalized.is_empty()
                || matches_query(
                    &normalized,
                    &[
                        "entry point",
                        "main",
                        &entry_point.method_name,
                        &entry_point.signature,
                        &entry_point.type_name,
                        &subtitle,
                        entry_point.metadata_token.as_deref().unwrap_or(""),
                    ],
                )
            {
                entries.push(PaletteEntry {
                    id: format!(
                        "entry-point::{}::{}::{}",
                        assembly.id, entry_point.type_name, entry_point.method_name
                    ),
                    kind: PaletteEntryKind::Method,
                    title: format!("Entry point: {}", entry_point.method_name),
                    subtitle,
                    group_label: "Metadata",
                    assembly_id: Some(assembly.id.clone()),
                    assembly_path: Some(assembly.path.clone()),
                    type_name: Some(entry_point.type_name.clone()),
                    method_name: Some(entry_point.method_name.clone()),
                    metadata_token: entry_point.metadata_token.clone(),
                    command_id: None,
                    shortcut_label: None,
                });
            }
        }

        for module_initializer in explore
            .methods
            .iter()
            .filter(|method| method.type_name == "<Module>" && method.method_name == ".cctor")
        {
            let subtitle = entry_subtitle_with_token(
                &assembly.name,
                &module_initializer.type_name,
                module_initializer.metadata_token.as_deref(),
            );
            if normalized.is_empty()
                || matches_query(
                    &normalized,
                    &[
                        "module initializer",
                        "module cctor",
                        ".cctor",
                        &module_initializer.signature,
                        &module_initializer.type_name,
                        &subtitle,
                        module_initializer.metadata_token.as_deref().unwrap_or(""),
                    ],
                )
            {
                entries.push(PaletteEntry {
                    id: format!(
                        "module-initializer::{}::{}",
                        assembly.id,
                        module_initializer
                            .metadata_token
                            .as_deref()
                            .unwrap_or(".cctor")
                    ),
                    kind: PaletteEntryKind::Method,
                    title: "Module initializer: .cctor".to_string(),
                    subtitle,
                    group_label: "Metadata",
                    assembly_id: Some(assembly.id.clone()),
                    assembly_path: Some(assembly.path.clone()),
                    type_name: Some(module_initializer.type_name.clone()),
                    method_name: Some(module_initializer.method_name.clone()),
                    metadata_token: module_initializer.metadata_token.clone(),
                    command_id: None,
                    shortcut_label: None,
                });
            }
        }

        for (index, type_entry) in explore.types.iter().enumerate() {
            let type_name = type_entry.type_name.clone();
            let metadata_token = type_entry.metadata_token.clone();
            let token_matches = token_query
                .as_deref()
                .is_some_and(|token| metadata_token_matches(metadata_token.as_deref(), token));
            if normalized.is_empty() && !token_matches && index >= 10 {
                continue;
            }
            let display_name = type_name
                .rsplit('.')
                .next()
                .unwrap_or(&type_name)
                .to_string();
            let subtitle =
                entry_subtitle_with_token(&assembly.name, &type_name, metadata_token.as_deref());
            if normalized.is_empty()
                || token_matches
                || matches_query(
                    &normalized,
                    &[
                        &display_name,
                        &type_name,
                        &subtitle,
                        metadata_token.as_deref().unwrap_or(""),
                    ],
                )
            {
                entries.push(PaletteEntry {
                    id: format!("type::{}::{}", assembly.id, type_name),
                    kind: PaletteEntryKind::Type,
                    title: display_name,
                    subtitle,
                    group_label: "Types",
                    assembly_id: Some(assembly.id.clone()),
                    assembly_path: Some(assembly.path.clone()),
                    type_name: Some(type_name),
                    method_name: None,
                    metadata_token,
                    command_id: None,
                    shortcut_label: None,
                });
            }

            for member in type_entry
                .fields
                .iter()
                .chain(type_entry.properties.iter())
                .chain(type_entry.events.iter())
                .chain(type_entry.nested_types.iter())
            {
                let member_token = member.metadata_token.clone();
                let member_token_matches = token_query
                    .as_deref()
                    .is_some_and(|token| metadata_token_matches(member_token.as_deref(), token));
                let member_subtitle = member_subtitle_with_token(
                    &assembly.name,
                    &type_entry.type_name,
                    &member.kind,
                    member_token.as_deref(),
                );

                if normalized.is_empty()
                    || member_token_matches
                    || matches_query(
                        &normalized,
                        &[
                            &member.name,
                            &member.kind,
                            &member.signature,
                            &type_entry.type_name,
                            &member_subtitle,
                            member.attributes.as_deref().unwrap_or(""),
                            member_token.as_deref().unwrap_or(""),
                        ],
                    )
                {
                    entries.push(PaletteEntry {
                        id: format!(
                            "member::{}::{}::{}::{}",
                            assembly.id, type_entry.type_name, member.kind, member.name
                        ),
                        kind: PaletteEntryKind::Member,
                        title: format!("{}: {}", member_kind_label(&member.kind), member.name),
                        subtitle: member_subtitle,
                        group_label: "Members",
                        assembly_id: Some(assembly.id.clone()),
                        assembly_path: Some(assembly.path.clone()),
                        type_name: Some(type_entry.type_name.clone()),
                        method_name: None,
                        metadata_token: member_token,
                        command_id: None,
                        shortcut_label: None,
                    });
                }
            }
        }

        for (index, method) in explore.methods.iter().enumerate() {
            let token_matches = token_query.as_deref().is_some_and(|token| {
                metadata_token_matches(method.metadata_token.as_deref(), token)
            });
            if normalized.is_empty() && !token_matches && index >= 30 {
                continue;
            }
            let subtitle = entry_subtitle_with_token(
                &assembly.name,
                &method.type_name,
                method.metadata_token.as_deref(),
            );
            if normalized.is_empty()
                || token_matches
                || matches_query(
                    &normalized,
                    &[
                        &method.method_name,
                        &method.signature,
                        &method.type_name,
                        &subtitle,
                        method.metadata_token.as_deref().unwrap_or(""),
                    ],
                )
            {
                entries.push(PaletteEntry {
                    id: format!(
                        "method::{}::{}::{}",
                        assembly.id, method.type_name, method.method_name
                    ),
                    kind: PaletteEntryKind::Method,
                    title: method.method_name.clone(),
                    subtitle,
                    group_label: "Methods",
                    assembly_id: Some(assembly.id.clone()),
                    assembly_path: Some(assembly.path.clone()),
                    type_name: Some(method.type_name.clone()),
                    method_name: Some(method.method_name.clone()),
                    metadata_token: method.metadata_token.clone(),
                    command_id: None,
                    shortcut_label: None,
                });
            }
        }

        for (index, resource) in explore.assembly_metadata.resources.iter().enumerate() {
            let metadata_token = resource.metadata_token.clone();
            let token_matches = token_query
                .as_deref()
                .is_some_and(|token| metadata_token_matches(metadata_token.as_deref(), token));
            if normalized.is_empty() && !token_matches && index >= 30 {
                continue;
            }
            let hash = resource.sha256_hash.as_deref().unwrap_or("");
            let resource_type = resource.resource_type.as_str();
            let subtitle =
                entry_subtitle_with_token(&assembly.name, resource_type, metadata_token.as_deref());

            if normalized.is_empty()
                || token_matches
                || matches_query(
                    &normalized,
                    &[
                        &resource.name,
                        resource_type,
                        &subtitle,
                        metadata_token.as_deref().unwrap_or(""),
                        hash,
                    ],
                )
            {
                entries.push(PaletteEntry {
                    id: format!("resource::{}::{}", assembly.id, resource.name),
                    kind: PaletteEntryKind::Resource,
                    title: resource.name.clone(),
                    subtitle,
                    group_label: "Metadata",
                    assembly_id: Some(assembly.id.clone()),
                    assembly_path: Some(assembly.path.clone()),
                    type_name: None,
                    method_name: None,
                    metadata_token,
                    command_id: None,
                    shortcut_label: None,
                });
            }
        }
    }

    entries
}

fn normalize_query(query: &str) -> String {
    query.trim().to_ascii_lowercase()
}

fn normalize_metadata_token_query(query: &str) -> Option<String> {
    let mut normalized = query.trim().to_ascii_lowercase();
    if let Some(stripped) = normalized.strip_prefix("token:") {
        normalized = stripped.trim().to_string();
    }
    if let Some(stripped) = normalized.strip_prefix("0x") {
        normalized = stripped.trim().to_string();
    }

    if normalized.len() == 8 && normalized.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Some(normalized)
    } else {
        None
    }
}

fn metadata_token_matches(token: Option<&str>, normalized_query: &str) -> bool {
    token.and_then(normalize_metadata_token_query).as_deref() == Some(normalized_query)
}

fn matches_query(query: &str, fields: &[&str]) -> bool {
    fields
        .iter()
        .any(|field| field.to_ascii_lowercase().contains(query))
}

fn entry_subtitle_with_token(assembly_name: &str, type_name: &str, token: Option<&str>) -> String {
    match token {
        Some(token) if !token.is_empty() => format!("{assembly_name} - {type_name} - {token}"),
        _ => format!("{assembly_name} - {type_name}"),
    }
}

fn member_subtitle_with_token(
    assembly_name: &str,
    type_name: &str,
    member_kind: &str,
    token: Option<&str>,
) -> String {
    match token {
        Some(token) if !token.is_empty() => {
            format!("{assembly_name} - {type_name} - {member_kind} - {token}")
        }
        _ => format!("{assembly_name} - {type_name} - {member_kind}"),
    }
}

fn member_kind_label(kind: &str) -> &'static str {
    match kind {
        "field" => "Field",
        "property" => "Property",
        "event" => "Event",
        "class" | "struct" | "interface" | "enum" | "delegate" => "Nested type",
        _ => "Member",
    }
}

fn resolve_entry_point_method<'a>(
    methods: &'a [crate::ipc::MethodEntry],
    entry_point: &str,
) -> Option<&'a crate::ipc::MethodEntry> {
    let (type_name, method_name) = parse_cecil_method_full_name(entry_point)?;
    methods
        .iter()
        .find(|method| method.type_name == type_name && method.method_name == method_name)
}

fn parse_cecil_method_full_name(entry_point: &str) -> Option<(&str, &str)> {
    let (left, right) = entry_point.rsplit_once("::")?;
    let type_name = left
        .rsplit_once(' ')
        .map(|(_, ty)| ty)
        .unwrap_or(left)
        .trim();
    let method_name = right
        .split_once('(')
        .map(|(name, _)| name)
        .unwrap_or(right)
        .trim();

    if type_name.is_empty() || method_name.is_empty() {
        None
    } else {
        Some((type_name, method_name))
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{ActiveMode, AnalysisEntry, AnalysisResult, AnalysisStatus};

    use super::*;

    #[test]
    fn build_palette_entries_includes_actions_and_method_matches() {
        let assemblies = vec![OpenAssembly {
            id: "asm-1".to_string(),
            path: r"C:\samples\First.dll".to_string(),
            name: "First.dll".to_string(),
            loaded_at: 1,
        }];
        let mut analysis_entries = HashMap::new();
        analysis_entries.insert(
            "asm-1::explore".to_string(),
            AnalysisEntry {
                assembly_id: "asm-1".to_string(),
                assembly_path: r"C:\samples\First.dll".to_string(),
                mode: ActiveMode::Explore,
                status: AnalysisStatus::Done,
                result: Some(AnalysisResult {
                    assembly_path: r"C:\samples\First.dll".to_string(),
                    mode: "combined".to_string(),
                    explore: Some(crate::ipc::ExplorePayload {
                        assembly_path: r"C:\samples\First.dll".to_string(),
                        assembly_metadata: crate::ipc::AssemblyMetadataEntry {
                            resources: vec![crate::ipc::ResourceMetadataEntry {
                                name: "payload.txt".to_string(),
                                resource_type: "Embedded".to_string(),
                                attributes: Some("Private".to_string()),
                                size_bytes: Some(14),
                                implementation: None,
                                metadata_token: Some("0x28000001".to_string()),
                                sha256_hash: Some("abc123".to_string()),
                                preview_kind: Some("text".to_string()),
                                preview: Some("powershell -nop".to_string()),
                                preview_truncated: false,
                            }],
                            ..Default::default()
                        },
                        methods: vec![crate::ipc::MethodEntry {
                            type_name: "Example.Loader".to_string(),
                            method_name: "DownloadPayload".to_string(),
                            metadata_token: Some("0x06000002".to_string()),
                            signature: "void DownloadPayload()".to_string(),
                            has_body: Some(true),
                            instructions: Vec::new(),
                            p_invoke: None,
                        }],
                        types: vec![crate::ipc::TypeEntry {
                            type_name: "Example.Loader".to_string(),
                            metadata_token: Some("0x02000002".to_string()),
                            kind: "class".to_string(),
                            fields: vec![crate::ipc::MemberMetadataEntry {
                                name: "EndpointUrl".to_string(),
                                metadata_token: Some("0x04000001".to_string()),
                                kind: "field".to_string(),
                                signature: "private String EndpointUrl".to_string(),
                                attributes: Some("Private".to_string()),
                            }],
                            properties: vec![crate::ipc::MemberMetadataEntry {
                                name: "Timeout".to_string(),
                                metadata_token: Some("0x17000001".to_string()),
                                kind: "property".to_string(),
                                signature: "Int32 Timeout {get; set;}".to_string(),
                                attributes: None,
                            }],
                            events: Vec::new(),
                            nested_types: Vec::new(),
                            custom_attributes: Vec::new(),
                            methods: Vec::new(),
                        }],
                    }),
                    scan: None,
                    stderr: String::new(),
                }),
                error: None,
                started_at: None,
                finished_at: None,
            },
        );

        let entries = build_palette_entries(&assemblies, &analysis_entries, &[], "download");

        assert!(entries
            .iter()
            .any(|entry| entry.kind == PaletteEntryKind::Method));
        assert!(!entries
            .iter()
            .any(|entry| entry.kind == PaletteEntryKind::Action));

        let token_entries = build_palette_entries(&assemblies, &analysis_entries, &[], "06000002");
        let token_method = token_entries
            .iter()
            .find(|entry| entry.kind == PaletteEntryKind::Method)
            .expect("method token should be searchable");
        assert_eq!(token_method.metadata_token.as_deref(), Some("0x06000002"));
        assert!(token_method.subtitle.contains("0x06000002"));

        let resource_entries =
            build_palette_entries(&assemblies, &analysis_entries, &[], "28000001");
        let resource_entry = resource_entries
            .iter()
            .find(|entry| entry.kind == PaletteEntryKind::Resource)
            .expect("resource token should be searchable");
        assert_eq!(resource_entry.title, "payload.txt");
        assert_eq!(resource_entry.metadata_token.as_deref(), Some("0x28000001"));
        assert!(resource_entry.subtitle.contains("0x28000001"));

        let member_entries =
            build_palette_entries(&assemblies, &analysis_entries, &[], "endpointurl");
        let member_entry = member_entries
            .iter()
            .find(|entry| entry.kind == PaletteEntryKind::Member)
            .expect("member name should be searchable");
        assert_eq!(member_entry.title, "Field: EndpointUrl");
        assert_eq!(member_entry.type_name.as_deref(), Some("Example.Loader"));
        assert_eq!(member_entry.metadata_token.as_deref(), Some("0x04000001"));

        let property_token_entries =
            build_palette_entries(&assemblies, &analysis_entries, &[], "17000001");
        let property_entry = property_token_entries
            .iter()
            .find(|entry| entry.kind == PaletteEntryKind::Member)
            .expect("property token should be searchable");
        assert_eq!(property_entry.title, "Property: Timeout");
        assert!(property_entry.subtitle.contains("0x17000001"));

        let default_entries = build_palette_entries(
            &assemblies,
            &analysis_entries,
            &[PaletteCommandItem {
                command_id: CommandId::OpenExportFolder,
                title: "Open export folder",
                description: "Reveal the most recently exported decompiled project",
                shortcut_label: Some("Ctrl+Shift+O".to_string()),
            }],
            "",
        );
        assert!(default_entries
            .iter()
            .any(|entry| entry.kind == PaletteEntryKind::Action));
        assert!(default_entries
            .iter()
            .any(|entry| entry.command_id == Some(CommandId::OpenExportFolder)));
    }

    #[test]
    fn build_palette_entries_token_queries_bypass_preview_caps() {
        let assemblies = vec![OpenAssembly {
            id: "asm-1".to_string(),
            path: r"C:\samples\First.dll".to_string(),
            name: "First.dll".to_string(),
            loaded_at: 1,
        }];
        let methods = (0..35)
            .map(|index| crate::ipc::MethodEntry {
                type_name: "Example.Loader".to_string(),
                method_name: format!("Method{index:02}"),
                metadata_token: Some(format!("0x060000{index:02X}")),
                signature: format!("void Method{index:02}()"),
                has_body: Some(true),
                instructions: Vec::new(),
                p_invoke: None,
            })
            .collect::<Vec<_>>();
        let resources = (0..35)
            .map(|index| crate::ipc::ResourceMetadataEntry {
                name: format!("payload-{index:02}.bin"),
                resource_type: "Embedded".to_string(),
                metadata_token: Some(format!("0x280000{index:02X}")),
                ..Default::default()
            })
            .collect::<Vec<_>>();
        let types = (0..12)
            .map(|index| crate::ipc::TypeEntry {
                type_name: format!("Example.Type{index:02}"),
                metadata_token: Some(format!("0x020000{index:02X}")),
                kind: "class".to_string(),
                fields: if index == 11 {
                    vec![crate::ipc::MemberMetadataEntry {
                        name: "LateField".to_string(),
                        metadata_token: Some("0x0400000B".to_string()),
                        kind: "field".to_string(),
                        signature: "private String LateField".to_string(),
                        attributes: None,
                    }]
                } else {
                    Vec::new()
                },
                properties: Vec::new(),
                events: Vec::new(),
                nested_types: Vec::new(),
                custom_attributes: Vec::new(),
                methods: Vec::new(),
            })
            .collect::<Vec<_>>();
        let mut analysis_entries = HashMap::new();
        analysis_entries.insert(
            "asm-1::explore".to_string(),
            AnalysisEntry {
                assembly_id: "asm-1".to_string(),
                assembly_path: r"C:\samples\First.dll".to_string(),
                mode: ActiveMode::Explore,
                status: AnalysisStatus::Done,
                result: Some(AnalysisResult {
                    assembly_path: r"C:\samples\First.dll".to_string(),
                    mode: "combined".to_string(),
                    explore: Some(crate::ipc::ExplorePayload {
                        assembly_path: r"C:\samples\First.dll".to_string(),
                        assembly_metadata: crate::ipc::AssemblyMetadataEntry {
                            resources,
                            ..Default::default()
                        },
                        methods,
                        types,
                    }),
                    scan: None,
                    stderr: String::new(),
                }),
                error: None,
                started_at: None,
                finished_at: None,
            },
        );

        let type_entries =
            build_palette_entries(&assemblies, &analysis_entries, &[], "token: 0x0200000b");
        assert!(type_entries.iter().any(|entry| {
            entry.kind == PaletteEntryKind::Type
                && entry.metadata_token.as_deref() == Some("0x0200000B")
        }));

        let method_entries =
            build_palette_entries(&assemblies, &analysis_entries, &[], "TOKEN: 0X06000022");
        assert!(method_entries.iter().any(|entry| {
            entry.kind == PaletteEntryKind::Method
                && entry.metadata_token.as_deref() == Some("0x06000022")
        }));

        let resource_entries =
            build_palette_entries(&assemblies, &analysis_entries, &[], "28000022");
        assert!(resource_entries.iter().any(|entry| {
            entry.kind == PaletteEntryKind::Resource
                && entry.metadata_token.as_deref() == Some("0x28000022")
        }));

        let member_entries =
            build_palette_entries(&assemblies, &analysis_entries, &[], "token: 0x0400000b");
        assert!(member_entries.iter().any(|entry| {
            entry.kind == PaletteEntryKind::Member
                && entry.type_name.as_deref() == Some("Example.Type11")
                && entry.metadata_token.as_deref() == Some("0x0400000B")
        }));
    }

    #[test]
    fn build_palette_entries_includes_entry_point_and_module_initializer_jumps() {
        let assemblies = vec![OpenAssembly {
            id: "asm-1".to_string(),
            path: r"C:\samples\App.exe".to_string(),
            name: "App.exe".to_string(),
            loaded_at: 1,
        }];
        let methods = vec![
            crate::ipc::MethodEntry {
                type_name: "Example.Program".to_string(),
                method_name: "Main".to_string(),
                metadata_token: Some("0x06000001".to_string()),
                signature: "private static Void Main(String[] args)".to_string(),
                has_body: Some(true),
                instructions: Vec::new(),
                p_invoke: None,
            },
            crate::ipc::MethodEntry {
                type_name: "<Module>".to_string(),
                method_name: ".cctor".to_string(),
                metadata_token: Some("0x06000002".to_string()),
                signature: "private static Void .cctor()".to_string(),
                has_body: Some(true),
                instructions: Vec::new(),
                p_invoke: None,
            },
        ];
        let mut analysis_entries = HashMap::new();
        analysis_entries.insert(
            "asm-1::explore".to_string(),
            AnalysisEntry {
                assembly_id: "asm-1".to_string(),
                assembly_path: r"C:\samples\App.exe".to_string(),
                mode: ActiveMode::Explore,
                status: AnalysisStatus::Done,
                result: Some(AnalysisResult {
                    assembly_path: r"C:\samples\App.exe".to_string(),
                    mode: "combined".to_string(),
                    explore: Some(crate::ipc::ExplorePayload {
                        assembly_path: r"C:\samples\App.exe".to_string(),
                        assembly_metadata: crate::ipc::AssemblyMetadataEntry {
                            entry_point: Some(
                                "System.Void Example.Program::Main(System.String[])".to_string(),
                            ),
                            ..Default::default()
                        },
                        methods,
                        types: Vec::new(),
                    }),
                    scan: None,
                    stderr: String::new(),
                }),
                error: None,
                started_at: None,
                finished_at: None,
            },
        );

        let entry_entries = build_palette_entries(&assemblies, &analysis_entries, &[], "entry");
        let entry_point = entry_entries
            .iter()
            .find(|entry| entry.title == "Entry point: Main")
            .expect("entry point should be searchable");
        assert_eq!(entry_point.group_label, "Metadata");
        assert_eq!(entry_point.kind, PaletteEntryKind::Method);
        assert_eq!(entry_point.type_name.as_deref(), Some("Example.Program"));
        assert_eq!(entry_point.method_name.as_deref(), Some("Main"));
        assert_eq!(entry_point.metadata_token.as_deref(), Some("0x06000001"));

        let initializer_entries =
            build_palette_entries(&assemblies, &analysis_entries, &[], "module initializer");
        let initializer = initializer_entries
            .iter()
            .find(|entry| entry.title == "Module initializer: .cctor")
            .expect("module initializer should be searchable");
        assert_eq!(initializer.group_label, "Metadata");
        assert_eq!(initializer.kind, PaletteEntryKind::Method);
        assert_eq!(initializer.type_name.as_deref(), Some("<Module>"));
        assert_eq!(initializer.method_name.as_deref(), Some(".cctor"));
        assert_eq!(initializer.metadata_token.as_deref(), Some("0x06000002"));
    }
}
