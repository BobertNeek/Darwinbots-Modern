param(
    [string]$OutputDirectory = (Join-Path $PSScriptRoot "modern")
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../../..")).Path
$env:DARWINBOTS_GUI_AUDIT_DIR = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Force -Path $env:DARWINBOTS_GUI_AUDIT_DIR | Out-Null

Push-Location $repoRoot
try {
    dotnet test modern/desktop/tests/Darwinbots.Desktop.Tests/Darwinbots.Desktop.Tests.csproj `
        --logger "console;verbosity=minimal"
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    python docs/verification/control-surface-audit/validate_matrix.py
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
finally {
    Pop-Location
}

Write-Host "Rendered GUI evidence: $env:DARWINBOTS_GUI_AUDIT_DIR"
