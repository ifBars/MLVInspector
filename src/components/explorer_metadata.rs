use std::collections::BTreeSet;

use dioxus::prelude::*;

use crate::ipc::{
    AssemblyMetadataEntry, AssemblyReferenceEntry, AttributeMetadataEntry, ModuleMetadataEntry,
    ResourceMetadataEntry,
};

use super::theme::{
    C_BG_ELEVATED, C_BG_SURFACE, C_BORDER, C_TEXT_MUTED, C_TEXT_PRIMARY, C_TEXT_SECONDARY,
    FONT_MONO,
};

const DEFAULT_COLLAPSED_METADATA_SECTIONS: [&str; 5] = [
    "overview",
    "modules",
    "references",
    "resources",
    "attributes",
];

pub(crate) fn has_metadata(metadata: &AssemblyMetadataEntry) -> bool {
    !metadata.assembly_name.trim().is_empty()
        || !metadata.full_name.trim().is_empty()
        || has_text(metadata.version.as_deref())
        || has_text(metadata.culture.as_deref())
        || has_text(metadata.public_key_token.as_deref())
        || has_text(metadata.target_framework.as_deref())
        || has_text(metadata.inferred_target_framework.as_deref())
        || has_text(metadata.runtime_version.as_deref())
        || has_text(metadata.architecture.as_deref())
        || has_text(metadata.module_kind.as_deref())
        || has_text(metadata.entry_point.as_deref())
        || has_text(metadata.mvid.as_deref())
        || !metadata.modules.is_empty()
        || !metadata.assembly_references.is_empty()
        || !metadata.resources.is_empty()
        || !metadata.custom_attributes.is_empty()
}

pub(crate) fn default_collapsed_metadata_sections() -> BTreeSet<String> {
    DEFAULT_COLLAPSED_METADATA_SECTIONS
        .into_iter()
        .map(str::to_string)
        .collect()
}

