using System.Text.RegularExpressions;
using ICSharpCode.Decompiler;
using ICSharpCode.Decompiler.CSharp;
using ICSharpCode.Decompiler.Metadata;
using ICSharpCode.Decompiler.TypeSystem;
using Mono.Cecil;
using Mono.Cecil.Cil;
using MLVScan;
using MLVScan.Models;
using MLVScan.Models.Dto;
using MLVScan.Services;

namespace ILInspector.Worker;

internal sealed class Dispatcher
{
    private readonly AssemblyCache _cache;

    public Dispatcher(AssemblyCache cache) => _cache = cache;

    public ExplorePayload Explore(ExploreParams p)
    {
        var assembly = _cache.Load(p.Assembly);
        var methods = new List<MethodEntry>();
        var types = new List<TypeEntry>();

        foreach (var module in assembly.Modules)
        {
            foreach (var type in EnumerateTypes(module))
            {
                // Skip the synthetic <Module> pseudo-type and all compiler-generated
                // types (async state machines, iterators, closures, lambdas).
                // These are surfaced as reconstructed async/yield code when their
                // declaring types are decompiled, so listing them separately adds
                // noise and confusion (e.g. <DownloadAsync>d__5.MoveNext).
                if (type.Name == "<Module>" ||
                    IsCompilerGeneratedType(type) ||
                    !MatchesTypeFilter(type, p.TypeFilter, p.NamespaceFilter))
                {
                    continue;
                }

                var typeMethods = new List<MethodEntry>();

                foreach (var method in type.Methods)
                {
                    if (p.MethodFilter != null && !MatchesGlob(method.Name, p.MethodFilter))
                    {
                        continue;
                    }

                    var entry = new MethodEntry
                    {
                        TypeName = type.FullName,
                        MethodName = method.Name,
                        Signature = FormatSignature(method),
                        HasBody = method.HasBody,
                    };

                    if (method.HasBody)
                    {
                        entry.Instructions = method.Body.Instructions
                            .Select(i => new ILInstructionEntry
                            {
                                Offset = i.Offset,
                                OpCode = i.OpCode.ToString(),
                                Operand = FormatOperand(i.Operand),
                            })
                            .ToList();
                    }

                    if (method.PInvokeInfo != null)
                    {
                        entry.PInvoke = new PInvokeEntry
                        {
                            DllName = method.PInvokeInfo.Module.Name,
                            EntryPoint = method.PInvokeInfo.EntryPoint ?? method.Name,
                            IsPInvoke = true,
                        };
                    }

                    methods.Add(entry);
                    typeMethods.Add(entry);
                }

                if (typeMethods.Count > 0)
                {
                    types.Add(new TypeEntry
                    {
                        TypeName = type.FullName,
                        Methods = typeMethods,
                    });
                }
            }
        }

        return new ExplorePayload
        {
            AssemblyPath = p.Assembly,
            Methods = methods,
            Types = types,
        };
    }

