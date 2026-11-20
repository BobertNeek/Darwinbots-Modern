#requires -Version 7.0
param(
    [Parameter(Mandatory=$true)][string]$HelperPath,
    [string]$EvidenceRepo
)
$ErrorActionPreference = 'Stop'
$HelperPath = [IO.Path]::GetFullPath($HelperPath)
if (!$EvidenceRepo) { $EvidenceRepo = Split-Path -Parent (Split-Path -Parent $HelperPath) }
$EvidenceRepo = [IO.Path]::GetFullPath($EvidenceRepo)
. $HelperPath

# Only copied evidence and source bytes in this owned Temp directory are modified.
$tempParent = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd([char[]]@([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar))
$fixture = Join-Path $tempParent ('DbNativeAdmission54-' + [Guid]::NewGuid().ToString('N'))
$source = '1' * 40
$origin = 'e0300ca1391e0028b39aef2b61031a98934f00db'
$basisName = 'output/pass54-core-basis53.json'
$summaryName = 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass53/summary.json'
$trxName = 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass53/core.trx'
$baseRustName = 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass52/rust-target-results.json'
$cliName = 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass53/rust-cli-target-result.json'
$projectionName = 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass54/rust-source-equivalence.json'
$equivalenceName = 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass54/core-source-equivalence.json'
$markerName = 'docs/parity/evidence/batch-2026-10-14/gpu-' + $source.Substring(0,12) + '/cpu-ready.json'
$checks = [Collections.Generic.List[string]]::new()

function Get-FixturePath([string]$Relative) {
    if ([IO.Path]::IsPathRooted($Relative)) { throw 'Fixture paths must be relative.' }
    $absolute = [IO.Path]::GetFullPath((Join-Path $fixture $Relative))
    if (!$absolute.StartsWith($fixture + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Fixture path escaped its owned Temp directory.'
    }
    return $absolute
}
function Write-FixtureBytes([string]$Relative, [byte[]]$Bytes) {
    $path = Get-FixturePath $Relative
    [IO.Directory]::CreateDirectory((Split-Path -Parent $path)) | Out-Null
    [IO.File]::WriteAllBytes($path, $Bytes)
}
function Get-FixturePin([string]$Relative) {
    return [ordered]@{file=$Relative;sha256=(Get-FileHash -LiteralPath (Get-FixturePath $Relative) -Algorithm SHA256).Hash}
}
function Write-FixtureJson([string]$Relative, $Value) {
    Write-FixtureBytes $Relative ([Text.UTF8Encoding]::new($false).GetBytes(($Value | ConvertTo-Json -Depth 32)))
    return Get-FixturePin $Relative
}
function Write-Fixture([string]$Case='valid') {
    $basis = $basisText | ConvertFrom-Json
    $summary = $summaryText | ConvertFrom-Json
    $projection = $projectionText | ConvertFrom-Json
    if ($Case -eq 'failed_trx') {
        # Keep the passing totals deliberately: an actual failed result row must still reject.
        $changed = [regex]::new('outcome="Passed"').Replace([Text.UTF8Encoding]::new($false, $true).GetString($trxBytes), 'outcome="Failed"', 1)
        Write-FixtureBytes $trxName ([Text.UTF8Encoding]::new($false).GetBytes($changed))
        $trxPin = Get-FixturePin $trxName
        foreach ($pin in $summary.evidence) { if ($pin.file -ceq $trxName) { $pin.sha256 = $trxPin.sha256 } }
        $summaryPin = Write-FixtureJson $summaryName $summary
        $basis.trx = $trxPin
        $basis.summary = $summaryPin
        $basisPin = Write-FixtureJson $basisName $basis
    } else {
        Write-FixtureBytes $trxName $trxBytes
        Write-FixtureBytes $summaryName $summaryBytes
        Write-FixtureBytes $basisName $basisBytes
        $trxPin = Get-FixturePin $trxName
        $summaryPin = Get-FixturePin $summaryName
        $basisPin = Get-FixturePin $basisName
    }
    $equivalencePin = Write-FixtureJson $equivalenceName ([ordered]@{
        schema_version=1;source_head=$source;base_source_head=$origin;core_basis=$basisPin
        core_input_count=$basis.core_input_count;core_inputs=$basis.core_inputs
        all_core_inputs_unchanged=$true;git_source_delta_audited=$true;projection_passed=$true
    })
    $projection.source_head = $source
    $projection.batch = 54
    $projectionPin = Write-FixtureJson $projectionName $projection
    $marker = [ordered]@{
        schema_version=4;source_head=$source
        qualification_method='source_equivalent_full_cpu52_plus_fresh_cli53_and_desktop_package'
        cpu_workspace_passed=$true;core_passed=$true;desktop_passed=$true;package_passed=$true;source_qualification_passed=$true
        core_fresh_invocation=($Case -eq 'fresh_core');core_source_equivalent=($Case -ne 'equivalence_false')
        desktop_fresh_invocation=$true;package_fresh_invocation=$true;source_qualification_fresh_invocation=$true
        rust_cpu=$projection.rust_cpu;projection=$projectionPin
        source_file_count=$sourcePins.Count
        source_files=@($sourcePins | ForEach-Object { [ordered]@{file=$_.file;sha256=$_.sha256} })
        evidence=@($projectionPin, (Get-FixturePin $baseRustName), (Get-FixturePin $cliName), $basisPin, $summaryPin, $trxPin, $equivalencePin)
        retained_core=[ordered]@{source_head=$origin;basis=$basisPin;summary=$summaryPin;trx=$trxPin;equivalence=$equivalencePin}
    }
    if ($Case -eq 'wrong_origin') { $marker.retained_core.source_head = '2' * 40 }
    if ($Case -eq 'missing_pin') { $marker.evidence = @($marker.evidence | Where-Object file -cne $trxName) }
    if ($Case -eq 'fresh_core') {
        $marker.Remove('retained_core')
        $marker.Remove('core_source_equivalent')
    }
    $null = Write-FixtureJson $markerName $marker
}
function Expect-Rejected([string]$Case, [string]$ExpectedMessage) {
    Write-Fixture $Case
    $rejected = $false
    try { $null = Read-NativeCpuAdmission53 $fixture $source }
    catch {
        if (!$_.Exception.Message.Contains($ExpectedMessage)) { throw ('Wrong rejection for ' + $Case + ': ' + $_.Exception.Message) }
        $rejected = $true
    }
    if (!$rejected) { throw ('Invalid admission unexpectedly accepted: ' + $Case) }
    $checks.Add($Case)
}

try {
    [IO.Directory]::CreateDirectory($fixture) | Out-Null
    $basisBytes = [IO.File]::ReadAllBytes((Join-Path $EvidenceRepo $basisName))
    $basisText = [Text.UTF8Encoding]::new($false, $true).GetString($basisBytes)
    $basis = $basisText | ConvertFrom-Json
    $summaryBytes = [IO.File]::ReadAllBytes((Join-Path $EvidenceRepo $summaryName))
    $summaryText = [Text.UTF8Encoding]::new($false, $true).GetString($summaryBytes)
    $trxBytes = [IO.File]::ReadAllBytes((Join-Path $EvidenceRepo $trxName))
    $projectionText = [IO.File]::ReadAllText((Join-Path $EvidenceRepo 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass53/rust-source-equivalence.json'))
    foreach ($relative in @($baseRustName, $cliName)) {
        Write-FixtureBytes $relative ([IO.File]::ReadAllBytes((Join-Path $EvidenceRepo $relative)))
    }
    $sourcePins = @()
    foreach ($pin in $basis.core_inputs) {
        $bytes = [IO.File]::ReadAllBytes((Join-Path $EvidenceRepo $pin.file))
        Write-FixtureBytes $pin.file $bytes
        $sourcePins += Get-FixturePin $pin.file
    }
    for ($index=$sourcePins.Count; $index -lt 162; $index++) {
        $relative = 'fixture-sources/source-' + $index + '.txt'
        Write-FixtureBytes $relative ([Text.Encoding]::UTF8.GetBytes('owned-fixture-' + $index))
        $sourcePins += Get-FixturePin $relative
    }
    Write-Fixture
    $admitted = Read-NativeCpuAdmission53 $fixture $source
    if ($admitted.marker.core_fresh_invocation -ne $false) { throw 'Valid retained Core fixture was not admitted as retained.' }
    $checks.Add('actual_Core53_retained')
    Expect-Rejected 'wrong_origin' 'Retained Core requires the exact fresh Core53 origin'
    Expect-Rejected 'failed_trx' 'Retained Core TRX must contain exactly 285 executed passing tests'
    Expect-Rejected 'missing_pin' 'Retained Core evidence must be pinned explicitly: trx'
    Expect-Rejected 'equivalence_false' 'Retained Core requires typed core_source_equivalent=true'

    Write-Fixture
    $changedPin = $sourcePins[0]
    $changedPath = Get-FixturePath $changedPin.file
    $originalBytes = [IO.File]::ReadAllBytes($changedPath)
    try {
        [IO.File]::AppendAllText($changedPath, 'changed-Core-input')
        $marker = [IO.File]::ReadAllText((Get-FixturePath $markerName)) | ConvertFrom-Json
        foreach ($pin in $marker.source_files) {
            if ($pin.file -ceq $changedPin.file) { $pin.sha256 = (Get-FixturePin $changedPin.file).sha256 }
        }
        $null = Write-FixtureJson $markerName $marker
        $rejected = $false
        try { $null = Read-NativeCpuAdmission53 $fixture $source }
        catch {
            if (!$_.Exception.Message.Contains('Qualified evidence/source pin changed')) { throw }
            $rejected = $true
        }
        if (!$rejected) { throw 'Changed Core input was accepted despite its captured basis hash.' }
        $checks.Add('changed_Core_input_despite_updated_current_source_pin')
    } finally { [IO.File]::WriteAllBytes($changedPath, $originalBytes) }

    Write-Fixture 'fresh_core'
    $null = Read-NativeCpuAdmission53 $fixture $source
    $checks.Add('existing_fresh_Core_branch')
    $lease = Enter-NativeQualificationLease53 $fixture $source 'test-first'
    try {
        $collided = $false
        try {
            $second = Enter-NativeQualificationLease53 $fixture $source 'test-second'
            $second.Dispose()
        } catch {
            if (!$_.Exception.Message.Contains('exclusive lease')) { throw }
            $collided = $true
        }
        if (!$collided) { throw 'Second qualification lease was accepted while the first was open.' }
    } finally { $lease.Dispose() }
    $after = Enter-NativeQualificationLease53 $fixture $source 'test-after-release'
    $after.Dispose()
    $checks.Add('exclusive_lease_collision_and_release')
    [ordered]@{status='passed';checks=$checks.ToArray();count=$checks.Count;scope='CPU-only admission and file-lease probes in an isolated owned Temp repo; no actual workspace marker, GPU, UI, build, or Git action.'} | ConvertTo-Json -Depth 4
} finally {
    $resolvedFixture = [IO.Path]::GetFullPath($fixture)
    if (!$resolvedFixture.StartsWith($tempParent + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -or
        ![IO.Path]::GetFileName($resolvedFixture).StartsWith('DbNativeAdmission54-', [StringComparison]::Ordinal)) {
        throw 'Refusing cleanup outside the verified owned Temp fixture.'
    }
    if (Test-Path -LiteralPath $resolvedFixture) { Remove-Item -LiteralPath $resolvedFixture -Recurse -Force }
}
