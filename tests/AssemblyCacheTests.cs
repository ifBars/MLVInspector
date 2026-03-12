using ILInspector.Worker;
using Xunit;

namespace MLVInspector.Tests;

public class AssemblyCacheTests
{
    [Fact]
    public void Load_ReturnsSameInstanceForSamePathUntilEvicted()
    {
        using var cache = new AssemblyCache();
        var assemblyPath = typeof(AssemblyCacheTests).Assembly.Location;

        var first = cache.Load(assemblyPath);
        var second = cache.Load(assemblyPath);

        Assert.Same(first, second);
    }

    [Fact]
    public void Evict_RemovesCachedAssemblySoNextLoadCreatesNewInstance()
    {
        using var cache = new AssemblyCache();
        var assemblyPath = typeof(AssemblyCacheTests).Assembly.Location;

        var first = cache.Load(assemblyPath);
        cache.Evict(assemblyPath);
        var second = cache.Load(assemblyPath);

        Assert.NotSame(first, second);
    }
}
