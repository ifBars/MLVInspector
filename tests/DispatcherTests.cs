using ILInspector.Worker;
using Mono.Cecil;
using System.Text;
using Xunit;

namespace MLVInspector.Tests;

public class DispatcherTests
{
    [Fact]
    public void BuildTypeWiseReconstruction_AggregatesSuccessfulAndFailedTypes()
    {
        var typeNames = new[] { "Ns.A", "Ns.B" };

        var result = Dispatcher.BuildTypeWiseReconstruction(
            typeNames,
            typeName => typeName == "Ns.B"
                ? throw new InvalidOperationException("boom")
                : $"class {typeName} {{ }}",
            new Exception("module broke"));

        Assert.Contains("Full-module decompilation failed: module broke", result);
        Assert.Contains("class Ns.A { }", result);
        Assert.Contains("Failed to decompile Ns.B: boom", result);
    }

    [Fact]
    public void BuildTypeWiseReconstruction_ReturnsNoTypesMessageWhenEmpty()
    {
        var result = Dispatcher.BuildTypeWiseReconstruction(Array.Empty<string>(), _ => "unused");

        Assert.Equal("// Decompilation error: no reconstructable types found.", result);
    }

    // -----------------------------------------------------------------------
    // IsCompilerGeneratedTypeName
    // -----------------------------------------------------------------------

    [Theory]
    // Async state machines
    [InlineData("<DownloadAsync>d__5", true)]
    [InlineData("<SendRequestAsync>d__12", true)]
    // Iterator state machines
    [InlineData("<GetItems>d__0", true)]
    [InlineData("<Enumerate>c__Iterator0", true)]
    // Closures / display classes
    [InlineData("<DoWork>c__DisplayClass3_0", true)]
    // Anonymous method containers
    [InlineData("<Main>b__0", true)]
    // Regular types — must NOT match
    [InlineData("MyClass", false)]
    [InlineData("d__5", false)]          // no angle brackets at start
    [InlineData("Foo<T>", false)]        // angle bracket not at position 0
    [InlineData("", false)]
    public void IsCompilerGeneratedTypeName_MatchesCompilerPatterns(string name, bool expected)
    {
        Assert.Equal(expected, Dispatcher.IsCompilerGeneratedTypeName(name));
    }

    // -----------------------------------------------------------------------
    // GetSimpleTypeName
    // -----------------------------------------------------------------------

    [Theory]
    // Nested type: last slash-delimited segment
    [InlineData("MyNs.MyClass/<DownloadAsync>d__5", "<DownloadAsync>d__5")]
    [InlineData("A/B/C", "C")]
    // Top-level type: last dot-delimited segment
    [InlineData("MyNs.MyClass", "MyClass")]
    [InlineData("MyClass", "MyClass")]
    // Mixed (nested type whose name contains dots is unusual but handled)
    [InlineData("Ns.Outer/Inner", "Inner")]
    public void GetSimpleTypeName_ExtractsLastSegment(string fullName, string expected)
    {
        Assert.Equal(expected, Dispatcher.GetSimpleTypeName(fullName));
    }

    // -----------------------------------------------------------------------
    // FindStateMachineOwner — uses in-memory Mono.Cecil objects
    // -----------------------------------------------------------------------

    [Fact]
    public void FindStateMachineOwner_ReturnsNullForTopLevelType()
    {
        // A type with no declaring type has no owner to find.
        var module = Mono.Cecil.ModuleDefinition.CreateModule("Test", Mono.Cecil.ModuleKind.Dll);
        var smType = new Mono.Cecil.TypeDefinition("", "<DoStuff>d__0", Mono.Cecil.TypeAttributes.Class);
        module.Types.Add(smType);

        Assert.Null(Dispatcher.FindStateMachineOwner(smType));
    }

