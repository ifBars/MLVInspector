/// UI-facing view models derived from the worker IPC payloads.
///
/// These types are separate from the domain types in `crate::types` so that
/// display logic can live here without polluting the core data model.

#[derive(Clone, PartialEq)]
pub struct UiMethod {
    pub type_name: String,
    pub method_name: String,
    pub metadata_token: Option<String>,
    pub signature: String,
    pub instructions: Vec<UiInstruction>,
}

#[derive(Clone, PartialEq)]
pub struct UiInstruction {
    pub offset: i64,
    pub op_code: String,
    pub operand: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiFinding {
    pub rule_id: String,
    pub severity: String,
    pub location: String,
    pub description: String,
    pub code_snippet: String,
    pub visibility: Option<String>,
    pub risk_score: Option<i32>,
    pub developer_guidance: Option<UiDeveloperGuidance>,
    pub il_offset: Option<i64>,
    pub navigation: Option<UiFindingNavigation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiDeveloperGuidance {
    pub remediation: String,
    pub documentation_url: Option<String>,
    pub alternative_apis: Vec<String>,
    pub is_remediable: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiScanOverview {
    pub classification: String,
    pub headline: String,
    pub summary: String,
    pub blocking_recommended: bool,
    pub primary_threat_family_id: Option<String>,
    pub completeness_status: String,
    pub review_recommended: bool,
    pub completeness_reasons: Vec<String>,
    pub threat_families: Vec<UiThreatFamily>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiThreatFamily {
    pub family_id: String,
    pub display_name: String,
    pub summary: String,
    pub match_kind: String,
    pub confidence: f64,
    pub exact_hash_match: bool,
    pub advisory_slugs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiFindingNavigation {
    pub primary_type_name: String,
    pub primary_method_name: String,
    pub method_spans: Vec<UiFindingMethodSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiFindingMethodSpan {
    pub type_name: String,
    pub method_name: String,
    pub il_offsets: Vec<i64>,
    pub csharp_snippets: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiScanNeighborhood {
    pub finding_index: usize,
    pub rule_id: String,
    pub severity: String,
    pub chain_kind: String,
    pub title: String,
    pub description: String,
    pub location: String,
    pub primary_type_name: Option<String>,
    pub primary_method_name: Option<String>,
    pub nodes: Vec<UiScanNeighborhoodNode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiScanNeighborhoodNode {
    pub node_type: String,
    pub location: String,
    pub operation: String,
    pub description: String,
    pub instruction_offset: Option<i32>,
}

#[derive(Clone, PartialEq)]
pub struct UiTypeGroup {
    pub full_type_name: String,
    pub display_name: String,
    pub metadata_token: Option<String>,
    pub kind: String,
    pub methods: Vec<UiMethod>,
}

#[derive(Clone, PartialEq)]
pub struct UiTypeDetails {
    pub full_type_name: String,
    pub metadata_token: Option<String>,
    pub kind: String,
    pub fields: Vec<UiMemberMetadata>,
    pub properties: Vec<UiMemberMetadata>,
    pub events: Vec<UiMemberMetadata>,
    pub nested_types: Vec<UiMemberMetadata>,
    pub custom_attributes: Vec<UiAttributeMetadata>,
}

#[derive(Clone, PartialEq)]
pub struct UiMemberMetadata {
    pub name: String,
    pub metadata_token: Option<String>,
    pub kind: String,
    pub signature: String,
    pub attributes: Option<String>,
}

#[derive(Clone, PartialEq)]
pub struct UiAttributeMetadata {
    pub attribute_type: String,
    pub summary: Option<String>,
}

#[derive(Clone, PartialEq)]
pub struct UiNamespaceGroup {
    pub namespace_name: String,
    pub types: Vec<UiTypeGroup>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum IlTabKind {
    AssemblyMetadata,
    Type,
    Method,
}

#[derive(Clone, PartialEq)]
pub struct IlTab {
    pub id: String,
    pub kind: IlTabKind,
    pub type_name: String,
    pub method_name: Option<String>,
    pub metadata_token: Option<String>,
    pub title: String,
    pub subtitle: String,
}

/// Toggle between raw IL and decompiled C# source in the main view panel.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Il,
    CSharp,
    Analyze,
}

pub fn format_symbol_reference_location(reference: &crate::ipc::SymbolReferenceEntry) -> String {
    match reference.instruction_offset {
        Some(offset) => format!("IL_{offset:04X} {}", reference.operation),
        None => reference.operation.clone(),
    }
}

pub fn symbol_reference_display_name(reference: &crate::ipc::SymbolReferenceEntry) -> String {
    format!("{}.{}", reference.type_name, reference.method_name)
}

pub fn format_symbol_evidence_location(evidence: &crate::ipc::SymbolEvidenceEntry) -> String {
    match evidence.instruction_offset {
        Some(offset) => format!("IL_{offset:04X} {}", evidence.operation),
        None => evidence.operation.clone(),
    }
}

pub fn symbol_evidence_display_value(evidence: &crate::ipc::SymbolEvidenceEntry) -> String {
    evidence
        .value
        .as_ref()
        .or(evidence.operand.as_ref())
        .cloned()
        .unwrap_or_else(|| evidence.label.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_symbol_reference_location_includes_hex_offset() {
        let reference = crate::ipc::SymbolReferenceEntry {
            type_name: "Ns.Loader".to_string(),
            method_name: "Run".to_string(),
            signature: "void Run()".to_string(),
            depth: 1,
            instruction_offset: Some(31),
            operation: "call".to_string(),
            operand: Some("Process.Start".to_string()),
        };

        assert_eq!(format_symbol_reference_location(&reference), "IL_001F call");
    }

    #[test]
    fn symbol_reference_display_name_joins_type_and_method() {
        let reference = crate::ipc::SymbolReferenceEntry {
            type_name: "Ns.Loader".to_string(),
            method_name: "Run".to_string(),
            signature: "void Run()".to_string(),
            depth: 1,
            instruction_offset: None,
            operation: "call".to_string(),
            operand: None,
        };

        assert_eq!(symbol_reference_display_name(&reference), "Ns.Loader.Run");
    }

    #[test]
    fn symbol_evidence_display_value_prefers_literal_value() {
        let evidence = crate::ipc::SymbolEvidenceEntry {
            category: "string".to_string(),
            label: "String literal".to_string(),
            type_name: "Ns.Loader".to_string(),
            method_name: "Run".to_string(),
            signature: "void Run()".to_string(),
            instruction_offset: Some(10),
            operation: "ldstr".to_string(),
            operand: Some("\"fallback\"".to_string()),
            value: Some("powershell".to_string()),
        };

        assert_eq!(symbol_evidence_display_value(&evidence), "powershell");
        assert_eq!(format_symbol_evidence_location(&evidence), "IL_000A ldstr");
    }
}
