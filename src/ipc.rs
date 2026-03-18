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
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributeMetadataEntry {
    pub attribute_type: String,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeEntry {
    pub type_name: String,
    pub methods: Vec<MethodEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MethodEntry {
    pub type_name: String,
    pub method_name: String,
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
    pub summary: ScanSummaryEntry,
    pub findings: Vec<FindingEntry>,
    pub call_chains: Option<Vec<CallChainEntry>>,
    pub data_flows: Option<Vec<DataFlowChainEntry>>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummaryEntry {
    pub total_findings: i32,
    pub count_by_severity: HashMap<String, i32>,
    pub triggered_rules: Vec<String>,
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
    pub call_chain: Option<CallChainEntry>,
    pub data_flow_chain: Option<DataFlowChainEntry>,
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
    pub confidence: f64,
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

#[cfg(test)]
mod tests {
    use super::{
        DecompilePayload, ExplorePayload, NoParams, ScanParams, WorkerRequest, WorkerResponse,
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
}
