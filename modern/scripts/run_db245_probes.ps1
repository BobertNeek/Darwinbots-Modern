[CmdletBinding()]
param(
    [string]$OutputPath = (Join-Path $PSScriptRoot '..\docs\evidence\db245\modern-probes.jsonl')
)

$ErrorActionPreference = 'Stop'
$modernRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$repoRoot = (Resolve-Path (Join-Path $modernRoot '..')).Path
$manifest = Join-Path $modernRoot 'tests\fixtures\db245-bots.sha256'
$bots = Join-Path $repoRoot 'Installer\bots'
$alga = Join-Path $bots 'Alga_Minimalis.txt'
$outputDirectory = Split-Path -Parent $OutputPath
New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null
Remove-Item -LiteralPath $OutputPath -ErrorAction SilentlyContinue

Push-Location $repoRoot
try {
    foreach ($line in Get-Content -LiteralPath $manifest) {
        if (-not $line.Trim()) { continue }
        $name = ($line -split '\|', 2)[1]
        $result = cargo run -q -p darwinbots-cli -- run `
            --bot (Join-Path $bots $name) `
            --population 5 `
            --vegetable-bot $alga `
            --vegetable-population 20 `
            --initial-energy 1000 `
            --ticks 1000 `
            --seed 24501 `
            --backend cpu
        if ($LASTEXITCODE -ne 0) { throw "Modern probe failed for $name" }
        $report = $result | ConvertFrom-Json
        [ordered]@{ bot = $name; report = $report } |
            ConvertTo-Json -Compress -Depth 8 |
            Add-Content -LiteralPath $OutputPath -Encoding utf8
    }
} finally {
    Pop-Location
}

$records = @(Get-Content -LiteralPath $OutputPath)
if ($records.Count -ne 23) { throw "Expected 23 probe records, found $($records.Count)." }
Write-Output $OutputPath
