using System.Text.Json;
using System.Text.Json.Serialization;

namespace ILInspector.Worker;

public sealed class WorkerRequest
{
    [JsonPropertyName("id")]
    public ulong Id { get; set; }

    [JsonPropertyName("method")]
    public string Method { get; set; } = "";

    [JsonPropertyName("params")]
    public JsonElement Params { get; set; }
}

public sealed class ExploreParams
{
    [JsonPropertyName("assembly")]
    public string Assembly { get; set; } = "";

    [JsonPropertyName("typeFilter")]
    public string? TypeFilter { get; set; }

    [JsonPropertyName("methodFilter")]
    public string? MethodFilter { get; set; }

    [JsonPropertyName("namespaceFilter")]
    public string? NamespaceFilter { get; set; }
}

public sealed class ScanParams
{
    [JsonPropertyName("assembly")]
    public string Assembly { get; set; } = "";

    [JsonPropertyName("typeFilter")]
    public string? TypeFilter { get; set; }

    [JsonPropertyName("methodFilter")]
    public string? MethodFilter { get; set; }

    [JsonPropertyName("namespaceFilter")]
    public string? NamespaceFilter { get; set; }

    [JsonPropertyName("includeRules")]
    public List<string>? IncludeRules { get; set; }

    [JsonPropertyName("excludeRules")]
    public List<string>? ExcludeRules { get; set; }

    [JsonPropertyName("showClean")]
    public bool ShowClean { get; set; }
}

public sealed class DecompileParams
{
    [JsonPropertyName("assembly")]
    public string Assembly { get; set; } = "";

    /// <summary>Fully-qualified type name, e.g. "MyNs.MyClass". If null, decompiles the whole assembly.</summary>
    [JsonPropertyName("typeName")]
    public string? TypeName { get; set; }

    /// <summary>Method name within the type. If null, decompiles the whole type.</summary>
    [JsonPropertyName("methodName")]
    public string? MethodName { get; set; }

    /// <summary>Decompiler profile. Supported values: readable, analysis.</summary>
    [JsonPropertyName("profile")]
    public string? Profile { get; set; }
}

public sealed class DecompilePayload
{
    [JsonPropertyName("assemblyPath")]
    public string AssemblyPath { get; set; } = "";

    [JsonPropertyName("typeName")]
    public string? TypeName { get; set; }

    [JsonPropertyName("methodName")]
    public string? MethodName { get; set; }

    /// <summary>The reconstructed C# source code.</summary>
    [JsonPropertyName("csharpSource")]
    public string CsharpSource { get; set; } = "";

    [JsonPropertyName("profile")]
    public string Profile { get; set; } = "readable";

    [JsonPropertyName("sourceSpans")]
    public List<SourceSpanEntry> SourceSpans { get; set; } = new();
}

public sealed class SourceSpanEntry
{
    [JsonPropertyName("typeName")]
    public string? TypeName { get; set; }

    [JsonPropertyName("methodName")]
    public string? MethodName { get; set; }

    [JsonPropertyName("ilStartOffset")]
    public int IlStartOffset { get; set; }

    [JsonPropertyName("ilEndOffset")]
    public int IlEndOffset { get; set; }

    [JsonPropertyName("startLine")]
    public int StartLine { get; set; }

    [JsonPropertyName("endLine")]
    public int EndLine { get; set; }
}

public sealed class CompareParams
{
    [JsonPropertyName("assembly")]
    public string Assembly { get; set; } = "";

    [JsonPropertyName("typeFilter")]
    public string? TypeFilter { get; set; }

    [JsonPropertyName("methodFilter")]
    public string? MethodFilter { get; set; }

    [JsonPropertyName("namespaceFilter")]
    public string? NamespaceFilter { get; set; }

    [JsonPropertyName("expectedRule")]
    public string? ExpectedRule { get; set; }
}

