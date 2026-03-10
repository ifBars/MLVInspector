using ILInspector.Worker;
using Xunit;
using Xunit.Abstractions;

namespace MLVInspector.Tests;

public class DecompilerWebClientTests
{
    private readonly ITestOutputHelper _output;

    public DecompilerWebClientTests(ITestOutputHelper output)
    {
        _output = output;
    }

    [Fact]
    public void Decompile_SyntheticWebClientAssembly_ResolvesFrameworkTypes()
    {
        // Create a minimal synthetic test assembly with WebClient reference
        var testAssemblyPath = CreateMinimalWebClientAssembly();
        _output.WriteLine($"Created test assembly: {testAssemblyPath}");

        try
        {
            using var cache = new AssemblyCache();
            var dispatcher = new Dispatcher(cache);

            // Try to decompile - this should NOT throw "Could not find type definition System.Net.WebClient"
            var payload = dispatcher.Decompile(new DecompileParams
            {
                Assembly = testAssemblyPath,
                TypeName = null,
                MethodName = null,
                Profile = "analysis",
            });

            _output.WriteLine("=== Decompiled Source ===");
            _output.WriteLine(payload.CsharpSource);
            _output.WriteLine("=========================");

            // The decompilation should succeed without the WebClient error
            Assert.NotEmpty(payload.CsharpSource);
            Assert.DoesNotContain("Could not find type definition System.Net.WebClient", payload.CsharpSource);
            Assert.DoesNotContain("Decompilation error", payload.CsharpSource);
        }
        finally
        {
            // Clean up
            try { File.Delete(testAssemblyPath); } catch { }
        }
    }

    [Fact]
    public void Decompile_SyntheticNet6WebClientAssembly_ResolvesFrameworkTypes()
    {
        var testAssemblyPath = CreateMinimalWebClientAssembly(".NETCoreApp,Version=v6.0");
        _output.WriteLine($"Created test assembly: {testAssemblyPath}");

        try
        {
            using var cache = new AssemblyCache();
            var dispatcher = new Dispatcher(cache);

            var payload = dispatcher.Decompile(new DecompileParams
            {
                Assembly = testAssemblyPath,
                TypeName = null,
                MethodName = null,
                Profile = "analysis",
            });

            Assert.NotEmpty(payload.CsharpSource);
            Assert.DoesNotContain("Could not find type definition System.Net.WebClient", payload.CsharpSource);
            Assert.DoesNotContain("Decompilation error", payload.CsharpSource);
        }
        finally
        {
            try { File.Delete(testAssemblyPath); } catch { }
        }
    }

    private static string CreateMinimalWebClientAssembly(string targetFrameworkMoniker = ".NETFramework,Version=v4.8")
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"TestWebClient_{Guid.NewGuid()}.dll");

        // Create a minimal assembly using Mono.Cecil with a reference to System.Net.WebClient
        using var module = Mono.Cecil.ModuleDefinition.CreateModule("TestWebClient", Mono.Cecil.ModuleKind.Dll);

        var targetFrameworkAttr = new Mono.Cecil.TypeReference(
            "System.Runtime.Versioning",
            "TargetFrameworkAttribute",
            module,
            module.TypeSystem.CoreLibrary);

        var constructor = new Mono.Cecil.MethodReference(".ctor", module.TypeSystem.Void, targetFrameworkAttr);
        constructor.Parameters.Add(new Mono.Cecil.ParameterDefinition(module.TypeSystem.String));

        var attribute = new Mono.Cecil.CustomAttribute(constructor);
        attribute.ConstructorArguments.Add(new Mono.Cecil.CustomAttributeArgument(
            module.TypeSystem.String,
            targetFrameworkMoniker));

        module.Assembly.CustomAttributes.Add(attribute);

        // Create a simple class with a method that references WebClient
        var type = new Mono.Cecil.TypeDefinition(
            "TestNS",
            "TestClass",
            Mono.Cecil.TypeAttributes.Class | Mono.Cecil.TypeAttributes.Public);

        var method = new Mono.Cecil.MethodDefinition(
            "DoDownload",
            Mono.Cecil.MethodAttributes.Public | Mono.Cecil.MethodAttributes.Static,
            module.TypeSystem.String);

        method.Parameters.Add(new Mono.Cecil.ParameterDefinition("url", Mono.Cecil.ParameterAttributes.None, module.TypeSystem.String));

        // Create IL that uses WebClient
        var il = method.Body.GetILProcessor();
        var webClientType = new Mono.Cecil.TypeReference("System.Net", "WebClient", module, module.TypeSystem.CoreLibrary);
        var webClientCtor = new Mono.Cecil.MethodReference(".ctor", module.TypeSystem.Void, webClientType);
        webClientCtor.HasThis = true;

        var downloadString = new Mono.Cecil.MethodReference(
            "DownloadString",
            module.TypeSystem.String,
            webClientType);
        downloadString.HasThis = true;
        downloadString.Parameters.Add(new Mono.Cecil.ParameterDefinition(module.TypeSystem.String));

        il.Emit(Mono.Cecil.Cil.OpCodes.Newobj, webClientCtor);
        il.Emit(Mono.Cecil.Cil.OpCodes.Ldarg_0);
        il.Emit(Mono.Cecil.Cil.OpCodes.Callvirt, downloadString);
        il.Emit(Mono.Cecil.Cil.OpCodes.Ret);

        type.Methods.Add(method);
        module.Types.Add(type);

        module.Write(tempPath);
        return tempPath;
    }
}
