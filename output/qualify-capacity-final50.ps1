#requires -Version 7.0
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[0-9a-fA-F]{40}$')]
    [string]$SourceCommit,
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[0-9a-fA-F]{64}$')]
    [string]$QualifiedCliSha256,
    [ValidateSet('Cpu', 'Gpu')]
    [string]$Backend = 'Cpu',
    [ValidateNotNullOrEmpty()]
    [ValidateSet('1000', '4000', '8000', '32000')]
    [int[]]$Populations = @(1000, 4000, 8000, 32000),
    [ValidateRange(300, 86400)]
    [int]$IdleSeconds = 300
)

$ErrorActionPreference = 'Stop'
$Backend = if ($Backend -ieq 'Gpu') { 'Gpu' } else { 'Cpu' }
$backendArgument = $Backend.ToLowerInvariant()
if (@($Populations | Sort-Object -Unique).Count -ne $Populations.Count) {
    throw 'Populations must not contain duplicates.'
}
[Diagnostics.Process]::GetCurrentProcess().PriorityClass = 'BelowNormal'
$env:RAYON_NUM_THREADS = '1'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$source = $SourceCommit.ToLowerInvariant()
$admissionHelper50 = Join-Path $PSScriptRoot 'native-qualification-admission50.ps1'
. $admissionHelper50
$admission50 = Read-NativeCpuAdmission50 $repo $source
$admissionHelperHash50 = (Get-FileHash -LiteralPath $admissionHelper50 -Algorithm SHA256).Hash
$runnerHash50 = (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash
$runnerPath50 = $PSCommandPath
$qualificationLease50 = Enter-NativeQualificationLease50 $repo $source ('capacity-'+$backendArgument)
$cli = Join-Path $repo 'target/release/darwinbots-cli.exe'
$dnaRelative = 'Installer/bots/Animal_Minimalis.txt'
$dna = Join-Path $repo $dnaRelative
$gitExe = (Get-Command git.exe -ErrorAction Stop).Source
$guardPaths = @('Cargo.toml', 'Cargo.lock', '.cargo', 'modern/engine/Cargo.toml',
    'modern/engine/src', 'modern/cli/Cargo.toml', 'modern/cli/src', $dnaRelative)
$cliHash = (Get-FileHash -LiteralPath $cli -Algorithm SHA256).Hash
if ($cliHash -ne $QualifiedCliSha256) { throw 'CLI does not match the qualified build receipt hash.' }
$dnaHash = (Get-FileHash -LiteralPath $dna -Algorithm SHA256).Hash
$plans = @(
    @{ population = 1000; ticks = 2000; timeout_seconds = 1200 },
    @{ population = 4000; ticks = 300; timeout_seconds = 1800 },
    @{ population = 8000; ticks = 100; timeout_seconds = 2400 },
    @{ population = 32000; ticks = 30; timeout_seconds = 7200 }
)
$plans = @($plans | Where-Object { $Populations -contains $_.population })

if (!('DbCapacityPass42Input' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class DbCapacityPass42Input {
    [StructLayout(LayoutKind.Sequential)]
    public struct Info { public uint cbSize; public uint time; }
    [DllImport("user32.dll", SetLastError = true)]
    private static extern bool GetLastInputInfo(ref Info info);
    public static uint[] Read() {
        var info = new Info { cbSize = (uint)Marshal.SizeOf<Info>() };
        if (!GetLastInputInfo(ref info))
            throw new InvalidOperationException("Windows idle information unavailable");
        return new[] { info.time, unchecked((uint)Environment.TickCount - info.time) };
    }
}
'@
}

function Get-IdleContext {
    $sample = [DbCapacityPass42Input]::Read()
    return [ordered]@{
        observed_utc = [DateTime]::UtcNow.ToString('o')
        last_input_tick = $sample[0]
        idle_seconds = $sample[1] / 1000.0
        required_idle_seconds = $IdleSeconds
        method = 'Windows GetLastInputInfo; observation only'
    }
}

function New-ProcessInfo([string]$Executable, [string[]]$Argv) {
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $Executable
    $info.WorkingDirectory = $repo
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.WindowStyle = [Diagnostics.ProcessWindowStyle]::Hidden
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $info.Environment['RAYON_NUM_THREADS'] = '1'
    foreach ($argument in $Argv) { $info.ArgumentList.Add($argument) }
    return $info
}

function Stop-OwnedProcess([Diagnostics.Process]$Process) {
    if ($null -eq $Process) { return }
    $Process.Refresh()
    if (!$Process.HasExited) {
        try { $Process.Kill($true) } catch {
            $Process.Refresh()
            if (!$Process.HasExited) { throw }
        }
        if (!$Process.WaitForExit(10000)) { throw 'Owned process tree did not exit after termination.' }
    }
}

function Read-OwnedOutput($Task) {
    if (!$Task.Wait(10000)) { throw 'Owned process output did not close within 10 seconds.' }
    return $Task.GetAwaiter().GetResult()
}

function Invoke-ReadOnlyGit([string[]]$GitArguments) {
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = New-ProcessInfo $gitExe (@('-C', $repo) + $GitArguments)
    $launched = $false
    try {
        if (!$process.Start()) { throw 'Unable to start read-only Git query.' }
        $launched = $true
        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()
        try { $process.PriorityClass = [Diagnostics.ProcessPriorityClass]::BelowNormal } catch {
            if (!$process.HasExited) { throw }
        }
        if (!$process.WaitForExit(10000)) { throw 'Read-only Git query exceeded 10 seconds.' }
        return [ordered]@{
            exit_code = $process.ExitCode
            stdout = Read-OwnedOutput $stdoutTask
            stderr = Read-OwnedOutput $stderrTask
        }
    } finally {
        if ($launched) { Stop-OwnedProcess $process }
        $process.Dispose()
    }
}

function Assert-Provenance {
    if ((Get-FileHash -LiteralPath $admission50.path -Algorithm SHA256).Hash -ne $admission50.sha256) { throw 'CPU admission marker changed.' }
    if ((Get-FileHash -LiteralPath $admissionHelper50 -Algorithm SHA256).Hash -ne $admissionHelperHash50 -or (Get-FileHash -LiteralPath $runnerPath50 -Algorithm SHA256).Hash -ne $runnerHash50) { throw 'Qualification runner changed.' }
    foreach ($pin50 in $admission50.marker.evidence) { if ((Get-FileHash -LiteralPath (Join-Path $repo $pin50.file) -Algorithm SHA256).Hash -ne $pin50.sha256) { throw ('Sealed qualification evidence changed: '+$pin50.file) } }
    $head = Invoke-ReadOnlyGit @('rev-parse', 'HEAD')
    if ($head.exit_code -ne 0 -or $head.stdout.Trim() -ne $source) {
        throw 'HEAD differs from the required committed source SHA.'
    }
    $difference = Invoke-ReadOnlyGit (@('diff', '--quiet', $source, '--') + $guardPaths)
    if ($difference.exit_code -ne 0) { throw 'Pinned production source or workload DNA changed.' }
    $untracked = Invoke-ReadOnlyGit (@('ls-files', '--others', '--exclude-standard', '--') + $guardPaths)
    if ($untracked.exit_code -ne 0 -or $untracked.stdout.Trim().Length -ne 0) {
        throw 'Uncommitted production source was found.'
    }
    $trackedDna = Invoke-ReadOnlyGit @('ls-files', '--error-unmatch', '--', $dnaRelative)
    if ($trackedDna.exit_code -ne 0) { throw 'Workload DNA must be tracked at the pinned source commit.' }
    if ((Get-FileHash -LiteralPath $cli -Algorithm SHA256).Hash -ne $cliHash) { throw 'CLI binary changed.' }
    if ((Get-FileHash -LiteralPath $dna -Algorithm SHA256).Hash -ne $dnaHash) { throw 'Workload DNA hash changed.' }
}

function Write-NewJson([string]$Path, $Value) {
    $stream = [IO.File]::Open($Path, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::Read)
    $writer = [IO.StreamWriter]::new($stream, [Text.UTF8Encoding]::new($false))
    try { $writer.WriteLine(($Value | ConvertTo-Json -Depth 12)) } finally { $writer.Dispose() }
}

function Write-NewText([string]$Path, [string]$Value) {
    $stream = [IO.File]::Open($Path, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::Read)
    $writer = [IO.StreamWriter]::new($stream, [Text.UTF8Encoding]::new($false))
    try { $writer.Write($Value) } finally { $writer.Dispose() }
}

# Every invocation owns a fresh evidence directory. Existing receipts are never reused.
Assert-Provenance
$label = 'capacity-' + $backendArgument + '-pass50-' + $source.Substring(0, 12) + '-' +
    [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfff') + '-' + [Guid]::NewGuid().ToString('N').Substring(0, 8)
$evidenceParent = Join-Path $repo 'docs/parity/evidence/batch-2026-10-14'
$evidence = (New-Item -Path $evidenceParent -Name $label -ItemType Directory -ErrorAction Stop).FullName
$stopMarker = Join-Path $evidence 'STOP'
$results = [Collections.Generic.List[object]]::new()
$blockedReason = $null
Write-NewJson (Join-Path $evidence 'manifest.json') ([ordered]@{
    source_commit = $source
    cpu_ready = $admission50.path
    cpu_ready_sha256 = $admission50.sha256
    runner_sha256 = $runnerHash50
    admission_helper_sha256 = $admissionHelperHash50
    exclusive_lease = 'output/native-qualification50.lock'
    production_guard_paths = $guardPaths
    cli = $cli
    cli_sha256 = $cliHash
    qualified_cli_sha256 = $QualifiedCliSha256
    dna = $dna
    dna_sha256 = $dnaHash
    seed = 24832
    backend_requested = $Backend
    requested_populations = @($Populations)
    selected_populations = @($plans | ForEach-Object { $_.population })
    selected_task_count = $plans.Count
    adapter_policy = 'Each run must report its own adapter: explicit null for Cpu; named DiscreteGpu/IntegratedGpu for Gpu. CPU fallback and software adapters cannot qualify Gpu.'
    initial_energy = 10000
    rayon_num_threads = 1
    priority = 'BelowNormal'
    provenance_guard_interval_seconds = 5
    runner_pid = $PID
    created_utc = [DateTime]::UtcNow.ToString('o')
    idle_context = Get-IdleContext
    plans = $plans
    timing_scope = 'CLI run: simulation cycles plus per-cycle candidate identity tracking; no warmup; excludes import/spawn'
    simulation_policy = 'No external trainer. Normal engine mutation remains enabled.'
    provenance_boundary = 'CLI and DNA hashes are recorded; matching the binary to this commit also requires its build receipt.'
    stop_marker = $stopMarker
})
Write-Output ('Capacity evidence: ' + $evidence)

foreach ($plan in $plans) {
    $task = $backendArgument + '-' + $plan.population
    $argv = @('run', '--bot', $dna, '--population', [string]$plan.population,
        '--initial-energy', '10000', '--ticks', [string]$plan.ticks,
        '--backend', $backendArgument, '--seed', '24832')
    $receipt = [ordered]@{
        task = $task
        source_commit = $source
    cpu_ready = $admission50.path
    cpu_ready_sha256 = $admission50.sha256
    runner_sha256 = $runnerHash50
    admission_helper_sha256 = $admissionHelperHash50
    exclusive_lease = 'output/native-qualification50.lock'
        cli_sha256 = $cliHash
        dna_sha256 = $dnaHash
        executable = $cli
        arguments = $argv
        argument_transport = 'ProcessStartInfo.ArgumentList; no shell or joined command string'
        requested_population = $plan.population
        requested_ticks = $plan.ticks
        initial_energy = 10000
        seed = 24832
        backend_requested = $Backend
        rayon_num_threads = 1
        priority = 'BelowNormal'
        timeout_seconds = $plan.timeout_seconds
        minimum_measured_seconds = 60
        status = 'not_run'
        error = $null
        started_utc = $null
        finished_utc = $null
        owned_cli_pid = $null
        exit_code = $null
        process_seconds = $null
        idle_before = $null
        idle_after = $null
        actual_backend = $null
        gpu_adapter_field_present = $false
        actual_gpu_adapter = $null
        adapter_status = 'not_checked'
        duration_status = 'not_measured'
        held_population = $false
        report = $null
        stdout = $task + '.stdout.json'
        stderr = $task + '.stderr.log'
    }
    $process = $null
    $launched = $false
    $stdoutTask = $null
    $stderrTask = $null
    $stdout = ''
    $stderr = ''
    $clock = $null
    try {
        if ($blockedReason) {
            $receipt.status = 'not_run_after_guard_failure'
            $receipt.error = $blockedReason
        } elseif (Test-Path -LiteralPath $stopMarker) {
            $receipt.status = 'not_run_owner_stop'
            $blockedReason = 'Owned STOP marker present.'
        } else {
            try { Assert-Provenance } catch { $blockedReason = $_.Exception.Message; throw }
            $receipt.idle_before = Get-IdleContext
            if ($receipt.idle_before.idle_seconds -lt $IdleSeconds) {
                $receipt.status = 'deferred_pc_in_use'
                $receipt.error = 'Required Windows idle interval was not observed; no retry in this run.'
            } else {
                $process = [Diagnostics.Process]::new()
                $process.StartInfo = New-ProcessInfo $cli $argv
                $receipt.started_utc = [DateTime]::UtcNow.ToString('o')
                $clock = [Diagnostics.Stopwatch]::StartNew()
                if (!$process.Start()) { throw 'Unable to start owned CLI process.' }
                $launched = $true
                $receipt.owned_cli_pid = $process.Id
                $stdoutTask = $process.StandardOutput.ReadToEndAsync()
                $stderrTask = $process.StandardError.ReadToEndAsync()
                $process.PriorityClass = [Diagnostics.ProcessPriorityClass]::BelowNormal
                $nextGuardSeconds = 5.0
                $receipt.status = 'running'
                while (!$process.WaitForExit(500)) {
                    if (Test-Path -LiteralPath $stopMarker) {
                        $receipt.status = 'owner_stop'
                        $blockedReason = 'Owned STOP marker present.'
                        break
                    }
                    if ((Get-IdleContext).last_input_tick -ne $receipt.idle_before.last_input_tick) {
                        $receipt.status = 'interrupted_by_input'
                        $receipt.error = 'New Windows input was observed; owned CLI stopped without retry.'
                        break
                    }
                    if ($clock.Elapsed.TotalSeconds -ge $plan.timeout_seconds) {
                        $receipt.status = 'timed_out'
                        break
                    }
                    if ($clock.Elapsed.TotalSeconds -ge $nextGuardSeconds) {
                        try { Assert-Provenance } catch { $blockedReason = $_.Exception.Message; throw }
                        $nextGuardSeconds = $clock.Elapsed.TotalSeconds + 5.0
                    }
                }
                Stop-OwnedProcess $process
                $stdout = Read-OwnedOutput $stdoutTask
                $stderr = Read-OwnedOutput $stderrTask
                $receipt.exit_code = $process.ExitCode
                # The child may exit before the next running-loop STOP poll.
                if (Test-Path -LiteralPath $stopMarker) {
                    $receipt.status = 'owner_stop'
                    $receipt.error = 'Owned STOP marker observed immediately after child exit.'
                    $blockedReason = $receipt.error
                }
                if ($receipt.status -eq 'running') {
                    try { Assert-Provenance } catch { $blockedReason = $_.Exception.Message; throw }
                    $receipt.idle_after = Get-IdleContext
                    if ($receipt.idle_after.last_input_tick -ne $receipt.idle_before.last_input_tick) {
                        $receipt.status = 'interrupted_by_input'
                    } elseif ($process.ExitCode -ne 0) {
                        $receipt.status = 'subprocess_failed'
                    } else {
                        $report = $stdout | ConvertFrom-Json -AsHashtable
                        $receipt.report = $report
                        $receipt.actual_backend = $report.backend
                        $receipt.gpu_adapter_field_present = $report.ContainsKey('gpu_adapter')
                        $receipt.actual_gpu_adapter = $report.gpu_adapter
                        $adapter = $report.gpu_adapter
                        if (!$receipt.gpu_adapter_field_present) {
                            $receipt.adapter_status = 'adapter_metadata_missing'
                        } elseif ($Backend -eq 'Cpu') {
                            $receipt.adapter_status = if ($null -eq $adapter) { 'verified_cpu_null' }
                                else { 'unexpected_cpu_adapter' }
                        } elseif ($adapter -isnot [Collections.IDictionary] -or
                            !$adapter.Contains('name') -or $adapter.name -isnot [string] -or
                            [string]::IsNullOrWhiteSpace($adapter.name) -or
                            !$adapter.Contains('backend') -or $adapter.backend -isnot [string] -or
                            [string]::IsNullOrWhiteSpace($adapter.backend) -or
                            !$adapter.Contains('device_type') -or $adapter.device_type -isnot [string]) {
                            $receipt.adapter_status = 'adapter_metadata_invalid'
                        } elseif ($adapter.device_type -cnotin @('DiscreteGpu', 'IntegratedGpu')) {
                            $receipt.adapter_status = 'nonphysical_gpu_adapter'
                        } else {
                            $receipt.adapter_status = 'verified_physical_gpu'
                        }
                        $validElapsed = $report.ContainsKey('elapsed_seconds') -and
                            $report.elapsed_seconds -is [ValueType] -and
                            [double]::IsFinite([double]$report.elapsed_seconds) -and $report.elapsed_seconds -gt 0
                        $receipt.duration_status = if (!$validElapsed) { 'invalid_elapsed_seconds' }
                            elseif ($report.elapsed_seconds -lt 60) { 'insufficient_sustained_duration' } else { 'sustained' }
                        $held = $true
                        foreach ($field in @('starting_population', 'candidate_starting_population',
                                'final_population', 'candidate_final_population', 'population')) {
                            if (!$report.ContainsKey($field) -or $report[$field] -ne $plan.population) { $held = $false }
                        }
                        foreach ($field in @('births', 'deaths', 'candidate_births', 'candidate_deaths',
                                'vegetable_starting_population', 'vegetable_final_population')) {
                            if (!$report.ContainsKey($field) -or $report[$field] -ne 0) { $held = $false }
                        }
                        $receipt.held_population = $held
                        if ($report.backend -cne $Backend) { $receipt.status = 'backend_mismatch' }
                        elseif ($receipt.adapter_status -notin @('verified_cpu_null', 'verified_physical_gpu')) {
                            $receipt.status = $receipt.adapter_status
                        }
                        elseif ($report.ticks -ne $plan.ticks -or $report.tick -ne $plan.ticks) { $receipt.status = 'tick_mismatch' }
                        elseif (!$held) { $receipt.status = 'population_not_held' }
                        elseif ($receipt.duration_status -ne 'sustained') { $receipt.status = $receipt.duration_status }
                        else { $receipt.status = 'passed' }
                        if ($validElapsed) { $receipt['whole_probe_ticks_per_second'] = $plan.ticks / $report.elapsed_seconds }
                    }
                }
            }
        }
    } catch {
        $receipt.status = if ($blockedReason) { 'guard_failed' } else { 'runner_error' }
        $receipt.error = $_.Exception.Message
    } finally {
        if ($launched) {
            try {
                Stop-OwnedProcess $process
                if ($null -ne $stdoutTask) { $stdout = Read-OwnedOutput $stdoutTask }
                if ($null -ne $stderrTask) { $stderr = Read-OwnedOutput $stderrTask }
                $receipt.exit_code = $process.ExitCode
            } catch { $receipt.status = 'cleanup_failed'; $receipt.error = $_.Exception.Message; $blockedReason = $receipt.error }
        }
        if ($null -ne $process) { $process.Dispose() }
        if ($null -ne $clock) { $clock.Stop(); $receipt.process_seconds = $clock.Elapsed.TotalSeconds }
        $receipt.finished_utc = [DateTime]::UtcNow.ToString('o')
        try {
            $receipt.idle_after = Get-IdleContext
            # Keep future blocked plans marked not-run; this final check applies
            # to the task that actually launched, including a just-exited child.
            if ($launched -and (Test-Path -LiteralPath $stopMarker)) {
                $receipt.status = 'owner_stop'
                $receipt.error = 'Owned STOP marker observed before the final receipt.'
                $blockedReason = $receipt.error
            } elseif ($receipt.status -eq 'passed' -and
                $receipt.idle_after.last_input_tick -ne $receipt.idle_before.last_input_tick) {
                $receipt.status = 'interrupted_by_input'
                $receipt.error = 'New input was observed before the final receipt; sustained qualification withheld.'
            }
        } catch { $receipt.status = 'idle_observation_failed'; $receipt.error = $_.Exception.Message }
    }
    Write-NewText (Join-Path $evidence $receipt.stdout) $stdout
    Write-NewText (Join-Path $evidence $receipt.stderr) $stderr
    Write-NewJson (Join-Path $evidence ($task + '.receipt.json')) $receipt
    $results.Add($receipt)
    Write-Output ($task + ': ' + $receipt.status)
}

$failed = @($results | Where-Object { $_.status -ne 'passed' })
$summary = [ordered]@{
    source_commit = $source
    cpu_ready = $admission50.path
    cpu_ready_sha256 = $admission50.sha256
    runner_sha256 = $runnerHash50
    admission_helper_sha256 = $admissionHelperHash50
    exclusive_lease = 'output/native-qualification50.lock'
    cli_sha256 = $cliHash
    dna_sha256 = $dnaHash
    backend_requested = $Backend
    requested_populations = @($Populations)
    selected_populations = @($plans | ForEach-Object { $_.population })
    selected_task_count = $plans.Count
    status = if ($failed.Count) { 'incomplete_or_failed' } else { 'passed' }
    evidence_directory = $evidence
    completed_utc = [DateTime]::UtcNow.ToString('o')
    tasks = $results.ToArray()
    failed_or_insufficient_tasks = @($failed | ForEach-Object { [ordered]@{ task = $_.task; status = $_.status; error = $_.error } })
    qualification_scope = $Backend + ' headless whole-probe throughput for selected populations only; each run verifies its own adapter identity; no rendered, original-runtime, trainer, or unselected-population claims'
}
Write-NewJson (Join-Path $evidence 'summary.json') $summary
if ($failed.Count) {
    throw ('Capacity collection finished with ' + $failed.Count + ' failed, deferred, or insufficient task(s). See ' + (Join-Path $evidence 'summary.json'))
}
Write-Output ('All ' + $plans.Count + ' selected sustained ' + $Backend + ' capacity task(s) passed. See ' + (Join-Path $evidence 'summary.json'))