public sealed class AnalyzeSymbolParams
{
    [JsonPropertyName("assembly")]
    public string Assembly { get; set; } = "";

    [JsonPropertyName("typeName")]
    public string TypeName { get; set; } = "";

    [JsonPropertyName("methodName")]
    public string? MethodName { get; set; }

    [JsonPropertyName("metadataToken")]
    public string? MetadataToken { get; set; }

    [JsonPropertyName("maxDepth")]
    public int? MaxDepth { get; set; }
}

public sealed class AnalyzeSymbolPayload
{
    [JsonPropertyName("assemblyPath")]
    public string AssemblyPath { get; set; } = "";

    [JsonPropertyName("typeName")]
    public string TypeName { get; set; } = "";

    [JsonPropertyName("methodName")]
    public string? MethodName { get; set; }

    [JsonPropertyName("targetSignature")]
    public string? TargetSignature { get; set; }

    [JsonPropertyName("maxDepth")]
    public int MaxDepth { get; set; } = 1;

    [JsonPropertyName("callers")]
    public List<SymbolReferenceEntry> Callers { get; set; } = new();

    [JsonPropertyName("callees")]
    public List<SymbolReferenceEntry> Callees { get; set; } = new();

    [JsonPropertyName("evidence")]
    public List<SymbolEvidenceEntry> Evidence { get; set; } = new();
}

public sealed class SymbolReferenceEntry
{
    [JsonPropertyName("typeName")]
    public string TypeName { get; set; } = "";

    [JsonPropertyName("methodName")]
    public string MethodName { get; set; } = "";

    [JsonPropertyName("signature")]
    public string Signature { get; set; } = "";

    [JsonPropertyName("depth")]
    public int Depth { get; set; } = 1;

    [JsonPropertyName("instructionOffset")]
    public int? InstructionOffset { get; set; }

    [JsonPropertyName("operation")]
    public string Operation { get; set; } = "";

    [JsonPropertyName("operand")]
    public string? Operand { get; set; }
}

public sealed class SymbolEvidenceEntry
{
    [JsonPropertyName("category")]
    public string Category { get; set; } = "";

    [JsonPropertyName("label")]
    public string Label { get; set; } = "";

    [JsonPropertyName("typeName")]
    public string TypeName { get; set; } = "";

    [JsonPropertyName("methodName")]
    public string MethodName { get; set; } = "";

    [JsonPropertyName("signature")]
    public string Signature { get; set; } = "";

    [JsonPropertyName("instructionOffset")]
    public int? InstructionOffset { get; set; }

    [JsonPropertyName("operation")]
    public string Operation { get; set; } = "";

    [JsonPropertyName("operand")]
    public string? Operand { get; set; }

    [JsonPropertyName("value")]
    public string? Value { get; set; }
}

public sealed class WorkerResponse<T>
{
    [JsonPropertyName("id")]
    public ulong Id { get; set; }

    [JsonPropertyName("ok")]
    public bool Ok { get; set; }

    [JsonPropertyName("payload")]
    public T? Payload { get; set; }

    [JsonPropertyName("error")]
    public string? Error { get; set; }
}

public sealed class ExplorePayload
{
    [JsonPropertyName("assemblyPath")]
    public string AssemblyPath { get; set; } = "";

    [JsonPropertyName("assemblyMetadata")]
    public AssemblyMetadataEntry AssemblyMetadata { get; set; } = new();

    [JsonPropertyName("methods")]
    public List<MethodEntry> Methods { get; set; } = new();

    [JsonPropertyName("types")]
    public List<TypeEntry> Types { get; set; } = new();
}

public sealed class AssemblyMetadataEntry
{
    [JsonPropertyName("assemblyName")]
    public string AssemblyName { get; set; } = "";

    [JsonPropertyName("fullName")]
    public string FullName { get; set; } = "";

    [JsonPropertyName("version")]
    public string? Version { get; set; }

    [JsonPropertyName("culture")]
    public string? Culture { get; set; }