    [Fact]
    public void FindStateMachineOwner_FindsOwnerViaAsyncStateMachineAttribute()
    {
        var module = Mono.Cecil.ModuleDefinition.CreateModule("Test", Mono.Cecil.ModuleKind.Dll);

        // Parent type
        var parentType = new Mono.Cecil.TypeDefinition("Ns", "MyClass", Mono.Cecil.TypeAttributes.Class);
        module.Types.Add(parentType);

        // Compiler-generated state machine nested inside parent
        var smType = new Mono.Cecil.TypeDefinition("", "<DoStuffAsync>d__0", Mono.Cecil.TypeAttributes.Class);
        parentType.NestedTypes.Add(smType);

        // The logical async method on the parent
        var method = new Mono.Cecil.MethodDefinition(
            "DoStuffAsync",
            Mono.Cecil.MethodAttributes.Public,
            module.TypeSystem.Void);
        parentType.Methods.Add(method);

        // Attach [AsyncStateMachine(typeof(<DoStuffAsync>d__0))] to the method
        var attrType = new Mono.Cecil.TypeDefinition(
            "System.Runtime.CompilerServices", "AsyncStateMachineAttribute",
            Mono.Cecil.TypeAttributes.Class);
        var attrCtor = new Mono.Cecil.MethodDefinition(
            ".ctor",
            Mono.Cecil.MethodAttributes.Public,
            module.TypeSystem.Void);
        attrCtor.Parameters.Add(new Mono.Cecil.ParameterDefinition(
            new Mono.Cecil.TypeReference("System", "Type", module, module.TypeSystem.CoreLibrary)));
        attrType.Methods.Add(attrCtor);

        var attr = new Mono.Cecil.CustomAttribute(attrCtor);
        attr.ConstructorArguments.Add(new Mono.Cecil.CustomAttributeArgument(
            new Mono.Cecil.TypeReference("System", "Type", module, module.TypeSystem.CoreLibrary),
            smType));
        method.CustomAttributes.Add(attr);

        var owner = Dispatcher.FindStateMachineOwner(smType);

        Assert.NotNull(owner);
        Assert.Equal("DoStuffAsync", owner.Name);
    }

    [Fact]
    public void FindStateMachineOwner_ReturnsNullWhenNoAttributePresent()
    {
        var module = Mono.Cecil.ModuleDefinition.CreateModule("Test", Mono.Cecil.ModuleKind.Dll);

        var parentType = new Mono.Cecil.TypeDefinition("Ns", "MyClass", Mono.Cecil.TypeAttributes.Class);
        module.Types.Add(parentType);

        var smType = new Mono.Cecil.TypeDefinition("", "<DoStuffAsync>d__0", Mono.Cecil.TypeAttributes.Class);
        parentType.NestedTypes.Add(smType);

        // A method on the parent but with no [AsyncStateMachine] attribute
        var method = new Mono.Cecil.MethodDefinition("DoStuffAsync",
            Mono.Cecil.MethodAttributes.Public, module.TypeSystem.Void);
        parentType.Methods.Add(method);

        Assert.Null(Dispatcher.FindStateMachineOwner(smType));
    }

    [Fact]
    public void Decompile_ReturnsReadableProfileAndMethodSourceSpans()
    {
        using var cache = new AssemblyCache();
        var dispatcher = new Dispatcher(cache);
        var assemblyPath = typeof(DispatcherTests).Assembly.Location;

        var payload = dispatcher.Decompile(new DecompileParams
        {
            Assembly = assemblyPath,
            TypeName = "MLVInspector.Tests.DispatcherTests",
            MethodName = nameof(BuildTypeWiseReconstruction_ReturnsNoTypesMessageWhenEmpty),
            Profile = "readable",
        });

        Assert.Equal("readable", payload.Profile);
        Assert.NotEmpty(payload.CsharpSource);
        Assert.NotEmpty(payload.SourceSpans);
        Assert.All(payload.SourceSpans, span =>
        {
            Assert.Equal("MLVInspector.Tests.DispatcherTests", span.TypeName);
            Assert.Equal(nameof(BuildTypeWiseReconstruction_ReturnsNoTypesMessageWhenEmpty), span.MethodName);
            Assert.True(span.StartLine > 0);
            Assert.True(span.EndLine >= span.StartLine);
        });
    }