    public ScanPayload Scan(ScanParams p)
    {
        var rules = RuleFactory.CreateDefaultRules();

        if (p.IncludeRules is { Count: > 0 })
        {
            rules = rules.Where(r => p.IncludeRules.Contains(r.RuleId)).ToList();
        }
        else if (p.ExcludeRules is { Count: > 0 })
        {
            rules = rules.Where(r => !p.ExcludeRules.Contains(r.RuleId)).ToList();
        }

        var scanner = new AssemblyScanner(rules);
        var findings = scanner.Scan(p.Assembly).ToList();

        var fileBytes = File.ReadAllBytes(p.Assembly);
        var fileName = Path.GetFileName(p.Assembly);

        var options = ScanResultOptions.ForDesktop(developerMode: false);
        options.ScanMode = "scan";
        options.PlatformVersion = "worker-1.0.0";

        var dto = ScanResultMapper.ToDto(findings, fileName, fileBytes, options);

        var filteredFindings = dto.Findings.AsEnumerable();
        if (p.TypeFilter != null)
        {
            filteredFindings = filteredFindings.Where(f =>
                f.Location.Contains(p.TypeFilter, StringComparison.OrdinalIgnoreCase));
        }
        if (p.NamespaceFilter != null)
        {
            filteredFindings = filteredFindings.Where(f =>
                f.Location.Contains(p.NamespaceFilter, StringComparison.OrdinalIgnoreCase));
        }
        if (p.MethodFilter != null)
        {
            filteredFindings = filteredFindings.Where(f =>
                f.Location.Contains(p.MethodFilter, StringComparison.OrdinalIgnoreCase));
        }

        var filteredList = filteredFindings.ToList();

        return new ScanPayload
        {
            AssemblyPath = p.Assembly,
            SchemaVersion = dto.SchemaVersion,
            Metadata = new ScanMetaEntry
            {
                ScannerVersion = dto.Metadata.ScannerVersion,
                Timestamp = dto.Metadata.Timestamp,
                ScanMode = dto.Metadata.ScanMode,
                Platform = dto.Metadata.Platform,
            },
            Input = new ScanInputEntry
            {
                FileName = dto.Input.FileName,
                SizeBytes = (long)dto.Input.SizeBytes,
                Sha256Hash = dto.Input.Sha256Hash,
            },
            Summary = new ScanSummaryEntry
            {
                TotalFindings = filteredList.Count,
                CountBySeverity = filteredList
                    .GroupBy(f => f.Severity)
                    .ToDictionary(g => g.Key, g => g.Count()),
                TriggeredRules = filteredList
                    .Where(f => f.RuleId != null)
                    .Select(f => f.RuleId!)
                    .Distinct()
                    .OrderBy(x => x)
                    .ToList(),
            },
            Findings = filteredList.Select(MapFinding).ToList(),
            CallChains = dto.CallChains?.Select(MapCallChain).ToList(),
            DataFlows = dto.DataFlows?.Select(MapDataFlow).ToList(),
        };
    }

    public DecompilePayload Decompile(DecompileParams p)
    {
        var decompiler = CreateDecompiler(p.Assembly);
        var requestedTypeName = NormalizeRequestedName(p.TypeName);
        var requestedMethodName = NormalizeRequestedName(p.MethodName);
        string source;

        try
        {
            if (requestedTypeName != null)
            {
                // Mono.Cecil uses '/' for nested types; ICSharpCode.Decompiler
                // uses '+' in its reflection-style names (same as System.Reflection).
                var reflectionName = requestedTypeName.Replace('/', '+');
                var fullName = new FullTypeName(reflectionName);

                // If the requested type is a compiler-generated type (async state
                // machine, iterator, closure), redirect to the logical method in
                // the parent type so the user sees clean async/yield code instead
                // of raw MoveNext IL.
                if (IsCompilerGeneratedTypeName(GetSimpleTypeName(requestedTypeName)))
                {
                    source = DecompileCompilerGeneratedType(p.Assembly, requestedTypeName, decompiler)
                        ?? decompiler.DecompileTypeAsString(fullName);
                }
                else if (requestedMethodName != null)
                {
                    // FindType handles both top-level and nested types when
                    // given a reflection-style name (dots for namespaces, + for
                    // nested classes).
                    var typeDef = decompiler.TypeSystem
                        .FindType(fullName)
                        .GetDefinition();

                    if (typeDef != null)
                    {
                        var method = typeDef.Methods
                            .FirstOrDefault(m => m.Name == requestedMethodName);

                        source = method != null
                            ? decompiler.DecompileAsString(method.MetadataToken)
                            : decompiler.DecompileTypeAsString(fullName);
                    }
                    else
                    {
                        // FindType returned null — decompile the whole type.
                        source = decompiler.DecompileTypeAsString(fullName);
                    }
                }
                else
                {
                    source = decompiler.DecompileTypeAsString(fullName);
                }
            }
            else
            {
                source = decompiler.DecompileWholeModuleAsString();
            }
        }
        catch (Exception ex)
        {
            source = requestedTypeName == null
                ? BuildTypeWiseReconstruction(
                    EnumerateReconstructableTypeNames(_cache.Load(p.Assembly)),
                    typeName => decompiler.DecompileTypeAsString(new FullTypeName(typeName)),
                    ex)
                : $"// Decompilation error:\n// {ex.Message}";
        }

        return new DecompilePayload
        {
            AssemblyPath = p.Assembly,
            TypeName = requestedTypeName,
            MethodName = requestedMethodName,
            CsharpSource = source,
        };
    }