    [JsonPropertyName("publicKeyToken")]
    public string? PublicKeyToken { get; set; }

    [JsonPropertyName("targetFramework")]
    public string? TargetFramework { get; set; }

    [JsonPropertyName("inferredTargetFramework")]
    public string? InferredTargetFramework { get; set; }

    [JsonPropertyName("runtimeVersion")]
    public string? RuntimeVersion { get; set; }

    [JsonPropertyName("architecture")]
    public string? Architecture { get; set; }

    [JsonPropertyName("moduleKind")]
    public string? ModuleKind { get; set; }

    [JsonPropertyName("entryPoint")]
    public string? EntryPoint { get; set; }

    [JsonPropertyName("mvid")]
    public string? Mvid { get; set; }

    [JsonPropertyName("modules")]
    public List<ModuleMetadataEntry> Modules { get; set; } = new();

    [JsonPropertyName("assemblyReferences")]
    public List<AssemblyReferenceEntry> AssemblyReferences { get; set; } = new();

    [JsonPropertyName("resources")]
    public List<ResourceMetadataEntry> Resources { get; set; } = new();

    [JsonPropertyName("customAttributes")]
    public List<AttributeMetadataEntry> CustomAttributes { get; set; } = new();

    [JsonPropertyName("metadataTables")]
    public List<MetadataTableEntry> MetadataTables { get; set; } = new();
}

public sealed class MetadataTableEntry
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = "";

    [JsonPropertyName("tokenPrefix")]
    public string TokenPrefix { get; set; } = "";

    [JsonPropertyName("rowCount")]
    public int RowCount { get; set; }

    [JsonPropertyName("description")]
    public string Description { get; set; } = "";
}

public sealed class ModuleMetadataEntry
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = "";

    [JsonPropertyName("runtimeVersion")]
    public string? RuntimeVersion { get; set; }

    [JsonPropertyName("architecture")]
    public string? Architecture { get; set; }

    [JsonPropertyName("moduleKind")]
    public string? ModuleKind { get; set; }

    [JsonPropertyName("mvid")]
    public string? Mvid { get; set; }

    [JsonPropertyName("fileName")]
    public string? FileName { get; set; }
}

public sealed class AssemblyReferenceEntry
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = "";

    [JsonPropertyName("fullName")]
    public string FullName { get; set; } = "";

    [JsonPropertyName("version")]
    public string? Version { get; set; }

    [JsonPropertyName("culture")]
    public string? Culture { get; set; }

    [JsonPropertyName("publicKeyToken")]
    public string? PublicKeyToken { get; set; }
}

public sealed class ResourceMetadataEntry
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = "";

    [JsonPropertyName("resourceType")]
    public string ResourceType { get; set; } = "";

    [JsonPropertyName("attributes")]
    public string? Attributes { get; set; }

    [JsonPropertyName("sizeBytes")]
    public long? SizeBytes { get; set; }

    [JsonPropertyName("implementation")]
    public string? Implementation { get; set; }

    [JsonPropertyName("metadataToken")]
    public string? MetadataToken { get; set; }

    [JsonPropertyName("sha256Hash")]
    public string? Sha256Hash { get; set; }

    [JsonPropertyName("previewKind")]
    public string? PreviewKind { get; set; }

    [JsonPropertyName("preview")]
    public string? Preview { get; set; }

    [JsonPropertyName("previewTruncated")]
    public bool PreviewTruncated { get; set; }
}

public sealed class AttributeMetadataEntry
{
    [JsonPropertyName("attributeType")]
    public string AttributeType { get; set; } = "";

    [JsonPropertyName("summary")]
    public string? Summary { get; set; }
}

public sealed class TypeEntry
{
    [JsonPropertyName("typeName")]
    public string TypeName { get; set; } = "";

    [JsonPropertyName("metadataToken")]
    public string? MetadataToken { get; set; }

