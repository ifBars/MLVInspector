using System.Text;
using System.Text.RegularExpressions;
using DecompilerSequencePoint = ICSharpCode.Decompiler.DebugInfo.SequencePoint;
using ICSharpCode.Decompiler;
using ICSharpCode.Decompiler.CSharp;
using ICSharpCode.Decompiler.CSharp.Syntax;
using ICSharpCode.Decompiler.Metadata;
using ICSharpCode.Decompiler.CSharp.OutputVisitor;
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

    private enum DecompileProfile
    {
        Readable,
        Analysis,
    }

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
            AssemblyMetadata = BuildAssemblyMetadata(assembly, p.Assembly),
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
        var profile = ParseDecompileProfile(p.Profile);
        var decompiler = CreateDecompiler(p.Assembly, profile);
        var requestedTypeName = NormalizeRequestedName(p.TypeName);
        var requestedMethodName = NormalizeRequestedName(p.MethodName);
        string source;
        var sourceSpans = new List<SourceSpanEntry>();
        var assembly = _cache.Load(p.Assembly);

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

                        if (method != null)
                        {
                            (source, sourceSpans) = DecompileMethodDocument(
                                decompiler,
                                method,
                                requestedTypeName,
                                requestedMethodName);
                        }
                        else
                        {
                            source = decompiler.DecompileTypeAsString(fullName);
                        }
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
            sourceSpans.Clear();
                source = requestedTypeName == null
                    ? BuildTypeWiseReconstruction(
                    EnumerateReconstructableTypeNames(assembly),
                    typeName => TryDecompileTypeWithFallback(assembly, decompiler, typeName),
                    ex)
                : BuildRequestedFallback(assembly, requestedTypeName, requestedMethodName, ex);
        }

        return new DecompilePayload
        {
            AssemblyPath = p.Assembly,
            TypeName = requestedTypeName,
            MethodName = requestedMethodName,
            CsharpSource = source,
            Profile = ToWireProfileName(profile),
            SourceSpans = sourceSpans,
        };
    }

    /// <summary>
    /// Builds a <see cref="CSharpDecompiler"/> that uses a
    /// <see cref="UniversalAssemblyResolver"/> seeded with the assembly's own
    /// directory. This lets the decompiler find sibling DLLs, framework
    /// reference assemblies, and NuGet packages automatically instead of
    /// failing on unresolved type references.
    /// </summary>
    private static CSharpDecompiler CreateDecompiler(string assemblyPath, DecompileProfile profile)
    {
        var settings = CreateDecompilerSettings(profile);

        var targetFramework = DetectTargetFrameworkWithFallback(assemblyPath);

        var resolver = new UniversalAssemblyResolver(
            assemblyPath,
            throwOnError: false,
            targetFramework: targetFramework);

        // Search the assembly's own directory first (catches sibling DLLs,
        // plugins shipped alongside the target, etc.).
        var dir = Path.GetDirectoryName(assemblyPath);
        if (!string.IsNullOrEmpty(dir))
            resolver.AddSearchDirectory(dir);

        // Add reference assembly paths for .NET Framework
        if (IsDotNetFrameworkTarget(targetFramework))
        {
            AddDotNetFrameworkReferencePaths(resolver);
        }

        return new CSharpDecompiler(assemblyPath, resolver, settings);
    }

    private static string? DetectTargetFramework(string assemblyPath)
    {
        try
        {
            using var assembly = AssemblyDefinition.ReadAssembly(assemblyPath);
            foreach (var attr in assembly.CustomAttributes)
            {
                if (attr.AttributeType.FullName == "System.Runtime.Versioning.TargetFrameworkAttribute")
                {
                    if (attr.ConstructorArguments.Count > 0 &&
                        attr.ConstructorArguments[0].Value is string frameworkMoniker)
                    {
                        return frameworkMoniker;
                    }
                }
            }
        }
        catch
        {
        }

        return null;
    }

    private static AssemblyMetadataEntry BuildAssemblyMetadata(AssemblyDefinition assembly, string assemblyPath)
    {
        var assemblyName = assembly.Name;
        var mainModule = assembly.MainModule;
        var targetFramework = DetectTargetFrameworkFromAssembly(assembly);

        return new AssemblyMetadataEntry
        {
            AssemblyName = assemblyName.Name ?? Path.GetFileNameWithoutExtension(assemblyPath),
            FullName = assemblyName.FullName,
            Version = assemblyName.Version?.ToString(),
            Culture = NullIfEmpty(assemblyName.Culture),
            PublicKeyToken = FormatPublicKeyToken(assemblyName.PublicKeyToken),
            TargetFramework = targetFramework,
            InferredTargetFramework = TryInferFromReferences(assemblyPath),
            RuntimeVersion = NullIfEmpty(mainModule.RuntimeVersion),
            Architecture = mainModule.Architecture.ToString(),
            ModuleKind = mainModule.Kind.ToString(),
            EntryPoint = assembly.EntryPoint?.FullName,
            Mvid = mainModule.Mvid.ToString(),
            Modules = assembly.Modules.Select(MapModule).ToList(),
            AssemblyReferences = assembly.Modules
                .SelectMany(module => module.AssemblyReferences)
                .GroupBy(reference => reference.FullName, StringComparer.Ordinal)
                .Select(group => MapAssemblyReference(group.First()))
                .ToList(),
            Resources = assembly.Modules
                .SelectMany(module => module.Resources)
                .GroupBy(BuildResourceIdentity, StringComparer.Ordinal)
                .Select(group => MapResource(group.First()))
                .ToList(),
            CustomAttributes = assembly.CustomAttributes.Select(MapAttribute).ToList(),
        };
    }

    private static ModuleMetadataEntry MapModule(ModuleDefinition module)
    {
        return new ModuleMetadataEntry
        {
            Name = module.Name,
            RuntimeVersion = NullIfEmpty(module.RuntimeVersion),
            Architecture = module.Architecture.ToString(),
            ModuleKind = module.Kind.ToString(),
            Mvid = module.Mvid.ToString(),
            FileName = NullIfEmpty(module.FileName),
        };
    }

    private static AssemblyReferenceEntry MapAssemblyReference(Mono.Cecil.AssemblyNameReference reference)
    {
        return new AssemblyReferenceEntry
        {
            Name = reference.Name,
            FullName = reference.FullName,
            Version = reference.Version?.ToString(),
            Culture = NullIfEmpty(reference.Culture),
            PublicKeyToken = FormatPublicKeyToken(reference.PublicKeyToken),
        };
    }

    private static ResourceMetadataEntry MapResource(Mono.Cecil.Resource resource)
    {
        // Avoid materializing embedded resource contents during Explore().
        long? sizeBytes = null;

        var implementation = resource switch
        {
            LinkedResource linked => linked.File,
            AssemblyLinkedResource linkedAssembly => linkedAssembly.Assembly?.FullName,
            _ => null,
        };

        return new ResourceMetadataEntry
        {
            Name = resource.Name,
            ResourceType = resource.ResourceType.ToString(),
            Attributes = resource.Attributes.ToString(),
            SizeBytes = sizeBytes,
            Implementation = NullIfEmpty(implementation),
        };
    }

    private static AttributeMetadataEntry MapAttribute(CustomAttribute attribute)
    {
        return new AttributeMetadataEntry
        {
            AttributeType = attribute.AttributeType.FullName,
            Summary = BuildAttributeSummary(attribute),
        };
    }

    private static string? BuildAttributeSummary(CustomAttribute attribute)
    {
        const int summaryBudget = 240;
        var builder = new StringBuilder(summaryBudget);
        var remainingBudget = summaryBudget;

        AppendAttributeSection(
            builder,
            ref remainingBudget,
            "ctor(",
            attribute.ConstructorArguments,
            (argument, budget) => FormatAttributeArgument(argument, budget));
        AppendAttributeSection(
            builder,
            ref remainingBudget,
            "props(",
            attribute.Properties,
            (property, budget) => FormatNamedAttributeValue(property.Name, property.Argument.Value, budget));
        AppendAttributeSection(
            builder,
            ref remainingBudget,
            "fields(",
            attribute.Fields,
            (field, budget) => FormatNamedAttributeValue(field.Name, field.Argument.Value, budget));

        if (builder.Length == 0)
            return null;

        var summary = builder.ToString();
        return summary.Length > 240 ? $"{summary[..237]}..." : summary;
    }

    private static string FormatAttributeArgument(CustomAttributeArgument argument)
    {
        return FormatAttributeArgument(argument, 80);
    }

    private static string FormatAttributeArgument(CustomAttributeArgument argument, int maxLength)
    {
        return FormatAttributeValue(argument.Value, maxLength);
    }

    private static string FormatAttributeValue(object? value)
    {
        return FormatAttributeValue(value, 80);
    }

    private static string FormatAttributeValue(object? value, int maxLength)
    {
        if (maxLength <= 0)
            return string.Empty;

        return value switch
        {
            null => "null",
            string s => QuoteAndTruncate(s, maxLength),
            CustomAttributeArgument nested => FormatAttributeArgument(nested, maxLength),
            CustomAttributeArgument[] array => FormatAttributeArgumentList(array, maxLength),
            IEnumerable<CustomAttributeArgument> enumerable => FormatAttributeArgumentList(enumerable, maxLength),
            TypeReference typeRef => typeRef.FullName,
            _ => value.ToString() ?? string.Empty,
        };
    }

    private static void AppendAttributeSection<T>(
        StringBuilder builder,
        ref int remainingBudget,
        string sectionPrefix,
        IEnumerable<T> items,
        Func<T, int, string> formatItem)
    {
        const string sectionSeparator = "; ";
        const string itemSeparator = ", ";
        const string sectionSuffix = ")";

        using var enumerator = items.GetEnumerator();
        if (!enumerator.MoveNext())
            return;

        var leadingSeparatorLength = builder.Length == 0 ? 0 : sectionSeparator.Length;
        var minimumItemBudget = remainingBudget - leadingSeparatorLength - sectionPrefix.Length - sectionSuffix.Length;
        if (minimumItemBudget <= 0)
            return;

        var initialLength = builder.Length;
        var initialBudget = remainingBudget;

        if (builder.Length > 0)
        {
            builder.Append(sectionSeparator);
            remainingBudget -= sectionSeparator.Length;
        }

        builder.Append(sectionPrefix);
        remainingBudget -= sectionPrefix.Length;

        var appendedAny = false;

        do
        {
            var currentSeparatorLength = appendedAny ? itemSeparator.Length : 0;
            var itemBudget = remainingBudget - currentSeparatorLength - sectionSuffix.Length;
            if (itemBudget <= 0)
                break;

            var itemText = TruncateWithEllipsis(formatItem(enumerator.Current, itemBudget), itemBudget);
            if (string.IsNullOrEmpty(itemText))
                break;

            if (appendedAny)
            {
                builder.Append(itemSeparator);
                remainingBudget -= itemSeparator.Length;
            }

            builder.Append(itemText);
            remainingBudget -= itemText.Length;
            appendedAny = true;
        }
        while (enumerator.MoveNext());

        if (!appendedAny)
        {
            builder.Length = initialLength;
            remainingBudget = initialBudget;
            return;
        }

        builder.Append(sectionSuffix);
        remainingBudget -= sectionSuffix.Length;
    }

    private static string FormatNamedAttributeValue(string name, object? value, int maxLength)
    {
        var prefix = $"{name}=";
        if (maxLength <= prefix.Length)
            return TruncateWithEllipsis(prefix, maxLength);

        return prefix + FormatAttributeValue(value, maxLength - prefix.Length);
    }

    private static string FormatAttributeArgumentList(IEnumerable<CustomAttributeArgument> arguments, int maxLength)
    {
        if (maxLength <= 0)
            return string.Empty;

        var builder = new StringBuilder(Math.Min(maxLength, 80));
        var remainingBudget = maxLength;

        builder.Append('[');
        remainingBudget--;

        var appendedAny = false;
        foreach (var argument in arguments)
        {
            var separatorLength = appendedAny ? 2 : 0;
            var itemBudget = remainingBudget - separatorLength - 1;
            if (itemBudget <= 0)
                break;

            var itemText = TruncateWithEllipsis(FormatAttributeArgument(argument, itemBudget), itemBudget);
            if (string.IsNullOrEmpty(itemText))
                break;

            if (appendedAny)
            {
                builder.Append(", ");
                remainingBudget -= 2;
            }

            builder.Append(itemText);
            remainingBudget -= itemText.Length;
            appendedAny = true;
        }

        if (remainingBudget > 0)
        {
            builder.Append(']');
            remainingBudget--;
        }

        return builder.ToString();
    }

    private static string QuoteAndTruncate(string value, int maxLength)
    {
        if (maxLength <= 0)
            return string.Empty;

        if (maxLength == 1)
            return "\"";

        return $"\"{TruncateWithEllipsis(value, maxLength - 2)}\"";
    }

    private static string TruncateWithEllipsis(string value, int maxLength)
    {
        if (maxLength <= 0)
            return string.Empty;

        if (value.Length <= maxLength)
            return value;

        if (maxLength <= 3)
            return value[..maxLength];

        return $"{value[..(maxLength - 3)]}...";
    }

    private static string BuildResourceIdentity(Mono.Cecil.Resource resource)
    {
        var implementation = resource switch
        {
            LinkedResource linked => linked.File,
            AssemblyLinkedResource linkedAssembly => linkedAssembly.Assembly?.FullName,
            _ => null,
        };

        return string.Join(
            "|",
            resource.Name,
            resource.ResourceType,
            NullIfEmpty(resource.Attributes.ToString()) ?? string.Empty,
            NullIfEmpty(implementation) ?? string.Empty);
    }

    private static string? FormatPublicKeyToken(byte[]? token)
    {
        if (token == null || token.Length == 0)
            return null;

        return string.Concat(token.Select(b => b.ToString("x2")));
    }

    private static string? DetectTargetFrameworkFromAssembly(AssemblyDefinition assembly)
    {
        foreach (var attr in assembly.CustomAttributes)
        {
            if (attr.AttributeType.FullName == "System.Runtime.Versioning.TargetFrameworkAttribute" &&
                attr.ConstructorArguments.Count > 0 &&
                attr.ConstructorArguments[0].Value is string frameworkMoniker)
            {
                return frameworkMoniker;
            }
        }

        return null;
    }

    private static string? NullIfEmpty(string? value)
    {
        return string.IsNullOrWhiteSpace(value) ? null : value;
    }

    private static string? DetectTargetFrameworkWithFallback(string assemblyPath)
    {
        var detected = DetectTargetFramework(assemblyPath);
        if (detected != null)
            return detected;

        return TryInferFromReferences(assemblyPath);
    }

    private static string? TryInferFromReferences(string assemblyPath)
    {
        var dir = Path.GetDirectoryName(assemblyPath);
        if (string.IsNullOrEmpty(dir))
            return null;

        var dlls = Directory.GetFiles(dir, "*.dll");
        foreach (var dll in dlls)
        {
            try
            {
                using var asm = AssemblyDefinition.ReadAssembly(dll);
                foreach (var attr in asm.CustomAttributes)
                {
                    if (attr.AttributeType.FullName == "System.Runtime.Versioning.TargetFrameworkAttribute")
                    {
                        if (attr.ConstructorArguments.Count > 0 &&
                            attr.ConstructorArguments[0].Value is string moniker)
                        {
                            return moniker;
                        }
                    }
                }
            }
            catch
            {
            }
        }

        return "netstandard2.1";
    }

    private static bool IsDotNetFrameworkTarget(string? moniker)
    {
        return !string.IsNullOrEmpty(moniker) &&
            moniker.StartsWith(".NETFramework,Version=v", StringComparison.Ordinal);
    }

    private static void AddDotNetFrameworkReferencePaths(UniversalAssemblyResolver resolver)
    {
        var programFilesX86 = Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86);
        var windows = Environment.GetFolderPath(Environment.SpecialFolder.Windows);

        // Reference Assemblies (preferred)
        var refAssembliesBase = Path.Combine(programFilesX86, "Reference Assemblies", "Microsoft", "Framework", ".NETFramework");
        if (Directory.Exists(refAssembliesBase))
        {
            // Look for v4.x folders
            foreach (var dir in Directory.GetDirectories(refAssembliesBase, "v4.*"))
            {
                resolver.AddSearchDirectory(dir);
            }
        }

        // GAC paths
        var gacPaths = new[]
        {
            Path.Combine(windows, "assembly", "GAC_MSIL"),
            Path.Combine(windows, "assembly", "GAC_32"),
            Path.Combine(windows, "assembly", "GAC_64"),
            Path.Combine(windows, "Microsoft.NET", "assembly", "GAC_MSIL"),
            Path.Combine(windows, "Microsoft.NET", "assembly", "GAC_32"),
            Path.Combine(windows, "Microsoft.NET", "assembly", "GAC_64"),
        };

        foreach (var gacPath in gacPaths)
        {
            if (Directory.Exists(gacPath))
            {
                resolver.AddSearchDirectory(gacPath);
            }
        }

        // .NET Framework runtime assemblies
        var frameworkPaths = new[]
        {
            Path.Combine(windows, "Microsoft.NET", "Framework", "v4.0.30319"),
            Path.Combine(windows, "Microsoft.NET", "Framework64", "v4.0.30319"),
        };

        foreach (var fwPath in frameworkPaths)
        {
            if (Directory.Exists(fwPath))
            {
                resolver.AddSearchDirectory(fwPath);
            }
        }
    }

    private static DecompilerSettings CreateDecompilerSettings(DecompileProfile profile)
    {
        var settings = new DecompilerSettings(LanguageVersion.Latest)
        {
            ThrowOnAssemblyResolveErrors = false,
            AsyncAwait = true,
            YieldReturn = true,
            LocalFunctions = true,
            PatternMatching = true,
            FileScopedNamespaces = true,
            UseEnhancedUsing = true,
        };

        if (profile == DecompileProfile.Readable)
        {
            settings.RemoveDeadCode = true;
            settings.RemoveDeadStores = true;
            settings.AggressiveInlining = true;
        }
        else
        {
            settings.RemoveDeadCode = false;
            settings.RemoveDeadStores = false;
        }

        return settings;
    }

    private static DecompileProfile ParseDecompileProfile(string? value)
    {
        return string.Equals(value, "analysis", StringComparison.OrdinalIgnoreCase)
            ? DecompileProfile.Analysis
            : DecompileProfile.Readable;
    }

    private static string ToWireProfileName(DecompileProfile profile)
    {
        return profile == DecompileProfile.Analysis ? "analysis" : "readable";
    }

    private static (string Source, List<SourceSpanEntry> SourceSpans) DecompileMethodDocument(
        CSharpDecompiler decompiler,
        IMethod method,
        string typeName,
        string methodName)
    {
        var syntaxTree = decompiler.Decompile(method.MetadataToken);
        var source = RenderSyntaxTreeWithLocations(syntaxTree);
        var sourceSpans = BuildSourceSpans(
            decompiler
                .CreateSequencePoints(syntaxTree)
                .Values
                .SelectMany(points => points),
            typeName,
            methodName);

        return (source, sourceSpans);
    }

    private static string RenderSyntaxTreeWithLocations(SyntaxTree syntaxTree)
    {
        using var writer = new StringWriter();
        var formatting = FormattingOptionsFactory.CreateAllman();
        var tokenWriter = TokenWriter.CreateWriterThatSetsLocationsInAST(writer);
        syntaxTree.AcceptVisitor(new CSharpOutputVisitor(tokenWriter, formatting));
        return writer.ToString();
    }

    internal static List<SourceSpanEntry> BuildSourceSpans(
        IEnumerable<DecompilerSequencePoint> sequencePoints,
        string? typeName,
        string? methodName)
    {
        return sequencePoints
            .Where(point =>
                !point.IsHidden &&
                point.StartLine > 0 &&
                point.EndLine >= point.StartLine)
            .Select(point => new SourceSpanEntry
            {
                TypeName = typeName,
                MethodName = methodName,
                IlStartOffset = point.Offset,
                IlEndOffset = point.EndOffset,
                StartLine = point.StartLine,
                EndLine = point.EndLine,
            })
            .ToList();
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

    private static string TryDecompileTypeWithFallback(
        AssemblyDefinition assembly,
        CSharpDecompiler decompiler,
        string reflectionTypeName)
    {
        try
        {
            return decompiler.DecompileTypeAsString(new FullTypeName(reflectionTypeName));
        }
        catch (Exception ex)
        {
            var type = FindTypeDefinition(assembly, reflectionTypeName);
            return type != null
                ? RenderTypeFallback(type, null, ex)
                : $"// Failed to decompile {reflectionTypeName}: {ex.Message}";
        }
    }

    private static string BuildRequestedFallback(
        AssemblyDefinition assembly,
        string requestedTypeName,
        string? requestedMethodName,
        Exception error)
    {
        var type = FindTypeDefinition(assembly, requestedTypeName);
        if (type == null)
        {
            return $"// Decompilation error:\n// {error.Message}";
        }

        return RenderTypeFallback(type, requestedMethodName, error);
    }

    private static TypeDefinition? FindTypeDefinition(AssemblyDefinition assembly, string typeName)
    {
        foreach (var module in assembly.Modules)
        {
            foreach (var type in EnumerateTypes(module))
            {
                if (string.Equals(type.FullName, typeName, StringComparison.Ordinal) ||
                    string.Equals(type.FullName.Replace('/', '+'), typeName, StringComparison.Ordinal))
                {
                    return type;
                }
            }
        }

        return null;
    }

    private static string RenderTypeFallback(
        TypeDefinition type,
        string? requestedMethodName,
        Exception error)
    {
        var lines = new List<string>
        {
            $"// Decompiler fallback for {type.FullName}",
            $"// Original decompilation failed: {error.Message}",
            $"// Namespace: {type.Namespace}",
            $"// Type: {type.Name}",
            ""
        };

        IEnumerable<MethodDefinition> methods = requestedMethodName == null
            ? type.Methods
            : type.Methods.Where(method => string.Equals(method.Name, requestedMethodName, StringComparison.Ordinal)).ToList();

        foreach (var method in methods)
        {
            lines.Add($"// Method: {FormatSignature(method)}");

            if (!method.HasBody)
            {
                lines.Add("// <no body>");
                lines.Add(string.Empty);
                continue;
            }

            foreach (var instruction in method.Body.Instructions)
            {
                lines.Add($"// IL_{instruction.Offset:X4}: {instruction.OpCode} {FormatOperand(instruction.Operand)}".TrimEnd());
            }

            lines.Add(string.Empty);
        }

        if (requestedMethodName != null && !methods.Any())
        {
            lines.Add($"// Method not found: {requestedMethodName}");
        }

        return string.Join("\n", lines);
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
