# ILInspector.Worker

`ILInspector.Worker` is the .NET sidecar process used by `MLVInspector.Dioxus`.

It accepts newline-delimited JSON requests over `stdin`, performs explore / scan operations, and returns typed NDJSON responses over `stdout`.

## Build

```bash
dotnet build MLVInspector.Worker/ILInspector.Worker.csproj
```

## Local Core Development

By default, the worker consumes the published `MLVScan.Core` package.

To build against a local sibling checkout of `MLVScan.Core` instead:

```bash
dotnet build MLVInspector.Worker/ILInspector.Worker.csproj -p:LocalCoreBuild=true
```

That expects `MLVScan.Core` to exist as a sibling directory outside this repo, matching the original local development layout.
