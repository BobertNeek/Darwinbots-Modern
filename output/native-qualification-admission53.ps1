$ErrorActionPreference = 'Stop'

function Read-NativeCpuAdmission53([string]$Repo, [string]$Source) {
    if ($Source -cnotmatch '^[0-9a-f]{40}$') { throw 'An exact qualified final commit is required.' }
    $repoAbsolute = [IO.Path]::GetFullPath($Repo).TrimEnd([char[]]@([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar))
    $prefix = $repoAbsolute + [IO.Path]::DirectorySeparatorChar
    $path = Join-Path $repoAbsolute ('docs/parity/evidence/batch-2026-10-14/gpu-' + $Source.Substring(0,12) + '/cpu-ready.json')
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) { throw 'Exact-source CPU qualification marker is required.' }
    $markerBytes = [IO.File]::ReadAllBytes($path)
    $markerHash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($markerBytes))
    $marker = [Text.UTF8Encoding]::new($false, $true).GetString($markerBytes) | ConvertFrom-Json
    if ($marker.schema_version -ne 4 -or $marker.source_head -cne $Source -or
        $marker.qualification_method -cne 'source_equivalent_full_cpu52_plus_fresh_cli53_and_desktop_package') {
        throw 'Schema4 exact-final-source CPU52 plus fresh CLI53 qualification is required.'
    }
    foreach ($flag in @('cpu_workspace_passed', 'core_passed', 'desktop_passed', 'package_passed',
            'source_qualification_passed', 'core_fresh_invocation', 'desktop_fresh_invocation',
            'package_fresh_invocation', 'source_qualification_fresh_invocation')) {
        if ($marker.$flag -isnot [bool] -or $marker.$flag -ne $true) {
            throw ('Fresh Core/Desktop/package/source checks and a qualified projected Rust result are required: ' + $flag)
        }
    }
    function Assert-NativeCounts53($Counts, [string]$Label, [bool]$RequirePass) {
        foreach ($field in @('passed', 'failed', 'ignored', 'filtered_out')) {
            $value = $Counts.$field
            if ($null -eq $value -or $value -is [bool] -or $value -isnot [ValueType] -or
                ![double]::IsFinite([double]$value) -or $value -lt 0 -or [Math]::Truncate([double]$value) -ne $value) {
                throw ($Label + ' contains a missing or invalid count: ' + $field)
            }
        }
        if ($Counts.failed -ne 0 -or $Counts.ignored -ne 0 -or ($RequirePass -and $Counts.passed -le 0)) {
            throw ($Label + ' must pass without failed or ignored tests.')
        }
    }
    function Get-NativeTargetKey53($Target) {
        if ($Target.package -isnot [string] -or [string]::IsNullOrWhiteSpace($Target.package) -or
            $Target.name -isnot [string] -or [string]::IsNullOrWhiteSpace($Target.name) -or
            $Target.kind -cnotin @('lib', 'bin', 'test', 'doctest')) {
            throw 'Projection targets require package, name, and a normalized lib/bin/test/doctest kind.'
        }
        $name = if ($Target.package -ceq 'darwinbots-cli' -and $Target.kind -ceq 'lib' -and
            $Target.name -cin @('darwinbots-cli', 'darwinbots_cli')) { 'darwinbots-cli' } else { $Target.name }
        return $Target.package + ':' + $Target.kind + ':' + $name
    }
    $cpu = $marker.rust_cpu
    Assert-NativeCounts53 $cpu 'Projected Rust CPU result' $true
    if ($cpu.fresh_full_workspace_invocation -isnot [bool] -or $cpu.fresh_full_workspace_invocation -ne $false -or
        $cpu.base_fresh_full_workspace_invocation -isnot [bool] -or $cpu.base_fresh_full_workspace_invocation -ne $true -or
        $cpu.hardware_filtered -isnot [bool] -or $cpu.hardware_filtered -ne $true -or
        $cpu.expected_distinct_targets -ne 66 -or $cpu.total_distinct_targets -ne 66) {
        throw 'Projected CPU53 must retain the full fresh CPU52 basis, exclude hardware, and cover exactly 66 distinct targets; it is not a fresh full workspace invocation.'
    }
    if (@($marker.evidence).Count -lt 3 -or $marker.source_file_count -lt 156 -or
        @($marker.source_files).Count -ne $marker.source_file_count) {
        throw 'CPU marker is missing sealed evidence or at least 156 distinct final source pins.'
    }
    $evidencePins = [Collections.Generic.Dictionary[string,string]]::new([StringComparer]::OrdinalIgnoreCase)
    $allPins = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    function Resolve-NativePin53($Pin) {
        if ($Pin.file -isnot [string] -or [string]::IsNullOrWhiteSpace($Pin.file) -or
            [IO.Path]::IsPathRooted($Pin.file) -or $Pin.sha256 -isnot [string] -or $Pin.sha256 -notmatch '^[0-9a-fA-F]{64}$') {
            throw 'Qualification pins require relative paths and exact SHA256 hashes.'
        }
        $absolute = [IO.Path]::GetFullPath((Join-Path $repoAbsolute $Pin.file))
        if (!$absolute.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) -or
            !(Test-Path -LiteralPath $absolute -PathType Leaf) -or
            (Get-FileHash -LiteralPath $absolute -Algorithm SHA256).Hash -ne $Pin.sha256) {
            throw ('Qualified evidence/source pin changed or escapes the checkout: ' + $Pin.file)
        }
        return $absolute
    }
    foreach ($pin in @($marker.evidence)) {
        $absolute = Resolve-NativePin53 $pin
        if (!$allPins.Add($absolute)) { throw 'Duplicate evidence paths cannot qualify the required pin count.' }
        $evidencePins.Add($absolute, $pin.sha256.ToUpperInvariant())
    }
    foreach ($pin in @($marker.source_files)) {
        $absolute = Resolve-NativePin53 $pin
        if (!$allPins.Add($absolute)) { throw 'Duplicate source/evidence paths cannot qualify the required pin count.' }
    }
    $projectionPath = Resolve-NativePin53 $marker.projection
    if (!$evidencePins.ContainsKey($projectionPath) -or $evidencePins[$projectionPath] -ne $marker.projection.sha256) {
        throw 'The source-equivalence projection must be pinned explicitly among sealed evidence.'
    }
    $projectionBytes = [IO.File]::ReadAllBytes($projectionPath)
    if ([Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($projectionBytes)) -ne $marker.projection.sha256) {
        throw 'The source-equivalence projection changed while being read.'
    }
    $projection = [Text.UTF8Encoding]::new($false, $true).GetString($projectionBytes) | ConvertFrom-Json
    if ($projection.base_source_head -cne '0ea730616161a0fe25a8516a089a52cf516cc435' -or
        $projection.source_head -cne $Source) {
        throw 'The sealed projection must link exact CPU52 source 0ea7306 to the provided final commit.'
    }
    foreach ($flag in @('production_prefix_unchanged', 'all_other_rust_inputs_unchanged',
            'git_source_delta_audited', 'projection_passed')) {
        if ($projection.$flag -isnot [bool] -or $projection.$flag -ne $true) {
            throw ('The sealed projection is not accepted: ' + $flag)
        }
    }
    if ($projection.base_rust_result.file -cne 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass52/rust-target-results.json') {
        throw 'The projection must identify the actual sealed batch52 Rust target receipt.'
    }
    $baseResultPath = Resolve-NativePin53 $projection.base_rust_result
    if (!$evidencePins.ContainsKey($baseResultPath) -or $evidencePins[$baseResultPath] -ne $projection.base_rust_result.sha256) {
        throw 'The actual batch52 Rust target receipt must be pinned explicitly among sealed evidence.'
    }
    $baseResultBytes = [IO.File]::ReadAllBytes($baseResultPath)
    if ([Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($baseResultBytes)) -ne $projection.base_rust_result.sha256) {
        throw 'The sealed batch52 Rust target receipt changed while being read.'
    }
    $baseResult = [Text.UTF8Encoding]::new($false, $true).GetString($baseResultBytes) | ConvertFrom-Json
    if ($baseResult.schema_version -ne 1 -or $baseResult.batch -ne 52 -or $baseResult.state -cne 'passed' -or
        $baseResult.source_head -cne $projection.base_source_head -or
        $baseResult.full_workspace_passed -isnot [bool] -or $baseResult.full_workspace_passed -ne $true -or
        $baseResult.invocation.fresh_full_workspace_invocation -isnot [bool] -or $baseResult.invocation.fresh_full_workspace_invocation -ne $true -or
        $baseResult.invocation.hardware_filtered -isnot [bool] -or $baseResult.invocation.hardware_filtered -ne $true -or
        $baseResult.invocation.exit_code -ne 0 -or
        $baseResult.expected_distinct_targets -ne 66 -or $baseResult.total_distinct_targets -ne 66 -or
        @($baseResult.all_targets).Count -ne 66) {
        throw 'The actual batch52 receipt must prove a passing fresh full hardware-filtered workspace at the exact base source with 66 targets.'
    }
    Assert-NativeCounts53 $baseResult.counts 'Actual sealed batch52 Rust result' $true
    $baseTargets = [Collections.Generic.Dictionary[string,object]]::new([StringComparer]::Ordinal)
    $baseSums = @{passed=[long]0;failed=[long]0;ignored=[long]0;filtered_out=[long]0}
    foreach ($target in @($baseResult.all_targets)) {
        $key = Get-NativeTargetKey53 $target
        if ($baseTargets.ContainsKey($key) -or $target.status -cne 'ok') {
            throw ('The actual batch52 receipt contains a duplicate or unsuccessful target: ' + $key)
        }
        Assert-NativeCounts53 $target ('Actual batch52 target ' + $key) $false
        $baseTargets.Add($key, $target)
        foreach ($field in @('passed', 'failed', 'ignored', 'filtered_out')) { $baseSums[$field] += [long]$target.$field }
    }
    if (!$baseTargets.ContainsKey('darwinbots-cli:lib:darwinbots-cli')) {
        throw 'The actual batch52 receipt must contain the CLI library target being replaced.'
    }
    $baseCpu = $projection.base_rust_cpu
    Assert-NativeCounts53 $baseCpu 'Fresh full CPU52 basis' $true
    if ($baseCpu.full_workspace_passed -isnot [bool] -or $baseCpu.full_workspace_passed -ne $true -or
        $baseCpu.fresh_full_workspace_invocation -isnot [bool] -or $baseCpu.fresh_full_workspace_invocation -ne $true -or
        $baseCpu.hardware_filtered -isnot [bool] -or $baseCpu.hardware_filtered -ne $true -or
        $baseCpu.expected_distinct_targets -ne 66 -or $baseCpu.total_distinct_targets -ne 66) {
        throw 'The sealed projection must prove a passing fresh full CPU52 workspace with exactly 66 hardware-filtered targets.'
    }
    foreach ($field in @('passed', 'failed', 'ignored', 'filtered_out')) {
        if ($baseSums[$field] -ne $baseResult.counts.$field -or $baseCpu.$field -ne $baseResult.counts.$field) {
            throw ('The projected CPU52 basis differs from its actual sealed target receipt: ' + $field)
        }
    }
    $fresh = @($projection.fresh_targets)
    $retained = @($projection.retained_targets)
    if ($fresh.Count -ne 1 -or $retained.Count -ne 65) {
        throw 'The projection must replace exactly one CLI library target and retain exactly 65 targets.'
    }
    $targetKeys = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $sums = @{passed=[long]0;failed=[long]0;ignored=[long]0;filtered_out=[long]0}
    foreach ($target in $fresh + $retained) {
        $key = Get-NativeTargetKey53 $target
        if (!$targetKeys.Add($key)) { throw ('Duplicate projected Rust target: ' + $key) }
        Assert-NativeCounts53 $target ('Projected target ' + $key) $false
        foreach ($field in @('passed', 'failed', 'ignored', 'filtered_out')) { $sums[$field] += [long]$target.$field }
    }
    foreach ($target in $retained) {
        $key = Get-NativeTargetKey53 $target
        if ($key -ceq 'darwinbots-cli:lib:darwinbots-cli' -or !$baseTargets.ContainsKey($key)) {
            throw ('A retained target is not one of the 65 unchanged actual batch52 targets: ' + $key)
        }
        foreach ($field in @('passed', 'failed', 'ignored', 'filtered_out')) {
            if ($target.$field -ne $baseTargets[$key].$field) {
                throw ('A retained target differs from its actual sealed batch52 result: ' + $key + ':' + $field)
            }
        }
    }
    $replacement = $fresh[0]
    if ($replacement.package -cne 'darwinbots-cli' -or $replacement.kind -cne 'lib' -or
        $replacement.name -cnotin @('darwinbots-cli', 'darwinbots_cli') -or $replacement.passed -le 0 -or
        $targetKeys.Count -ne 66) {
        throw 'The only fresh replacement must be the passing CLI library target within 66 distinct projected targets.'
    }
    foreach ($field in @('passed', 'failed', 'ignored', 'filtered_out')) {
        if ($sums[$field] -ne $cpu.$field) { throw ('Projected target aggregate differs from the CPU marker: ' + $field) }
    }
    return [ordered]@{path=$path;sha256=$markerHash;marker=$marker;projection_path=$projectionPath;projection=$projection}
}

function Enter-NativeQualificationLease53([string]$Repo, [string]$Source, [string]$Role) {
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
