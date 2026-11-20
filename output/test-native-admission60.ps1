#requires -Version 7.0
param([Parameter(Mandatory=$true)][string]$HelperPath)
$ErrorActionPreference = 'Stop'
$HelperPath = [IO.Path]::GetFullPath($HelperPath)
. $HelperPath
# This constructs synthetic receipts only in a unique owned Temp directory.
# It never reads/writes the real checkout's CPU-ready marker or launches a product.
$tempParent = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd([char[]]@([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar))
$fixture = Join-Path $tempParent ('DbNativeAdmission60-' + [Guid]::NewGuid().ToString('N'))
$source = '1' * 40
$markerName = 'docs/parity/evidence/batch-2026-10-14/gpu-' + $source.Substring(0,12) + '/cpu-ready.json'
$checks = [Collections.Generic.List[string]]::new()
function Get-FixturePath([string]$Relative) {
    if ([IO.Path]::IsPathRooted($Relative)) { throw 'Fixture paths must be relative.' }
    $absolute = [IO.Path]::GetFullPath((Join-Path $fixture $Relative))
    if (!$absolute.StartsWith($fixture + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Fixture write escaped owned Temp.' }
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
    Write-FixtureBytes $Relative ([Text.UTF8Encoding]::new($false).GetBytes(($Value | ConvertTo-Json -Depth 40)))
    return Get-FixturePin $Relative
}
function Write-FixtureTrx([string]$Relative, [int]$Count, [bool]$FailedRow, [bool]$WrongCounters=$false) {
    $rows = for ($index=0; $index -lt $Count; $index++) {
        $outcome = if ($FailedRow -and $index -eq 0) { 'Failed' } else { 'Passed' }
        '<UnitTestResult executionId="probe-' + $index + '" testId="test-' + $index + '" testName="Probe' + $index + '" outcome="' + $outcome + '" />'
    }
    $counterText = foreach ($field in @('total', 'executed', 'passed', 'failed', 'error', 'timeout', 'aborted', 'inconclusive',
            'passedButRunAborted', 'notRunnable', 'notExecuted', 'disconnected', 'warning', 'completed', 'inProgress', 'pending')) {
        $value = if ($field -in @('total', 'executed', 'passed')) { $(if ($WrongCounters) { $Count + 1 } else { $Count }) } else { 0 }
        $field + '="' + $value + '"'
    }
    # Deliberately leave ALL counters passing when an individual row fails.
    $xml = '<TestRun xmlns="http://microsoft.com/schemas/VisualStudio/TeamTest/2010"><Results>' + ($rows -join '') +
        '</Results><ResultSummary outcome="Completed"><Counters ' + ($counterText -join ' ') + ' /></ResultSummary></TestRun>'
    Write-FixtureBytes $Relative ([Text.UTF8Encoding]::new($false).GetBytes($xml))
    return Get-FixturePin $Relative
}
function Write-Fixture([string]$Case='valid', [int]$Batch=60) {
    $proof = 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass' + $Batch + '/'
    # Deliberately synthetic counts prove admission still reads the full manifest/TRX contract.
    $coreCount = if ($Batch -eq 55) { 285 } else { 288 }
    $desktopCount = if ($Batch -eq 55) { 189 } else { 193 }
    $sourcePins = @()
    foreach ($relative in @('modern/engine/src/owned-fixture.rs', 'modern/engine/tests/db24832_genuine_load_transients.rs')) {
        Write-FixtureBytes $relative ([Text.Encoding]::UTF8.GetBytes('// owned isolated source ' + $relative))
        $sourcePins += Get-FixturePin $relative
    }
    $cycleSet = if ($Batch -ge 60 -or $Case -eq 'legacy_extra_cycle') { @(1,10,100,102) } else { @(1,10,102) }
    $declarationCycles = $cycleSet
    $originals = @()
    foreach ($cycle in $cycleSet) {
        $relative = 'modern/engine/tests/fixtures/owned-original-cycle' + $cycle + '.sim'
        Write-FixtureBytes $relative ([Text.Encoding]::UTF8.GetBytes('synthetic data only, cycle ' + $cycle))
        $pin = Get-FixturePin $relative
        $sourcePins += $pin
        $originals += [ordered]@{cycle=$cycle;candidate_exit_code=0;compare_exit_code=0;passed=$true;
            original=$pin.file;original_sha256=$pin.sha256;compared_fields=1379;difference_count=0;absolute_tolerance=0;relative_tolerance=0}
    }
    $probeCycle = if ($Batch -ge 60) { 100 } else { 102 }
    $targetRow = @($originals | Where-Object { $_.cycle -eq $probeCycle })[0]
    switch ($Case) {
        'missing_checkpoint' { $originals=@($originals | Where-Object { $_.cycle -ne $probeCycle }) }
        'wrong_expected_cycles' { $declarationCycles=@(1,10,102) }
        'duplicate_cycle' { $targetRow.cycle=10 }
        'unexpected_cycle' { $targetRow.cycle=$(if ($Batch -ge 60) { 99 } else { 100 }) }
        'wrong_field_count' { $targetRow.compared_fields=1378 }
        'nonzero_difference' { $targetRow.difference_count=1 }
        'nonzero_absolute_tolerance' { $targetRow.absolute_tolerance=1 }
        'nonzero_relative_tolerance' { $targetRow.relative_tolerance=1 }
        'failed_compare' { $targetRow.compare_exit_code=1 }
        'false_checkpoint_passed' { $targetRow.passed=$false }
        'unpinned_original' {
            $relative='modern/engine/tests/fixtures/unfrozen-original.sim'
            Write-FixtureBytes $relative ([Text.Encoding]::UTF8.GetBytes('owned data deliberately absent from the frozen source set'))
            $pin=Get-FixturePin $relative
            $targetRow.original=$pin.file
            $targetRow.original_sha256=$pin.sha256
        }
    }
    for ($index=$sourcePins.Count; $index -lt 156; $index++) {
        $relative='owned-source-pins/source-' + $index + '.txt'
        Write-FixtureBytes $relative ([Text.Encoding]::UTF8.GetBytes('owned synthetic pin ' + $index))
        $sourcePins += Get-FixturePin $relative
    }
    $manifestBody = [ordered]@{
        qualification_state=('all' + $Batch + '_code_and_tests_authored_frozen');parent_head=$source;comparison_base=$source;files=$sourcePins
    }
    if ($Batch -ge 56) {
        $manifestBody.expected_managed_test_counts=[ordered]@{core=$coreCount;desktop=$desktopCount}
    }
    $manifestPin=Write-FixtureJson ('output/native-roundtrip-pass' + $Batch + '-source.json') $manifestBody
    $targets=@([ordered]@{package='darwinbots-cli';name='darwinbots_cli';kind='lib';status='ok';passed=3;failed=0;ignored=0;filtered_out=0},
        [ordered]@{package='darwinbots-engine';name='db24832_genuine_load_transients';kind='test';status='ok';passed=3;failed=0;ignored=0;filtered_out=0})
    for ($index=2; $index -lt 68; $index++) {
        $targets += [ordered]@{package='darwinbots-engine';name=('owned_probe_' + $index);kind='test';status='ok';passed=1;failed=0;ignored=0;filtered_out=0}
    }
    $passed=[long]0
    foreach ($target in $targets) { $passed += [long]$target.passed }
    $cpu=[ordered]@{fresh_full_workspace_invocation=$true;hardware_filtered=$true;passed=$passed;failed=0;ignored=0;filtered_out=0;
        expected_distinct_targets=$targets.Count;total_distinct_targets=$targets.Count}
    $rustPin=Write-FixtureJson ($proof + 'rust-target-results.json') ([ordered]@{
        schema_version=1;batch=$Batch;state='passed';source_head=$source;source_manifest=$manifestPin
        command=@('test','--release','--workspace','--no-fail-fast','--','--skip','gpu','--skip','switch','--nocapture','--test-threads=1')
        invocation=[ordered]@{fresh_full_workspace_invocation=$true;hardware_filtered=$true;exit_code=0};full_workspace_passed=$true
        expected_distinct_targets=$targets.Count;total_distinct_targets=$targets.Count
        counts=[ordered]@{passed=$passed;failed=0;ignored=0;filtered_out=0};all_targets=$targets
    })
    $corePin=Write-FixtureTrx ($proof + 'core.trx') $coreCount $false
    $desktopPin=Write-FixtureTrx ($proof + 'desktop.trx') $desktopCount $false
    $genuinePin=Write-FixtureJson ($proof + 'genuine-movement-comparisons.json') ([ordered]@{
        schema_version=1;batch=$Batch;source_head=$source;source_manifest_sha256=$manifestPin.sha256;passed=$true;expected_cycles=$declarationCycles;checkpoints=$originals
    })
    $core=[ordered]@{source_head=$source;fresh_invocation=$true;passed=$coreCount;failed=0;skipped=0;total=$coreCount;trx=$corePin}
    $desktop=[ordered]@{source_head=$source;fresh_invocation=$true;passed=$desktopCount;failed=0;skipped=0;total=$desktopCount;trx=$desktopPin;exact_package_native_dll=$true}
    $summaryPin=Write-FixtureJson ($proof + 'summary.json') ([ordered]@{
        schema_version=1;batch=$Batch;source_head=$source;source_manifest=$manifestPin
        all_ordinary_local_outcomes_collected=$true;local_cpu_passed=$true;source_inputs_unchanged=$true;format_exit_code=0;strict_clippy_exit_code=0
        rust_cpu=$cpu;core=$core;desktop=$desktop
        package=[ordered]@{fresh_invocation=$true;publish_exit_code=0;smoke=[ordered]@{status='passed';backend='CPU'};
            assets=48;asset_mismatches=0;native_dll_identity=[ordered]@{target_release=('A'*64);package=('A'*64);desktop=('A'*64)}}
        evidence=@($rustPin,$corePin,$desktopPin,$genuinePin)
    })
    $extraPin=Write-FixtureJson ($proof + 'runner-identity.json') ([ordered]@{scope='synthetic probe only'})
    $marker=[ordered]@{
        schema_version=5;batch=$Batch;source_head=$source;qualification_method='fresh_full_cpu_workspace_and_core_desktop_package'
        cpu_workspace_passed=$true;core_passed=$true;desktop_passed=$true;package_passed=$true;source_qualification_passed=$true
        core_fresh_invocation=$true;desktop_fresh_invocation=$true;package_fresh_invocation=$true;source_qualification_fresh_invocation=$true
        rust_cpu=$cpu;rust_result=$rustPin;summary=$summaryPin;source_manifest=$manifestPin;core=$core;desktop=$desktop;genuine_movement=$genuinePin
        source_file_count=$sourcePins.Count;source_files=$sourcePins;evidence=@($manifestPin,$rustPin,$corePin,$desktopPin,$genuinePin,$summaryPin,$extraPin)
    }
    $null=Write-FixtureJson $markerName $marker
}
function Expect-Rejected([string]$Case, [string]$ExpectedMessage, [int]$Batch=60) {
    Write-Fixture $Case $Batch
    $rejected=$false
    try { $null=Read-NativeCpuAdmission53 $fixture $source }
    catch { if (!$_.Exception.Message.Contains($ExpectedMessage)) { throw ('Wrong rejection for batch' + $Batch + '/' + $Case + ': ' + $_.Exception.Message) };$rejected=$true }
    if (!$rejected) { throw ('Invalid fresh admission unexpectedly accepted: batch' + $Batch + '/' + $Case) }
    $checks.Add('batch' + $Batch + ':' + $Case)
}
try {
    [IO.Directory]::CreateDirectory($fixture) | Out-Null
    foreach ($batch in @(55,56,57,58,59,60,61)) {
        Write-Fixture 'valid' $batch
        $admitted=Read-NativeCpuAdmission53 $fixture $source
        $expectedCycles=if ($batch -ge 60) { '1|10|100|102' } else { '1|10|102' }
        $expectedCore=if ($batch -eq 55) { 285 } else { 288 }
        $expectedDesktop=if ($batch -eq 55) { 189 } else { 193 }
        if ($admitted.marker.schema_version -ne 5 -or $admitted.marker.batch -ne $batch -or
            $admitted.rust_result.total_distinct_targets -ne 68 -or
            $admitted.marker.core.total -ne $expectedCore -or $admitted.marker.desktop.total -ne $expectedDesktop) {
            throw 'Valid exact-batch fresh qualification was not admitted.'
        }
        # Read the fully pinned receipt back from the admitted marker, rather than
        # treating its top-level passing Boolean as checkpoint evidence.
        $genuine=Get-Content -LiteralPath (Get-FixturePath $admitted.marker.genuine_movement.file) -Raw | ConvertFrom-Json
        if ((@($genuine.expected_cycles) -join '|') -cne $expectedCycles -or
            @($genuine.checkpoints).Count -ne $(if ($batch -ge 60) { 4 } else { 3 })) { throw 'The admitted genuine checkpoint set changed.' }
        $checks.Add('valid_batch' + $batch + '_exact_movement_checkpoints')
    }
    Expect-Rejected 'missing_checkpoint' 'wrong origin or checkpoints'
    Expect-Rejected 'wrong_expected_cycles' 'wrong origin or checkpoints'
    Expect-Rejected 'duplicate_cycle' 'cycles must be distinct C1/C10/C100/C102'
    Expect-Rejected 'unexpected_cycle' 'cycles must be distinct C1/C10/C100/C102'
    Expect-Rejected 'wrong_field_count' 'must compare every expected live field'
    Expect-Rejected 'nonzero_difference' 'failed difference_count'
    Expect-Rejected 'nonzero_absolute_tolerance' 'failed absolute_tolerance'
    Expect-Rejected 'nonzero_relative_tolerance' 'failed relative_tolerance'
    Expect-Rejected 'failed_compare' 'failed compare_exit_code'
    Expect-Rejected 'false_checkpoint_passed' 'requires typed true: genuine movement checkpoint passed'
    Expect-Rejected 'unpinned_original' 'not one of the frozen source fixtures'
    Expect-Rejected 'legacy_extra_cycle' 'wrong origin or checkpoints' 59
    Expect-Rejected 'missing_checkpoint' 'wrong origin or checkpoints' 59
    Expect-Rejected 'duplicate_cycle' 'cycles must be distinct C1/C10/C102' 59
    Expect-Rejected 'unexpected_cycle' 'cycles must be distinct C1/C10/C102' 59
    [ordered]@{status='passed';checks=$checks.ToArray();count=$checks.Count;scope='CPU-only synthetic fully pinned receipt probes in an isolated owned Temp directory; no real workspace marker, product, build, Git, UI, native helper or GPU.'} | ConvertTo-Json -Depth 4
} finally {
    $resolvedFixture=[IO.Path]::GetFullPath($fixture)
    if (!$resolvedFixture.StartsWith($tempParent + [IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase) -or
        ![IO.Path]::GetFileName($resolvedFixture).StartsWith('DbNativeAdmission60-',[StringComparison]::Ordinal)) { throw 'Refusing cleanup outside verified owned Temp.' }
    if (Test-Path -LiteralPath $resolvedFixture) { Remove-Item -LiteralPath $resolvedFixture -Recurse -Force }
}
