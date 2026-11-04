$ErrorActionPreference = 'Stop'

function Read-NativeCpuAdmission52([string]$Repo, [string]$Source) {
    $path = Join-Path $Repo ('docs/parity/evidence/batch-2026-10-14/gpu-' + $Source.Substring(0,12) + '/cpu-ready.json')
    if (!(Test-Path -LiteralPath $path)) { throw 'Exact-source CPU qualification marker is required.' }
    $marker = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
    if ($marker.schema_version -ne 3 -or $marker.source_head -ne $Source -or
        $marker.cpu_workspace_passed -ne $true -or $marker.core_passed -ne $true -or
        $marker.desktop_passed -ne $true -or $marker.package_passed -ne $true -or
        $marker.source_qualification_passed -ne $true -or
        $marker.qualification_method -ne 'fresh_full_cpu_workspace52_and_core_desktop_package') {
        throw 'Fresh CPU workspace, Core, desktop, package and exact-source qualification are required.'
    }
    $cpu = $marker.rust_cpu
    if ($cpu.fresh_full_workspace_invocation -ne $true -or $cpu.hardware_filtered -ne $true -or
        $cpu.failed -ne 0 -or $cpu.ignored -ne 0 -or $cpu.passed -le 0 -or
        $cpu.expected_distinct_targets -lt 66 -or $cpu.total_distinct_targets -ne $cpu.expected_distinct_targets) {
        throw 'CPU marker must prove all expected targets in one fresh workspace invocation, with hardware excluded.'
    }
    if (@($marker.evidence).Count -lt 2 -or $marker.source_file_count -lt 79 -or
        @($marker.source_files).Count -ne $marker.source_file_count) {
        throw 'CPU marker is missing sealed evidence or final source pins.'
    }
    $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($pin in @($marker.evidence) + @($marker.source_files)) {
        $absolute = [IO.Path]::GetFullPath((Join-Path $Repo $pin.file))
        $prefix = [IO.Path]::GetFullPath($Repo).TrimEnd([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
        if (!$absolute.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) -or
            (Get-FileHash -LiteralPath $absolute -Algorithm SHA256).Hash -ne $pin.sha256) {
            throw ('Qualified evidence/source pin changed: ' + $pin.file)
        }
    }
    foreach ($pin in $marker.source_files) {
        if (!$seen.Add($pin.file)) { throw 'Duplicate source pins cannot qualify the expected file count.' }
    }
    return [ordered]@{path=$path;sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash;marker=$marker}
}

function Enter-NativeQualificationLease52([string]$Repo, [string]$Source, [string]$Role) {
    # Keep one lock identity shared with prior runners; a stale file alone is not a lease.
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
