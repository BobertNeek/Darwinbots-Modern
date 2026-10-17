#requires -Version 7.0
param([Parameter(Mandatory=$true)][string]$HelperPath)
$ErrorActionPreference = 'Stop'
$HelperPath = [IO.Path]::GetFullPath($HelperPath)
. $HelperPath
# This constructs synthetic receipts only in a unique owned Temp directory.
# It never reads/writes the real checkout's CPU-ready marker or launches a product.
$tempParent = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd([char[]]@([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar))
$fixture = Join-Path $tempParent ('DbNativeAdmission56-' + [Guid]::NewGuid().ToString('N'))
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
function Write-Fixture([string]$Case='valid', [int]$Batch=56) {
    $proof = 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass' + $Batch + '/'
    # Synthetic counts intentionally differ from the actual batch56 test census.
    $coreCount = if ($Batch -eq 55) { 285 } else { 288 }
    $desktopCount = if ($Batch -eq 55) { 189 } else { 193 }
    $sourcePins = @()
    foreach ($relative in @('modern/engine/src/owned-fixture.rs', 'modern/engine/tests/db24832_genuine_load_transients.rs')) {
        Write-FixtureBytes $relative ([Text.Encoding]::UTF8.GetBytes('// owned isolated source ' + $relative))
        $sourcePins += Get-FixturePin $relative
    }
    $originals = @()
    foreach ($cycle in @(1,10,102)) {
        $relative = 'modern/engine/tests/fixtures/owned-original-cycle' + $cycle + '.sim'
        Write-FixtureBytes $relative ([Text.Encoding]::UTF8.GetBytes('synthetic data only, cycle ' + $cycle))
        $pin = Get-FixturePin $relative
        $sourcePins += $pin
        $originals += [ordered]@{cycle=$cycle;candidate_exit_code=0;compare_exit_code=0;passed=$true;
            original=$pin.file;original_sha256=$pin.sha256;compared_fields=1379;difference_count=0;absolute_tolerance=0;relative_tolerance=0}
    }
    for ($index=$sourcePins.Count; $index -lt 156; $index++) {
        $relative='owned-source-pins/source-' + $index + '.txt'
        Write-FixtureBytes $relative ([Text.Encoding]::UTF8.GetBytes('owned synthetic pin ' + $index))
        $sourcePins += Get-FixturePin $relative
    }
    $manifestBody = [ordered]@{
        qualification_state=('all' + $Batch + '_code_and_tests_authored_frozen');parent_head=$source;comparison_base=$source;files=$sourcePins
    }
    if ($Batch -ge 56 -and $Case -ne 'absent_expected_managed_counts') {
        $manifestBody.expected_managed_test_counts = [ordered]@{core=$coreCount;desktop=$desktopCount}
        switch ($Case) {
            'zero_core_expected_count' { $manifestBody.expected_managed_test_counts.core=0 }
            'zero_desktop_expected_count' { $manifestBody.expected_managed_test_counts.desktop=0 }
            'string_core_expected_count' { $manifestBody.expected_managed_test_counts.core=[string]$coreCount }
            'mismatched_core_expected_count' { $manifestBody.expected_managed_test_counts.core=$coreCount + 1 }
            'mismatched_desktop_expected_count' { $manifestBody.expected_managed_test_counts.desktop=$desktopCount + 1 }
        }
    }
    $manifestPin = Write-FixtureJson ('output/native-roundtrip-pass' + $Batch + '-source.json') $manifestBody
    $targets = @([ordered]@{package='darwinbots-cli';name='darwinbots_cli';kind='lib';status='ok';passed=3;failed=0;ignored=0;filtered_out=0},
        [ordered]@{package='darwinbots-engine';name='db24832_genuine_load_transients';kind='test';status='ok';passed=3;failed=0;ignored=0;filtered_out=0})
    for ($index=2; $index -lt 67; $index++) {
        $targets += [ordered]@{package='darwinbots-engine';name=('owned_probe_' + $index);kind='test';status='ok';passed=1;failed=0;ignored=0;filtered_out=0}
    }
    if ($Case -eq 'failed_target_status') { $targets[0].status='FAILED' }
    if ($Case -eq 'failed_target_count') { $targets[0].failed=1 }
    if ($Case -eq 'duplicate_target') {
        $targets += [ordered]@{package='darwinbots-cli';name='darwinbots-cli';kind='lib';status='ok';passed=3;failed=0;ignored=0;filtered_out=0}
    }
    # Measure-Object -Property does not read OrderedDictionary keys.
    # Sum the actual target fields explicitly, just as admission validates them.
    $passed = [long]0
    foreach ($target in $targets) { $passed += [long]$target.passed }
    $cpu = [ordered]@{fresh_full_workspace_invocation=$true;hardware_filtered=$true;passed=$passed;failed=0;ignored=0;filtered_out=0;
        expected_distinct_targets=$targets.Count;total_distinct_targets=$targets.Count}
    $rustPin = Write-FixtureJson ($proof + 'rust-target-results.json') ([ordered]@{
        schema_version=1;batch=$Batch;state='passed';source_head=$(if ($Case -eq 'wrong_origin') {'2' * 40} else {$source});source_manifest=$manifestPin
        command=@('test','--release','--workspace','--no-fail-fast','--','--skip','gpu','--skip','switch','--nocapture','--test-threads=1')
        invocation=[ordered]@{fresh_full_workspace_invocation=$true;hardware_filtered=$true;exit_code=0};full_workspace_passed=$true
        expected_distinct_targets=$targets.Count;total_distinct_targets=$targets.Count
        counts=[ordered]@{passed=$passed;failed=0;ignored=0;filtered_out=0};all_targets=$targets
    })
    $coreTrxCount = if ($Case -eq 'missing_core_result_row') { $coreCount - 1 } else { $coreCount }
    $corePin = Write-FixtureTrx ($proof + 'core.trx') $coreTrxCount ($Case -eq 'failed_core_trx') ($Case -eq 'mismatched_core_trx_counters')
    $desktopPin = Write-FixtureTrx ($proof + 'desktop.trx') $desktopCount ($Case -eq 'failed_desktop_trx')
    $genuinePin = Write-FixtureJson ($proof + 'genuine-movement-comparisons.json') ([ordered]@{
        schema_version=1;batch=$Batch;source_head=$source;source_manifest_sha256=$manifestPin.sha256;passed=$true;expected_cycles=@(1,10,102);checkpoints=$originals
    })
    $core = [ordered]@{source_head=$source;fresh_invocation=$true;passed=$coreCount;failed=0;skipped=0;total=$coreCount;trx=$corePin}
    $desktop = [ordered]@{source_head=$source;fresh_invocation=$true;passed=$desktopCount;failed=0;skipped=0;total=$desktopCount;trx=$desktopPin;exact_package_native_dll=$true}
    if ($Case -eq 'wrong_core_origin') { $core.source_head='2' * 40 }
    $summaryPin = Write-FixtureJson ($proof + 'summary.json') ([ordered]@{
        schema_version=1;batch=$Batch;source_head=$source;source_manifest=$manifestPin
        all_ordinary_local_outcomes_collected=$true;local_cpu_passed=$true;source_inputs_unchanged=$true;format_exit_code=0;strict_clippy_exit_code=0
        rust_cpu=$cpu;core=$core;desktop=$desktop
        package=[ordered]@{fresh_invocation=$true;publish_exit_code=0;smoke=[ordered]@{status='passed';backend='CPU'};
            assets=48;asset_mismatches=0;native_dll_identity=[ordered]@{target_release=('A'*64);package=('A'*64);desktop=('A'*64)}}
        evidence=@($rustPin,$corePin,$desktopPin,$genuinePin)
    })
    $extraPin = Write-FixtureJson ($proof + 'runner-identity.json') ([ordered]@{scope='synthetic probe only'})
    $marker = [ordered]@{
        schema_version=5;batch=$Batch;source_head=$source;qualification_method='fresh_full_cpu_workspace_and_core_desktop_package'
        cpu_workspace_passed=$true;core_passed=$true;desktop_passed=$true;package_passed=$true;source_qualification_passed=$true
        core_fresh_invocation=$true;desktop_fresh_invocation=$true;package_fresh_invocation=$true;source_qualification_fresh_invocation=$true
        rust_cpu=$cpu;rust_result=$rustPin;summary=$summaryPin;source_manifest=$manifestPin;core=$core;desktop=$desktop;genuine_movement=$genuinePin
        source_file_count=$sourcePins.Count;source_files=$sourcePins;evidence=@($manifestPin,$rustPin,$corePin,$desktopPin,$genuinePin,$summaryPin,$extraPin)
    }
    if ($Case -eq 'missing_pin') { $marker.evidence=@($marker.evidence | Where-Object file -cne $corePin.file) }
    if ($Case -eq 'false_fresh_flag') { $marker.desktop_fresh_invocation='true' }
    if ($Case -eq 'source_alias_duplicate') {
        $marker.source_files += [ordered]@{file='modern/engine/src/../src/owned-fixture.rs';sha256=$sourcePins[0].sha256}
        $marker.source_file_count=$marker.source_files.Count
    }
    if ($Case -eq 'escape_source') {
        $marker.source_files += [ordered]@{file='../unowned-file';sha256=('A'*64)}
        $marker.source_file_count=$marker.source_files.Count
    }
    $null = Write-FixtureJson $markerName $marker
}
function Expect-Rejected([string]$Case, [string]$ExpectedMessage) {
    Write-Fixture $Case
    $rejected = $false
    try { $null = Read-NativeCpuAdmission53 $fixture $source }
    catch { if (!$_.Exception.Message.Contains($ExpectedMessage)) { throw ('Wrong rejection for ' + $Case + ': ' + $_.Exception.Message) }; $rejected=$true }
    if (!$rejected) { throw ('Invalid fresh admission unexpectedly accepted: ' + $Case) }
    $checks.Add($Case)
}
try {
    [IO.Directory]::CreateDirectory($fixture) | Out-Null
    Write-Fixture 'valid' 55
    $admitted55 = Read-NativeCpuAdmission53 $fixture $source
    if ($admitted55.marker.batch -ne 55 -or $admitted55.marker.core.total -ne 285 -or $admitted55.marker.desktop.total -ne 189) { throw 'Valid batch55 legacy census was not admitted.' }
    $checks.Add('valid_batch55_fixed_managed_census_without_manifest_counts')
    Write-Fixture
    $admitted = Read-NativeCpuAdmission53 $fixture $source
    if ($admitted.marker.schema_version -ne 5 -or $admitted.marker.batch -ne 56 -or
        $admitted.rust_result.total_distinct_targets -ne 67 -or $admitted.marker.core.total -ne 288 -or $admitted.marker.desktop.total -ne 193) {
        throw 'Valid batch56 declared managed census was not admitted.'
    }
    $checks.Add('valid_batch56_declared_managed_census_288_core_193_desktop')
    Expect-Rejected 'absent_expected_managed_counts' 'Missing or invalid integer: manifest expected_managed_test_counts.core'
    Expect-Rejected 'zero_core_expected_count' 'Missing or invalid integer: manifest expected_managed_test_counts.core'
    Expect-Rejected 'zero_desktop_expected_count' 'Missing or invalid integer: manifest expected_managed_test_counts.desktop'
    Expect-Rejected 'string_core_expected_count' 'Missing or invalid integer: manifest expected_managed_test_counts.core'
    Expect-Rejected 'mismatched_core_expected_count' 'Fresh core counts do not match the executed test set'
    Expect-Rejected 'mismatched_desktop_expected_count' 'Fresh desktop counts do not match the executed test set'
    Expect-Rejected 'missing_core_result_row' 'Fresh core TRX contains a failed, skipped, missing, or unexpected result'
    Expect-Rejected 'mismatched_core_trx_counters' 'Fresh core TRX counters do not match its executed passing results'
    Expect-Rejected 'wrong_origin' 'Fresh Rust target receipt has the wrong origin'
    Expect-Rejected 'wrong_core_origin' 'Fresh core has the wrong origin'
    Expect-Rejected 'missing_pin' 'Fresh evidence must be pinned explicitly: core TRX'
    Expect-Rejected 'failed_target_status' 'Duplicate or unsuccessful fresh Rust target'
    Expect-Rejected 'failed_target_count' 'has failed, ignored, or missing passing tests'
    Expect-Rejected 'duplicate_target' 'Duplicate or unsuccessful fresh Rust target'
    Expect-Rejected 'failed_core_trx' 'Fresh core TRX contains a failed'
    Expect-Rejected 'failed_desktop_trx' 'Fresh desktop TRX contains a failed'
    Expect-Rejected 'false_fresh_flag' 'Fresh qualification requires typed true: desktop_fresh_invocation'
    Expect-Rejected 'source_alias_duplicate' 'Duplicate canonical fresh evidence/source path'
    Expect-Rejected 'escape_source' 'Fresh pin escapes the checkout'
    Write-Fixture
    [IO.File]::AppendAllText((Get-FixturePath 'modern/engine/src/owned-fixture.rs'), 'changed-source')
    $rejected = $false
    try { $null = Read-NativeCpuAdmission53 $fixture $source }
    catch { if (!$_.Exception.Message.Contains('Fresh evidence/source pin changed')) { throw }; $rejected=$true }
    if (!$rejected) { throw 'Changed source was admitted.' }
    $checks.Add('changed_source')
    Write-Fixture
    $lease = Enter-NativeQualificationLease53 $fixture $source 'test-first'
    try {
        $collided = $false
        try { $second=Enter-NativeQualificationLease53 $fixture $source 'test-second';$second.Dispose() }
        catch { if (!$_.Exception.Message.Contains('exclusive lease')) { throw };$collided=$true }
        if (!$collided) { throw 'Concurrent qualification lease was admitted.' }
    } finally { $lease.Dispose() }
    $after = Enter-NativeQualificationLease53 $fixture $source 'test-after-release'
    $after.Dispose()
    $checks.Add('exclusive_lease_collision_and_release')
    [ordered]@{status='passed';checks=$checks.ToArray();count=$checks.Count;scope='CPU-only synthetic receipt/lease probes in an isolated owned Temp directory; no real workspace marker, build, Git, UI, native helper or GPU.'} | ConvertTo-Json -Depth 4
} finally {
    $resolvedFixture=[IO.Path]::GetFullPath($fixture)
    if (!$resolvedFixture.StartsWith($tempParent + [IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase) -or
        ![IO.Path]::GetFileName($resolvedFixture).StartsWith('DbNativeAdmission56-',[StringComparison]::Ordinal)) { throw 'Refusing cleanup outside verified owned Temp.' }
    if (Test-Path -LiteralPath $resolvedFixture) { Remove-Item -LiteralPath $resolvedFixture -Recurse -Force }
}