    [JsonPropertyName("kind")]
    public string Kind { get; set; } = "";

    [JsonPropertyName("fields")]
    public List<MemberMetadataEntry> Fields { get; set; } = new();

    [JsonPropertyName("properties")]
    public List<MemberMetadataEntry> Properties { get; set; } = new();

    [JsonPropertyName("events")]
    public List<MemberMetadataEntry> Events { get; set; } = new();

    [JsonPropertyName("nestedTypes")]
    public List<MemberMetadataEntry> NestedTypes { get; set; } = new();

    [JsonPropertyName("customAttributes")]
    public List<AttributeMetadataEntry> CustomAttributes { get; set; } = new();

    [JsonPropertyName("methods")]
    public List<MethodEntry> Methods { get; set; } = new();
}

public sealed class MemberMetadataEntry
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = "";

    [JsonPropertyName("metadataToken")]
    public string? MetadataToken { get; set; }

    [JsonPropertyName("kind")]
    public string Kind { get; set; } = "";

    [JsonPropertyName("signature")]
    public string Signature { get; set; } = "";

    [JsonPropertyName("attributes")]
    public string? Attributes { get; set; }
}

public sealed class MethodEntry
{
    [JsonPropertyName("typeName")]
    public string TypeName { get; set; } = "";

    [JsonPropertyName("methodName")]
    public string MethodName { get; set; } = "";

    [JsonPropertyName("metadataToken")]
    public string? MetadataToken { get; set; }

    [JsonPropertyName("signature")]
    public string Signature { get; set; } = "";

    [JsonPropertyName("hasBody")]
    public bool HasBody { get; set; }

    [JsonPropertyName("instructions")]
    public List<ILInstructionEntry> Instructions { get; set; } = new();

    [JsonPropertyName("pInvoke")]
    public PInvokeEntry? PInvoke { get; set; }
}

public sealed class ILInstructionEntry
{
    [JsonPropertyName("offset")]
    public int Offset { get; set; }

    [JsonPropertyName("opCode")]
    public string OpCode { get; set; } = "";

    [JsonPropertyName("operand")]
    public string? Operand { get; set; }
}

public sealed class PInvokeEntry
{
    [JsonPropertyName("dllName")]
    public string DllName { get; set; } = "";

    [JsonPropertyName("entryPoint")]
    public string EntryPoint { get; set; } = "";

    [JsonPropertyName("isPInvoke")]
    public bool IsPInvoke { get; set; }
}

public sealed class ScanPayload
{
    [JsonPropertyName("assemblyPath")]
    public string AssemblyPath { get; set; } = "";

    [JsonPropertyName("schemaVersion")]
    public string SchemaVersion { get; set; } = "";

    [JsonPropertyName("metadata")]
    public ScanMetaEntry Metadata { get; set; } = new();

    [JsonPropertyName("input")]
    public ScanInputEntry Input { get; set; } = new();

    [JsonPropertyName("assembly")]
    public ScanAssemblyEntry? Assembly { get; set; }

    [JsonPropertyName("summary")]
    public ScanSummaryEntry Summary { get; set; } = new();

    [JsonPropertyName("analysisCompleteness")]
    public AnalysisCompletenessEntry AnalysisCompleteness { get; set; } = new();

    [JsonPropertyName("findings")]
    public List<FindingEntry> Findings { get; set; } = new();

    [JsonPropertyName("callChains")]
    public List<CallChainEntry>? CallChains { get; set; }

    [JsonPropertyName("dataFlows")]
    public List<DataFlowChainEntry>? DataFlows { get; set; }

    [JsonPropertyName("developerGuidance")]
    public List<DeveloperGuidanceEntry>? DeveloperGuidance { get; set; }

    [JsonPropertyName("threatFamilies")]
    public List<ThreatFamilyEntry>? ThreatFamilies { get; set; }

    [JsonPropertyName("disposition")]
    public ThreatDispositionEntry? Disposition { get; set; }
}

