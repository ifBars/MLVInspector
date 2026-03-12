using System.Text;
using System.Text.Json;
using ILInspector.Worker;
using Xunit;

namespace MLVInspector.Tests;

public class WorkerProtocolTests
{
    [Fact]
    public async Task WriteErrorAsync_WritesExpectedErrorEnvelope()
    {
        using var stream = new MemoryStream();
        using var writer = new StreamWriter(stream, new UTF8Encoding(false), leaveOpen: true)
        {
            AutoFlush = true,
        };

        await WorkerProtocol.WriteErrorAsync(writer, 12, "boom");

        stream.Position = 0;
        using var reader = new StreamReader(stream, Encoding.UTF8, detectEncodingFromByteOrderMarks: false);
        var json = await reader.ReadToEndAsync();
        var response = JsonSerializer.Deserialize<WorkerResponse<object>>(json, WorkerJsonContext.Default.WorkerResponseObject);

        Assert.NotNull(response);
        Assert.Equal((ulong)12, response!.Id);
        Assert.False(response.Ok);
        Assert.Null(response.Payload);
        Assert.Equal("boom", response.Error);
    }

    [Fact]
    public async Task DispatchAsync_UnknownMethod_ReturnsErrorEnvelope()
    {
        using var cache = new AssemblyCache();
        var dispatcher = new Dispatcher(cache);
        using var stream = new MemoryStream();
        using var writer = new StreamWriter(stream, new UTF8Encoding(false), leaveOpen: true)
        {
            AutoFlush = true,
        };

        using var document = JsonDocument.Parse("{}");
        await WorkerProtocol.DispatchAsync(
            new WorkerRequest
            {
                Id = 7,
                Method = "unknown",
                Params = document.RootElement.Clone(),
            },
            dispatcher,
            writer);

        stream.Position = 0;
        using var reader = new StreamReader(stream, Encoding.UTF8, detectEncodingFromByteOrderMarks: false);
        var json = await reader.ReadToEndAsync();
        var response = JsonSerializer.Deserialize<WorkerResponse<object>>(json, WorkerJsonContext.Default.WorkerResponseObject);

        Assert.NotNull(response);
        Assert.Equal((ulong)7, response!.Id);
        Assert.False(response.Ok);
        Assert.Equal("unknown method: unknown", response.Error);
    }
}
