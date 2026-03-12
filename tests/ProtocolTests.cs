using System.Text.Json;
using ILInspector.Worker;
using Xunit;

namespace MLVInspector.Tests;

public class ProtocolTests
{
    [Fact]
    public void WorkerRequest_DeserializesCamelCaseFields()
    {
        const string json = """
            {"id":5,"method":"scan","params":{"assembly":"sample.dll","showClean":true}}
            """;

        var request = JsonSerializer.Deserialize(json, WorkerJsonContext.Default.WorkerRequest);

        Assert.NotNull(request);
        Assert.Equal((ulong)5, request!.Id);
        Assert.Equal("scan", request.Method);

        var scanParams = request.Params.Deserialize(WorkerJsonContext.Default.ScanParams);
        Assert.NotNull(scanParams);
        Assert.Equal("sample.dll", scanParams!.Assembly);
        Assert.True(scanParams.ShowClean);
    }

    [Fact]
    public void WorkerResponse_SerializesListRulesEnvelope()
    {
        var response = new WorkerResponse<List<RuleEntry>>
        {
            Id = 3,
            Ok = true,
            Payload =
            [
                new RuleEntry
                {
                    RuleId = "MLV-001",
                    Description = "Example",
                    Severity = "High",
                },
            ],
            Error = null,
        };

        var json = JsonSerializer.Serialize(response, WorkerJsonContext.Default.WorkerResponseListRuleEntry);

        Assert.Contains("\"ruleId\":\"MLV-001\"", json);
        Assert.Contains("\"ok\":true", json);
        Assert.Contains("\"error\":null", json);
    }
}
