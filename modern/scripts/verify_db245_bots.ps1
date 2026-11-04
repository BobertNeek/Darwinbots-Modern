[CmdletBinding()]
param(
    [string]$SourceRoot = 'C:\Users\PC\Darwinbots2\Robots',
    [string]$RepositoryRoot = (Join-Path $PSScriptRoot '..\..\Installer\bots'),
    [string]$PublishRoot
)

$ErrorActionPreference = 'Stop'
$manifestPath = Join-Path $PSScriptRoot '..\tests\fixtures\db245-bots.sha256'
$entries = @(Get-Content -LiteralPath $manifestPath | Where-Object { $_.Trim().Length -gt 0 } | ForEach-Object {
    $parts = $_ -split '\|', 2
    if ($parts.Count -ne 2 -or $parts[0] -notmatch '^[0-9A-F]{64}$') {
        throw "Invalid DB2 bot manifest row: $_"
    }
    [pscustomobject]@{ Hash = $parts[0]; Name = $parts[1] }
})
if ($entries.Count -ne 23) {
    throw "Expected 23 DB2 bot manifest entries, found $($entries.Count)."
}

$roots = [ordered]@{ source = $SourceRoot; repository = $RepositoryRoot }
if ($PublishRoot) { $roots['publish'] = $PublishRoot }
$failures = [System.Collections.Generic.List[string]]::new()
$verified = [System.Collections.Generic.List[object]]::new()

foreach ($rootEntry in $roots.GetEnumerator()) {
    foreach ($entry in $entries) {
        $path = Join-Path $rootEntry.Value $entry.Name
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            $failures.Add("$($rootEntry.Key): missing $($entry.Name)")
            continue
        }
        $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
        if ($actual -ne $entry.Hash) {
            $failures.Add("$($rootEntry.Key): hash mismatch for $($entry.Name): $actual")
            continue
        }
        $verified.Add([pscustomobject]@{
            location = $rootEntry.Key
            name = $entry.Name
            sha256 = $actual
        })
    }
}

if ($failures.Count -gt 0) {
    throw ($failures -join [Environment]::NewLine)
}

[pscustomobject]@{
    manifest_entries = $entries.Count
    locations = @($roots.Keys)
    verified_files = $verified.Count
    status = 'ok'
} | ConvertTo-Json -Compress
