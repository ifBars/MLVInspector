using ILInspector.Worker;

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
}
