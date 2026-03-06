/// UI-facing view models derived from the worker IPC payloads.
///
/// These types are separate from the domain types in `crate::types` so that
/// display logic can live here without polluting the core data model.

#[derive(Clone, PartialEq)]
pub struct UiMethod {
    pub type_name: String,
    pub method_name: String,
    pub signature: String,
    pub instructions: Vec<UiInstruction>,
}

#[derive(Clone, PartialEq)]
pub struct UiInstruction {
    pub offset: i64,
    pub op_code: String,
    pub operand: String,
}

#[derive(Clone, PartialEq)]
pub struct UiFinding {
    pub rule_id: String,
    pub severity: String,
    pub location: String,
    pub description: String,
    pub code_snippet: String,
    pub il_offset: Option<i64>,
}

#[derive(Clone, PartialEq)]
pub struct UiTypeGroup {
    pub full_type_name: String,
    pub display_name: String,
    pub methods: Vec<UiMethod>,
}

#[derive(Clone, PartialEq)]
pub struct UiNamespaceGroup {
    pub namespace_name: String,
    pub types: Vec<UiTypeGroup>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum IlTabKind {
    Type,
    Method,
}

#[derive(Clone, PartialEq)]
pub struct IlTab {
    pub id: String,
    pub kind: IlTabKind,
    pub type_name: String,
    pub method_name: Option<String>,
    pub title: String,
    pub subtitle: String,
}

/// Toggle between raw IL and decompiled C# source in the main view panel.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Il,
    CSharp,
}
