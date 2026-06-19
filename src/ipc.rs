#![allow(dead_code)]
//! IPC protocol types shared between the Rust client and the C# worker.
//!
//! Wire format: newline-delimited JSON (NDJSON).
//! Every message is a single JSON object terminated with `\n`.
//!
//! Request  (Rust → Worker):  `{ "id": u64, "method": "...", "params": { ... } }`
//! Response (Worker → Rust):  `{ "id": u64, "ok": true,  "payload": { ... } }`
//!                         or `{ "id": u64, "ok": false, "error": "..." }`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_true() -> bool {
    true
}

// ─── Outbound (Rust → Worker) ────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct WorkerRequest<P: Serialize> {
    pub id: u64,
    pub method: &'static str,
    pub params: P,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExploreParams {
    pub assembly: String,
    pub type_filter: Option<String>,
    pub method_filter: Option<String>,
    pub namespace_filter: Option<String>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScanParams {
    pub assembly: String,
    pub type_filter: Option<String>,
    pub method_filter: Option<String>,
    pub namespace_filter: Option<String>,
    pub include_rules: Option<Vec<String>>,
    pub exclude_rules: Option<Vec<String>>,
    pub show_clean: bool,
}

#[derive(Debug, Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DecompileParams {
    pub assembly: String,
    /// Fully-qualified type name. `None` = decompile whole assembly.
    pub type_name: Option<String>,
    /// Method name within the type. `None` = decompile whole type.
    pub method_name: Option<String>,
    /// Decompiler profile. Supported values: `readable`, `analysis`.
    pub profile: Option<String>,
}

#[derive(Debug, Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeSymbolParams {
    pub assembly: String,
    pub type_name: String,
    pub method_name: Option<String>,
    pub metadata_token: Option<String>,
    pub max_depth: Option<i32>,
}

/// Empty params for methods that don't need any.
#[derive(Debug, Serialize)]
pub struct NoParams {}

// ─── Inbound (Worker → Rust) ─────────────────────────────────────────────────

/// Generic envelope — `payload` is deserialized lazily from the raw JSON value.
#[derive(Debug, Deserialize)]
pub struct WorkerResponse {
    pub id: u64,
    pub ok: bool,
    pub payload: Option<serde_json::Value>,
    pub error: Option<String>,
}

