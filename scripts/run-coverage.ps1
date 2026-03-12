param(
    [switch]$SkipRust,
    [switch]$SkipDotnet,
    [switch]$OpenReports
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

$coverageRoot = Join-Path $repoRoot "coverage"
$rustRoot = Join-Path $coverageRoot "rust"
$dotnetRoot = Join-Path $coverageRoot "dotnet"
$dotnetRawRoot = Join-Path $dotnetRoot "raw"
$dotnetReportRoot = Join-Path $dotnetRoot "report"

function Remove-IfExists {
    param([string]$Path)

    if (Test-Path $Path) {
        Remove-Item $Path -Recurse -Force
    }
}

function Ensure-Command {
    param(
        [string]$Name,
        [string]$InstallHint
    )

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "$Name was not found. $InstallHint"
    }
}

function Assert-LastExitCode {
    param([string]$CommandName)

    if ($LASTEXITCODE -ne 0) {
        throw "$CommandName failed with exit code $LASTEXITCODE"
    }
}

Remove-IfExists $coverageRoot
New-Item -ItemType Directory -Path $rustRoot | Out-Null
New-Item -ItemType Directory -Path $dotnetRawRoot | Out-Null

if (-not $SkipRust) {
    Ensure-Command "cargo" "Install Rust first."
    Ensure-Command "cargo-llvm-cov" "Install it with: cargo install cargo-llvm-cov --locked"

    cargo llvm-cov --html --output-dir "$rustRoot/html"
    Assert-LastExitCode "cargo llvm-cov --html"

    cargo llvm-cov --cobertura --output-path "$rustRoot/cobertura.xml"
    Assert-LastExitCode "cargo llvm-cov --cobertura"
}

if (-not $SkipDotnet) {
    Ensure-Command "dotnet" "Install the .NET 8 SDK first."
    Ensure-Command "reportgenerator" "Install it with: dotnet tool install --global dotnet-reportgenerator-globaltool"

    dotnet test "MLVInspector.Worker.Tests.csproj" --settings "coverage.runsettings" --collect:"XPlat Code Coverage" --results-directory "$dotnetRawRoot"
    Assert-LastExitCode "dotnet test"

    reportgenerator "-reports:$dotnetRawRoot/**/coverage.cobertura.xml" "-targetdir:$dotnetReportRoot" "-reporttypes:HtmlInline;Cobertura;TextSummary"
    Assert-LastExitCode "reportgenerator"
}

Write-Host "Coverage reports generated:"

if (-not $SkipRust) {
    Write-Host "- Rust HTML: $rustRoot/html/index.html"
    Write-Host "- Rust Cobertura: $rustRoot/cobertura.xml"
}

if (-not $SkipDotnet) {
    Write-Host "- .NET HTML: $dotnetReportRoot/index.html"
    Write-Host "- .NET Text summary: $dotnetReportRoot/Summary.txt"
    Write-Host "- .NET Cobertura: $dotnetReportRoot/Cobertura.xml"
}

if ($OpenReports) {
    if (-not $SkipRust) {
        Start-Process "$rustRoot/html/index.html"
    }

    if (-not $SkipDotnet) {
        Start-Process "$dotnetReportRoot/index.html"
    }
}