public sealed class ScanMetaEntry
{
    [JsonPropertyName("scannerVersion")]
    public string ScannerVersion { get; set; } = "";

    [JsonPropertyName("timestamp")]
    public string Timestamp { get; set; } = "";

    [JsonPropertyName("scanMode")]
    public string ScanMode { get; set; } = "";

    [JsonPropertyName("platform")]
    public string Platform { get; set; } = "";
}

public sealed class ScanInputEntry
{
    [JsonPropertyName("fileName")]
    public string FileName { get; set; } = "";

    [JsonPropertyName("sizeBytes")]
    public long SizeBytes { get; set; }

    [JsonPropertyName("sha256Hash")]
    public string? Sha256Hash { get; set; }
}

public sealed class ScanAssemblyEntry
{
    [JsonPropertyName("name")]
    public string? Name { get; set; }

    [JsonPropertyName("assemblyVersion")]
    public string? AssemblyVersion { get; set; }

    [JsonPropertyName("fileVersion")]
    public string? FileVersion { get; set; }

    [JsonPropertyName("informationalVersion")]
    public string? InformationalVersion { get; set; }

    [JsonPropertyName("targetFramework")]
    public string? TargetFramework { get; set; }

    [JsonPropertyName("moduleRuntimeVersion")]
    public string? ModuleRuntimeVersion { get; set; }

    [JsonPropertyName("referencedAssemblies")]
    public List<string>? ReferencedAssemblies { get; set; }
}

public sealed class ScanSummaryEntry
{
    [JsonPropertyName("totalFindings")]
    public int TotalFindings { get; set; }

    [JsonPropertyName("countBySeverity")]
    public Dictionary<string, int> CountBySeverity { get; set; } = new();

    [JsonPropertyName("triggeredRules")]
    public List<string> TriggeredRules { get; set; } = new();
}

public sealed class AnalysisCompletenessEntry
{
    [JsonPropertyName("status")]
    public string Status { get; set; } = "";

    [JsonPropertyName("isComplete")]
    public bool IsComplete { get; set; } = true;

    [JsonPropertyName("reviewRecommended")]
    public bool ReviewRecommended { get; set; }

    [JsonPropertyName("reasons")]
    public List<AnalysisCompletenessReasonEntry> Reasons { get; set; } = new();
}

public sealed class AnalysisCompletenessReasonEntry
{
    [JsonPropertyName("reasonId")]
    public string ReasonId { get; set; } = "";

    [JsonPropertyName("summary")]
    public string Summary { get; set; } = "";

    [JsonPropertyName("phase")]
    public string? Phase { get; set; }

    [JsonPropertyName("ruleId")]
    public string? RuleId { get; set; }

    [JsonPropertyName("location")]
    public string? Location { get; set; }
}

public sealed class FindingEntry
{
    [JsonPropertyName("id")]
    public string? Id { get; set; }

    [JsonPropertyName("ruleId")]
    public string? RuleId { get; set; }

    [JsonPropertyName("severity")]
    public string Severity { get; set; } = "";

    [JsonPropertyName("location")]
    public string Location { get; set; } = "";

    [JsonPropertyName("description")]
    public string Description { get; set; } = "";

    [JsonPropertyName("codeSnippet")]
    public string? CodeSnippet { get; set; }

    [JsonPropertyName("riskScore")]
    public int? RiskScore { get; set; }

    [JsonPropertyName("callChainId")]
    public string? CallChainId { get; set; }

    [JsonPropertyName("dataFlowChainId")]
    public string? DataFlowChainId { get; set; }

    [JsonPropertyName("developerGuidance")]
    public DeveloperGuidanceEntry? DeveloperGuidance { get; set; }

    [JsonPropertyName("callChain")]
    public CallChainEntry? CallChain { get; set; }

    [JsonPropertyName("dataFlowChain")]
    public DataFlowChainEntry? DataFlowChain { get; set; }