// ─── Payload types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplorePayload {
    pub assembly_path: String,
    #[serde(default)]
    pub assembly_metadata: AssemblyMetadataEntry,
    pub methods: Vec<MethodEntry>,
    #[serde(default)]
    pub types: Vec<TypeEntry>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssemblyMetadataEntry {
    pub assembly_name: String,
    pub full_name: String,
    pub version: Option<String>,
    pub culture: Option<String>,
    pub public_key_token: Option<String>,
    pub target_framework: Option<String>,
    pub inferred_target_framework: Option<String>,
    pub runtime_version: Option<String>,
    pub architecture: Option<String>,
    pub module_kind: Option<String>,
    pub entry_point: Option<String>,
    pub mvid: Option<String>,
    #[serde(default)]
    pub modules: Vec<ModuleMetadataEntry>,
    #[serde(default)]
    pub assembly_references: Vec<AssemblyReferenceEntry>,
    #[serde(default)]
    pub resources: Vec<ResourceMetadataEntry>,
    #[serde(default)]
    pub custom_attributes: Vec<AttributeMetadataEntry>,
    #[serde(default)]
    pub metadata_tables: Vec<MetadataTableEntry>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleMetadataEntry {
    pub name: String,
    pub runtime_version: Option<String>,
    pub architecture: Option<String>,
    pub module_kind: Option<String>,
    pub mvid: Option<String>,
    pub file_name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssemblyReferenceEntry {
    pub name: String,
    pub full_name: String,
    pub version: Option<String>,
    pub culture: Option<String>,
    pub public_key_token: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceMetadataEntry {
    pub name: String,
    pub resource_type: String,
    pub attributes: Option<String>,
    pub size_bytes: Option<i64>,
    pub implementation: Option<String>,
    pub metadata_token: Option<String>,
    pub sha256_hash: Option<String>,
    pub preview_kind: Option<String>,
    pub preview: Option<String>,
    #[serde(default)]
    pub preview_truncated: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributeMetadataEntry {
    pub attribute_type: String,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataTableEntry {
    pub name: String,
    pub token_prefix: String,
    pub row_count: i32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeEntry {
    pub type_name: String,
    #[serde(default)]
    pub metadata_token: Option<String>,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub fields: Vec<MemberMetadataEntry>,
    #[serde(default)]
    pub properties: Vec<MemberMetadataEntry>,
    #[serde(default)]
    pub events: Vec<MemberMetadataEntry>,
    #[serde(default)]
    pub nested_types: Vec<MemberMetadataEntry>,
    #[serde(default)]
    pub custom_attributes: Vec<AttributeMetadataEntry>,
    pub methods: Vec<MethodEntry>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberMetadataEntry {
    pub name: String,
    pub metadata_token: Option<String>,
    #[serde(default)]
    pub kind: String,
    pub signature: String,
    pub attributes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MethodEntry {
    pub type_name: String,
    pub method_name: String,
    #[serde(default)]
    pub metadata_token: Option<String>,
    pub signature: String,
    pub has_body: Option<bool>,
    pub instructions: Vec<ILInstructionEntry>,
    pub p_invoke: Option<PInvokeEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ILInstructionEntry {
    pub offset: i32,
    pub op_code: String,
    pub operand: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PInvokeEntry {
    pub dll_name: String,
    pub entry_point: String,
    pub is_p_invoke: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanPayload {
    pub assembly_path: String,
    pub schema_version: String,
    pub metadata: ScanMetaEntry,
    pub input: ScanInputEntry,
    #[serde(default)]
    pub assembly: Option<ScanAssemblyEntry>,
    pub summary: ScanSummaryEntry,
    #[serde(default)]
    pub analysis_completeness: AnalysisCompletenessEntry,
    pub findings: Vec<FindingEntry>,
    #[serde(default)]
    pub call_chains: Option<Vec<CallChainEntry>>,
    #[serde(default)]
    pub data_flows: Option<Vec<DataFlowChainEntry>>,
    #[serde(default)]
    pub developer_guidance: Option<Vec<DeveloperGuidanceEntry>>,
    #[serde(default)]
    pub threat_families: Option<Vec<ThreatFamilyEntry>>,
    #[serde(default)]
    pub disposition: Option<ThreatDispositionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanMetaEntry {
    pub scanner_version: String,
    pub timestamp: String,
    pub scan_mode: String,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanInputEntry {
    pub file_name: String,
    pub size_bytes: i64,
    pub sha256_hash: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanAssemblyEntry {
    pub name: Option<String>,
    pub assembly_version: Option<String>,
    pub file_version: Option<String>,
    pub informational_version: Option<String>,
    pub target_framework: Option<String>,
    pub module_runtime_version: Option<String>,
    pub referenced_assemblies: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummaryEntry {
    pub total_findings: i32,
    pub count_by_severity: HashMap<String, i32>,
    pub triggered_rules: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisCompletenessEntry {
    #[serde(default)]
    pub status: String,
    #[serde(default = "default_true")]
    pub is_complete: bool,
    #[serde(default)]
    pub review_recommended: bool,
    #[serde(default)]
    pub reasons: Vec<AnalysisCompletenessReasonEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisCompletenessReasonEntry {
    #[serde(default)]
    pub reason_id: String,
    #[serde(default)]
    pub summary: String,
    pub phase: Option<String>,
    pub rule_id: Option<String>,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindingEntry {
    pub id: Option<String>,
    pub rule_id: Option<String>,
    pub severity: String,
    pub location: String,
    pub description: String,
    pub code_snippet: Option<String>,
    #[serde(default)]
    pub risk_score: Option<i32>,
    #[serde(default)]
    pub call_chain_id: Option<String>,
    #[serde(default)]
    pub data_flow_chain_id: Option<String>,
    #[serde(default)]
    pub developer_guidance: Option<DeveloperGuidanceEntry>,
    #[serde(default)]
    pub call_chain: Option<CallChainEntry>,
    #[serde(default)]
    pub data_flow_chain: Option<DataFlowChainEntry>,
    #[serde(default)]
    pub visibility: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperGuidanceEntry {
    pub rule_id: Option<String>,
    pub rule_ids: Option<Vec<String>>,
    #[serde(default)]
    pub remediation: String,
    pub documentation_url: Option<String>,
    pub alternative_apis: Option<Vec<String>>,
    #[serde(default)]
    pub is_remediable: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreatFamilyEntry {
    #[serde(default)]
    pub family_id: String,
    #[serde(default)]
    pub variant_id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub match_kind: String,
    #[serde(default)]
    pub confidence: f64,
    #[serde(default)]
    pub exact_hash_match: bool,
    #[serde(default)]
    pub matched_rules: Vec<String>,
    #[serde(default)]
    pub advisory_slugs: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<ThreatFamilyEvidenceEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreatFamilyEvidenceEntry {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub value: String,
    pub rule_id: Option<String>,
    pub location: Option<String>,
    pub call_chain_id: Option<String>,
    pub data_flow_chain_id: Option<String>,
    pub pattern: Option<String>,
    pub method_location: Option<String>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreatDispositionEntry {
    #[serde(default)]
    pub classification: String,
    #[serde(default)]
    pub headline: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub blocking_recommended: bool,
    pub primary_threat_family_id: Option<String>,
    #[serde(default)]
    pub related_finding_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallChainEntry {
    pub id: String,
    pub rule_id: String,
    pub description: String,
    pub severity: String,
    pub nodes: Vec<CallChainNodeEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallChainNodeEntry {
    pub node_type: String,
    pub location: String,
    pub description: String,
    pub code_snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataFlowChainEntry {
    pub id: String,
    pub description: String,
    pub severity: String,
    pub pattern: String,
    pub source_variable: Option<String>,
    pub method_location: String,
    pub is_cross_method: Option<bool>,
    pub involved_methods: Option<Vec<String>>,
    pub nodes: Vec<DataFlowNodeEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataFlowNodeEntry {
    pub node_type: String,
    pub location: String,
    pub operation: String,
    pub data_description: String,
    pub instruction_offset: i32,
    pub method_key: Option<String>,
    pub is_method_boundary: Option<bool>,
    pub target_method_key: Option<String>,
    pub code_snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleEntry {
    pub rule_id: String,
    pub description: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecompilePayload {
    pub assembly_path: String,
    pub type_name: Option<String>,
    pub method_name: Option<String>,
    /// The reconstructed C# source code.
    pub csharp_source: String,
    #[serde(default)]
    pub profile: String,
    #[serde(default)]
    pub source_spans: Vec<DecompileSourceSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DecompileSourceSpan {
    pub type_name: Option<String>,
    pub method_name: Option<String>,
    pub il_start_offset: i32,
    pub il_end_offset: i32,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeSymbolPayload {
    pub assembly_path: String,
    pub type_name: String,
    pub method_name: Option<String>,
    pub target_signature: Option<String>,
    #[serde(default = "default_analyze_depth")]
    pub max_depth: i32,
    pub callers: Vec<SymbolReferenceEntry>,
    pub callees: Vec<SymbolReferenceEntry>,
    #[serde(default)]
    pub evidence: Vec<SymbolEvidenceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SymbolReferenceEntry {
    pub type_name: String,
    pub method_name: String,
    pub signature: String,
    #[serde(default = "default_symbol_reference_depth")]
    pub depth: i32,
    pub instruction_offset: Option<i32>,
    pub operation: String,
    pub operand: Option<String>,
}

fn default_analyze_depth() -> i32 {
    1
}

fn default_symbol_reference_depth() -> i32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SymbolEvidenceEntry {
    pub category: String,
    pub label: String,
    pub type_name: String,
    pub method_name: String,
    pub signature: String,
    pub instruction_offset: Option<i32>,
    pub operation: String,
    pub operand: Option<String>,
    pub value: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{
        AnalyzeSymbolParams, AnalyzeSymbolPayload, DecompilePayload, ExplorePayload, NoParams,
        ScanParams, WorkerRequest, WorkerResponse,
    };

    #[test]
    fn worker_request_serializes_camel_case_scan_params() {
        let request = WorkerRequest {
            id: 9,
            method: "scan",
            params: ScanParams {
                assembly: "sample.dll".to_string(),
                show_clean: true,
                ..Default::default()
            },
        };

        let json = serde_json::to_value(&request).expect("request should serialize");

        assert_eq!(json["id"], 9);
        assert_eq!(json["method"], "scan");
        assert_eq!(json["params"]["assembly"], "sample.dll");
        assert_eq!(json["params"]["showClean"], true);
    }

    #[test]
    fn analyze_symbol_params_serializes_metadata_token() {
        let json = serde_json::to_value(AnalyzeSymbolParams {
            assembly: "sample.dll".to_string(),
            type_name: "Demo.Widget".to_string(),
            method_name: None,
            metadata_token: Some("0x04000001".to_string()),
            max_depth: Some(1),
        })
        .expect("params should serialize");

        assert_eq!(json["typeName"], "Demo.Widget");
        assert_eq!(json["metadataToken"], "0x04000001");
        assert_eq!(json["maxDepth"], 1);
    }

    #[test]
    fn explore_payload_deserializes_missing_types_as_empty() {
        let payload: ExplorePayload =
            serde_json::from_str(r#"{"assemblyPath":"sample.dll","methods":[]}"#)
                .expect("payload should deserialize");

        assert_eq!(payload.assembly_path, "sample.dll");
        assert!(payload.assembly_metadata.assembly_name.is_empty());
        assert!(payload.methods.is_empty());
        assert!(payload.types.is_empty());
    }

    #[test]
    fn explore_payload_deserializes_type_and_method_metadata_tokens() {
        let payload: ExplorePayload = serde_json::from_str(
            r#"{"assemblyPath":"sample.dll","methods":[{"typeName":"Demo.Runner","methodName":"Run","metadataToken":"0x06000002","signature":"void Run()","hasBody":true,"instructions":[],"pInvoke":null}],"types":[{"typeName":"Demo.Runner","metadataToken":"0x02000002","kind":"class","methods":[]}]}"#,
        )
        .expect("payload should deserialize");

        assert_eq!(
            payload.types[0].metadata_token.as_deref(),
            Some("0x02000002")
        );
        assert_eq!(
            payload.methods[0].metadata_token.as_deref(),
            Some("0x06000002")
        );
    }

    #[test]
    fn explore_payload_deserializes_metadata_table_summary() {
        let payload: ExplorePayload = serde_json::from_str(
            r#"{"assemblyPath":"sample.dll","assemblyMetadata":{"assemblyName":"sample","fullName":"sample, Version=1.0.0.0","metadataTables":[{"name":"TypeDef","tokenPrefix":"0x02","rowCount":4,"description":"Defined types"}]},"methods":[],"types":[]}"#,
        )
        .expect("payload should deserialize");

        assert_eq!(payload.assembly_metadata.metadata_tables.len(), 1);
        assert_eq!(payload.assembly_metadata.metadata_tables[0].name, "TypeDef");
        assert_eq!(
            payload.assembly_metadata.metadata_tables[0].token_prefix,
            "0x02"
        );
        assert_eq!(payload.assembly_metadata.metadata_tables[0].row_count, 4);
    }

    #[test]
    fn decompile_payload_deserializes_default_profile_and_source_spans() {
        let payload: DecompilePayload = serde_json::from_str(
            r#"{"assemblyPath":"sample.dll","typeName":null,"methodName":null,"csharpSource":"class Demo {}"}"#,
        )
        .expect("payload should deserialize");

        assert_eq!(payload.profile, "");
        assert!(payload.source_spans.is_empty());
        assert_eq!(payload.csharp_source, "class Demo {}");
    }

    #[test]
    fn worker_response_deserializes_error_without_payload() {
        let response: WorkerResponse =
            serde_json::from_str(r#"{"id":7,"ok":false,"payload":null,"error":"boom"}"#)
                .expect("response should deserialize");

        assert_eq!(response.id, 7);
        assert!(!response.ok);
        assert!(response.payload.is_none());
        assert_eq!(response.error.as_deref(), Some("boom"));
    }

    #[test]
    fn empty_params_serializes_as_empty_object() {
        let json = serde_json::to_string(&NoParams {}).expect("empty params should serialize");
        assert_eq!(json, "{}");
    }

    #[test]
    fn analyze_symbol_payload_deserializes_callers_and_callees() {
        let payload: AnalyzeSymbolPayload = serde_json::from_str(
            r#"{"assemblyPath":"sample.dll","typeName":"Ns.A","methodName":"Run","targetSignature":"void Run()","maxDepth":2,"callers":[{"typeName":"Ns.B","methodName":"Call","signature":"void Call()","depth":2,"instructionOffset":4,"operation":"call","operand":"A.Run"}],"callees":[],"evidence":[{"category":"string","label":"String literal","typeName":"Ns.A","methodName":"Run","signature":"void Run()","instructionOffset":8,"operation":"ldstr","operand":"\"powershell\"","value":"powershell"}]}"#,
        )
        .expect("payload should deserialize");

        assert_eq!(payload.type_name, "Ns.A");
        assert_eq!(payload.method_name.as_deref(), Some("Run"));
        assert_eq!(payload.max_depth, 2);
        assert_eq!(payload.callers.len(), 1);
        assert_eq!(payload.callers[0].depth, 2);
        assert!(payload.callees.is_empty());
        assert_eq!(payload.evidence.len(), 1);
        assert_eq!(payload.evidence[0].category, "string");
    }

    #[test]
    fn analyze_symbol_payload_defaults_missing_evidence_to_empty() {
        let payload: AnalyzeSymbolPayload = serde_json::from_str(
            r#"{"assemblyPath":"sample.dll","typeName":"Ns.A","methodName":null,"targetSignature":"Ns.A","callers":[],"callees":[]}"#,
        )
        .expect("payload should deserialize");

        assert_eq!(payload.max_depth, 1);
        assert!(payload.evidence.is_empty());
    }
}
