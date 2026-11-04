$ErrorActionPreference = 'Stop'

function Read-NativeCpuAdmission50([string]$Repo, [string]$Source) {
    $path = Join-Path $Repo ('docs/parity/evidence/batch-2026-10-14/gpu-' + $Source.Substring(0,12) + '/cpu-ready.json')
    if (!(Test-Path -LiteralPath $path)) { throw 'Exact-source CPU qualification marker is required.' }
    $marker = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
    if ($marker.schema_version -ne 2 -or $marker.source_head -ne $Source -or
        $marker.cpu_workspace_passed -ne $true -or $marker.desktop_passed -ne $true -or
        $marker.package_passed -ne $true -or $marker.test_only_projection_passed -ne $true -or
        $marker.qualification_method -ne 'fresh49_affected_and_retained48_with_physical_test_only_projection50') {
        throw 'CPU, desktop, package and physical-test-only source projection must qualify this exact commit.'
    }
    $cpu = $marker.rust_cpu
    if ($cpu.fresh_full_workspace_invocation -ne $false -or $cpu.failed -ne 0 -or
        $cpu.passed -le 0 -or $cpu.fresh49_target_invocations -ne 10 -or
        $cpu.retained48_target_count -ne 56 -or $cpu.total_distinct_targets -ne 66) {
        throw 'CPU marker must retain the explicit 10 fresh / 56 retained target boundary.'
    }
    if (@($marker.evidence).Count -lt 2 -or @($marker.source_files).Count -ne 79) {
        throw 'CPU marker is missing sealed evidence or final source pins.'
    }
    foreach ($pin in @($marker.evidence) + @($marker.source_files)) {
        $absolute = [IO.Path]::GetFullPath((Join-Path $Repo $pin.file))
        $prefix = [IO.Path]::GetFullPath($Repo).TrimEnd('\','/') + [IO.Path]::DirectorySeparatorChar
        if (!$absolute.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) -or
            (Get-FileHash -LiteralPath $absolute -Algorithm SHA256).Hash -ne $pin.sha256) {
            throw ('Qualified evidence/source pin changed: ' + $pin.file)
        }
    }
    return [ordered]@{path=$path;sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash;marker=$marker}
}

function Enter-NativeQualificationLease50([string]$Repo, [string]$Source, [string]$Role) {
    $path = Join-Path $Repo 'output/native-qualification50.lock'
    try { $stream = [IO.File]::Open($path, [IO.FileMode]::OpenOrCreate, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None) }
    catch { throw 'Another owned hardware/capacity qualification holds the exclusive lease.' }
    try {
        $body = [Text.Encoding]::UTF8.GetBytes(([ordered]@{pid=$PID;source=$Source;role=$Role;utc=[DateTime]::UtcNow.ToString('o')} | ConvertTo-Json -Compress))
        $stream.SetLength(0)
        $stream.Write($body,0,$body.Length)
        $stream.Flush($true)
        return $stream
    } catch { $stream.Dispose(); throw }
}