    /// <summary>
    /// Builds a <see cref="CSharpDecompiler"/> that uses a
    /// <see cref="UniversalAssemblyResolver"/> seeded with the assembly's own
    /// directory. This lets the decompiler find sibling DLLs, framework
    /// reference assemblies, and NuGet packages automatically instead of
    /// failing on unresolved type references.
    /// </summary>
    private static CSharpDecompiler CreateDecompiler(string assemblyPath)
    {
        var settings = new DecompilerSettings
        {
            ThrowOnAssemblyResolveErrors = false,
            // Never strip code — for security analysis we want to see everything.
            RemoveDeadCode = false,
            RemoveDeadStores = false,
        };

        var resolver = new UniversalAssemblyResolver(
            assemblyPath,
            throwOnError: false,
            targetFramework: null);

        // Search the assembly's own directory first (catches sibling DLLs,
        // plugins shipped alongside the target, etc.).
        var dir = Path.GetDirectoryName(assemblyPath);
        if (!string.IsNullOrEmpty(dir))
            resolver.AddSearchDirectory(dir);

        return new CSharpDecompiler(assemblyPath, resolver, settings);
    }

    // Matches compiler-generated type simple names: <MethodName>d__0 (async),
    // <MethodName>c__Iterator0 (iterator), <MethodName>b__0 (lambda),
    // <MethodName>c__DisplayClass0 (closure), etc.
    private static readonly Regex s_compilerGeneratedPattern =
        new(@"^<[^>]*>", RegexOptions.Compiled);

    /// <summary>Returns true when the simple type name (not namespace-qualified)
    /// looks like a compiler-generated type.</summary>
    internal static bool IsCompilerGeneratedTypeName(string simpleName) =>
        s_compilerGeneratedPattern.IsMatch(simpleName);

    /// <summary>Returns true when the type has a compiler-generated name AND
    /// carries <c>[CompilerGenerated]</c>, making it definitively synthetic.</summary>
    internal static bool IsCompilerGeneratedType(TypeDefinition type) =>
        IsCompilerGeneratedTypeName(type.Name) &&
        type.CustomAttributes.Any(a => a.AttributeType.Name == "CompilerGeneratedAttribute");

    /// <summary>Extracts the simple (unqualified) type name from a Mono.Cecil
    /// full name, which uses '/' for nested types and '.' for namespaces.</summary>
    internal static string GetSimpleTypeName(string monoCecilFullName)
    {
        var slashIdx = monoCecilFullName.LastIndexOf('/');
        if (slashIdx >= 0)
            return monoCecilFullName[(slashIdx + 1)..];
        var dotIdx = monoCecilFullName.LastIndexOf('.');
        return dotIdx >= 0 ? monoCecilFullName[(dotIdx + 1)..] : monoCecilFullName;
    }

    /// <summary>
    /// Finds the user-written method that owns a compiler-generated state machine
    /// type by inspecting <c>[AsyncStateMachine]</c> and
    /// <c>[IteratorStateMachine]</c> attributes on the declaring type's methods.
    /// </summary>
    internal static MethodDefinition? FindStateMachineOwner(TypeDefinition stateMachineType)
    {
        var declaringType = stateMachineType.DeclaringType;
        if (declaringType == null)
            return null;

        foreach (var method in declaringType.Methods)
        {
            foreach (var attr in method.CustomAttributes)
            {
                var name = attr.AttributeType.Name;
                if ((name == "AsyncStateMachineAttribute" || name == "IteratorStateMachineAttribute") &&
                    attr.ConstructorArguments.Count == 1 &&
                    attr.ConstructorArguments[0].Value is TypeReference tr &&
                    tr.FullName == stateMachineType.FullName)
                {
                    return method;
                }
            }
        }

        return null;
    }

