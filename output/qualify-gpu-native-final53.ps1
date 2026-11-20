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
$admissionHelper50 = Join-Path $PSScriptRoot 'native-qualification-admission53.ps1'
. $admissionHelper50
$admission50 = Read-NativeCpuAdmission53 $repo $source
$ready = $admission50.path
$qualification = $admission50.marker
$admissionHelperHash50 = (Get-FileHash -LiteralPath $admissionHelper50 -Algorithm SHA256).Hash
$runnerHash50 = (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash
$runnerPath50 = $PSCommandPath
$qualificationLease50 = Enter-NativeQualificationLease53 $repo $source 'gpu'
$guardPaths50 = @('modern','Cargo.toml','Cargo.lock','.cargo','Installer/bots','Installer/sets/Simple Simulation.set','Darwinbots2/DNATokenizing.bas','.gitattributes','.github/workflows/modern-cross-platform.yml')
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
function Read-SealedGpuLog([string]$path, [int]$timeoutMilliseconds = 10000) {
    $clock = [Diagnostics.Stopwatch]::StartNew()
    $attempts = 0
    $lastError = $null
    do {
        $attempts++
        $stream = $null
        $memory = $null
        $capture = $null
        $readError = $null
        try {
            # Allow other readers, but require every inherited writer to close.
            # Read, decode and hash the same bytes while this handle excludes writers.
            $stream = [IO.File]::Open($path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
            $memory = [IO.MemoryStream]::new()
            $stream.CopyTo($memory)
            $bytes = $memory.ToArray()
            $text = [Text.UTF8Encoding]::new($false, $true).GetString($bytes)
            $capture = [ordered]@{
                status='captured';text=$text;sha256=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes))
                bytes=$bytes.Length;writer_quiescent=$true;error=$null
            }
        } catch { $readError = $_.Exception.GetBaseException() } finally {
            foreach ($resource in @($memory, $stream)) {
                if ($null -ne $resource) {
                    try { $resource.Dispose() } catch {
                        if ($null -eq $readError) { $readError = $_.Exception.GetBaseException() }
                    }
                }
            }
        }
        if ($null -eq $readError) {
            $capture['attempts'] = $attempts
            $capture['elapsed_milliseconds'] = $clock.ElapsedMilliseconds
            return $capture
        }
        $lastError = $readError.Message
        $nativeError = $readError.HResult -band 0xffff
        if ($nativeError -notin @(32, 33)) {
            $captureStatus = if ($nativeError -in @(2, 3)) { 'not_created' } else { 'unavailable' }
            return [ordered]@{status=$captureStatus;text=$null;sha256=$null;bytes=$null;writer_quiescent=$false;error=$lastError;attempts=$attempts;elapsed_milliseconds=$clock.ElapsedMilliseconds}
        }
        if ($clock.ElapsedMilliseconds -lt $timeoutMilliseconds) { Start-Sleep -Milliseconds 100 }
    } while ($clock.ElapsedMilliseconds -lt $timeoutMilliseconds)
    return [ordered]@{status='still_locked';text=$null;sha256=$null;bytes=$null;writer_quiescent=$false;error=$lastError;attempts=$attempts;elapsed_milliseconds=$clock.ElapsedMilliseconds}
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
        $executionError = $null
        $guardErrors = [Collections.Generic.List[string]]::new()
        $cleanupErrors = [Collections.Generic.List[string]]::new()
        $logErrors = [Collections.Generic.List[string]]::new()
        $observationErrors = [Collections.Generic.List[string]]::new()
        $stateErrors = [Collections.Generic.List[string]]::new()
        $binaryError = $null
        $interrupted = $false
        $timedOut = $false
        $stopReason = $null
        $stopObservedUtc = $null
        $inputTickAtStop = $null
        $inputObservedUtc = $null
        $finalInputTick = $null
        $idleAfter = $null
        $operation = 'launch'
        try {
            $process = Start-Process -FilePath $cargo -ArgumentList $arguments -WorkingDirectory $repo -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
            $null = $process.Handle
            $ownedPid = $process.Id
            try { $process.PriorityClass = [Diagnostics.ProcessPriorityClass]::BelowNormal } catch {
                if (!$process.HasExited) { throw }
            }
            Save-State 'running' $label @{owned_cargo_pid=$ownedPid;input_tick=$inputTick;idle_before_ms=$idleBefore;executable=$cargo;arguments=$arguments;minimum_passes=$minimumPasses;stdout=$stdout;stderr=$stderr}
            while (!$process.HasExited) {
                $operation = 'guard'
                Check-Guards
                $operation = 'input_observation'
                $observedInputTick = [DbParityGpuInput]::Tick()
                if ($observedInputTick -ne $inputTick) {
                    $interrupted = $true
                    $inputTickAtStop = $observedInputTick
                    $stopReason = 'input'
                    $inputObservedUtc = [DateTime]::UtcNow.ToString('o')
                    $stopObservedUtc = $inputObservedUtc
                    try { Save-State 'stopping_for_input' $label @{owned_cargo_pid=$ownedPid;input_tick_before=$inputTick;input_tick_at_stop=$inputTickAtStop;observed_utc=$inputObservedUtc;stdout=$stdout;stderr=$stderr} }
                        catch { $stateErrors.Add($_.Exception.Message) }
                    break
                }
                if ($clock.Elapsed.TotalSeconds -gt $timeoutSeconds) {
                    $timedOut = $true
                    $stopReason = 'timeout'
                    $stopObservedUtc = [DateTime]::UtcNow.ToString('o')
                    try { Save-State 'stopping_for_timeout' $label @{owned_cargo_pid=$ownedPid;observed_utc=$stopObservedUtc;stdout=$stdout;stderr=$stderr} }
                        catch { $stateErrors.Add($_.Exception.Message) }
                    break
                }
                Start-Sleep -Milliseconds 500
                $operation = 'process_observation'
                $process.Refresh()
            }
        } catch {
            if ($operation -eq 'guard') { $guardErrors.Add($_.Exception.Message) }
            else { $executionError = $_.Exception.Message }
            if (!$stopReason) {
                $stopReason = if ($operation -eq 'guard') { 'guard' } else { 'runner' }
                $stopObservedUtc = [DateTime]::UtcNow.ToString('o')
            }
            try { Save-State 'stopping_for_error' $label @{owned_cargo_pid=$ownedPid;stop_reason=$stopReason;observed_utc=$stopObservedUtc;guard_errors=$guardErrors.ToArray();execution_error=$executionError;stdout=$stdout;stderr=$stderr} }
                catch { $stateErrors.Add($_.Exception.Message) }
        } finally {
            # Capture input before cleanup or log I/O can obscure the stop reason.
            try {
                $finalInputTick = [DbParityGpuInput]::Tick()
                $idleAfter = [DbParityGpuInput]::IdleMs()
                if ($finalInputTick -ne $inputTick -or $idleAfter -lt $IdleSeconds*1000) {
                    $interrupted = $true
                    if ($null -eq $inputTickAtStop) { $inputTickAtStop = $finalInputTick }
                    if ($null -eq $inputObservedUtc) { $inputObservedUtc = [DateTime]::UtcNow.ToString('o') }
                    if (!$stopReason) { $stopReason='input'; $stopObservedUtc=[DateTime]::UtcNow.ToString('o') }
                    try { Save-State 'stopping_for_input' $label @{owned_cargo_pid=$ownedPid;input_tick_before=$inputTick;input_tick_at_stop=$inputTickAtStop;stop_reason=$stopReason;observed_utc=$inputObservedUtc;stdout=$stdout;stderr=$stderr} }
                        catch { $stateErrors.Add($_.Exception.Message) }
                }
            } catch { $observationErrors.Add($_.Exception.Message) }
            if ($null -ne $process) {
                try { Stop-Owned $process } catch { $cleanupErrors.Add($_.Exception.Message) }
                try { $exitCode = $process.ExitCode } catch { $cleanupErrors.Add($_.Exception.Message) }
                try { $process.Dispose() } catch { $cleanupErrors.Add($_.Exception.Message) }
            }
            $clock.Stop()
        }
        # Native Windows inherited writers can outlive Cargo's process handle.
        # A capture timeout is evidence loss, never permission to hash an active log.
        $stdoutCapture = Read-SealedGpuLog $stdout
        $stderrCapture = Read-SealedGpuLog $stderr
        foreach ($capture in @(@{name='stdout';value=$stdoutCapture}, @{name='stderr';value=$stderrCapture})) {
            if ($capture.value.status -ne 'captured') { $logErrors.Add($capture.name+': '+$capture.value.error) }
        }
        $testText = if ($null -ne $stdoutCapture.text) { $stdoutCapture.text } else { '' }
        $errorText = if ($null -ne $stderrCapture.text) { $stderrCapture.text } else { '' }
        $passes = 0
        foreach ($match in [regex]::Matches($testText, 'test result: ok\. (\d+) passed;')) { $passes += [int]$match.Groups[1].Value }
        $terminalSummaries = [regex]::Matches($testText, '(?m)^\s*test result: (?:ok|FAILED)\. \d+ passed;').Count
        $binaries = @()
        try { $binaries = @(Get-TestBinaryHashes ($testText + [Environment]::NewLine + $errorText)) }
            catch { $binaryError = $_.Exception.Message }
        # Always perform final guards/input observation even if no logs were readable.
        try { Check-Guards } catch { $guardErrors.Add($_.Exception.Message) }
        try {
            $finalInputTick = [DbParityGpuInput]::Tick()
            $idleAfter = [DbParityGpuInput]::IdleMs()
            if ($finalInputTick -ne $inputTick -or $idleAfter -lt $IdleSeconds*1000) {
                $interrupted = $true
                if ($null -eq $inputTickAtStop) { $inputTickAtStop = $finalInputTick }
                if ($null -eq $inputObservedUtc) { $inputObservedUtc = [DateTime]::UtcNow.ToString('o') }
                if (!$stopReason) { $stopReason='input'; $stopObservedUtc=[DateTime]::UtcNow.ToString('o') }
            }
        } catch { $observationErrors.Add($_.Exception.Message) }
        if ($clock.Elapsed.TotalSeconds -gt $timeoutSeconds) { $timedOut = $true }
        $errors = @($guardErrors.ToArray()) + @($cleanupErrors.ToArray()) + @($logErrors.ToArray()) + @($observationErrors.ToArray()) + @($stateErrors.ToArray())
        if ($executionError) { $errors += $executionError }
        if ($binaryError) { $errors += $binaryError }
        $failure = if ($errors.Count) { $errors -join '; ' } else { $null }
        $passed = !$failure -and !$interrupted -and !$timedOut -and $exitCode -eq 0 -and
            $passes -ge $minimumPasses -and $terminalSummaries -gt 0 -and $binaries.Count -gt 0 -and
            $stdoutCapture.status -eq 'captured' -and $stderrCapture.status -eq 'captured'
        $status = if ($interrupted) { 'interrupted_by_input' } elseif ($guardErrors.Count) { 'guard_or_provenance_failed' }
            elseif ($failure) { 'runner_or_evidence_failed' } elseif ($passed) { 'passed' } else { 'failed' }
        if (!$stopReason) {
            $stopReason = if ($guardErrors.Count) { 'guard' } elseif ($failure) { 'runner_or_evidence' }
                elseif ($timedOut) { 'timeout' } else { 'process_exit' }
            $stopObservedUtc = [DateTime]::UtcNow.ToString('o')
        }
        $result = [ordered]@{
            source_head=$source;status=$status;task=$label;attempt=$attempt;exit_code=$exitCode
            executable=$cargo;executable_sha256=$cargoHash;arguments=$arguments;minimum_passes=$minimumPasses
            timed_out=$timedOut;timeout_seconds=$timeoutSeconds;elapsed_process_seconds=$clock.Elapsed.TotalSeconds
            passed_cases=$passes;terminal_harness_summary_present=($terminalSummaries -gt 0);terminal_harness_summary_count=$terminalSummaries
            test_binaries=$binaries;stdout=$stdout;stdout_sha256=$stdoutCapture.sha256;stderr=$stderr;stderr_sha256=$stderrCapture.sha256
            stdout_capture=[ordered]@{status=$stdoutCapture.status;bytes=$stdoutCapture.bytes;writer_quiescent=$stdoutCapture.writer_quiescent;error=$stdoutCapture.error;attempts=$stdoutCapture.attempts;elapsed_milliseconds=$stdoutCapture.elapsed_milliseconds}
            stderr_capture=[ordered]@{status=$stderrCapture.status;bytes=$stderrCapture.bytes;writer_quiescent=$stderrCapture.writer_quiescent;error=$stderrCapture.error;attempts=$stderrCapture.attempts;elapsed_milliseconds=$stderrCapture.elapsed_milliseconds}
            interrupted_by_input=$interrupted;input_tick_before=$inputTick;input_tick_at_stop=$inputTickAtStop;input_observed_utc=$inputObservedUtc;input_tick_after=$finalInputTick;idle_before_ms=$idleBefore;idle_after_ms=$idleAfter
            stop_reason=$stopReason;stop_observed_utc=$stopObservedUtc
            guard_error=($guardErrors.ToArray() -join '; ');cleanup_errors=$cleanupErrors.ToArray();log_errors=$logErrors.ToArray()
            execution_error=$executionError;binary_error=$binaryError;input_observation_errors=$observationErrors.ToArray();state_errors=$stateErrors.ToArray()
            required_idle_seconds=$IdleSeconds;priority='BelowNormal';rayon_threads=1;owned_cargo_pid=$ownedPid
            error=$failure;started_utc=$started.ToString('o');finished_utc=[DateTime]::UtcNow.ToString('o')
            log_provenance='Writer-quiescent read handle excludes writers; text and SHA256 from the same captured bytes. Missing/locked/error captures have no immutable hash. A killed or summary-less log is partial evidence only.'
            binary_provenance='Fresh Cargo test invocation at the verified exact source; SHA256 of Cargo-reported executed test binaries after exit; no cached receipt reuse'
        }
        $attemptReceipt = Join-Path $evidence ($base+'.receipt.json')
        # Log/cleanup failures cannot bypass this receipt attempt or erase input.
        Write-NewReceipt $attemptReceipt $result
        if ($interrupted) {
            Save-State 'interrupted_by_input' $label @{receipt=$attemptReceipt;stdout=$stdout;stderr=$stderr;stop_reason=$stopReason;secondary_errors=$errors}
            throw ('New Windows input interrupted GPU qualification: ' + $label)
        }
        if ($failure) { throw $failure }
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
