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
}
