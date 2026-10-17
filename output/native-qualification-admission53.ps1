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
    if ($marker.schema_version -eq 5) {
        return Read-NativeFreshCpuAdmission55 $repoAbsolute $Source $path $markerHash $marker
    }
    if ($marker.schema_version -ne 4 -or $marker.source_head -cne $Source -or
        $marker.qualification_method -cne 'source_equivalent_full_cpu52_plus_fresh_cli53_and_desktop_package') {
        throw 'Schema4 exact-final-source CPU52 plus fresh CLI53 qualification is required.'
    }
    foreach ($flag in @('cpu_workspace_passed', 'core_passed', 'desktop_passed', 'package_passed',
            'source_qualification_passed', 'desktop_fresh_invocation',
            'package_fresh_invocation', 'source_qualification_fresh_invocation')) {
        if ($marker.$flag -isnot [bool] -or $marker.$flag -ne $true) {
            throw ('Qualified Core and fresh Desktop/package/source checks with a projected Rust result are required: ' + $flag)
        }
    }
    if ($marker.core_fresh_invocation -isnot [bool]) { throw 'Core fresh-invocation flag must be Boolean.' }
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
    if (!$marker.core_fresh_invocation) {
        $coreOrigin = 'e0300ca1391e0028b39aef2b61031a98934f00db'
        if ($marker.core_source_equivalent -isnot [bool] -or $marker.core_source_equivalent -ne $true) {
            throw 'Retained Core requires typed core_source_equivalent=true.'
        }
        $retainedCore = $marker.retained_core
        if ($retainedCore.source_head -cne $coreOrigin -or
            $retainedCore.basis.file -cne 'output/pass54-core-basis53.json' -or
            $retainedCore.summary.file -cne 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass53/summary.json' -or
            $retainedCore.trx.file -cne 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass53/core.trx') {
            throw 'Retained Core requires the exact fresh Core53 origin e0300ca and canonical basis/summary/TRX.'
        }
        $coreBytes = @{}
        foreach ($entry in @(@{name='basis';pin=$retainedCore.basis}, @{name='summary';pin=$retainedCore.summary},
                @{name='trx';pin=$retainedCore.trx}, @{name='equivalence';pin=$retainedCore.equivalence})) {
            $absolute = Resolve-NativePin53 $entry.pin
            if (!$evidencePins.ContainsKey($absolute) -or $evidencePins[$absolute] -ne $entry.pin.sha256) {
                throw ('Retained Core evidence must be pinned explicitly: ' + $entry.name)
            }
            $bytes = [IO.File]::ReadAllBytes($absolute)
            if ([Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)) -ne $entry.pin.sha256) {
                throw ('Retained Core evidence changed while being read: ' + $entry.name)
            }
            $coreBytes[$entry.name] = $bytes
        }
        $coreBasis = [Text.UTF8Encoding]::new($false, $true).GetString($coreBytes['basis']) | ConvertFrom-Json
        $coreSummary = [Text.UTF8Encoding]::new($false, $true).GetString($coreBytes['summary']) | ConvertFrom-Json
        $coreEquivalence = [Text.UTF8Encoding]::new($false, $true).GetString($coreBytes['equivalence']) | ConvertFrom-Json
        if ($coreBasis.schema_version -ne 1 -or $coreBasis.source_head -cne $coreOrigin -or
            $coreBasis.projects_have_only_local_Core_project_reference -isnot [bool] -or
            $coreBasis.projects_have_only_local_Core_project_reference -ne $true -or
            $coreBasis.trx.file -cne $retainedCore.trx.file -or $coreBasis.trx.sha256 -ne $retainedCore.trx.sha256 -or
            $coreBasis.summary.file -cne $retainedCore.summary.file -or $coreBasis.summary.sha256 -ne $retainedCore.summary.sha256 -or
            $coreSummary.schema_version -ne 1 -or $coreSummary.batch -ne 53 -or $coreSummary.source_head -cne $coreOrigin -or
            $coreSummary.core.fresh_invocation -isnot [bool] -or $coreSummary.core.fresh_invocation -ne $true) {
            throw 'Retained Core basis and summary must identify the actual fresh Core53 result and its pinned TRX.'
        }
        foreach ($counts in @($coreBasis.runtime_counts, $coreSummary.core)) {
            foreach ($field in @('passed', 'failed', 'skipped', 'total')) {
                $value = $counts.$field
                $expected = if ($field -in @('passed', 'total')) { 285 } else { 0 }
                if ($null -eq $value -or $value -is [bool] -or $value -isnot [ValueType] -or
                    ![double]::IsFinite([double]$value) -or $value -ne $expected) {
                    throw 'Retained Core53 summary/basis must record actual 285 passed, zero failed/skipped, 285 total.'
                }
            }
        }
        if ($coreEquivalence.schema_version -ne 1 -or $coreEquivalence.source_head -cne $Source -or
            $coreEquivalence.base_source_head -cne $coreOrigin -or
            $coreEquivalence.core_basis.file -cne $retainedCore.basis.file -or
            $coreEquivalence.core_basis.sha256 -ne $retainedCore.basis.sha256) {
            throw 'Retained Core equivalence must link the pinned Core53 basis to the exact final source.'
        }
        foreach ($flag in @('all_core_inputs_unchanged', 'git_source_delta_audited', 'projection_passed')) {
            if ($coreEquivalence.$flag -isnot [bool] -or $coreEquivalence.$flag -ne $true) {
                throw ('Retained Core source equivalence is not accepted: ' + $flag)
            }
        }
        $basisInputs = @($coreBasis.core_inputs)
        $currentInputs = @($coreEquivalence.core_inputs)
        if ($coreBasis.core_input_count -is [bool] -or $coreBasis.core_input_count -isnot [ValueType] -or
            $coreBasis.core_input_count -le 0 -or $coreBasis.core_input_count -ne $basisInputs.Count -or
            $coreEquivalence.core_input_count -is [bool] -or $coreEquivalence.core_input_count -isnot [ValueType] -or
            $coreEquivalence.core_input_count -ne $basisInputs.Count -or $currentInputs.Count -ne $basisInputs.Count) {
            throw 'Retained Core equivalence requires the complete nonempty captured input set.'
        }
        $basisPins = [Collections.Generic.Dictionary[string,string]]::new([StringComparer]::OrdinalIgnoreCase)
        foreach ($pin in $basisInputs) {
            $absolute = Resolve-NativePin53 $pin
            if ($basisPins.ContainsKey($absolute)) { throw 'Retained Core basis has duplicate canonical input paths.' }
            $basisPins.Add($absolute, $pin.sha256.ToUpperInvariant())
        }
        $currentPins = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
        foreach ($pin in $currentInputs) {
            $absolute = Resolve-NativePin53 $pin
            if (!$currentPins.Add($absolute) -or !$basisPins.ContainsKey($absolute) -or
                $basisPins[$absolute] -ne $pin.sha256) {
                throw 'Retained Core input paths/hashes differ from the captured Core53 basis.'
            }
        }
        $trxStream = [IO.MemoryStream]::new($coreBytes['trx'], $false)
        $trxSettings = [Xml.XmlReaderSettings]::new()
        $trxSettings.DtdProcessing = [Xml.DtdProcessing]::Prohibit
        $trxSettings.XmlResolver = $null
        $trxReader = $null
        try {
            $trxReader = [Xml.XmlReader]::Create($trxStream, $trxSettings)
            $trxDocument = [Xml.XmlDocument]::new()
            $trxDocument.XmlResolver = $null
            $trxDocument.Load($trxReader)
        } finally {
            if ($null -ne $trxReader) { $trxReader.Dispose() }
            $trxStream.Dispose()
        }
        $trxSummary = $trxDocument.SelectSingleNode("/*[local-name()='TestRun']/*[local-name()='ResultSummary']")
        $trxCounts = if ($null -ne $trxSummary) { $trxSummary.SelectSingleNode("*[local-name()='Counters']")
        } else { $null }
        $trxResults = $trxDocument.SelectNodes("/*[local-name()='TestRun']/*[local-name()='Results']/*[local-name()='UnitTestResult']")
        if ($null -eq $trxSummary -or $null -eq $trxCounts -or $trxSummary.GetAttribute('outcome') -cnotin @('Completed', 'Passed') -or
            $trxResults.Count -ne 285 -or @($trxResults | Where-Object { $_.GetAttribute('outcome') -cne 'Passed' }).Count) {
            throw 'Retained Core TRX must contain exactly 285 executed passing tests with no skipped or failed outcomes.'
        }
        foreach ($field in @('total', 'executed', 'passed', 'failed', 'error', 'timeout', 'aborted', 'inconclusive',
                'passedButRunAborted', 'notRunnable', 'notExecuted', 'disconnected', 'warning', 'completed', 'inProgress', 'pending')) {
            $value = $trxCounts.GetAttribute($field)
            $expected = if ($field -in @('total', 'executed', 'passed')) { 285 } else { 0 }
            if ($value -notmatch '^\d+$' -or [long]$value -ne $expected) {
                throw 'Retained Core TRX must contain exactly 285 executed passing tests with no skipped or failed outcomes.'
            }
        }
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