    [Fact]
    public void AnalyzeSymbol_ForMethod_ReturnsCallersAndCallees()
    {
        using var cache = new AssemblyCache();
        var dispatcher = new Dispatcher(cache);
        var assemblyPath = typeof(Dispatcher).Assembly.Location;

        var payload = dispatcher.AnalyzeSymbol(new AnalyzeSymbolParams
        {
            Assembly = assemblyPath,
            TypeName = "ILInspector.Worker.Dispatcher",
            MethodName = "Decompile",
            MaxDepth = 2,
        });

        Assert.Equal("ILInspector.Worker.Dispatcher", payload.TypeName);
        Assert.Equal("Decompile", payload.MethodName);
        Assert.Equal(2, payload.MaxDepth);
        Assert.Contains(payload.Callers, caller =>
            caller.TypeName == "ILInspector.Worker.WorkerProtocol" &&
            caller.MethodName == "DispatchAsync" &&
            caller.Depth == 1);
        Assert.Contains(payload.Callees, callee =>
            callee.TypeName == "ILInspector.Worker.Dispatcher" &&
            callee.MethodName == "ParseDecompileProfile" &&
            callee.Depth == 1);
        Assert.All(payload.Callers.Concat(payload.Callees), reference =>
            Assert.InRange(reference.Depth, 1, 2));
        Assert.Contains(payload.Evidence, evidence =>
            evidence.Category == "allocation" ||
            evidence.Category == "string" ||
            evidence.Category == "field-read");
    }

    [Fact]
    public void AnalyzeSymbol_ForResourceRead_ReturnsResourceReadEvidence()
    {
        using var cache = new AssemblyCache();
        var dispatcher = new Dispatcher(cache);
        var assemblyPath = typeof(DispatcherTests).Assembly.Location;

        var payload = dispatcher.AnalyzeSymbol(new AnalyzeSymbolParams
        {
            Assembly = assemblyPath,
            TypeName = "MLVInspector.Tests.DispatcherTests",
            MethodName = nameof(OpenOwnManifestResource),
        });

        Assert.Contains(payload.Evidence, evidence =>
            evidence.Category == "resource-read" &&
            evidence.Label == "Manifest resource access" &&
            evidence.MethodName == nameof(OpenOwnManifestResource) &&
            evidence.Operand is { } operand &&
            operand.Contains("GetManifestResourceStream", StringComparison.Ordinal));
    }

    private static object? OpenOwnManifestResource()
    {
        return typeof(DispatcherTests).Assembly.GetManifestResourceStream("missing");
    }

    [Fact]
    public void Explore_ReturnsTypeAndMethodMetadataTokens()
    {
        using var cache = new AssemblyCache();
        var dispatcher = new Dispatcher(cache);
        var assemblyPath = typeof(Dispatcher).Assembly.Location;

        var payload = dispatcher.Explore(new ExploreParams { Assembly = assemblyPath });

        var dispatcherType = Assert.Single(payload.Types, type =>
            type.TypeName == "ILInspector.Worker.Dispatcher");
        Assert.StartsWith("0x02", dispatcherType.MetadataToken);

        var decompileMethod = Assert.Single(payload.Methods, method =>
            method.TypeName == "ILInspector.Worker.Dispatcher" &&
            method.MethodName == "Decompile");
        Assert.StartsWith("0x06", decompileMethod.MetadataToken);
    }

    [Fact]
    public void Explore_ReturnsReadOnlyMetadataTableSummary()
    {
        using var cache = new AssemblyCache();
        var dispatcher = new Dispatcher(cache);
        var assemblyPath = typeof(Dispatcher).Assembly.Location;

        var payload = dispatcher.Explore(new ExploreParams { Assembly = assemblyPath });

        Assert.Contains(payload.AssemblyMetadata.MetadataTables, table =>
            table.Name == "TypeDef" &&
            table.TokenPrefix == "0x02" &&
            table.RowCount > 0);
        Assert.Contains(payload.AssemblyMetadata.MetadataTables, table =>
            table.Name == "MethodDef" &&
            table.TokenPrefix == "0x06" &&
            table.RowCount > 0);
        Assert.Contains(payload.AssemblyMetadata.MetadataTables, table =>
            table.Name == "AssemblyRef" &&
            table.TokenPrefix == "0x23" &&
            table.RowCount > 0);
    }

