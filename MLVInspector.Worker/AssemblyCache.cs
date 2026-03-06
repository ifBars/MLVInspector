using Mono.Cecil;

namespace ILInspector.Worker;

internal sealed class AssemblyCache : IDisposable
{
    private readonly Dictionary<string, AssemblyDefinition> _cache =
        new(StringComparer.OrdinalIgnoreCase);
    private readonly object _lock = new();

    public AssemblyDefinition Load(string path)
    {
        lock (_lock)
        {
            if (_cache.TryGetValue(path, out var cached))
            {
                return cached;
            }

            var def = AssemblyDefinition.ReadAssembly(path);
            _cache[path] = def;
            return def;
        }
    }

    public void Evict(string path)
    {
        lock (_lock)
        {
            if (_cache.TryGetValue(path, out var def))
            {
                def.Dispose();
                _cache.Remove(path);
            }
        }
    }

    public void Dispose()
    {
        lock (_lock)
        {
            foreach (var def in _cache.Values)
            {
                def.Dispose();
            }

            _cache.Clear();
        }
    }
}
