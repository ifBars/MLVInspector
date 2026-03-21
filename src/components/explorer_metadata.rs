use std::collections::BTreeSet;

use dioxus::prelude::*;

use crate::ipc::{AssemblyMetadataEntry, AttributeMetadataEntry, ResourceMetadataEntry};

use super::theme::{
    C_BG_BASE, C_BG_ELEVATED, C_BORDER, C_BORDER_ACCENT, C_TEXT_MUTED, C_TEXT_PRIMARY,
    C_TEXT_SECONDARY, FONT_MONO,
};

const METADATA_SECTION_KEYS: [&str; 5] = [
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
    METADATA_SECTION_KEYS
        .into_iter()
        .map(str::to_string)
        .collect()
}

#[component]
pub(crate) fn ExplorerMetadataCard(
    assembly_name: String,
    assembly_path: String,
    metadata: AssemblyMetadataEntry,
    collapsed_sections: Signal<BTreeSet<String>>,
) -> Element {
    let resources_count = metadata.resources.len();
    let references_count = metadata.assembly_references.len();
    let attributes_count = metadata.custom_attributes.len();
    let modules_count = metadata.modules.len();
    let display_name = display_or_fallback(&metadata.assembly_name, &assembly_name);

    rsx! {
        div {
            style: format!(
                "border: 1px solid {C_BORDER}; border-radius: 10px; overflow: hidden; background: {C_BG_ELEVATED};"
            ),

            div {
                style: format!(
                    "padding: 8px; display: flex; flex-direction: column; gap: 8px; border-bottom: 1px solid {C_BORDER}; background: rgba(255,255,255,0.015);"
                ),

                div {
                    style: "display: flex; align-items: flex-start; justify-content: space-between; gap: 8px; flex-wrap: wrap;",

                    div {
                        style: "min-width: 0; display: flex; flex-direction: column; gap: 3px; flex: 1 1 180px;",
                        span {
                            style: format!("font-size: 9px; font-weight: 700; letter-spacing: 0.1em; color: {C_TEXT_MUTED}; text-transform: uppercase;"),
                            "Assembly metadata"
                        }
                        span {
                            style: format!("font-size: 12px; font-weight: 700; color: {C_TEXT_PRIMARY}; line-height: 1.25; word-break: break-word;"),
                            "{display_name}"
                        }
                        span {
                            style: format!("font-size: 9px; color: {C_TEXT_MUTED}; font-family: {FONT_MONO}; line-height: 1.35; word-break: break-all;"),
                            "{assembly_path}"
                        }
                    }
                }
            }

            div {
                style: "padding: 8px; display: flex; flex-direction: column; gap: 6px;",

                MetadataSection {
                    section_key: "overview".to_string(),
                    title: "Overview".to_string(),
                    count_label: None,
                    collapsed_sections,
                    div {
                        style: "display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 6px;",
                        MetadataField { label: "Assembly name".to_string(), value: metadata_value(metadata.assembly_name.as_str()) }
                        MetadataField { label: "Version".to_string(), value: optional_value(metadata.version.as_deref()) }
                        MetadataField {
                            label: target_framework_label(&metadata).to_string(),
                            value: target_framework_value(&metadata),
                        }
                        MetadataField { label: "Runtime".to_string(), value: optional_value(metadata.runtime_version.as_deref()) }
                        MetadataField { label: "Architecture".to_string(), value: optional_value(metadata.architecture.as_deref()) }
                        MetadataField { label: "Module kind".to_string(), value: optional_value(metadata.module_kind.as_deref()) }
                        MetadataField { label: "Culture".to_string(), value: optional_value(metadata.culture.as_deref()) }
                        MetadataField { label: "Public key token".to_string(), value: optional_value(metadata.public_key_token.as_deref()) }
                        MetadataField { label: "Entry point".to_string(), value: optional_value(metadata.entry_point.as_deref()) }
                        MetadataField { label: "MVID".to_string(), value: optional_value(metadata.mvid.as_deref()) }
                        MetadataFieldWide { label: "Full name".to_string(), value: metadata_value(metadata.full_name.as_str()) }
                    }
                }

                MetadataSection {
                    section_key: "modules".to_string(),
                    title: "Modules".to_string(),
                    count_label: Some(modules_count.to_string()),
                    collapsed_sections,
                    if metadata.modules.is_empty() {
                        MetadataEmpty { message: "No module metadata exposed by the worker.".to_string() }
                    } else {
                        for module in metadata.modules.iter() {
                            MetadataListCard {
                                title: display_or_fallback(&module.name, "Unnamed module"),
                                subtitle: optional_value(module.file_name.as_deref()),
                                rows: vec![
                                    ("Runtime".to_string(), optional_value(module.runtime_version.as_deref())),
                                    ("Architecture".to_string(), optional_value(module.architecture.as_deref())),
                                    ("Kind".to_string(), optional_value(module.module_kind.as_deref())),
                                    ("MVID".to_string(), optional_value(module.mvid.as_deref())),
                                ],
                            }
                        }
                    }
                }

                MetadataSection {
                    section_key: "references".to_string(),
                    title: "Assembly references".to_string(),
                    count_label: Some(references_count.to_string()),
                    collapsed_sections,
                    if metadata.assembly_references.is_empty() {
                        MetadataEmpty { message: "No assembly references were found.".to_string() }
                    } else {
                        for reference in metadata.assembly_references.iter() {
                            MetadataListCard {
                                title: display_or_fallback(&reference.name, "Unnamed reference"),
                                subtitle: optional_value(reference.version.as_deref()),
                                rows: vec![
                                    ("Full name".to_string(), metadata_value(reference.full_name.as_str())),
                                    ("Culture".to_string(), optional_value(reference.culture.as_deref())),
                                    ("Public key token".to_string(), optional_value(reference.public_key_token.as_deref())),
                                ],
                            }
                        }
                    }
                }

                MetadataSection {
                    section_key: "resources".to_string(),
                    title: "Manifest resources".to_string(),
                    count_label: Some(resources_count.to_string()),
                    collapsed_sections,
                    if metadata.resources.is_empty() {
                        MetadataEmpty { message: "No manifest resources were found.".to_string() }
                    } else {
                        for resource in metadata.resources.iter() {
                            ResourceCard { resource: resource.clone() }
                        }
                    }
                }

                MetadataSection {
                    section_key: "attributes".to_string(),
                    title: "Assembly attributes".to_string(),
                    count_label: Some(attributes_count.to_string()),
                    collapsed_sections,
                    if metadata.custom_attributes.is_empty() {
                        MetadataEmpty { message: "No custom assembly attributes were found.".to_string() }
                    } else {
                        for attribute in metadata.custom_attributes.iter() {
                            AttributeCard { attribute: attribute.clone() }
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
    children: Element,
) -> Element {
    let is_open = !collapsed_sections.read().contains(&section_key);
    let toggle_key = section_key.clone();
    let chevron_rotation = if is_open { "90deg" } else { "0deg" };

    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 8px;",
            button {
                style: format!(
                    "display: flex; align-items: center; gap: 8px; width: 100%; padding: 7px 9px; border-radius: 8px; border: 1px solid {}; background: {}; color: {C_TEXT_PRIMARY}; cursor: pointer; text-align: left; transition: all 120ms ease;",
                    if is_open { C_BORDER_ACCENT } else { C_BORDER },
                    if is_open { C_BG_ELEVATED } else { C_BG_BASE },
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

                div {
                    style: "display: flex; align-items: center; min-width: 0; flex: 1;",
                    span {
                        style: format!("font-size: 10px; font-weight: 700; color: {C_TEXT_PRIMARY}; text-transform: uppercase; letter-spacing: 0.08em;"),
                        "{title}"
                    }
                }

                if let Some(count) = count_label {
                    span {
                        style: format!(
                            "margin-left: auto; flex-shrink: 0; min-width: 20px; padding: 1px 6px; border-radius: 999px; border: 1px solid {C_BORDER}; background: rgba(16,17,19,0.72); font-size: 9px; color: {C_TEXT_SECONDARY}; text-align: center;"
                        ),
                        "{count}"
                    }
                }
            }

            if is_open {
                div {
                    style: format!(
                        "padding: 6px; border-radius: 8px; border: 1px solid {C_BORDER}; background: rgba(255,255,255,0.015); display: flex; flex-direction: column; gap: 6px;"
                    ),
                    {children}
                }
            }
        }
    }
}

#[component]
fn MetadataField(label: String, value: String) -> Element {
    rsx! {
        div {
            style: format!(
                "padding: 7px 8px; border-radius: 8px; border: 1px solid {C_BORDER}; background: {C_BG_BASE}; display: flex; flex-direction: column; gap: 4px; min-width: 0;"
            ),
            span {
                style: format!("font-size: 8px; color: {C_TEXT_MUTED}; text-transform: uppercase; letter-spacing: 0.06em;"),
                "{label}"
            }
            span {
                style: format!("font-size: 10px; line-height: 1.35; color: {C_TEXT_PRIMARY}; font-family: {FONT_MONO}; white-space: normal; word-break: break-word;"),
                "{value}"
            }
        }
    }
}

#[component]
fn MetadataFieldWide(label: String, value: String) -> Element {
    rsx! {
        div {
            style: format!(
                "grid-column: 1 / -1; padding: 7px 8px; border-radius: 8px; border: 1px solid {C_BORDER}; background: {C_BG_BASE}; display: flex; flex-direction: column; gap: 4px; min-width: 0;"
            ),
            span {
                style: format!("font-size: 8px; color: {C_TEXT_MUTED}; text-transform: uppercase; letter-spacing: 0.06em;"),
                "{label}"
            }
            span {
                style: format!("font-size: 10px; line-height: 1.35; color: {C_TEXT_PRIMARY}; font-family: {FONT_MONO}; white-space: normal; word-break: break-word;"),
                "{value}"
            }
        }
    }
}

#[component]
fn MetadataEmpty(message: String) -> Element {
    rsx! {
        div {
            style: format!(
                "padding: 8px; border-radius: 8px; border: 1px dashed {C_BORDER_ACCENT}; color: {C_TEXT_MUTED}; font-size: 10px; line-height: 1.4; background: rgba(16,17,19,0.54);"
            ),
            "{message}"
        }
    }
}

#[component]
fn MetadataListCard(title: String, subtitle: String, rows: Vec<(String, String)>) -> Element {
    rsx! {
        div {
            style: format!(
                "padding: 8px; border-radius: 8px; border: 1px solid {C_BORDER}; background: {C_BG_BASE}; display: flex; flex-direction: column; gap: 6px;"
            ),
            div {
                style: "display: flex; align-items: flex-start; justify-content: space-between; gap: 8px; min-width: 0;",
                span {
                    style: format!("font-size: 10px; font-weight: 700; color: {C_TEXT_PRIMARY}; line-height: 1.35; word-break: break-word;"),
                    "{title}"
                }
                span {
                    style: format!("font-size: 9px; color: {C_TEXT_MUTED}; font-family: {FONT_MONO}; text-align: right; line-height: 1.35; word-break: break-word;"),
                    "{subtitle}"
                }
            }
            for (label, value) in rows.iter() {
                div {
                    key: "{title}-{label}",
                    style: format!(
                        "display: grid; grid-template-columns: minmax(78px, 100px) 1fr; gap: 10px; padding-top: 6px; border-top: 1px solid rgba(255,255,255,0.04);"
                    ),
                    span {
                        style: format!("font-size: 9px; color: {C_TEXT_MUTED}; text-transform: uppercase; letter-spacing: 0.05em;"),
                        "{label}"
                    }
                    span {
                        style: format!("font-size: 9px; line-height: 1.4; color: {C_TEXT_PRIMARY}; font-family: {FONT_MONO}; white-space: normal; word-break: break-word;"),
                        "{value}"
                    }
                }
            }
        }
    }
}

#[component]
fn ResourceCard(resource: ResourceMetadataEntry) -> Element {
    rsx! {
        MetadataListCard {
            title: display_or_fallback(&resource.name, "Unnamed resource"),
            subtitle: display_or_fallback(&resource.resource_type, "Unknown"),
            rows: vec![
                ("Attributes".to_string(), optional_value(resource.attributes.as_deref())),
                ("Size".to_string(), format_bytes(resource.size_bytes)),
                ("Source".to_string(), optional_value(resource.implementation.as_deref())),
            ],
        }
    }
}

#[component]
fn AttributeCard(attribute: AttributeMetadataEntry) -> Element {
    rsx! {
        div {
            style: format!(
                "padding: 8px; border-radius: 8px; border: 1px solid {C_BORDER}; background: {C_BG_BASE}; display: flex; flex-direction: column; gap: 5px;"
            ),
            span {
                style: format!("font-size: 10px; line-height: 1.35; font-weight: 700; color: {C_TEXT_PRIMARY}; font-family: {FONT_MONO}; word-break: break-word;"),
                "{attribute.attribute_type}"
            }
            span {
                style: format!("font-size: 9px; line-height: 1.4; color: {C_TEXT_SECONDARY}; word-break: break-word;"),
                "{optional_value(attribute.summary.as_deref())}"
            }
        }
    }
}

fn has_text(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

fn optional_value(value: Option<&str>) -> String {
    value
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("-")
        .to_string()
}

fn target_framework_value(metadata: &AssemblyMetadataEntry) -> String {
    optional_value(
        metadata
            .target_framework
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .or(metadata.inferred_target_framework.as_deref()),
    )
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

fn metadata_value(value: &str) -> String {
    if value.trim().is_empty() {
        "-".to_string()
    } else {
        value.to_string()
    }
}

fn display_or_fallback(primary: &str, fallback: &str) -> String {
    if primary.trim().is_empty() {
        fallback.to_string()
    } else {
        primary.to_string()
    }
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
        default_collapsed_metadata_sections, format_bytes, has_metadata, target_framework_label,
        target_framework_value,
    };
    use crate::ipc::AssemblyMetadataEntry;

    #[test]
    fn default_collapsed_sections_starts_with_every_section_closed() {
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