    [JsonPropertyName("visibility")]
    public string? Visibility { get; set; }
}

public sealed class DeveloperGuidanceEntry
{
    [JsonPropertyName("ruleId")]
    public string? RuleId { get; set; }

    [JsonPropertyName("ruleIds")]
    public List<string>? RuleIds { get; set; }

    [JsonPropertyName("remediation")]
    public string Remediation { get; set; } = "";

    [JsonPropertyName("documentationUrl")]
    public string? DocumentationUrl { get; set; }

    [JsonPropertyName("alternativeApis")]
    public string[]? AlternativeApis { get; set; }

    [JsonPropertyName("isRemediable")]
    public bool IsRemediable { get; set; }
}

public sealed class ThreatFamilyEntry
{
    [JsonPropertyName("familyId")]
    public string FamilyId { get; set; } = "";

    [JsonPropertyName("variantId")]
    public string VariantId { get; set; } = "";

    [JsonPropertyName("displayName")]
    public string DisplayName { get; set; } = "";

    [JsonPropertyName("summary")]
    public string Summary { get; set; } = "";

    [JsonPropertyName("matchKind")]
    public string MatchKind { get; set; } = "";

    [JsonPropertyName("confidence")]
    public double Confidence { get; set; }

    [JsonPropertyName("exactHashMatch")]
    public bool ExactHashMatch { get; set; }

    [JsonPropertyName("matchedRules")]
    public List<string> MatchedRules { get; set; } = new();

    [JsonPropertyName("advisorySlugs")]
    public List<string> AdvisorySlugs { get; set; } = new();

    [JsonPropertyName("evidence")]
    public List<ThreatFamilyEvidenceEntry> Evidence { get; set; } = new();
}

public sealed class ThreatFamilyEvidenceEntry
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = "";

    [JsonPropertyName("value")]
    public string Value { get; set; } = "";

    [JsonPropertyName("ruleId")]
    public string? RuleId { get; set; }

    [JsonPropertyName("location")]
    public string? Location { get; set; }

    [JsonPropertyName("callChainId")]
    public string? CallChainId { get; set; }

    [JsonPropertyName("dataFlowChainId")]
    public string? DataFlowChainId { get; set; }

    [JsonPropertyName("pattern")]
    public string? Pattern { get; set; }

    [JsonPropertyName("methodLocation")]
    public string? MethodLocation { get; set; }

    [JsonPropertyName("confidence")]
    public double? Confidence { get; set; }
}

public sealed class ThreatDispositionEntry
{
    [JsonPropertyName("classification")]
    public string Classification { get; set; } = "";

    [JsonPropertyName("headline")]
    public string Headline { get; set; } = "";

    [JsonPropertyName("summary")]
    public string Summary { get; set; } = "";

    [JsonPropertyName("blockingRecommended")]
    public bool BlockingRecommended { get; set; }

    [JsonPropertyName("primaryThreatFamilyId")]
    public string? PrimaryThreatFamilyId { get; set; }

    [JsonPropertyName("relatedFindingIds")]
    public List<string> RelatedFindingIds { get; set; } = new();
}

public sealed class CallChainEntry
{
    [JsonPropertyName("id")]
    public string Id { get; set; } = "";

    [JsonPropertyName("ruleId")]
    public string RuleId { get; set; } = "";

    [JsonPropertyName("description")]
    public string Description { get; set; } = "";

    [JsonPropertyName("severity")]
    public string Severity { get; set; } = "";

    [JsonPropertyName("nodes")]
    public List<CallChainNodeEntry> Nodes { get; set; } = new();
}

public sealed class CallChainNodeEntry
{
    [JsonPropertyName("nodeType")]
    public string NodeType { get; set; } = "";

    [JsonPropertyName("location")]
    public string Location { get; set; } = "";

    [JsonPropertyName("description")]
    public string Description { get; set; } = "";

