using System.Text.Json;
using System.Text.Json.Serialization.Metadata;
using ILInspector.Worker;

Console.Error.WriteLine("[worker] ILInspector.Worker started");

using var cache = new AssemblyCache();
var dispatcher = new Dispatcher(cache);

var utf8NoBom = new System.Text.UTF8Encoding(encoderShouldEmitUTF8Identifier: false);
using var stdin = new StreamReader(Console.OpenStandardInput(), utf8NoBom);
using var stdout = new StreamWriter(Console.OpenStandardOutput(), utf8NoBom)
{
    AutoFlush = true,
};

while (true)
{
    string? line;
    try
    {
        line = await stdin.ReadLineAsync();
    }
    catch
    {
        break;
    }

    if (line is null)
    {
        break;
    }

    if (string.IsNullOrWhiteSpace(line))
    {
        continue;
    }

    WorkerRequest? req = null;
    try
    {
        req = JsonSerializer.Deserialize(line, WorkerJsonContext.Default.WorkerRequest);
    }
    catch (Exception ex)
    {
        await WriteErrorAsync(stdout, 0, $"malformed request: {ex.Message}");
        continue;
    }

    if (req is null)
    {
        await WriteErrorAsync(stdout, 0, "null request");
        continue;
    }

    if (req.Method == "shutdown")
    {
        break;
    }

    await DispatchAsync(req, dispatcher, stdout);
}

Console.Error.WriteLine("[worker] exiting");
return 0;

static async Task DispatchAsync(WorkerRequest req, Dispatcher dispatcher, StreamWriter stdout)
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

static async Task WriteOkAsync<T>(StreamWriter stdout, ulong id, T payload, JsonTypeInfo<T> typeInfo)
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

static async Task WriteErrorAsync(StreamWriter stdout, ulong id, string message)
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
