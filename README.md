# MLVInspector

**MLVInspector** is a Rust + Dioxus desktop frontend for inspecting .NET assemblies and visualizing MLVScan / ILInspector analysis results.

It provides a desktop UI over a long-lived `ILInspector.Worker` subprocess, letting you open an assembly, explore its methods and IL, and review scan findings in a richer interface than a raw CLI response.

This repository now includes the worker project under `MLVInspector.Worker/` so the desktop app and sidecar process can live in the same GitHub repo.

## What It Does

- Opens `.NET` assemblies from the desktop UI or via drag and drop.
- Talks to `ILInspector.Worker` over newline-delimited JSON (NDJSON).
- Runs explore and scan operations for the selected assembly.
- Shows namespaces, types, methods, and IL instructions.
- Displays rule metadata and scan findings with severity information.
- Keeps worker orchestration in Rust instead of shipping UI logic through a web stack.

## Current Status

This project is functional, but still early in its architecture evolution.

- The app is currently implemented as a Dioxus desktop binary crate.
- The UI works against a long-lived worker process instead of a one-shot CLI invocation.
- `src/app.rs` is currently much too large and should be treated as refactor debt.
- The maintainability direction for the project is to split UI, view-model, and async workflow logic into smaller modules.
- The repository currently has no committed tests (`cargo test -- --list` reports `0 tests`).

## Why This Exists

The MLVScan ecosystem already has scanning engines and CLI-oriented tooling. This project focuses on the missing desktop analyst experience:

- browse an assembly visually
- inspect IL quickly
- pivot between findings and code locations
- work with a persistent subprocess instead of repeated process startup
- move toward a maintainable Rust desktop architecture for analysis tooling

## High-Level Architecture

```text
+------------------------+
| Dioxus Desktop UI      |
| Rust app state         |
| panels / IL viewer     |
+-----------+------------+
            |
            | typed requests/responses
            v
+------------------------+
| WorkerClient           |
| tokio subprocess layer |
| NDJSON request routing |
+-----------+------------+
            |
            v
+------------------------+
| ILInspector.Worker     |
| assembly analysis      |
| rule listing / scan    |
+------------------------+
```

Key boundaries:

- The desktop UI is written in Rust with Dioxus.
- Worker communication is typed in `src/ipc.rs`.
- Process orchestration lives in `src/services/worker_client.rs`.
- Assemblies are passed to the worker by file path; they are not loaded into this process.

## Repository Layout

```text
src/
|- main.rs                    # Entry point, tracing setup, window config
|- app.rs                     # Current top-level composition root (too large)
|- state.rs                   # Global AppState backed by Dioxus signals
|- ipc.rs                     # Worker protocol request/response types
|- types.rs                   # App-facing domain and UI types
|- error.rs                   # Shared AppError enum
`- services/
   |- worker_client.rs        # Active long-lived worker subprocess client
   `- inspector.rs            # Legacy one-shot CLI wrapper kept for reference

MLVInspector.Worker/
|- ILInspector.Worker.csproj  # .NET worker project bundled with this repo
|- Program.cs                 # NDJSON worker entry point
|- Dispatcher.cs              # Explore / scan / list-rules handling
|- Protocol.cs                # Worker-side protocol DTOs
`- AssemblyCache.cs           # Mono.Cecil assembly cache
```

## Requirements

Before running the app locally, make sure you have:

- Rust toolchain installed
- Dioxus CLI installed and available on `PATH` for `dx` commands
- A valid local build of `ILInspector.Worker`
- Windows environment support if you want to match the current default configuration

Important notes:

- `WorkerConfig::default()` now tries environment variables plus repo-relative discovery before falling back to `ILInspector.Worker.exe`.
- The legacy `InspectorConfig::default()` now does the same for `ILInspector.exe`.
- The current code assumes Windows in a few places, including window configuration and worker startup behavior.

That said, the Rust app now tries to discover the worker automatically from common repo and build-output locations before falling back to a plain executable name.

## Quick Start

### 1. Verify the toolchain

```bash
cargo check
dotnet build MLVInspector.Worker/ILInspector.Worker.csproj
```

### 2. Run the desktop app

```bash
dx serve --platform desktop
```

### 3. Open an assembly

You can open an assembly by:

- dragging a file into the window
- using the desktop file picker from the toolbar

Once loaded, the app will run explore and scan requests against the worker and populate the UI.

## Common Development Commands

### Fast compile checks

```bash
cargo check
cargo check --all-targets
```

### Formatting and linting

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

### Running tests

```bash
cargo test
cargo test -- --list
cargo test <substring>
cargo test <module>::tests::<test_name> -- --exact
cargo test --bin mlvinspector-dioxus <substring>
cargo test <substring> -- --nocapture
```

### Building

```bash
dotnet build MLVInspector.Worker/ILInspector.Worker.csproj
dx build --platform desktop
cargo build --release
```

## Worker Path Resolution

The desktop app now looks for the worker in this order:

- `MLVINSPECTOR_WORKER_PATH`
- `ILINSPECTOR_WORKER_PATH`
- common repo-relative locations such as `MLVInspector.Worker/bin/Debug/net8.0/ILInspector.Worker.exe`
- a plain `ILInspector.Worker.exe` on `PATH`

If you need to override discovery explicitly:

```bash
set MLVINSPECTOR_WORKER_PATH=C:\path\to\ILInspector.Worker.exe
dx serve --platform desktop
```

## How the App Works

On startup, the app creates global Dioxus state and initializes a `WorkerClient`.

When you open an assembly:

1. the assembly is added to app state
2. the UI kicks off worker `explore` and `scan` requests
3. both responses are captured into a combined typed `AnalysisResult`
4. the explorer panel shows namespaces, types, methods, and IL
5. the findings panel shows rule-triggered results and severity information

The worker stays alive across requests, which reduces repeated startup cost and keeps the desktop flow responsive.

## Development Direction

This repository should move toward a more maintainable, agent-friendly engineering shape.

- Do not grow `src/app.rs` further unless absolutely necessary.
- Prefer extracting named UI regions into `src/components/`, `src/panels/`, or `src/ui/`.
- Prefer pure transformation logic in helper or view-model modules.
- Prefer async workflows in `src/services/`, `src/actions/`, or `src/controllers/`.
- Add focused tests whenever non-trivial logic is extracted.

If you are making changes in this repo, read `AGENTS.md` first.

## Known Limitations

- The project currently has no committed automated tests.
- The default worker path is machine-specific.
- The worker currently targets `MLVScan.Core` from NuGet by default; local sibling-core development is still supported via `-p:LocalCoreBuild=true`.
- The codebase is still heavily centered around a very large `src/app.rs` file.
- Some architecture placeholders exist for modes like `compare` and `analyze-reflect`, but the desktop UX is still primarily centered on explore + scan.
- The current implementation is Windows-oriented.

## Contributing

Contributions are welcome, especially in these areas:

- extracting maintainable UI components from `src/app.rs`
- improving worker configuration and portability
- adding unit tests for extracted helpers and protocol logic
- improving findings navigation, IL navigation, and analysis workflows
- tightening the boundary between worker wire types and app-facing view models

When contributing, favor small, testable extractions over adding more inline logic to the main app module.