#[component]
pub(crate) fn AssemblyMetadataView(
    assembly_name: String,
    assembly_path: String,
    metadata: AssemblyMetadataEntry,
    collapsed_sections: Signal<BTreeSet<String>>,
    framed: bool,
) -> Element {
    let display_name = display_or_fallback(&metadata.assembly_name, &assembly_name);
    let summary = metadata_header_summary(&metadata);
    let overview_rows = overview_rows(&metadata, &assembly_name);
    let container_style = if framed {
        format!(
            "border: 1px solid {C_BORDER}; border-radius: 7px; overflow: hidden; background: {C_BG_SURFACE};"
        )
    } else {
        "display: flex; flex-direction: column;".to_string()
    };
    let header_style = if framed {
        format!(
            "padding: 7px 8px 6px; display: flex; flex-direction: column; gap: 3px; border-bottom: 1px solid {C_BORDER}; background: linear-gradient(180deg, rgba(255,255,255,0.03) 0%, rgba(255,255,255,0.015) 100%);"
        )
    } else {
        format!(
            "padding: 2px 0 10px; display: flex; flex-direction: column; gap: 4px; border-bottom: 1px solid {C_BORDER};"
        )
    };

    rsx! {
        div {
            style: "{container_style}",

            div {
                style: "{header_style}",
                span {
                    style: format!(
                        "font-size: 8px; font-weight: 700; letter-spacing: 0.08em; color: {C_TEXT_MUTED}; text-transform: uppercase;"
                    ),
                    "Assembly metadata"
                }
                span {
                    style: format!(
                        "font-size: 12px; font-weight: 700; color: {C_TEXT_PRIMARY}; line-height: 1.25; word-break: break-word;"
                    ),
                    "{display_name}"
                }
                if let Some(summary) = summary {
                    span {
                        style: format!(
                            "font-size: 9px; color: {C_TEXT_SECONDARY}; line-height: 1.3; word-break: break-word;"
                        ),
                        "{summary}"
                    }
                }
                span {
                    style: format!(
                        "font-size: 9px; color: {C_TEXT_MUTED}; font-family: {FONT_MONO}; line-height: 1.3; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                    ),
                    "{assembly_path}"
                }
            }

            div {
                style: "display: flex; flex-direction: column;",

                MetadataSection {
                    section_key: "overview".to_string(),
                    title: "Overview".to_string(),
                    count_label: None,
                    collapsed_sections,
                    framed,
                    is_first: true,
                    if overview_rows.is_empty() {
                        MetadataEmpty {
                            message: "Only the assembly identity is available from the current worker response."
                                .to_string(),
                        }
                    } else {
                        MetadataPropertyGrid { rows: overview_rows }
                    }
                }

                MetadataSection {
                    section_key: "modules".to_string(),
                    title: "Modules".to_string(),
                    count_label: Some(metadata.modules.len().to_string()),
                    collapsed_sections,
                    framed,
                    is_first: false,
                    if metadata.modules.is_empty() {
                        MetadataEmpty { message: "No module metadata exposed by the worker.".to_string() }
                    } else {
                        for (index, module) in metadata.modules.iter().enumerate() {
                            MetadataListEntry {
                                key: "module-{index}",
                                bordered: index > 0,
                                title: display_or_fallback(&module.name, "Unnamed module"),
                                subtitle: None,
                                rows: module_rows(module),
                            }
                        }
                    }
                }

                MetadataSection {
                    section_key: "references".to_string(),
                    title: "Assembly references".to_string(),
                    count_label: Some(metadata.assembly_references.len().to_string()),
                    collapsed_sections,
                    framed,
                    is_first: false,
                    if metadata.assembly_references.is_empty() {
                        MetadataEmpty { message: "No assembly references were found.".to_string() }
                    } else {
                        for (index, reference) in metadata.assembly_references.iter().enumerate() {
                            MetadataListEntry {
                                key: "reference-{index}",
                                bordered: index > 0,
                                title: display_or_fallback(&reference.name, "Unnamed reference"),
                                subtitle: present_text(reference.version.as_deref()).map(str::to_string),
                                rows: reference_rows(reference),
                            }
                        }
                    }
                }

                MetadataSection {
                    section_key: "resources".to_string(),
                    title: "Manifest resources".to_string(),
                    count_label: Some(metadata.resources.len().to_string()),
                    collapsed_sections,
                    framed,
                    is_first: false,
                    if metadata.resources.is_empty() {
                        MetadataEmpty { message: "No manifest resources were found.".to_string() }
                    } else {
                        for (index, resource) in metadata.resources.iter().enumerate() {
                            ResourceEntry {
                                key: "resource-{index}",
                                resource: resource.clone(),
                                bordered: index > 0,
                            }
                        }
                    }
                }

                MetadataSection {
                    section_key: "attributes".to_string(),
                    title: "Assembly attributes".to_string(),
                    count_label: Some(metadata.custom_attributes.len().to_string()),
                    collapsed_sections,
                    framed,
                    is_first: false,
                    if metadata.custom_attributes.is_empty() {
                        MetadataEmpty { message: "No custom assembly attributes were found.".to_string() }
                    } else {
                        for (index, attribute) in metadata.custom_attributes.iter().enumerate() {
                            AttributeEntry {
                                key: "attribute-{index}",
                                attribute: attribute.clone(),
                                bordered: index > 0,
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn MetadataSection(
    section_key: String,
    title: String,
    count_label: Option<String>,
    collapsed_sections: Signal<BTreeSet<String>>,
    framed: bool,
    is_first: bool,
    children: Element,
) -> Element {
    let is_open = !collapsed_sections.read().contains(&section_key);
    let toggle_key = section_key.clone();
    let chevron_rotation = if is_open { "90deg" } else { "0deg" };
    let border_top = if is_first {
        "0".to_string()
    } else {
        format!("1px solid {C_BORDER}")
    };
    let header_background = if is_open {
        if framed {
            C_BG_ELEVATED
        } else {
            "rgba(255,255,255,0.03)"
        }
    } else {
        "transparent"
    };
    let header_padding = if framed { "6px 8px" } else { "6px 0" };
    let content_background = if framed {
        "rgba(0,0,0,0.08)"
    } else {
        "transparent"
    };

    rsx! {
        div {
            style: "display: flex; flex-direction: column;",
            button {
                style: format!(
                    "display: flex; align-items: center; gap: 7px; width: 100%; padding: {header_padding}; border: none; border-top: {border_top}; background: {header_background}; color: {C_TEXT_PRIMARY}; cursor: pointer; text-align: left;"
                ),
                onclick: move |_| {
                    let mut set = collapsed_sections.write();
                    if set.contains(&toggle_key) {
                        set.remove(&toggle_key);
                    } else {
                        set.insert(toggle_key.clone());
                    }
                },

                div {
                    style: "width: 10px; flex-shrink: 0; display: flex; align-items: center; justify-content: center;",
                    svg {
                        width: "7",
                        height: "7",
                        view_box: "0 0 8 8",
                        fill: "none",
                        stroke: C_TEXT_MUTED,
                        stroke_width: "1.5",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        style: format!("transform: rotate({chevron_rotation}); transition: transform 120ms ease;"),
                        polyline { points: "2,1.5 5.5,4 2,6.5" }
                    }
                }

                span {
                    style: format!(
                        "min-width: 0; flex: 1; font-size: 9px; font-weight: 700; color: {C_TEXT_PRIMARY}; text-transform: uppercase; letter-spacing: 0.08em;"
                    ),
                    "{title}"
                }

                if let Some(count) = count_label {
                    span {
                        style: format!(
                            "margin-left: auto; flex-shrink: 0; font-size: 9px; color: {C_TEXT_SECONDARY}; font-family: {FONT_MONO};"
                        ),
                        "{count}"
                    }
                }
            }

            if is_open {
                div {
                    style: format!(
                        "display: flex; flex-direction: column; background: {content_background};"
                    ),
                    {children}
                }
            }
        }
    }
}

#[component]
fn MetadataPropertyGrid(rows: Vec<(String, String)>) -> Element {
    rsx! {
        div {
            style: "display: flex; flex-direction: column;",
            for (index, (label, value)) in rows.iter().enumerate() {
                div {
                    key: "{label}-{index}",
                    style: format!(
                        "display: grid; grid-template-columns: minmax(90px, 108px) minmax(0, 1fr); gap: 10px; align-items: start; padding: 4px 8px; border-top: {};",
                        if index == 0 { "0" } else { "1px solid rgba(255,255,255,0.04)" }
                    ),
                    span {
                        style: format!(
                            "font-size: 8px; color: {C_TEXT_MUTED}; text-transform: uppercase; letter-spacing: 0.06em; line-height: 1.4;"
                        ),
                        "{label}"
                    }
                    span {
                        style: format!(
                            "font-size: 9px; line-height: 1.45; color: {C_TEXT_PRIMARY}; font-family: {FONT_MONO}; white-space: normal; word-break: break-word;"
                        ),
                        "{value}"
                    }
                }
            }
        }
    }
}

#[component]
fn MetadataEmpty(message: String) -> Element {
    rsx! {
        div {
            style: format!(
                "padding: 7px 8px; color: {C_TEXT_MUTED}; font-size: 10px; line-height: 1.45;"
            ),
            "{message}"
        }
    }
}

#[component]
fn MetadataListEntry(
    bordered: bool,
    title: String,
    subtitle: Option<String>,
    rows: Vec<(String, String)>,
) -> Element {
    let border_top = if bordered {
        "1px solid rgba(255,255,255,0.05)"
    } else {
        "0"
    };

    rsx! {
        div {
            style: format!(
                "display: flex; flex-direction: column; gap: 4px; padding: 6px 8px 7px; border-top: {border_top};"
            ),
            div {
                style: "display: flex; align-items: flex-start; justify-content: space-between; gap: 8px; min-width: 0;",
                span {
                    style: format!(
                        "min-width: 0; flex: 1; font-size: 10px; font-weight: 700; color: {C_TEXT_PRIMARY}; line-height: 1.35; word-break: break-word;"
                    ),
                    "{title}"
                }
                if let Some(subtitle) = subtitle {
                    span {
                        style: format!(
                            "flex: 0 1 40%; min-width: 0; font-size: 9px; color: {C_TEXT_MUTED}; font-family: {FONT_MONO}; text-align: right; line-height: 1.35; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                        ),
                        "{subtitle}"
                    }
                }
            }
            if !rows.is_empty() {
                MetadataPropertyGrid { rows }
            }
        }
    }
}

#[component]
fn ResourceEntry(resource: ResourceMetadataEntry, bordered: bool) -> Element {
    rsx! {
        MetadataListEntry {
            bordered,
            title: display_or_fallback(&resource.name, "Unnamed resource"),
            subtitle: present_text(non_empty_string(&resource.resource_type)).map(str::to_string),
            rows: resource_rows(&resource),
        }
    }
}

#[component]
fn AttributeEntry(attribute: AttributeMetadataEntry, bordered: bool) -> Element {
    let border_top = if bordered {
        "1px solid rgba(255,255,255,0.05)"
    } else {
        "0"
    };

    rsx! {
        div {
            style: format!(
                "display: flex; flex-direction: column; gap: 3px; padding: 6px 8px 7px; border-top: {border_top};"
            ),
            span {
                style: format!("font-size: 10px; line-height: 1.35; font-weight: 700; color: {C_TEXT_PRIMARY}; font-family: {FONT_MONO}; word-break: break-word;"),
                "{attribute.attribute_type}"
            }
            if let Some(summary) = present_text(attribute.summary.as_deref()) {
                span {
                    style: format!(
                        "font-size: 9px; line-height: 1.45; color: {C_TEXT_SECONDARY}; word-break: break-word;"
                    ),
                    "{summary}"
                }
            }
        }
    }
}

fn has_text(value: Option<&str>) -> bool {
    present_text(value).is_some()
}

fn present_text(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.trim().is_empty())
}

fn non_empty_string(value: &str) -> Option<&str> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn metadata_header_summary(metadata: &AssemblyMetadataEntry) -> Option<String> {
    let mut parts = Vec::new();

    if target_framework_display(metadata).is_some() {
        parts.push(target_framework_value(metadata));
    }
    if let Some(version) = present_text(metadata.version.as_deref()) {
        parts.push(version.to_string());
    }
    if let Some(architecture) = present_text(metadata.architecture.as_deref()) {
        parts.push(architecture.to_string());
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" | "))
    }
}

fn overview_rows(metadata: &AssemblyMetadataEntry, assembly_name: &str) -> Vec<(String, String)> {
    let mut rows = vec![(
        "Assembly name".to_string(),
        display_or_fallback(&metadata.assembly_name, assembly_name),
    )];

    push_optional_row(&mut rows, "Version", metadata.version.as_deref());
    if target_framework_display(metadata).is_some() {
        rows.push((
            target_framework_label(metadata).to_string(),
            target_framework_value(metadata),
        ));
    }
    push_optional_row(&mut rows, "Runtime", metadata.runtime_version.as_deref());
    push_optional_row(&mut rows, "Architecture", metadata.architecture.as_deref());
    push_optional_row(&mut rows, "Module kind", metadata.module_kind.as_deref());
    push_optional_row(&mut rows, "Culture", metadata.culture.as_deref());
    push_optional_row(
        &mut rows,
        "Public key token",
        metadata.public_key_token.as_deref(),
    );
    push_optional_row(&mut rows, "Entry point", metadata.entry_point.as_deref());
    push_optional_row(&mut rows, "MVID", metadata.mvid.as_deref());
    push_optional_row(
        &mut rows,
        "Full name",
        non_empty_string(&metadata.full_name),
    );

    rows
}

fn module_rows(module: &ModuleMetadataEntry) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    push_optional_row(&mut rows, "File", module.file_name.as_deref());
    push_optional_row(&mut rows, "Runtime", module.runtime_version.as_deref());
    push_optional_row(&mut rows, "Architecture", module.architecture.as_deref());
    push_optional_row(&mut rows, "Kind", module.module_kind.as_deref());
    push_optional_row(&mut rows, "MVID", module.mvid.as_deref());
    rows
}

fn reference_rows(reference: &AssemblyReferenceEntry) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    push_optional_row(
        &mut rows,
        "Full name",
        non_empty_string(&reference.full_name),
    );
    push_optional_row(&mut rows, "Culture", reference.culture.as_deref());
    push_optional_row(
        &mut rows,
        "Public key token",
        reference.public_key_token.as_deref(),
    );
    rows
}

fn resource_rows(resource: &ResourceMetadataEntry) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    push_optional_row(&mut rows, "Attributes", resource.attributes.as_deref());
    if let Some(size) = format_present_bytes(resource.size_bytes) {
        rows.push(("Size".to_string(), size));
    }
    push_optional_row(&mut rows, "Source", resource.implementation.as_deref());
    rows
}

fn push_optional_row(rows: &mut Vec<(String, String)>, label: &str, value: Option<&str>) {
    if let Some(value) = present_text(value) {
        rows.push((label.to_string(), value.to_string()));
    }
}

fn target_framework_display(metadata: &AssemblyMetadataEntry) -> Option<&str> {
    present_text(metadata.target_framework.as_deref())
        .or_else(|| present_text(metadata.inferred_target_framework.as_deref()))
}

fn target_framework_value(metadata: &AssemblyMetadataEntry) -> String {
    target_framework_display(metadata)
        .unwrap_or("-")
        .to_string()
}

fn target_framework_label(metadata: &AssemblyMetadataEntry) -> &'static str {
    if metadata
        .target_framework
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        "Target framework"
    } else if metadata
        .inferred_target_framework
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        "Target framework (inferred)"
    } else {
        "Target framework"
    }
}

fn display_or_fallback(primary: &str, fallback: &str) -> String {
    if primary.trim().is_empty() {
        fallback.to_string()
    } else {
        primary.to_string()
    }
}

fn format_present_bytes(size: Option<i64>) -> Option<String> {
    size.map(|size| format_bytes(Some(size)))
}

fn format_bytes(size: Option<i64>) -> String {
    let Some(size) = size else {
        return "-".to_string();
    };

    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = size as f64;
    let mut unit_idx = 0usize;

    while value >= 1024.0 && unit_idx < UNITS.len() - 1 {
        value /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{size} {}", UNITS[unit_idx])
    } else {
        format!("{value:.1} {}", UNITS[unit_idx])
    }
}

#[cfg(test)]
mod tests {
    use super::{
        default_collapsed_metadata_sections, format_bytes, has_metadata, metadata_header_summary,
        overview_rows, target_framework_label, target_framework_value,
    };
    use crate::ipc::AssemblyMetadataEntry;

    #[test]
    fn default_collapsed_sections_start_with_every_section_closed() {
        let sections = default_collapsed_metadata_sections();

        assert!(sections.contains("overview"));
        assert!(sections.contains("modules"));
        assert!(sections.contains("references"));
        assert!(sections.contains("resources"));
        assert!(sections.contains("attributes"));
    }

    #[test]
    fn has_metadata_returns_false_for_empty_entry() {
        assert!(!has_metadata(&AssemblyMetadataEntry::default()));
    }

    #[test]
    fn has_metadata_returns_true_for_scalar_fields() {
        let metadata = AssemblyMetadataEntry {
            target_framework: Some("net8.0".to_string()),
            ..AssemblyMetadataEntry::default()
        };

        assert!(has_metadata(&metadata));
    }

    #[test]
    fn has_metadata_ignores_whitespace_only_optional_strings() {
        let metadata = AssemblyMetadataEntry {
            target_framework: Some("   ".to_string()),
            ..AssemblyMetadataEntry::default()
        };

        assert!(!has_metadata(&metadata));
    }

    #[test]
    fn metadata_header_summary_prefers_useful_identity_fields() {
        let metadata = AssemblyMetadataEntry {
            target_framework: Some("net8.0".to_string()),
            version: Some("1.2.3.4".to_string()),
            architecture: Some("AnyCPU".to_string()),
            ..AssemblyMetadataEntry::default()
        };

        assert_eq!(
            metadata_header_summary(&metadata),
            Some("net8.0 | 1.2.3.4 | AnyCPU".to_string())
        );
    }

    #[test]
    fn overview_rows_skip_blank_optional_fields() {
        let metadata = AssemblyMetadataEntry {
            version: Some("   ".to_string()),
            module_kind: Some("Console".to_string()),
            ..AssemblyMetadataEntry::default()
        };

        let rows = overview_rows(&metadata, "Fallback.Assembly");

        assert!(rows
            .iter()
            .any(|(label, value)| label == "Assembly name" && value == "Fallback.Assembly"));
        assert!(rows
            .iter()
            .any(|(label, value)| label == "Module kind" && value == "Console"));
        assert!(!rows.iter().any(|(label, _)| label == "Version"));
    }

    #[test]
    fn target_framework_prefers_declared_value() {
        let metadata = AssemblyMetadataEntry {
            target_framework: Some("net8.0".to_string()),
            inferred_target_framework: Some("netstandard2.1".to_string()),
            ..AssemblyMetadataEntry::default()
        };

        assert_eq!(target_framework_label(&metadata), "Target framework");
        assert_eq!(target_framework_value(&metadata), "net8.0");
    }

    #[test]
    fn target_framework_falls_back_to_inferred_value() {
        let metadata = AssemblyMetadataEntry {
            inferred_target_framework: Some("netstandard2.1".to_string()),
            ..AssemblyMetadataEntry::default()
        };

        assert_eq!(
            target_framework_label(&metadata),
            "Target framework (inferred)"
        );
        assert_eq!(target_framework_value(&metadata), "netstandard2.1");
    }

    #[test]
    fn format_bytes_handles_missing_sizes() {
        assert_eq!(format_bytes(None), "-");
    }

    #[test]
    fn format_bytes_formats_binary_units() {
        assert_eq!(format_bytes(Some(1536)), "1.5 KB");
    }
}
