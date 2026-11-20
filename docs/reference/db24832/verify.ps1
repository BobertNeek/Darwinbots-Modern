$ErrorActionPreference = 'Stop'
$expected = [ordered]@{
    'DARWINBOTS_2_PRODUCT_SPEC.md' = '43146CB5D511CC046372DA797A4C07738FA3D4B90DD7BB91A9FF262CC2627832'
    'UI_AND_INTERACTION_SPEC.md' = 'CDEE2F72CF09F7259164E61B40D3535BECA1EE4217FE15C89A3DED90DE936B36'
    'DNA_RUNTIME_SPEC.md' = '1A0C1C11E0C5DCFC41BFDFF19DD9CD35FCBE1F5292094868A4D67A6390F512D9'
    'SYSVARS_REFERENCE.md' = '7DCC7399D97EA5B306E5611FE6433C84D923F482C9F91C34808BDB4002387701'
    'DECLARATIVE_UI_INVENTORY.md' = 'C3CE0D2684ED7BE8FB6F925C54F52E225D208CA44C2C08DBAD022EE537C60595'
    'HISTORY_AND_SOURCES.md' = 'DD7E062CE448944210D9EA564DF8D63558FFB178CCA6CCC644A8F1991EAF2CD6'
}
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
foreach ($entry in $expected.GetEnumerator()) {
    $path = Join-Path $root $entry.Key
    if (-not (Test-Path -LiteralPath $path)) { throw "Missing canonical artifact: $($entry.Key)" }
    $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
    if ($actual -ne $entry.Value) { throw "Hash mismatch for $($entry.Key): $actual" }
}
Write-Output "DB2 2.48.32 reference package verified: $($expected.Count)/$($expected.Count)"