    [Fact]
    public void Explore_ReturnsFieldsPropertiesEventsNestedTypesAndAttributes()
    {
        using var cache = new AssemblyCache();
        var dispatcher = new Dispatcher(cache);
        var assemblyPath = typeof(DispatcherTests).Assembly.Location;

        var payload = dispatcher.Explore(new ExploreParams { Assembly = assemblyPath });

        var probeType = Assert.Single(payload.Types, type =>
            type.TypeName == "MLVInspector.Tests.DispatcherTests/MemberProbe");
        Assert.Contains(probeType.Fields, field =>
            field.Name == "Counter" &&
            field.Kind == "field" &&
            field.MetadataToken?.StartsWith("0x04", StringComparison.Ordinal) == true);
        Assert.Contains(probeType.Properties, property =>
            property.Name == "Name" &&
            property.Kind == "property" &&
            property.Signature.Contains("{get; set}", StringComparison.Ordinal));
        Assert.Contains(probeType.Events, eventEntry =>
            eventEntry.Name == "Changed" &&
            eventEntry.Kind == "event");
        Assert.Contains(probeType.NestedTypes, nested =>
            nested.Name == "MLVInspector.Tests.DispatcherTests/MemberProbe/Child" &&
            nested.Kind == "class" &&
            nested.MetadataToken?.StartsWith("0x02", StringComparison.Ordinal) == true);
        Assert.Contains(probeType.CustomAttributes, attribute =>
            attribute.AttributeType == "System.ObsoleteAttribute" &&
            attribute.Summary?.Contains("member probe", StringComparison.Ordinal) == true);
    }

    [Fact]
    public void AnalyzeSymbol_ForFieldMetadataToken_ReturnsMemberReadWriteUsage()
    {
        using var cache = new AssemblyCache();
        var dispatcher = new Dispatcher(cache);
        var assemblyPath = typeof(DispatcherTests).Assembly.Location;
        var explore = dispatcher.Explore(new ExploreParams { Assembly = assemblyPath });
        var probeType = Assert.Single(explore.Types, type =>
            type.TypeName == "MLVInspector.Tests.DispatcherTests/MemberProbe");
        var counterToken = Assert.Single(probeType.Fields, field => field.Name == "Counter")
            .MetadataToken;

        var payload = dispatcher.AnalyzeSymbol(new AnalyzeSymbolParams
        {
            Assembly = assemblyPath,
            TypeName = "MLVInspector.Tests.DispatcherTests/MemberProbe",
            MetadataToken = counterToken,
        });

        Assert.Equal("MLVInspector.Tests.DispatcherTests/MemberProbe", payload.TypeName);
        Assert.Null(payload.MethodName);
        Assert.Contains("Counter", payload.TargetSignature);
        Assert.Contains(payload.Callers, caller =>
            caller.TypeName == "MLVInspector.Tests.DispatcherTests/MemberProbe" &&
            caller.MethodName == nameof(MemberProbe.Bump));
        Assert.Contains(payload.Evidence, evidence =>
            evidence.Category == "field-read" &&
            evidence.MethodName == nameof(MemberProbe.Bump) &&
            evidence.Operand?.Contains("Counter", StringComparison.Ordinal) == true);
        Assert.Contains(payload.Evidence, evidence =>
            evidence.Category == "field-write" &&
            evidence.MethodName == nameof(MemberProbe.Bump) &&
            evidence.Operand?.Contains("Counter", StringComparison.Ordinal) == true);
    }

    [Obsolete("member probe")]
    private sealed class MemberProbe
    {
        public int Counter = 7;

        public string Name { get; set; } = "";

        public event EventHandler? Changed
        {
            add { }
            remove { }
        }

        public int Bump()
        {
            Counter++;
            return Counter;
        }

        public sealed class Child
        {
        }
    }

    [Fact]
    public void Explore_EmbeddedTextResource_ReturnsSafePreviewAndHash()
    {
        var assemblyPath = Path.Combine(Path.GetTempPath(), $"{Guid.NewGuid():N}.dll");
        try
        {
            using (var module = ModuleDefinition.CreateModule("ResourceProbe", ModuleKind.Dll))
            {
                module.Resources.Add(new EmbeddedResource(
                    "payload.txt",
                    ManifestResourceAttributes.Private,
                    Encoding.UTF8.GetBytes("powershell -nop")));
                module.Write(assemblyPath);
            }

            using var cache = new AssemblyCache();
            var dispatcher = new Dispatcher(cache);

            var payload = dispatcher.Explore(new ExploreParams { Assembly = assemblyPath });

            var resource = Assert.Single(payload.AssemblyMetadata.Resources);
            Assert.Equal("payload.txt", resource.Name);
            Assert.Equal("text", resource.PreviewKind);
            Assert.Equal("powershell -nop", resource.Preview);
            Assert.False(string.IsNullOrWhiteSpace(resource.Sha256Hash));
            Assert.StartsWith("0x", resource.MetadataToken);
            Assert.True(resource.SizeBytes > 0);
        }
        finally
        {
            if (File.Exists(assemblyPath))
                File.Delete(assemblyPath);
        }
    }
}
