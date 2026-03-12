using System.Text.Json;
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
        await ILInspector.Worker.WorkerProtocol.WriteErrorAsync(stdout, 0, $"malformed request: {ex.Message}");
        continue;
    }

    if (req is null)
    {
        await ILInspector.Worker.WorkerProtocol.WriteErrorAsync(stdout, 0, "null request");
        continue;
    }

    if (req.Method == "shutdown")
    {
        break;
    }

    await ILInspector.Worker.WorkerProtocol.DispatchAsync(req, dispatcher, stdout);
}

Console.Error.WriteLine("[worker] exiting");
return 0;
