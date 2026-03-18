use std::collections::BTreeSet;

use dioxus::prelude::*;

use crate::ipc::{AssemblyMetadataEntry, AttributeMetadataEntry, ResourceMetadataEntry};

use super::theme::{
    C_BG_BASE, C_BG_ELEVATED, C_BG_SURFACE, C_BORDER, C_TEXT_MUTED, C_TEXT_PRIMARY, FONT_MONO,
};

pub(crate) fn has_metadata(metadata: &AssemblyMetadataEntry) -> bool {
    !metadata.assembly_name.trim().is_empty()
        || !metadata.full_name.trim().is_empty()
        || metadata.version.is_some()
        || metadata.culture.is_some()
        || metadata.public_key_token.is_some()
        || metadata.target_framework.is_some()
        || metadata.runtime_version.is_some()
        || metadata.architecture.is_some()
        || metadata.module_kind.is_some()
        || metadata.entry_point.is_some()
        || metadata.mvid.is_some()
        || !metadata.modules.is_empty()
        || !metadata.assembly_references.is_empty()
        || !metadata.resources.is_empty()
        || !metadata.custom_attributes.is_empty()
}

#[component]
pub(crate) fn ExplorerMetadataCard(
    assembly_name: String,
    assembly_path: String,
    metadata: AssemblyMetadataEntry,
    methods_count: usize,
    class_count: usize,
    namespace_count: usize,
    collapsed_sections: Signal<BTreeSet<String>>,
) -> Element {
    let resources_count = metadata.resources.len();
    let references_count = metadata.assembly_references.len();
    let attributes_count = metadata.custom_attributes.len();
    let modules_count = metadata.modules.len();

    rsx! {
        div {
            style: format!(
                "border: 1px solid {C_BORDER}; background: {C_BG_SURFACE}; border-radius: 10px; \
                 padding: 10px; display: flex; flex-direction: column; gap: 10px;"
            ),

            div {
                style: "display: flex; align-items: flex-start; justify-content: space-between; gap: 10px;",
                div {
                    style: "min-width: 0; display: flex; flex-direction: column; gap: 3px;",
                    span {
                        style: format!("font-size: 10px; font-weight: 700; letter-spacing: 0.08em; color: {C_TEXT_MUTED}; text-transform: uppercase;"),
                        "Assembly metadata"
                    }
                    span {
                        style: format!("font-size: 13px; font-weight: 700; color: {C_TEXT_PRIMARY}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"),
                        "{display_or_fallback(&metadata.assembly_name, &assembly_name)}"
                    }
                    span {
                        style: format!("font-size: 10px; color: {C_TEXT_MUTED}; font-family: {FONT_MONO}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"),
                        "{assembly_path}"
                    }
                }
                div {
                    style: "display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 6px; min-width: 180px;",
                    MetadataBadge { label: "Types".to_string(), value: class_count.to_string() }
                    MetadataBadge { label: "Methods".to_string(), value: methods_count.to_string() }
                    MetadataBadge { label: "NS".to_string(), value: namespace_count.to_string() }
                    MetadataBadge { label: "Refs".to_string(), value: references_count.to_string() }
                    MetadataBadge { label: "Res".to_string(), value: resources_count.to_string() }
                    MetadataBadge { label: "Attr".to_string(), value: attributes_count.to_string() }
                }
            }

            MetadataSection {
                section_key: "overview".to_string(),
                title: "Overview".to_string(),
                count_label: None,
                collapsed_sections,
                div {
                    style: "display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px;",
                    MetadataField { label: "Assembly name".to_string(), value: metadata_value(metadata.assembly_name.as_str()) }
                    MetadataField { label: "Version".to_string(), value: optional_value(metadata.version.as_deref()) }
                    MetadataField { label: "Target framework".to_string(), value: optional_value(metadata.target_framework.as_deref()) }
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
                            title: module.name.clone(),
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
                            title: reference.name.clone(),
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

#[component]
fn MetadataBadge(label: String, value: String) -> Element {
    rsx! {
        div {
            style: format!(
                "padding: 6px 8px; border-radius: 8px; border: 1px solid {C_BORDER}; \
                 background: {C_BG_ELEVATED}; display: flex; flex-direction: column; gap: 2px;"
            ),
            span {
                style: format!("font-size: 9px; color: {C_TEXT_MUTED}; text-transform: uppercase; letter-spacing: 0.08em;"),
                "{label}"
            }
            span {
                style: format!("font-size: 12px; font-weight: 700; color: {C_TEXT_PRIMARY};"),
                "{value}"
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
    let chevron = if is_open { "v" } else { ">" };
    let toggle_key = section_key.clone();

    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 8px;",
            button {
                style: format!(
                    "display: flex; align-items: center; gap: 8px; width: 100%; padding: 8px 10px; \
                     border-radius: 8px; border: 1px solid {C_BORDER}; background: {C_BG_BASE}; \
                     color: {C_TEXT_PRIMARY}; cursor: pointer;"
                ),
                onclick: move |_| {
                    let mut set = collapsed_sections.write();
                    if set.contains(&toggle_key) {
                        set.remove(&toggle_key);
                    } else {
                        set.insert(toggle_key.clone());
                    }
                },
                span {
                    style: format!("font-size: 10px; color: {C_TEXT_MUTED}; width: 10px; text-align: center;"),
                    "{chevron}"
                }
                span {
                    style: format!("font-size: 11px; font-weight: 700; color: {C_TEXT_PRIMARY}; text-transform: uppercase; letter-spacing: 0.06em;"),
                    "{title}"
                }
                if let Some(count) = count_label {
                    span {
                        style: format!("margin-left: auto; font-size: 10px; color: {C_TEXT_MUTED};"),
                        "{count}"
                    }
                }
            }

            if is_open {
                div {
                    style: "display: flex; flex-direction: column; gap: 8px;",
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
                "padding: 8px 9px; border-radius: 8px; border: 1px solid {C_BORDER}; \
                 background: {C_BG_ELEVATED}; display: flex; flex-direction: column; gap: 4px; min-width: 0;"
            ),
            span {
                style: format!("font-size: 9px; color: {C_TEXT_MUTED}; text-transform: uppercase; letter-spacing: 0.08em;"),
                "{label}"
            }
            span {
                style: format!("font-size: 11px; color: {C_TEXT_PRIMARY}; font-family: {FONT_MONO}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"),
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
                "grid-column: 1 / -1; padding: 8px 9px; border-radius: 8px; border: 1px solid {C_BORDER}; \
                 background: {C_BG_ELEVATED}; display: flex; flex-direction: column; gap: 4px; min-width: 0;"
            ),
            span {
                style: format!("font-size: 9px; color: {C_TEXT_MUTED}; text-transform: uppercase; letter-spacing: 0.08em;"),
                "{label}"
            }
            span {
                style: format!("font-size: 11px; color: {C_TEXT_PRIMARY}; font-family: {FONT_MONO}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"),
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
                "padding: 10px; border-radius: 8px; border: 1px dashed {C_BORDER}; \
                 color: {C_TEXT_MUTED}; font-size: 11px; background: {C_BG_ELEVATED};"
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
                "padding: 9px 10px; border-radius: 8px; border: 1px solid {C_BORDER}; \
                 background: {C_BG_ELEVATED}; display: flex; flex-direction: column; gap: 6px;"
            ),
            div {
                style: "display: flex; align-items: baseline; justify-content: space-between; gap: 8px; min-width: 0;",
                span {
                    style: format!("font-size: 11px; font-weight: 700; color: {C_TEXT_PRIMARY}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"),
                    "{title}"
                }
                span {
                    style: format!("font-size: 10px; color: {C_TEXT_MUTED}; font-family: {FONT_MONO}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"),
                    "{subtitle}"
                }
            }
            for (label, value) in rows.iter() {
                div {
                    key: "{title}-{label}",
                    style: "display: flex; align-items: flex-start; justify-content: space-between; gap: 12px;",
                    span {
                        style: format!("font-size: 10px; color: {C_TEXT_MUTED}; min-width: 84px;"),
                        "{label}"
                    }
                    span {
                        style: format!("font-size: 10px; color: {C_TEXT_PRIMARY}; font-family: {FONT_MONO}; text-align: right; word-break: break-word;"),
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
            title: resource.name.clone(),
            subtitle: resource.resource_type.clone(),
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
                "padding: 9px 10px; border-radius: 8px; border: 1px solid {C_BORDER}; \
                 background: {C_BG_ELEVATED}; display: flex; flex-direction: column; gap: 6px;"
            ),
            span {
                style: format!("font-size: 11px; font-weight: 700; color: {C_TEXT_PRIMARY}; font-family: {FONT_MONO}; word-break: break-word;"),
                "{attribute.attribute_type}"
            }
            span {
                style: format!("font-size: 10px; color: {C_TEXT_MUTED}; word-break: break-word;"),
                "{optional_value(attribute.summary.as_deref())}"
            }
        }
    }
}

fn optional_value(value: Option<&str>) -> String {
    value
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("—")
        .to_string()
}

fn metadata_value(value: &str) -> String {
    if value.trim().is_empty() {
        "—".to_string()
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
        return "—".to_string();
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
    use super::{format_bytes, has_metadata};
    use crate::ipc::AssemblyMetadataEntry;

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
    fn format_bytes_handles_missing_sizes() {
        assert_eq!(format_bytes(None), "—");
    }

    #[test]
    fn format_bytes_formats_binary_units() {
        assert_eq!(format_bytes(Some(1536)), "1.5 KB");
    }
}
