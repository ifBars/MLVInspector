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

    [JsonPropertyName("methods")]
    public List<MethodEntry> Methods { get; set; } = new();

    [JsonPropertyName("types")]
    public List<TypeEntry> Types { get; set; } = new();
}

public sealed class TypeEntry
{
    [JsonPropertyName("typeName")]
    public string TypeName { get; set; } = "";

    [JsonPropertyName("methods")]
    public List<MethodEntry> Methods { get; set; } = new();
}

public sealed class MethodEntry
{
    [JsonPropertyName("typeName")]
    public string TypeName { get; set; } = "";

    [JsonPropertyName("methodName")]
    public string MethodName { get; set; } = "";

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

    [JsonPropertyName("summary")]
    public ScanSummaryEntry Summary { get; set; } = new();

    [JsonPropertyName("findings")]
    public List<FindingEntry> Findings { get; set; } = new();

    [JsonPropertyName("callChains")]
    public List<CallChainEntry>? CallChains { get; set; }

    [JsonPropertyName("dataFlows")]
    public List<DataFlowChainEntry>? DataFlows { get; set; }
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

public sealed class ScanSummaryEntry
{
    [JsonPropertyName("totalFindings")]
    public int TotalFindings { get; set; }

    [JsonPropertyName("countBySeverity")]
    public Dictionary<string, int> CountBySeverity { get; set; } = new();

    [JsonPropertyName("triggeredRules")]
    public List<string> TriggeredRules { get; set; } = new();
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

    [JsonPropertyName("callChain")]
    public CallChainEntry? CallChain { get; set; }

    [JsonPropertyName("dataFlowChain")]
    public DataFlowChainEntry? DataFlowChain { get; set; }
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

    [JsonPropertyName("confidence")]
    public double Confidence { get; set; }

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
[JsonSerializable(typeof(WorkerResponse<List<RuleEntry>>))]
[JsonSerializable(typeof(WorkerResponse<object>))]
[JsonSerializable(typeof(ExploreParams))]
[JsonSerializable(typeof(ScanParams))]
[JsonSerializable(typeof(CompareParams))]
[JsonSerializable(typeof(ExplorePayload))]
[JsonSerializable(typeof(ScanPayload))]
[JsonSerializable(typeof(List<RuleEntry>))]
[JsonSerializable(typeof(string))]
internal partial class WorkerJsonContext : JsonSerializerContext
{
}