# Schema4 above remains the retained52/freshCLI53 route. Schema5 requires a
# new full workspace at this exact source, with fresh Core/Desktop/package.
function Read-NativeFreshCpuAdmission55([string]$Repo, [string]$Source, [string]$MarkerPath, [string]$MarkerHash, $Marker) {
    $prefix = $Repo + [IO.Path]::DirectorySeparatorChar
    function Assert-FreshInteger55($Value, [string]$Label, [long]$Minimum = 0) {
        if ($null -eq $Value -or $Value -is [bool] -or $Value -isnot [ValueType] -or
            ![double]::IsFinite([double]$Value) -or $Value -lt $Minimum -or
            $Value -gt [int]::MaxValue -or [Math]::Truncate([double]$Value) -ne $Value) {
            throw ('Missing or invalid integer: ' + $Label)
        }
    }
    function Assert-FreshTrue55($Value, [string]$Label) {
        if ($Value -isnot [bool] -or $Value -ne $true) { throw ('Fresh qualification requires typed true: ' + $Label) }
    }
    function Assert-FreshRustCounts55($Counts, [string]$Label, [bool]$RequirePass) {
        foreach ($field in @('passed', 'failed', 'ignored', 'filtered_out')) {
            Assert-FreshInteger55 $Counts.$field ($Label + ':' + $field)
        }
        if ($Counts.failed -ne 0 -or $Counts.ignored -ne 0 -or ($RequirePass -and $Counts.passed -le 0)) {
            throw ($Label + ' has failed, ignored, or missing passing tests.')
        }
    }
    function Resolve-FreshPin55($Pin) {
        if ($Pin.file -isnot [string] -or [string]::IsNullOrWhiteSpace($Pin.file) -or
            [IO.Path]::IsPathRooted($Pin.file) -or $Pin.sha256 -isnot [string] -or $Pin.sha256 -notmatch '^[0-9a-fA-F]{64}$') {
            throw 'Fresh qualification requires relative paths and exact SHA256 pins.'
        }
        $absolute = [IO.Path]::GetFullPath((Join-Path $Repo $Pin.file))
        if (!$absolute.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) -or
            !(Test-Path -LiteralPath $absolute -PathType Leaf)) { throw ('Fresh pin escapes the checkout or is missing: ' + $Pin.file) }
        $bytes = [IO.File]::ReadAllBytes($absolute)
        $hash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes))
        if ($hash -ne $Pin.sha256) { throw ('Fresh evidence/source pin changed: ' + $Pin.file) }
        return [ordered]@{path=$absolute;sha256=$hash;bytes=$bytes}
    }
    $allPaths = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    $evidence = [Collections.Generic.Dictionary[string,object]]::new([StringComparer]::OrdinalIgnoreCase)
    $sources = [Collections.Generic.Dictionary[string,string]]::new([StringComparer]::OrdinalIgnoreCase)
    Assert-FreshInteger55 $Marker.batch 'batch' 55
    $batch = [int]$Marker.batch
    if ($Marker.source_head -cne $Source -or
        $Marker.qualification_method -cne 'fresh_full_cpu_workspace_and_core_desktop_package') {
        throw 'Schema5 requires a fresh full CPU workspace at the exact final source.'
    }
    foreach ($flag in @('cpu_workspace_passed', 'core_passed', 'desktop_passed', 'package_passed',
            'source_qualification_passed', 'core_fresh_invocation', 'desktop_fresh_invocation',
            'package_fresh_invocation', 'source_qualification_fresh_invocation')) {
        Assert-FreshTrue55 $Marker.$flag $flag
    }
    Assert-FreshInteger55 $Marker.source_file_count 'source_file_count' 156
    if (@($Marker.source_files).Count -ne $Marker.source_file_count -or @($Marker.evidence).Count -lt 6) {
        throw 'Fresh qualification is missing source or sealed evidence pins.'
    }
    foreach ($pin in @($Marker.evidence)) {
        $resolved = Resolve-FreshPin55 $pin
        if (!$allPaths.Add($resolved.path)) { throw 'Duplicate canonical fresh evidence/source path.' }
        $evidence.Add($resolved.path, $resolved)
    }
    foreach ($pin in @($Marker.source_files)) {
        $resolved = Resolve-FreshPin55 $pin
        if (!$allPaths.Add($resolved.path)) { throw 'Duplicate canonical fresh evidence/source path.' }
        $sources.Add($resolved.path, $resolved.sha256)
    }
    function Read-FreshEvidence55($Pin, [string]$Label) {
        $resolved = Resolve-FreshPin55 $Pin
        if (!$evidence.ContainsKey($resolved.path) -or $evidence[$resolved.path].sha256 -ne $Pin.sha256) {
            throw ('Fresh evidence must be pinned explicitly: ' + $Label)
        }
        return $resolved
    }
    function Read-FreshJson55($Pin, [string]$Label) {
        $resolved = Read-FreshEvidence55 $Pin $Label
        return ([Text.UTF8Encoding]::new($false, $true).GetString($resolved.bytes) | ConvertFrom-Json)
    }
    $proof = 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass' + $batch + '/'
    if ($Marker.source_manifest.file -cne ('output/native-roundtrip-pass' + $batch + '-source.json') -or
        $Marker.rust_result.file -cne ($proof + 'rust-target-results.json') -or
        $Marker.summary.file -cne ($proof + 'summary.json') -or
        $Marker.core.trx.file -cne ($proof + 'core.trx') -or
        $Marker.desktop.trx.file -cne ($proof + 'desktop.trx') -or
        $Marker.genuine_movement.file -cne ($proof + 'genuine-movement-comparisons.json')) {
        throw 'Fresh qualification must use its own canonical source/Rust/summary/TRX/original-comparison receipts.'
    }
    $manifest = Read-FreshJson55 $Marker.source_manifest 'source manifest'
    if ($manifest.parent_head -cne $Source -or $manifest.qualification_state -cne ('all' + $batch + '_code_and_tests_authored_frozen') -or
        @($manifest.files).Count -ne $sources.Count) { throw 'Fresh source manifest has the wrong origin or source set.' }
    $manifestPaths = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($pin in @($manifest.files)) {
        $resolved = Resolve-FreshPin55 $pin
        if (!$manifestPaths.Add($resolved.path) -or !$sources.ContainsKey($resolved.path) -or $sources[$resolved.path] -ne $resolved.sha256) {
            throw 'Fresh source pins differ from the exact frozen manifest.'
        }
    }
    $rust = Read-FreshJson55 $Marker.rust_result 'full Rust target result'
    if ($rust.schema_version -ne 1 -or $rust.batch -ne $batch -or $rust.state -cne 'passed' -or $rust.source_head -cne $Source -or
        $rust.source_manifest.file -cne $Marker.source_manifest.file -or $rust.source_manifest.sha256 -ne $Marker.source_manifest.sha256) {
        throw 'Fresh Rust target receipt has the wrong origin or manifest.'
    }
    Assert-FreshTrue55 $rust.full_workspace_passed 'Rust full_workspace_passed'
    Assert-FreshTrue55 $rust.invocation.fresh_full_workspace_invocation 'Rust fresh_full_workspace_invocation'
    Assert-FreshTrue55 $rust.invocation.hardware_filtered 'Rust hardware_filtered'
    Assert-FreshInteger55 $rust.invocation.exit_code 'Rust exit_code'
    if ($rust.invocation.exit_code -ne 0 -or
        (@($rust.command) -join '|') -cne 'test|--release|--workspace|--no-fail-fast|--|--skip|gpu|--skip|switch|--nocapture|--test-threads=1') {
        throw 'Fresh Rust requires the successful full hardware-filtered workspace invocation.'
    }
    Assert-FreshInteger55 $rust.expected_distinct_targets 'Rust expected targets' 67
    Assert-FreshInteger55 $rust.total_distinct_targets 'Rust actual targets' 67
    if ($rust.total_distinct_targets -ne $rust.expected_distinct_targets -or @($rust.all_targets).Count -ne $rust.total_distinct_targets) {
        throw 'Fresh Rust must contain every expected distinct target.'
    }
    $targetKeys = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $sums = @{passed=[long]0;failed=[long]0;ignored=[long]0;filtered_out=[long]0}
    foreach ($target in @($rust.all_targets)) {
        if ($target.package -isnot [string] -or [string]::IsNullOrWhiteSpace($target.package) -or
            $target.name -isnot [string] -or [string]::IsNullOrWhiteSpace($target.name) -or
            $target.kind -cnotin @('lib', 'bin', 'test', 'doctest')) { throw 'Fresh Rust target identity is invalid.' }
        $name = if ($target.kind -ceq 'lib' -and $target.package -ceq 'darwinbots-cli' -and
            $target.name -cin @('darwinbots-cli', 'darwinbots_cli')) { 'darwinbots-cli' } else { $target.name }
        $key = $target.package + ':' + $target.kind + ':' + $name
        if (!$targetKeys.Add($key) -or $target.status -cne 'ok') { throw ('Duplicate or unsuccessful fresh Rust target: ' + $key) }
        Assert-FreshRustCounts55 $target ('Fresh Rust target ' + $key) $false
        foreach ($field in @('passed', 'failed', 'ignored', 'filtered_out')) { $sums[$field] += [long]$target.$field }
    }
    if (!$targetKeys.Contains('darwinbots-cli:lib:darwinbots-cli') -or
        !$targetKeys.Contains('darwinbots-engine:test:db24832_genuine_load_transients')) {
        throw 'Fresh Rust is missing the CLI library or new genuine native-load integration target.'
    }
    foreach ($cpu in @($rust.counts, $Marker.rust_cpu)) {
        Assert-FreshRustCounts55 $cpu 'Fresh Rust aggregate' $true
        foreach ($field in @('passed', 'failed', 'ignored', 'filtered_out')) {
            if ($cpu.$field -ne $sums[$field]) { throw ('Fresh Rust aggregate differs from actual target rows: ' + $field) }
        }
    }
    Assert-FreshTrue55 $Marker.rust_cpu.fresh_full_workspace_invocation 'marker fresh_full_workspace_invocation'
    Assert-FreshTrue55 $Marker.rust_cpu.hardware_filtered 'marker hardware_filtered'
    if ($Marker.rust_cpu.expected_distinct_targets -ne $rust.expected_distinct_targets -or
        $Marker.rust_cpu.total_distinct_targets -ne $rust.total_distinct_targets) { throw 'Fresh marker Rust target counts differ from the actual receipt.' }
    $summary = Read-FreshJson55 $Marker.summary 'summary'
    if ($summary.schema_version -ne 1 -or $summary.batch -ne $batch -or $summary.source_head -cne $Source -or
        $summary.source_manifest.file -cne $Marker.source_manifest.file -or $summary.source_manifest.sha256 -ne $Marker.source_manifest.sha256) {
        throw 'Fresh summary has the wrong origin or manifest.'
    }
    foreach ($flag in @('all_ordinary_local_outcomes_collected', 'local_cpu_passed', 'source_inputs_unchanged')) {
        Assert-FreshTrue55 $summary.$flag ('summary ' + $flag)
    }
    foreach ($field in @('format_exit_code', 'strict_clippy_exit_code')) {
        Assert-FreshInteger55 $summary.$field ('summary ' + $field)
        if ($summary.$field -ne 0) { throw ('Fresh summary failed ' + $field) }
    }
    Assert-FreshTrue55 $summary.rust_cpu.fresh_full_workspace_invocation 'summary fresh_full_workspace_invocation'
    Assert-FreshTrue55 $summary.rust_cpu.hardware_filtered 'summary hardware_filtered'
    Assert-FreshRustCounts55 $summary.rust_cpu 'Fresh summary Rust aggregate' $true
    foreach ($field in @('passed', 'failed', 'ignored', 'filtered_out', 'expected_distinct_targets', 'total_distinct_targets')) {
        if ($summary.rust_cpu.$field -ne $Marker.rust_cpu.$field) { throw ('Fresh summary Rust result differs: ' + $field) }
    }
    # Batch55 keeps its qualified fixed census. Later fresh batches declare their
    # exact managed census in the same pinned manifest as their source inputs.
    $managedTestCounts = @{core=285;desktop=189}
    if ($batch -ge 56) {
        foreach ($name in @('core', 'desktop')) {
            $count = $manifest.expected_managed_test_counts.$name
            Assert-FreshInteger55 $count ('manifest expected_managed_test_counts.' + $name) 1
            $managedTestCounts[$name] = [int]$count
        }
    }
    foreach ($name in @('core', 'desktop')) {
        $count = $managedTestCounts[$name]
        $declared = $Marker.$name
        $sealed = $summary.$name
        if ($declared.source_head -cne $Source) { throw ('Fresh ' + $name + ' has the wrong origin.') }
        Assert-FreshTrue55 $declared.fresh_invocation ($name + ' marker fresh_invocation')
        Assert-FreshTrue55 $sealed.fresh_invocation ($name + ' summary fresh_invocation')
        foreach ($field in @('passed', 'failed', 'skipped', 'total')) {
            $expected = if ($field -in @('passed', 'total')) { $count } else { 0 }
            Assert-FreshInteger55 $declared.$field ($name + ':' + $field)
            Assert-FreshInteger55 $sealed.$field ($name + ' summary:' + $field)
            if ($declared.$field -ne $expected -or $sealed.$field -ne $expected) { throw ('Fresh ' + $name + ' counts do not match the executed test set.') }
        }
        $trx = Read-FreshEvidence55 $declared.trx ($name + ' TRX')
        $stream = [IO.MemoryStream]::new($trx.bytes, $false)
        $settings = [Xml.XmlReaderSettings]::new()
        $settings.DtdProcessing = [Xml.DtdProcessing]::Prohibit
        $settings.XmlResolver = $null
        $reader = $null
        try {
            $reader = [Xml.XmlReader]::Create($stream, $settings)
            $document = [Xml.XmlDocument]::new()
            $document.XmlResolver = $null
            $document.Load($reader)
        } finally { if ($null -ne $reader) { $reader.Dispose() }; $stream.Dispose() }
        $resultSummary = $document.SelectSingleNode("/*[local-name()='TestRun']/*[local-name()='ResultSummary']")
        $counters = if ($null -ne $resultSummary) { $resultSummary.SelectSingleNode("*[local-name()='Counters']") } else { $null }
        $rows = $document.SelectNodes("/*[local-name()='TestRun']/*[local-name()='Results']/*[local-name()='UnitTestResult']")
        if ($null -eq $resultSummary -or $null -eq $counters -or $resultSummary.GetAttribute('outcome') -cnotin @('Completed', 'Passed') -or
            $rows.Count -ne $count -or @($rows | Where-Object { $_.GetAttribute('outcome') -cne 'Passed' }).Count) {
            throw ('Fresh ' + $name + ' TRX contains a failed, skipped, missing, or unexpected result despite its counters.')
        }
        $executionIds = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
        foreach ($row in $rows) {
            $id = $row.GetAttribute('executionId')
            if ([string]::IsNullOrWhiteSpace($id) -or !$executionIds.Add($id)) { throw ('Fresh ' + $name + ' TRX has missing/duplicate execution identities.') }
        }
        foreach ($field in @('total', 'executed', 'passed', 'failed', 'error', 'timeout', 'aborted', 'inconclusive',
                'passedButRunAborted', 'notRunnable', 'notExecuted', 'disconnected', 'warning', 'completed', 'inProgress', 'pending')) {
            $value = $counters.GetAttribute($field)
            $expected = if ($field -in @('total', 'executed', 'passed')) { $count } else { 0 }
            if ($value -notmatch '^\d+$' -or [long]$value -ne $expected) { throw ('Fresh ' + $name + ' TRX counters do not match its executed passing results.') }
        }
    }
    Assert-FreshTrue55 $summary.desktop.exact_package_native_dll 'Desktop exact_package_native_dll'
    Assert-FreshTrue55 $summary.package.fresh_invocation 'package fresh_invocation'
    Assert-FreshInteger55 $summary.package.publish_exit_code 'package publish_exit_code'
    Assert-FreshInteger55 $summary.package.assets 'package assets'
    Assert-FreshInteger55 $summary.package.asset_mismatches 'package asset_mismatches'
    if ($summary.package.publish_exit_code -ne 0 -or $summary.package.smoke.status -cne 'passed' -or
        $summary.package.smoke.backend -cne 'CPU' -or $summary.package.assets -ne 48 -or $summary.package.asset_mismatches -ne 0) {
        throw 'Fresh package publish, CPU smoke, or all 48 bot assets did not pass.'
    }
    $dll = $summary.package.native_dll_identity
    if ($dll.target_release -notmatch '^[0-9a-fA-F]{64}$' -or $dll.target_release -ne $dll.package -or $dll.target_release -ne $dll.desktop) {
        throw 'Fresh package/Desktop/target native DLL identities differ.'
    }
    $expectedMovementCycles = if ($batch -ge 60) { @(1,10,100,102) } else { @(1,10,102) }
    $genuine = Read-FreshJson55 $Marker.genuine_movement 'genuine movement comparisons'
    Assert-FreshTrue55 $genuine.passed 'genuine movement passed'
    if ($genuine.schema_version -ne 1 -or $genuine.batch -ne $batch -or $genuine.source_head -cne $Source -or $genuine.source_manifest_sha256 -ne $Marker.source_manifest.sha256 -or
        (@($genuine.expected_cycles) -join '|') -cne ($expectedMovementCycles -join '|') -or @($genuine.checkpoints).Count -ne $expectedMovementCycles.Count) { throw 'Fresh genuine movement receipt has the wrong origin or checkpoints.' }
    $cycles = [Collections.Generic.HashSet[int]]::new()
    foreach ($row in @($genuine.checkpoints)) {
        Assert-FreshInteger55 $row.cycle 'genuine movement cycle' 1
        if ($row.cycle -notin $expectedMovementCycles -or !$cycles.Add([int]$row.cycle)) {
            if ($batch -ge 60) { throw 'Fresh genuine movement cycles must be distinct C1/C10/C100/C102.' }
            throw 'Fresh genuine movement cycles must be distinct C1/C10/C102.'
        }
        Assert-FreshTrue55 $row.passed 'genuine movement checkpoint passed'
        foreach ($field in @('candidate_exit_code', 'compare_exit_code', 'difference_count', 'absolute_tolerance', 'relative_tolerance')) {
            Assert-FreshInteger55 $row.$field ('genuine movement ' + $field)
            if ($row.$field -ne 0) { throw ('Fresh genuine movement failed ' + $field) }
        }
        if ($row.compared_fields -ne 1379) { throw 'Fresh genuine movement must compare every expected live field.' }
        $original = Resolve-FreshPin55 @{file=$row.original;sha256=$row.original_sha256}
        if (!$sources.ContainsKey($original.path) -or $sources[$original.path] -ne $original.sha256) { throw 'Fresh original capture is not one of the frozen source fixtures.' }
    }
    foreach ($pin in @($summary.evidence)) { $null = Read-FreshEvidence55 $pin 'summary evidence' }
    return [ordered]@{path=$MarkerPath;sha256=$MarkerHash;marker=$Marker;rust_result=$rust;summary=$summary}
}