    [JsonPropertyName("codeSnippet")]
    public string? CodeSnippet { get; set; }
}

public sealed class DataFlowChainEntry
{
    [JsonPropertyName("id")]
    public string Id { get; set; } = "";

    [JsonPropertyName("description")]
    public string Description { get; set; } = "";

    [JsonPropertyName("severity")]
    public string Severity { get; set; } = "";

    [JsonPropertyName("pattern")]
    public string Pattern { get; set; } = "";

    [JsonPropertyName("sourceVariable")]
    public string? SourceVariable { get; set; }

    [JsonPropertyName("methodLocation")]
    public string MethodLocation { get; set; } = "";

    [JsonPropertyName("isCrossMethod")]
    public bool? IsCrossMethod { get; set; }

    [JsonPropertyName("involvedMethods")]
    public List<string>? InvolvedMethods { get; set; }

    [JsonPropertyName("nodes")]
    public List<DataFlowNodeEntry> Nodes { get; set; } = new();
}

public sealed class DataFlowNodeEntry
{
    [JsonPropertyName("nodeType")]
    public string NodeType { get; set; } = "";

    [JsonPropertyName("location")]
    public string Location { get; set; } = "";

    [JsonPropertyName("operation")]
    public string Operation { get; set; } = "";

    [JsonPropertyName("dataDescription")]
    public string DataDescription { get; set; } = "";

    [JsonPropertyName("instructionOffset")]
    public int InstructionOffset { get; set; }

    [JsonPropertyName("methodKey")]
    public string? MethodKey { get; set; }

    [JsonPropertyName("isMethodBoundary")]
    public bool? IsMethodBoundary { get; set; }

    [JsonPropertyName("targetMethodKey")]
    public string? TargetMethodKey { get; set; }

    [JsonPropertyName("codeSnippet")]
    public string? CodeSnippet { get; set; }
}

public sealed class RuleEntry
{
    [JsonPropertyName("ruleId")]
    public string RuleId { get; set; } = "";

    [JsonPropertyName("description")]
    public string Description { get; set; } = "";

    [JsonPropertyName("severity")]
    public string Severity { get; set; } = "";
}

[JsonSourceGenerationOptions(PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase)]
[JsonSerializable(typeof(WorkerRequest))]
[JsonSerializable(typeof(WorkerResponse<ExplorePayload>))]
[JsonSerializable(typeof(WorkerResponse<ScanPayload>))]
[JsonSerializable(typeof(WorkerResponse<DecompilePayload>))]
[JsonSerializable(typeof(WorkerResponse<AnalyzeSymbolPayload>))]
[JsonSerializable(typeof(WorkerResponse<List<RuleEntry>>))]
[JsonSerializable(typeof(WorkerResponse<object>))]
[JsonSerializable(typeof(ExploreParams))]
[JsonSerializable(typeof(ScanParams))]
[JsonSerializable(typeof(DecompileParams))]
[JsonSerializable(typeof(CompareParams))]
[JsonSerializable(typeof(AnalyzeSymbolParams))]
[JsonSerializable(typeof(ExplorePayload))]
[JsonSerializable(typeof(ScanPayload))]
[JsonSerializable(typeof(DecompilePayload))]
[JsonSerializable(typeof(AnalyzeSymbolPayload))]
[JsonSerializable(typeof(AnalysisCompletenessEntry))]
[JsonSerializable(typeof(DeveloperGuidanceEntry))]
[JsonSerializable(typeof(ThreatDispositionEntry))]
[JsonSerializable(typeof(ThreatFamilyEntry))]
[JsonSerializable(typeof(SymbolEvidenceEntry))]
[JsonSerializable(typeof(MetadataTableEntry))]
[JsonSerializable(typeof(List<RuleEntry>))]
[JsonSerializable(typeof(string))]
internal partial class WorkerJsonContext : JsonSerializerContext
{
}
