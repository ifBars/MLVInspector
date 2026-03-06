#![allow(dead_code)]

//! MLVInspector type definitions.
//!
//! Mirrors the protocol types from the worker IPC layer.

use serde::{Deserialize, Serialize};

use crate::ipc::{ExplorePayload, ScanPayload};

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnalysisMode {
    Explore,
    Scan,
    Compare,
    AnalyzeReflect,
}

impl std::fmt::Display for AnalysisMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisMode::Explore => write!(f, "explore"),
            AnalysisMode::Scan => write!(f, "scan"),
            AnalysisMode::Compare => write!(f, "compare"),
            AnalysisMode::AnalyzeReflect => write!(f, "analyze-reflect"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleInfo {
    pub rule_id: String,
    pub description: String,
    pub severity: RuleSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum RuleSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
    Unknown,
}

impl std::fmt::Display for RuleSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleSeverity::Critical => write!(f, "Critical"),
            RuleSeverity::High => write!(f, "High"),
            RuleSeverity::Medium => write!(f, "Medium"),
            RuleSeverity::Low => write!(f, "Low"),
            RuleSeverity::Info => write!(f, "Info"),
            RuleSeverity::Unknown => write!(f, "Unknown"),
        }
    }
}

// ---------------------------------------------------------------------------
// Typed analysis result — replaces the old raw_json bag
// ---------------------------------------------------------------------------

/// The result of an analysis run, holding typed worker payloads.
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub assembly_path: String,
    pub mode: String,
    pub explore: Option<ExplorePayload>,
    pub scan: Option<ScanPayload>,
    pub stderr: String,
}

// ---------------------------------------------------------------------------
// UI state types
// ---------------------------------------------------------------------------

pub type AssemblyId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenAssembly {
    /// Unique ID for this assembly instance
    pub id: AssemblyId,
    /// Absolute file path on disk — NEVER contents, NEVER bytes
    pub path: String,
    /// Display name (basename)
    pub name: String,
    pub loaded_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActivePanel {
    Il,
    Findings,
    Compare,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActiveMode {
    Explore,
    Scan,
    Compare,
}

impl std::fmt::Display for ActiveMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActiveMode::Explore => write!(f, "explore"),
            ActiveMode::Scan => write!(f, "scan"),
            ActiveMode::Compare => write!(f, "compare"),
        }
    }
}

// ---------------------------------------------------------------------------
// Analysis entry for tracking status
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnalysisStatus {
    Idle,
    Running,
    Done,
    Error,
}

#[derive(Debug, Clone)]
pub struct AnalysisEntry {
    pub assembly_id: AssemblyId,
    pub assembly_path: String,
    pub mode: ActiveMode,
    pub status: AnalysisStatus,
    pub result: Option<AnalysisResult>,
    pub error: Option<String>,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
}
