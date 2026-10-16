param(
    [Parameter(Mandatory=$true)][string]$SourceCommit,
    [int]$IdleSeconds = 300,
    [ValidateSet('physical-library','physical-cli','physical-integration','physical-native-continuation','portable-gpu','backend-switches')]
    [string[]]$Groups = @('physical-library','physical-cli','physical-integration','physical-native-continuation','portable-gpu','backend-switches')
)
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
[Diagnostics.Process]::GetCurrentProcess().PriorityClass = 'BelowNormal'
$env:CARGO_BUILD_JOBS = '1'
$env:RUST_TEST_THREADS = '1'
$env:RAYON_NUM_THREADS = '1'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$source = $SourceCommit
if ($source -notmatch '^[0-9a-f]{40}$') { throw 'An exact qualified commit is required.' }
if ($IdleSeconds -lt 300) { throw 'GPU work requires at least five minutes of verified idle.' }
if (!$Groups.Count -or @($Groups | Sort-Object -Unique).Count -ne $Groups.Count) { throw 'GPU groups must be nonempty and unique.' }
$evidenceRoot = Join-Path $repo ('docs/parity/evidence/batch-2026-10-14/gpu-' + $source.Substring(0,12))
[IO.Directory]::CreateDirectory($evidenceRoot) | Out-Null
$admissionHelper50 = Join-Path $PSScriptRoot 'native-qualification-admission50.ps1'
. $admissionHelper50
$admission50 = Read-NativeCpuAdmission50 $repo $source
$ready = $admission50.path
$qualification = $admission50.marker
$admissionHelperHash50 = (Get-FileHash -LiteralPath $admissionHelper50 -Algorithm SHA256).Hash
$runnerHash50 = (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash
$runnerPath50 = $PSCommandPath
$qualificationLease50 = Enter-NativeQualificationLease50 $repo $source 'gpu'
$guardPaths50 = @('modern','Cargo.toml','Cargo.lock','.cargo','Installer/bots','Installer/sets/Simple Simulation.set','Darwinbots2/DNATokenizing.bas')
$runName = 'run-' + [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfff') + '-' + [Guid]::NewGuid().ToString('N')
$evidence = (New-Item -Path $evidenceRoot -Name $runName -ItemType Directory -ErrorAction Stop).FullName
$stopMarkers = @((Join-Path $evidenceRoot 'STOP'), (Join-Path $evidence 'STOP'))
$cargo = (Get-Command cargo.exe -ErrorAction Stop).Source
$cargoHash = (Get-FileHash -LiteralPath $cargo -Algorithm SHA256).Hash
Write-Output ('Fresh GPU evidence: ' + $evidence)
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class DbParityGpuInput {
 [StructLayout(LayoutKind.Sequential)] public struct Info { public uint cbSize; public uint time; }
 [DllImport("user32.dll")] static extern bool GetLastInputInfo(ref Info info);
 public static uint Tick() { var i = new Info { cbSize=(uint)Marshal.SizeOf<Info>() }; if (!GetLastInputInfo(ref i)) throw new InvalidOperationException("Last input unavailable"); return i.time; }
 public static uint IdleMs() => unchecked((uint)Environment.TickCount-Tick());
}
'@
function Save-State($status, $task, $extra) {
    [ordered]@{source_head=$source;status=$status;task=$task;pid=$PID;utc=[DateTime]::UtcNow.ToString('o');details=$extra} |
        ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $evidence 'state.json') -Encoding utf8
}
function Check-Source {
    if ((Get-FileHash -LiteralPath $ready -Algorithm SHA256).Hash -ne $admission50.sha256) { throw 'CPU admission marker changed.' }
    if ((Get-FileHash -LiteralPath $admissionHelper50 -Algorithm SHA256).Hash -ne $admissionHelperHash50 -or (Get-FileHash -LiteralPath $runnerPath50 -Algorithm SHA256).Hash -ne $runnerHash50) { throw 'Qualification runner changed.' }
    foreach ($pin50 in $qualification.evidence) { if ((Get-FileHash -LiteralPath (Join-Path $repo $pin50.file) -Algorithm SHA256).Hash -ne $pin50.sha256) { throw ('Sealed qualification evidence changed: '+$pin50.file) } }
    $head = git -C $repo rev-parse HEAD
    if ($LASTEXITCODE -ne 0 -or $head -ne $source) { throw 'The checkout no longer matches the qualified commit.' }
    git -C $repo diff --quiet $source -- @guardPaths50
    if ($LASTEXITCODE -ne 0) { throw 'Engine sources changed; GPU qualification stopped.' }
    $untracked = @(git -C $repo ls-files --others --exclude-standard -- @guardPaths50)
    if ($LASTEXITCODE -ne 0 -or $untracked.Count -ne 0) { throw 'Unqualified untracked source found.' }
    if ((Get-FileHash -LiteralPath $cargo -Algorithm SHA256).Hash -ne $cargoHash) { throw 'Pinned Cargo executable changed.' }
}
function Check-OwnedStop {
    foreach ($marker in $stopMarkers) {
        if (Test-Path -LiteralPath $marker) { throw ('Owned GPU stop marker found: ' + $marker) }
    }
}
function Check-Guards {
    Check-OwnedStop
    Check-Source
    Check-OwnedStop
}
function Stop-Owned($process) {
    if ($null -eq $process) { return }
    $process.Refresh()
    if (!$process.HasExited) {
        try { $process.Kill($true) } catch {
            $process.Refresh()
            if (!$process.HasExited) { throw }
        }
    }
    if (!$process.WaitForExit(10000)) { throw 'Owned GPU process tree did not exit after termination.' }
}
function Write-NewReceipt([string]$path, $value) {
    $stream = [IO.File]::Open($path, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::Read)
    $writer = [IO.StreamWriter]::new($stream, [Text.UTF8Encoding]::new($false))
    try { $writer.WriteLine(($value | ConvertTo-Json -Depth 8)) } finally { $writer.Dispose() }
}
function Get-TestBinaryHashes([string]$testText) {
    $clean = [regex]::Replace($testText, '\x1B\[[0-?]*[ -/]*[@-~]', '')
    $paths = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($match in [regex]::Matches($clean, '(?m)^\s*Running\s+[^\r\n]*\((?<binary>[^\r\n]+)\)\s*$')) {
        $path = $match.Groups['binary'].Value
        $path = if ([IO.Path]::IsPathRooted($path)) { [IO.Path]::GetFullPath($path) }
            else { [IO.Path]::GetFullPath((Join-Path $repo $path)) }
        if ($paths.Add($path)) {
            [ordered]@{path=$path;sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash}
        }
    }
}
function Run-GpuTask($label, [string[]]$arguments, [int]$minimumPasses, [int]$timeoutSeconds) {
    # A fresh invocation never accepts cached qualification receipts.
    Check-Guards
    for ($attempt=1; ; $attempt++) {
        Check-Guards
        while ([DbParityGpuInput]::IdleMs() -lt $IdleSeconds*1000) {
            Save-State 'waiting_for_idle' $label @{idle_seconds=[Math]::Round([DbParityGpuInput]::IdleMs()/1000,1)}
            Check-OwnedStop
            Start-Sleep -Seconds 2
        }
        Check-Guards
        # Git checks can take time. Input during them must not become a fresh
        # accepted baseline that bypasses the required five minutes of idle.
        $inputTick = [DbParityGpuInput]::Tick()
        $idleBefore = [DbParityGpuInput]::IdleMs()
        if ($idleBefore -lt $IdleSeconds*1000) { continue }
        $base = $label+'-'+[DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfff')+'-'+$attempt
        $stdout = Join-Path $evidence ($base+'.stdout.log')
        $stderr = Join-Path $evidence ($base+'.stderr.log')
        $started = [DateTime]::UtcNow
        $clock = [Diagnostics.Stopwatch]::StartNew()
        $process = $null
        $exitCode = $null
        $ownedPid = $null
        $failure = $null
        $interrupted = $false
        $timedOut = $false
        try {
            $process = Start-Process -FilePath $cargo -ArgumentList $arguments -WorkingDirectory $repo -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
            $null = $process.Handle
            $ownedPid = $process.Id
            try { $process.PriorityClass = [Diagnostics.ProcessPriorityClass]::BelowNormal } catch {
                if (!$process.HasExited) { throw }
            }
            Save-State 'running' $label @{owned_cargo_pid=$ownedPid;input_tick=$inputTick;idle_before_ms=$idleBefore;executable=$cargo;arguments=$arguments;minimum_passes=$minimumPasses;stdout=$stdout;stderr=$stderr}
            while (!$process.HasExited) {
                Check-Guards
                if ([DbParityGpuInput]::Tick() -ne $inputTick) { $interrupted=$true; break }
                if ($clock.Elapsed.TotalSeconds -gt $timeoutSeconds) { $timedOut=$true; break }
                Start-Sleep -Milliseconds 500
                $process.Refresh()
            }
        } catch { $failure = $_.Exception.Message } finally {
            if ($null -ne $process) {
                Stop-Owned $process
                $exitCode = $process.ExitCode
                $process.Dispose()
            }
            $clock.Stop()
        }
        $testText = if (Test-Path -LiteralPath $stdout) { Get-Content -LiteralPath $stdout -Raw } else { '' }
        $errorText = if (Test-Path -LiteralPath $stderr) { Get-Content -LiteralPath $stderr -Raw } else { '' }
        $passes = 0
        foreach ($match in [regex]::Matches($testText, 'test result: ok\. (\d+) passed;')) { $passes += [int]$match.Groups[1].Value }
        $binaries = @()
        try { $binaries = @(Get-TestBinaryHashes ($testText + [Environment]::NewLine + $errorText)) }
            catch { $failure = $_.Exception.Message }
        $stdoutHash = if (Test-Path -LiteralPath $stdout) { (Get-FileHash -LiteralPath $stdout -Algorithm SHA256).Hash } else { $null }
        $stderrHash = if (Test-Path -LiteralPath $stderr) { (Get-FileHash -LiteralPath $stderr -Algorithm SHA256).Hash } else { $null }
        # The final poll also covers input/source/STOP changes after the child
        # exits during its final sleep or during evidence collection.
        try { Check-Guards } catch { $failure = $_.Exception.Message }
        $finalInputTick = [DbParityGpuInput]::Tick()
        $idleAfter = [DbParityGpuInput]::IdleMs()
        if ($finalInputTick -ne $inputTick -or $idleAfter -lt $IdleSeconds*1000) { $interrupted = $true }
        if ($clock.Elapsed.TotalSeconds -gt $timeoutSeconds) { $timedOut = $true }
        $passed = !$failure -and !$interrupted -and !$timedOut -and $exitCode -eq 0 -and
            $passes -ge $minimumPasses -and $binaries.Count -gt 0 -and $stdoutHash -and $stderrHash
        $status = if ($failure) { 'guard_or_provenance_failed' } elseif ($interrupted) { 'interrupted_by_input' }
            elseif ($passed) { 'passed' } else { 'failed' }
        $result = [ordered]@{
            source_head=$source;status=$status;task=$label;attempt=$attempt;exit_code=$exitCode
            executable=$cargo;executable_sha256=$cargoHash;arguments=$arguments;minimum_passes=$minimumPasses
            timed_out=$timedOut;timeout_seconds=$timeoutSeconds;elapsed_process_seconds=$clock.Elapsed.TotalSeconds
            passed_cases=$passes;test_binaries=$binaries;stdout=$stdout;stdout_sha256=$stdoutHash;stderr=$stderr;stderr_sha256=$stderrHash
            input_tick_before=$inputTick;input_tick_after=$finalInputTick;idle_before_ms=$idleBefore;idle_after_ms=$idleAfter
            required_idle_seconds=$IdleSeconds;priority='BelowNormal';rayon_threads=1;owned_cargo_pid=$ownedPid
            error=$failure;started_utc=$started.ToString('o');finished_utc=[DateTime]::UtcNow.ToString('o')
            binary_provenance='Fresh Cargo test invocation at the verified exact source; SHA256 of Cargo-reported executed test binaries after exit; no cached receipt reuse'
        }
        $attemptReceipt = Join-Path $evidence ($base+'.receipt.json')
        Write-NewReceipt $attemptReceipt $result
        if ($failure) { throw $failure }
        if ($interrupted) {
            Save-State 'interrupted_by_input' $label @{receipt=$attemptReceipt;stdout=$stdout;stderr=$stderr}
            throw ('New Windows input interrupted GPU qualification: ' + $label)
        }
        Write-NewReceipt (Join-Path $evidence ($label+'.json')) $result
        # Ordinary test failure, insufficient passes, or timeout is a collected
        # outcome. Source/input/STOP/provenance failures above stop the run.
        return $result
    }
}
$allPlans = @(
    @{label='physical-library';arguments=@('test','--release','-p','darwinbots-engine','--lib','--no-fail-fast','--','--ignored','--nocapture','--test-threads=1');minimum_passes=7;timeout_seconds=1800},
    @{label='physical-cli';arguments=@('test','--release','-p','darwinbots-cli','--lib','--no-fail-fast','--','--ignored','--nocapture','--test-threads=1');minimum_passes=1;timeout_seconds=1800},
    @{label='physical-integration';arguments=@('test','--release','-p','darwinbots-engine','--test','db24832_starter_movement','--test','gpu_ffi','--test','w4_evolution_gpu','--no-fail-fast','--','--ignored','--nocapture','--test-threads=1');minimum_passes=3;timeout_seconds=1800},
    @{label='physical-native-continuation';arguments=@('test','--release','-p','darwinbots-engine','--test','native_continuation_qualification','--no-fail-fast','--','--ignored','--nocapture','--test-threads=1');minimum_passes=1;timeout_seconds=7200},
    @{label='portable-gpu';arguments=@('test','--release','--workspace','--no-fail-fast','gpu','--','--nocapture','--test-threads=1');minimum_passes=1;timeout_seconds=3600},
    @{label='backend-switches';arguments=@('test','--release','--workspace','--no-fail-fast','switch','--','--nocapture','--test-threads=1');minimum_passes=1;timeout_seconds=1800}
)
$plans = @($allPlans | Where-Object { $Groups -contains $_.label })
$results = [Collections.Generic.List[object]]::new()
$guardFailure = $null
$activeTask = $null
try {
    Check-Guards
    Write-NewReceipt (Join-Path $evidence 'run.json') ([ordered]@{source_head=$source;cpu_ready=$ready;cpu_ready_sha256=(Get-FileHash -LiteralPath $ready -Algorithm SHA256).Hash;cargo=$cargo;cargo_sha256=$cargoHash;runner_sha256=$runnerHash50;admission_helper_sha256=$admissionHelperHash50;guard_paths=$guardPaths50;exclusive_lease='output/native-qualification50.lock';stop_markers=$stopMarkers;idle_seconds=$IdleSeconds;selected_groups=@($plans | ForEach-Object {$_.label});created_utc=[DateTime]::UtcNow.ToString('o');cache_reuse=$false})
    foreach ($plan in $plans) {
        $activeTask = $plan.label
        $result = Run-GpuTask $plan.label $plan.arguments $plan.minimum_passes $plan.timeout_seconds
        $results.Add($result)
        $activeTask = $null
    }
    Check-Guards
} catch {
    $guardFailure = $_.Exception.Message
}
# Always distinguish completed outcomes from tasks not admitted after a guard.
foreach ($plan in $plans) {
    if (!($results | Where-Object { $_.task -eq $plan.label })) {
        $attempt = Get-ChildItem -LiteralPath $evidence -File -Filter ($plan.label+'-*.receipt.json') |
            Sort-Object Name -Descending | Select-Object -First 1
        $unrunStatus = if ($activeTask -eq $plan.label) { 'guard_or_runner_failed' } else { 'not_run_after_guard_failure' }
        $result = if ($null -ne $attempt) { Get-Content -LiteralPath $attempt.FullName -Raw | ConvertFrom-Json }
            else { [ordered]@{source_head=$source;task=$plan.label;status=$unrunStatus;error=$guardFailure;arguments=$plan.arguments;minimum_passes=$plan.minimum_passes;timeout_seconds=$plan.timeout_seconds} }
        $results.Add($result)
    }
}
$failed = @($results | Where-Object { $_.status -ne 'passed' })
$status = if ($guardFailure) { 'stopped' } elseif ($failed.Count) { 'needs_repair' } else { 'completed' }
Write-NewReceipt (Join-Path $evidence 'summary.json') ([ordered]@{
    source_head=$source;status=$status;planned_groups=$plans.Count;outcome_count=$results.Count
    full_plan_groups=$allPlans.Count;selected_groups=@($plans | ForEach-Object {$_.label})
    unselected_groups=@($allPlans | Where-Object {$Groups -notcontains $_.label} | ForEach-Object {$_.label})
    cpu_ready=$ready;cpu_ready_sha256=(Get-FileHash -LiteralPath $ready -Algorithm SHA256).Hash
    guard_failure=$guardFailure;failed_groups=@($failed | ForEach-Object {$_.task});groups=$results.ToArray()
    completed_utc=[DateTime]::UtcNow.ToString('o')
    collection_policy='Collect all selected groups and all Cargo targets after ordinary test failures; stop immediately on input/source/STOP/runner/provenance failure; no input retry; unselected groups have no fresh outcome'
})
Save-State $status 'all' @{outcome_count=$results.Count;failed_groups=@($failed | ForEach-Object {$_.task});guard_failure=$guardFailure}
if ($guardFailure) { throw $guardFailure }
if ($failed.Count) {
    throw ('All GPU groups collected with ' + $failed.Count + ' failed group(s). See ' + (Join-Path $evidence 'summary.json'))
}