    /// <summary>
    /// Decompiles a compiler-generated type by redirecting to its logical
    /// source. For state machines this produces clean async/await or yield
    /// return code instead of the raw <c>MoveNext</c> implementation.
    /// Returns <c>null</c> if the owner cannot be determined.
    /// </summary>
    private string? DecompileCompilerGeneratedType(
        string assemblyPath,
        string monoCecilTypeName,
        CSharpDecompiler decompiler)
    {
        var cecilAssembly = _cache.Load(assemblyPath);

        TypeDefinition? smType = null;
        foreach (var module in cecilAssembly.Modules)
        {
            foreach (var type in EnumerateTypes(module))
            {
                if (type.FullName == monoCecilTypeName)
                {
                    smType = type;
                    break;
                }
            }
            if (smType != null) break;
        }

        if (smType == null)
            return null;

        // Try the explicit attribute route first — most reliable.
        var ownerMethod = FindStateMachineOwner(smType);
        if (ownerMethod != null)
        {
            var ownerReflName = ownerMethod.DeclaringType.FullName.Replace('/', '+');
            var ownerFullName = new FullTypeName(ownerReflName);
            var icTypeDef = decompiler.TypeSystem.FindType(ownerFullName).GetDefinition();

            if (icTypeDef != null)
            {
                // Match by name; for overloads we take the first match which is
                // what the user most likely clicked.
                var icMethod = icTypeDef.Methods.FirstOrDefault(m => m.Name == ownerMethod.Name);
                if (icMethod != null)
                    return decompiler.DecompileAsString(icMethod.MetadataToken);
            }

            // Fell through — decompile the whole parent type (still better than
            // showing raw MoveNext).
            return decompiler.DecompileTypeAsString(ownerFullName);
        }

        // No attribute found — fall back to decompiling the declaring type.
        var declaringType = smType.DeclaringType;
        if (declaringType != null)
        {
            var reflName = declaringType.FullName.Replace('/', '+');
            return decompiler.DecompileTypeAsString(new FullTypeName(reflName));
        }

        return null;
    }

    internal static string BuildTypeWiseReconstruction(
        IEnumerable<string> typeNames,
        Func<string, string> decompileType,
        Exception? moduleError = null)
    {
        var blocks = new List<string>();

        if (moduleError != null)
        {
            blocks.Add($"// Full-module decompilation failed: {moduleError.Message}");
        }

        foreach (var typeName in typeNames)
        {
            try
            {
                blocks.Add(decompileType(typeName));
            }
            catch (Exception ex)
            {
                blocks.Add($"// Failed to decompile {typeName}: {ex.Message}");
            }
        }

        if (blocks.Count == 0)
        {
            blocks.Add("// Decompilation error: no reconstructable types found.");
        }

        return string.Join("\n\n", blocks);
    }

    public List<RuleEntry> ListRules()
    {
        return RuleFactory.CreateDefaultRules()
            .Select(r => new RuleEntry
            {
                RuleId = r.RuleId,
                Description = r.Description,
                Severity = r.Severity.ToString(),
            })
            .ToList();
    }

    private static IEnumerable<TypeDefinition> EnumerateTypes(ModuleDefinition module)
    {
        foreach (var type in module.Types)
        {
            yield return type;
            foreach (var nested in EnumerateNested(type))
            {
                yield return nested;
            }
        }
    }

    private static IEnumerable<TypeDefinition> EnumerateNested(TypeDefinition parent)
    {
        foreach (var nested in parent.NestedTypes)
        {
            yield return nested;
            foreach (var child in EnumerateNested(nested))
            {
                yield return child;
            }
        }
    }

    private static bool MatchesTypeFilter(TypeDefinition type, string? type_filter, string? ns_filter)
    {
        if (ns_filter != null && !MatchesGlob(type.Namespace, ns_filter))
        {
            return false;
        }
        if (type_filter != null && !MatchesGlob(type.Name, type_filter))
        {
            return false;
        }
        return true;
    }

