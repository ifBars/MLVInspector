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

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DecompileParams {
    pub assembly: String,
    /// Fully-qualified type name. `None` = decompile whole assembly.
    pub type_name: Option<String>,
    /// Method name within the type. `None` = decompile whole type.
    pub method_name: Option<String>,
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
    pub methods: Vec<MethodEntry>,
    #[serde(default)]
    pub types: Vec<TypeEntry>,
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
}
