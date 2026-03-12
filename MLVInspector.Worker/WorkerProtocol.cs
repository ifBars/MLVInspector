using System.Text.Json;
using System.Text.Json.Serialization.Metadata;

namespace ILInspector.Worker;

internal static class WorkerProtocol
{
    public static async Task DispatchAsync(WorkerRequest req, Dispatcher dispatcher, StreamWriter stdout)
    {
        try
        {
            switch (req.Method)
            {
                case "explore":
                {
                    var p = req.Params.Deserialize(WorkerJsonContext.Default.ExploreParams)
                        ?? throw new ArgumentException("missing explore params");
                    var payload = dispatcher.Explore(p);
                    await WriteOkAsync(stdout, req.Id, payload, WorkerJsonContext.Default.ExplorePayload);
                    break;
                }

                case "scan":
                {
                    var p = req.Params.Deserialize(WorkerJsonContext.Default.ScanParams)
                        ?? throw new ArgumentException("missing scan params");
                    var payload = dispatcher.Scan(p);
                    await WriteOkAsync(stdout, req.Id, payload, WorkerJsonContext.Default.ScanPayload);
                    break;
                }

                case "decompile":
                {
                    var p = req.Params.Deserialize(WorkerJsonContext.Default.DecompileParams)
                        ?? throw new ArgumentException("missing decompile params");
                    var payload = dispatcher.Decompile(p);
                    await WriteOkAsync(stdout, req.Id, payload, WorkerJsonContext.Default.DecompilePayload);
                    break;
                }

                case "list-rules":
                {
                    var payload = dispatcher.ListRules();
                    await WriteOkAsync(stdout, req.Id, payload, WorkerJsonContext.Default.ListRuleEntry);
                    break;
                }

                default:
                    await WriteErrorAsync(stdout, req.Id, $"unknown method: {req.Method}");
                    break;
            }
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"[worker] error handling {req.Method}: {ex}");
            await WriteErrorAsync(stdout, req.Id, ex.Message);
        }
    }

    public static async Task WriteOkAsync<T>(
        StreamWriter stdout,
        ulong id,
        T payload,
        JsonTypeInfo<T> typeInfo)
    {
        using var ms = new MemoryStream();
        using var wtr = new Utf8JsonWriter(ms);
        wtr.WriteStartObject();
        wtr.WriteNumber("id", id);
        wtr.WriteBoolean("ok", true);
        wtr.WritePropertyName("payload");
        JsonSerializer.Serialize(wtr, payload, typeInfo);
        wtr.WriteNull("error");
        wtr.WriteEndObject();
        wtr.Flush();
        await stdout.WriteLineAsync(System.Text.Encoding.UTF8.GetString(ms.ToArray()));
    }

    public static async Task WriteErrorAsync(StreamWriter stdout, ulong id, string message)
    {
        using var ms = new MemoryStream();
        using var wtr = new Utf8JsonWriter(ms);
        wtr.WriteStartObject();
        wtr.WriteNumber("id", id);
        wtr.WriteBoolean("ok", false);
        wtr.WriteNull("payload");
        wtr.WriteString("error", message);
        wtr.WriteEndObject();
        wtr.Flush();
        await stdout.WriteLineAsync(System.Text.Encoding.UTF8.GetString(ms.ToArray()));
    }
}
