param([Parameter(Mandatory=$true)][string]$SourceCommit, [int]$IdleSeconds = 300)
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
$evidenceRoot = Join-Path $repo ('docs/parity/evidence/batch-2026-10-14/gpu-' + $source.Substring(0,12))
[IO.Directory]::CreateDirectory($evidenceRoot) | Out-Null
$ready = Join-Path $evidenceRoot 'cpu-ready.json'
if (!(Test-Path -LiteralPath $ready)) { throw 'Current CPU qualification marker is required before GPU work.' }
$qualification = Get-Content -LiteralPath $ready -Raw | ConvertFrom-Json
if ($qualification.source_head -ne $source -or $qualification.cpu_workspace_passed -ne $true -or $qualification.desktop_passed -ne $true -or $qualification.package_passed -ne $true) {
    throw 'The exact source must pass CPU, desktop and package checks before GPU work.'
}
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
    $head = git -C $repo rev-parse HEAD
    if ($LASTEXITCODE -ne 0 -or $head -ne $source) { throw 'The checkout no longer matches the qualified commit.' }
    git -C $repo diff --quiet $source -- modern Cargo.toml Cargo.lock .cargo
    if ($LASTEXITCODE -ne 0) { throw 'Engine sources changed; GPU qualification stopped.' }
    $untracked = @(git -C $repo ls-files --others --exclude-standard -- modern Cargo.toml Cargo.lock .cargo)
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
        if ($interrupted) { Save-State 'interrupted_by_input' $label @{receipt=$attemptReceipt;stdout=$stdout;stderr=$stderr}; continue }
        Write-NewReceipt (Join-Path $evidence ($label+'.json')) $result
        if (!$passed) { throw ('GPU qualification failed: '+$label) }
        return
    }
}
try {
    Check-Guards
    Write-NewReceipt (Join-Path $evidence 'run.json') ([ordered]@{source_head=$source;cpu_ready=$ready;cpu_ready_sha256=(Get-FileHash -LiteralPath $ready -Algorithm SHA256).Hash;cargo=$cargo;cargo_sha256=$cargoHash;stop_markers=$stopMarkers;idle_seconds=$IdleSeconds;created_utc=[DateTime]::UtcNow.ToString('o');cache_reuse=$false})
    Run-GpuTask 'physical-library' @('test','--release','-p','darwinbots-engine','--lib','--','--ignored','--nocapture','--test-threads=1') 7 1800
    Run-GpuTask 'physical-cli' @('test','--release','-p','darwinbots-cli','--lib','--','--ignored','--nocapture','--test-threads=1') 1 1800
    Run-GpuTask 'physical-integration' @('test','--release','-p','darwinbots-engine','--test','db24832_starter_movement','--test','gpu_ffi','--test','w4_evolution_gpu','--','--ignored','--nocapture','--test-threads=1') 3 1800
    Run-GpuTask 'physical-native-continuation' @('test','--release','-p','darwinbots-engine','--test','native_continuation_qualification','--','--ignored','--nocapture','--test-threads=1') 1 7200
    Run-GpuTask 'portable-gpu' @('test','--release','--workspace','gpu','--','--nocapture','--test-threads=1') 1 3600
    Run-GpuTask 'backend-switches' @('test','--release','--workspace','switch','--','--nocapture','--test-threads=1') 1 1800
    Check-Guards
    Save-State 'completed' 'all' @{physical_cases=12;portable_and_switch_groups='passed'}
} catch {
    Save-State 'stopped' 'qualification' @{error=$_.Exception.Message}
    throw
}
