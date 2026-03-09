# MLVInspector

`MLVInspector` is a **Windows-only** Rust + Dioxus desktop application for inspecting .NET assemblies. No Mac, Linux, or web support is planned.

Think of it as a lightweight `dnSpy` / `ILSpy` style workflow focused on:

- browsing assemblies, namespaces, types, and methods
- viewing IL and decompiled C# side by side
- jumping from findings back into code
- running `MLVScan`-powered analysis through the bundled worker

It is not trying to match the full feature surface of `dnSpy` or `ILSpy`. The goal is a narrower desktop analyst experience with tighter `MLVScan` integration, a persistent worker process, and a modern Rust UI/runtime.

It is being developed specifically for malware reverse engineering and malware analysis workflows, especially around Unity mod malware. In that space, attackers often steal legitimate Unity game mods, modify them, and add runtime payload-loader behavior that can slip past common static scanners. `MLVInspector`, as part of the `MLVScan` ecosystem, is meant to make that review loop faster by combining familiar assembly inspection with `MLVScan`-driven findings.

## Why Use It

- Familiar inspection workflow for .NET assemblies
- Built-in `MLVScan` integration instead of bolting scan output on afterward
- Rust desktop frontend with a responsive, native-feeling UI
- Long-lived worker subprocess avoids repeated tool startup overhead
- Unified explorer sidebar for assemblies plus namespace/type/method navigation

## Primary Use Case

- reverse engineering suspicious .NET assemblies that act as loaders or stagers
- triaging stolen or trojanized Unity mods that bypass tools like `VirusTotal`
- inspecting suspicious runtime behavior such as `Process.Start`, PowerShell execution, download-and-drop flows, and shell execution chains
- reviewing IL and decompiled C# around `MLVScan` findings to understand how a payload is fetched or launched

## Current Feature Set

- Open `.dll` and `.exe` assemblies from the toolbar or drag and drop
- Open a toolbar search palette to jump across assemblies, types, methods, and actions
- Explore namespaces, types, methods, and IL
- View decompiled C# alongside IL output
- Run explore + scan analysis through `ILInspector.Worker`
- Review findings with rule metadata and severity information
- Export the selected assembly into a decompiled C# project layout with namespace folders, per-type files, inferred assembly metadata, a generated `.csproj`, analysis JSON, and quick-open access to the last export folder

## Positioning

Compared with `dnSpy` / `ILSpy`:

- fewer features overall
- much tighter focus on static analysis review
- built around `MLVScan` workflows
- Rust frontend orchestration, with the analysis engine living in the worker

## Architecture

```text
MLVInspector (Rust + Dioxus desktop UI)
    -> WorkerClient (typed NDJSON subprocess bridge)
        -> ILInspector.Worker (.NET analysis worker)
            -> Explore + Scan results from MLVScan/inspection pipeline
```

## Requirements

Before building locally, install:

- Rust toolchain
- .NET SDK 8+
- Dioxus CLI on `PATH` for `dx` commands
- Windows 10/11

Notes:

- `WorkerConfig::default()` prefers env vars plus repo-relative discovery
- the worker currently targets `MLVScan.Core` from NuGet by default

## Build From Source

Build order matters.

### 1. Build the worker first

```bash
dotnet build MLVInspector.Worker/ILInspector.Worker.csproj
```

If you are intentionally working against a sibling checkout of `MLVScan.Core`:

```bash
dotnet build MLVInspector.Worker/ILInspector.Worker.csproj -p:LocalCoreBuild=true
```

### 2. Build the Dioxus app

```bash
cargo check
dx build --platform desktop
```

Optional release build:

```bash
cargo build --release
```

## Run Locally

```bash
dx serve --platform desktop
```

Then open an assembly by:

- dragging a file into the window
- using the toolbar file picker

## Worker Discovery

The desktop app looks for the worker in this order:

- `MLVINSPECTOR_WORKER_PATH`
- `ILINSPECTOR_WORKER_PATH`
- repo-relative build output locations such as `MLVInspector.Worker/bin/Debug/net8.0/ILInspector.Worker.exe`
- `ILInspector.Worker.exe` on `PATH`

To override discovery explicitly on Windows:

```bash
set MLVINSPECTOR_WORKER_PATH=C:\path\to\ILInspector.Worker.exe
dx serve --platform desktop
```

## Development Notes

- keep `src/app.rs` as a composition root, not a new monolith
- prefer adding UI to focused modules under `src/components/`
- prefer pure helpers for grouping, sorting, mapping, and tab/view-model logic
- prefer tests near newly extracted pure logic

## Known Limitations

- feature scope is still much smaller than `dnSpy` / `ILSpy`
- test coverage is still limited
- worker discovery is improved, but local build output still matters