    private static bool MatchesGlob(string text, string pattern)
    {
        if (pattern == "*")
        {
            return true;
        }
        if (pattern.StartsWith("*") && pattern.EndsWith("*"))
        {
            return text.Contains(pattern.Trim('*'), StringComparison.OrdinalIgnoreCase);
        }
        if (pattern.StartsWith("*"))
        {
            return text.EndsWith(pattern.TrimStart('*'), StringComparison.OrdinalIgnoreCase);
        }
        if (pattern.EndsWith("*"))
        {
            return text.StartsWith(pattern.TrimEnd('*'), StringComparison.OrdinalIgnoreCase);
        }
        return text.Equals(pattern, StringComparison.OrdinalIgnoreCase);
    }

    private static string FormatSignature(MethodDefinition method)
    {
        var visibility = method.IsPublic ? "public " : method.IsPrivate ? "private " : "protected ";
        var is_static = method.IsStatic ? "static " : "";
        var return_type = method.ReturnType.Name;
        var parameters = string.Join(
            ", ",
            method.Parameters.Select(p => $"{p.ParameterType.Name} {p.Name}")
        );
        return $"{visibility}{is_static}{return_type} {method.Name}({parameters})";
    }

    private static string? FormatOperand(object? operand) => operand switch
    {
        null => null,
        string s => $"\"{s}\"",
        MethodReference m => $"{m.DeclaringType?.Name}.{m.Name}",
        TypeReference t => t.FullName,
        FieldReference f => $"{f.DeclaringType?.Name}.{f.Name}",
        Instruction i => $"IL_{i.Offset:X4}",
        _ => operand.ToString(),
    };

    private static FindingEntry MapFinding(FindingDto f) => new()
    {
        Id = f.Id,
        RuleId = f.RuleId,
        Severity = f.Severity,
        Location = f.Location,
        Description = f.Description,
        CodeSnippet = f.CodeSnippet,
        CallChain = f.CallChain != null ? MapCallChain(f.CallChain) : null,
        DataFlowChain = f.DataFlowChain != null ? MapDataFlow(f.DataFlowChain) : null,
    };

    private static CallChainEntry MapCallChain(CallChainDto c) => new()
    {
        Id = c.Id ?? "",
        RuleId = c.RuleId ?? "",
        Description = c.Description ?? "",
        Severity = c.Severity ?? "",
        Nodes = c.Nodes?.Select(n => new CallChainNodeEntry
        {
            NodeType = n.NodeType,
            Location = n.Location,
            Description = n.Description,
            CodeSnippet = n.CodeSnippet,
        }).ToList() ?? new(),
    };

    private static DataFlowChainEntry MapDataFlow(DataFlowChainDto d) => new()
    {
        Id = d.Id ?? "",
        Description = d.Description ?? "",
        Severity = d.Severity ?? "",
        Pattern = d.Pattern ?? "",
        Confidence = d.Confidence,
        SourceVariable = d.SourceVariable,
        MethodLocation = d.MethodLocation ?? "",
        IsCrossMethod = d.IsCrossMethod,
        InvolvedMethods = d.InvolvedMethods,
        Nodes = d.Nodes?.Select(n => new DataFlowNodeEntry
        {
            NodeType = n.NodeType,
            Location = n.Location,
            Operation = n.Operation,
            DataDescription = n.DataDescription,
            InstructionOffset = n.InstructionOffset,
            MethodKey = n.MethodKey,
            IsMethodBoundary = n.IsMethodBoundary,
            TargetMethodKey = n.TargetMethodKey,
            CodeSnippet = n.CodeSnippet,
        }).ToList() ?? new(),
    };

    private static string? NormalizeRequestedName(string? name)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            return null;
        }

        return name.Trim();
    }

    private static IEnumerable<string> EnumerateReconstructableTypeNames(AssemblyDefinition assembly)
    {
        foreach (var module in assembly.Modules)
        {
            foreach (var type in EnumerateTypes(module))
            {
                // Compiler-generated types (<MethodName>d__N, closures, etc.) are
                // already inlined into their parent type's decompiled output as
                // proper async/await or yield return code. Decompiling them again
                // separately produces duplicate, confusing raw-MoveNext blocks.
                if (type.Name == "<Module>" || IsCompilerGeneratedType(type))
                {
                    continue;
                }

                yield return type.FullName.Replace('/', '+');
            }
        }
    }
}
