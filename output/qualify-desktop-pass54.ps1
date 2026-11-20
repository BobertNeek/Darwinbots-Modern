#requires -Version 7.0
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
[Diagnostics.Process]::GetCurrentProcess().PriorityClass = 'BelowNormal'
$env:RAYON_NUM_THREADS = '1'
$env:MSBUILDDISABLENODEREUSE = '1'
$env:DOTNET_CLI_USE_MSBUILD_SERVER = '0'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Set-Location -LiteralPath $repo
$manifestPath = Join-Path $repo 'output/native-desktop-pass54-source.json'
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
$expectedParent = [string]$manifest.parent_head
$comparisonBase = [string]$manifest.comparison_base
if ($manifest.qualification_state -ne 'all54_code_and_tests_authored_frozen' -or
    $expectedParent -cnotmatch '^[0-9a-f]{40}$' -or $expectedParent -ceq $comparisonBase -or
    $comparisonBase -cne 'e0300ca1391e0028b39aef2b61031a98934f00db') {
    throw 'Batch54 requires its released exact final HEAD and exact source53 comparison base.'
}
$manifestHash = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash
$runnerHash = (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash
$proof = Join-Path $repo 'docs/parity/evidence/batch-2026-10-14/native-desktop-pass54'
$package = Join-Path $repo 'artifacts/win-x64-native-desktop-pass54'
foreach ($freshPath in @($proof, $package)) {
    if (Test-Path -LiteralPath $freshPath) { throw "Do not merge or overwrite a batch54 artifact: $freshPath" }
}
$pinnedNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
$pinnedAbsolute = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($file in $manifest.files) {
    $relative = ([string]$file.file).Replace('\', '/')
    $absolute = [IO.Path]::GetFullPath((Join-Path $repo $relative))
    if ([IO.Path]::IsPathRooted($relative) -or
        !$absolute.StartsWith($repo + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -or
        !$pinnedNames.Add($relative) -or !$pinnedAbsolute.Add($absolute) -or
        [string]$file.sha256 -notmatch '^[0-9a-fA-F]{64}$') { throw "Invalid or duplicate source pin: $relative" }
}
if ($manifest.source_file_count -lt 161 -or @($manifest.files).Count -ne $manifest.source_file_count -or
    $pinnedNames.Count -ne $manifest.source_file_count -or !$pinnedNames.Contains('output/qualify-desktop-pass54.ps1')) {
    throw 'The final source pin count must match the manifest and include this runner.'
}
$fixture = 'modern/desktop/tests/Darwinbots.Desktop.Tests/RenderedControlSurfaceAuditTests.cs'
$guardedPaths = @('modern', 'Cargo.toml', 'Cargo.lock', '.cargo', 'Installer/bots',
    '.gitattributes', '.github/workflows/modern-cross-platform.yml')
$retainedPaths = @(
    'target/release/darwinbots_engine.dll',
    'modern/desktop/src/Darwinbots.Desktop.Core/bin/Release/net10.0/Darwinbots.Desktop.Core.dll',
    'modern/desktop/src/Darwinbots.Desktop/bin/Release/net10.0/Darwinbots.Desktop.dll',
    'modern/desktop/src/Darwinbots.Desktop/bin/Release/net10.0/win-x64/Darwinbots.Desktop.dll',
    'modern/desktop/src/Darwinbots.Desktop/bin/Release/net10.0/win-x64/Darwinbots.Desktop.Core.dll',
    'modern/desktop/src/Darwinbots.Desktop/obj/project.assets.json',
    'modern/desktop/tests/Darwinbots.Desktop.Tests/obj/project.assets.json'
)
$retainedHashes = [Collections.Generic.Dictionary[string,string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($file in $manifest.retained_product_artifacts) {
    $relative = ([string]$file.file).Replace('\', '/')
    if ($relative -cnotin $retainedPaths -or $retainedHashes.ContainsKey($relative) -or
        [string]$file.sha256 -notmatch '^[0-9a-fA-F]{64}$') { throw 'Invalid retained source53 product artifact pin.' }
    $retainedHashes.Add($relative, [string]$file.sha256)
}
if ($retainedHashes.Count -ne $retainedPaths.Count -or $manifest.product_build_origin_source -cne $comparisonBase -or
    $manifest.product_source_inputs_equivalent_to53 -isnot [bool] -or
    $manifest.product_source_inputs_equivalent_to53 -ne $true) { throw 'Explicit source53 product artifact/input equivalence is required.' }
[IO.Directory]::CreateDirectory($proof) | Out-Null
$runLock = [IO.File]::Open((Join-Path $proof 'run.lock'), [IO.FileMode]::CreateNew, [IO.FileAccess]::ReadWrite, [IO.FileShare]::Read)
[IO.File]::Copy($manifestPath, (Join-Path $proof 'source.json'), $false)
$env:DARWINBOTS_GUI_AUDIT_DIR = Join-Path $proof 'gui'
$results = [Collections.Generic.List[object]]::new()
$qualificationExit = 1
$script:preservedImageRecorded = $false
$preservedImage = Join-Path $repo 'docs/parity/receipts/db24832-world06-shape-editor.png'
function Write-ProofJson($name, $value) {
    ConvertTo-Json -InputObject $value -Depth 24 |
        Set-Content -LiteralPath (Join-Path $proof $name) -Encoding utf8NoBOM
}
function Save-QualificationState($status, $step, $details) {
    Write-ProofJson 'state.json' ([ordered]@{
        status = $status; step = $step; pid = $PID; utc = [DateTime]::UtcNow.ToString('o')
        source_head = $expectedParent; comparison_base = $comparisonBase
        source_manifest_sha256 = $manifestHash; details = $details
    })
}
function Add-QualificationResult($result) {
    $results.Add($result)
    Write-ProofJson 'results.json' ($results.ToArray())
}
function Add-DependentResult($label, $reason) {
    Add-QualificationResult ([ordered]@{ step = $label; exit_code = 125; status = 'not_run_dependency'; reason = $reason })
}
function Get-PreservedImageState {
    if (!(Test-Path -LiteralPath $preservedImage)) { return [ordered]@{ kind = 'missing'; sha256 = $null } }
    if (!(Test-Path -LiteralPath $preservedImage -PathType Leaf)) { return [ordered]@{ kind = 'not_a_file'; sha256 = $null } }
    try {
        return [ordered]@{ kind = 'file'; sha256 = (Get-FileHash -LiteralPath $preservedImage -Algorithm SHA256).Hash
            bytes = (Get-Item -LiteralPath $preservedImage).Length }
    } catch {
        return [ordered]@{ kind = 'unreadable'; sha256 = $null; error = $_.Exception.Message }
    }
}
$preservedImageBefore = Get-PreservedImageState
Write-ProofJson 'preserved-wip-image-before.json' $preservedImageBefore
function Record-PreexistingImage {
    if ($script:preservedImageRecorded) { return }
    $after = Get-PreservedImageState
    $unchanged = $preservedImageBefore.kind -eq $after.kind -and $after.kind -ne 'unreadable' -and
        $preservedImageBefore.sha256 -eq $after.sha256
    Write-ProofJson 'preserved-wip-image.json' ([ordered]@{
        file = 'docs/parity/receipts/db24832-world06-shape-editor.png'
        before = $preservedImageBefore; after = $after; unchanged = $unchanged
        policy = 'Observed only. No copy, restore, capture or overwrite of this preexisting PNG.'
    })
    Add-QualificationResult ([ordered]@{ step = 'preexisting-wip-image'; exit_code = $(if ($unchanged) { 0 } else { 1 })
        log = 'preserved-wip-image.json' })
    $script:preservedImageRecorded = $true
}
function Check-QualificationSource {
    if (Test-Path -LiteralPath (Join-Path $repo 'output/STOP-native-pass54')) { throw 'Owned batch54 STOP marker present.' }
    if ((Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash -ne $manifestHash -or
        (Get-FileHash -LiteralPath (Join-Path $proof 'source.json') -Algorithm SHA256).Hash -ne $manifestHash -or
        (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash -ne $runnerHash) { throw 'Frozen batch54 manifest/runner changed.' }
    foreach ($file in $manifest.files) {
        if ((Get-FileHash -LiteralPath (Join-Path $repo $file.file) -Algorithm SHA256).Hash -ne $file.sha256) {
            throw "Qualified source changed: $($file.file)"
        }
    }
    foreach ($file in $retainedPaths) {
        if ((Get-FileHash -LiteralPath (Join-Path $repo $file) -Algorithm SHA256).Hash -ne $retainedHashes[$file]) {
            throw "Retained source53 product/restore artifact changed: $file"
        }
    }
    $head = git -C $repo rev-parse HEAD 2>> (Join-Path $proof 'source-check.stderr.log')
    if ($LASTEXITCODE -ne 0 -or $head -cne $expectedParent) { throw 'Final committed source54 HEAD changed.' }
    $delta = @(git -C $repo -c core.quotepath=false diff --no-renames --name-only $comparisonBase $expectedParent -- @guardedPaths 2>> (Join-Path $proof 'source-check.stderr.log'))
    if ($LASTEXITCODE -ne 0 -or $delta.Count -ne 1 -or $delta[0] -cne $fixture) { throw 'Source54 product scope exceeds the one reviewed Desktop fixture file.' }
    git -C $repo diff --quiet $expectedParent -- @guardedPaths 2>> (Join-Path $proof 'source-check.stderr.log')
    if ($LASTEXITCODE -ne 0) { throw 'Guarded source differs from final committed source54.' }
    $untracked = @(git -C $repo ls-files --others --exclude-standard -- @guardedPaths 2>> (Join-Path $proof 'source-check.stderr.log'))
    if ($LASTEXITCODE -ne 0 -or $untracked.Count -ne 0) { throw 'Unqualified untracked source found.' }
}
function Run-QualificationStep($label, $executable, [string[]]$arguments) {
    Check-QualificationSource
    $log = Join-Path $proof ($label + '.log')
    if (Test-Path -LiteralPath $log) { throw "Refusing to overwrite a qualification step log: $label" }
    Save-QualificationState 'running' $label @{ log = $log }
    $started = [DateTime]::UtcNow
    $invocationError = $null
    try {
        & $executable @arguments *> $log
        $exitCode = if ($null -eq $LASTEXITCODE) { 1 } else { [int]$LASTEXITCODE }
    } catch {
        $exitCode = 1
        $invocationError = $_.Exception.Message
        $_ | Out-String | Add-Content -LiteralPath $log
    }
    Add-QualificationResult ([ordered]@{
        step = $label; exit_code = $exitCode; seconds = ([DateTime]::UtcNow - $started).TotalSeconds
        executable = $executable; arguments = $arguments; log = $label + '.log'; invocation_error = $invocationError
    })
    Check-QualificationSource
    return $exitCode
}
function Get-NativeDllIdentity($includeDesktop) {
    [ordered]@{
        target_release = (Get-FileHash -LiteralPath (Join-Path $repo 'target/release/darwinbots_engine.dll') -Algorithm SHA256).Hash
        package = (Get-FileHash -LiteralPath (Join-Path $package 'darwinbots_engine.dll') -Algorithm SHA256).Hash
        desktop = if ($includeDesktop) {
            (Get-FileHash -LiteralPath (Join-Path $repo 'modern/desktop/tests/Darwinbots.Desktop.Tests/bin/Release/net10.0/darwinbots_engine.dll') -Algorithm SHA256).Hash
        } else { $null }
    }
}

$dotnetOptions = @('--disable-build-servers', '-m:1', '-nr:false', '-p:UseSharedCompilation=false')
$testProject = 'modern/desktop/tests/Darwinbots.Desktop.Tests/Darwinbots.Desktop.Tests.csproj'
$testOutput = Join-Path $repo 'modern/desktop/tests/Darwinbots.Desktop.Tests/bin/Release/net10.0'
Write-ProofJson 'runner-identity.json' ([ordered]@{
    file = 'output/qualify-desktop-pass54.ps1'; sha256 = $runnerHash; source_head = $expectedParent
    comparison_base = $comparisonBase; source_manifest_sha256 = $manifestHash; source_file_count = $pinnedNames.Count
    priority = 'BelowNormal'; msbuild_workers = 1; msbuild_node_reuse = $false
    test_restore = $false; test_project_references_built = $false; gui_audit_directory = $env:DARWINBOTS_GUI_AUDIT_DIR
    package_fresh_publish_invocation = $true; product_fresh_build = $false
    product_build_origin_source = $comparisonBase; product_source_inputs_equivalent_to53 = $true
    retained_product_artifacts = @($manifest.retained_product_artifacts)
    rust_tests_run = $false; core_tests_run = $false; cli_or_zero_or_candidate_or_ecology_run = $false
    fresh_full_cpu_workspace_invocation = $false
    scope = 'Fresh source54 Desktop tests and publish only; root seals retained source-equivalent53 Rust/Core/CLI/zero/candidate evidence separately.'
})
try {
    Check-QualificationSource
    $desktopBuilt = Run-QualificationStep 'desktop-test-build' 'dotnet.exe' (@(
        'build', $testProject, '-c', 'Release', '--no-dependencies', '--no-restore', '-p:BuildProjectReferences=false') + $dotnetOptions)
    if ($desktopBuilt -eq 0) {
        $managed = @(foreach ($name in @('Darwinbots.Desktop.dll', 'Darwinbots.Desktop.Core.dll')) {
            $origin = if ($name -eq 'Darwinbots.Desktop.dll') { $retainedPaths[2] } else { $retainedPaths[1] }
            $actual = (Get-FileHash -LiteralPath (Join-Path $testOutput $name) -Algorithm SHA256).Hash
            [ordered]@{ file = $name; origin = $origin; expected_sha256 = $retainedHashes[$origin]; actual_sha256 = $actual; exact_bytes = $actual -eq $retainedHashes[$origin] }
        })
        Write-ProofJson 'desktop-managed-dependencies.json' $managed
        if (@($managed | Where-Object { !$_.exact_bytes }).Count) { throw 'Desktop test product dependency differs from its retained53 artifact.' }
    }
    if (Test-Path -LiteralPath $package) { throw 'Fresh54 package directory appeared before publish.' }
    $published = Run-QualificationStep 'windows-publish' 'dotnet.exe' (@(
        'publish', 'modern/desktop/src/Darwinbots.Desktop/Darwinbots.Desktop.csproj', '-c', 'Release',
        '-r', 'win-x64', '--self-contained', 'false', '--no-build', '--no-restore', '-o', $package) + $dotnetOptions)
    if ($published -eq 0) {
        $publishedManaged = @(foreach ($name in @('Darwinbots.Desktop.dll', 'Darwinbots.Desktop.Core.dll')) {
            $origin = if ($name -eq 'Darwinbots.Desktop.dll') { $retainedPaths[3] } else { $retainedPaths[4] }
            $actual = (Get-FileHash -LiteralPath (Join-Path $package $name) -Algorithm SHA256).Hash
            [ordered]@{ file = $name; origin = $origin; expected_sha256 = $retainedHashes[$origin]; actual_sha256 = $actual; exact_bytes = $actual -eq $retainedHashes[$origin] }
        })
        Write-ProofJson 'package-managed-dependencies.json' $publishedManaged
        if (@($publishedManaged | Where-Object { !$_.exact_bytes }).Count) { throw 'Published product differs from its retained53 compiled artifact.' }
        if ($desktopBuilt -eq 0) {
            Copy-Item -LiteralPath (Join-Path $package 'darwinbots_engine.dll') -Destination (Join-Path $testOutput 'darwinbots_engine.dll')
        }
        $dllBefore = Get-NativeDllIdentity ($desktopBuilt -eq 0)
        Write-ProofJson 'native-dll-identity-before.json' $dllBefore
        if ($dllBefore.target_release -ne $retainedHashes[$retainedPaths[0]] -or
            $dllBefore.target_release -ne $dllBefore.package -or
            ($desktopBuilt -eq 0 -and $dllBefore.package -ne $dllBefore.desktop)) { throw 'Target/package/Desktop native DLL identity mismatch.' }
        foreach ($smokeFile in @('package-smoke.json', 'package-smoke.stderr.log')) {
            if (Test-Path -LiteralPath (Join-Path $proof $smokeFile)) { throw 'Do not reuse package smoke output' }
        }
        Save-QualificationState 'running' 'package-smoke' @{}
        $smokeStarted = [DateTime]::UtcNow
        $smokeTimedOut = $false
        $smoke = $null
        try {
            $smokeArguments = @{
                FilePath = Join-Path $package 'Darwinbots.Desktop.exe'; ArgumentList = '--smoke-test'
                WorkingDirectory = $package; WindowStyle = 'Hidden'; PassThru = $true
                RedirectStandardOutput = Join-Path $proof 'package-smoke.json'
                RedirectStandardError = Join-Path $proof 'package-smoke.stderr.log'
            }
            $smoke = Start-Process @smokeArguments
            $null = $smoke.Handle
            $smokeTimedOut = !$smoke.WaitForExit(120000)
            if ($smokeTimedOut) { $smoke.Kill($true); $smoke.WaitForExit() }
            $smoke.WaitForExit()
            if ($null -eq $smoke.ExitCode) { throw 'CPU package smoke completed without an exit code' }
            $smokeExit = if ($smokeTimedOut) { 124 } else { $smoke.ExitCode }
            $smokeError = $null
        } catch {
            $smokeExit = 1
            $smokeError = $_.Exception.Message
            if ($null -ne $smoke -and !$smoke.HasExited) {
                $smoke.Kill($true)
                $smoke.WaitForExit()
            }
        }
        Add-QualificationResult ([ordered]@{
            step = 'package-smoke'; exit_code = $smokeExit; timed_out = $smokeTimedOut
            seconds = ([DateTime]::UtcNow - $smokeStarted).TotalSeconds
            log = 'package-smoke.json'; invocation_error = $smokeError
        })
        Check-QualificationSource
        $smokeContractError = $null
        $smokeReport = $null
        try {
            $smokeReport = Get-Content -LiteralPath (Join-Path $proof 'package-smoke.json') -Raw | ConvertFrom-Json
            $smokeContract = $smokeExit -eq 0 -and $smokeReport.status -ceq 'passed' -and $smokeReport.backend -ceq 'CPU' -and
                $smokeReport.ticks -eq 13 -and $smokeReport.population -eq 1 -and $smokeReport.movement -eq $true -and
                $smokeReport.save_resume -eq $true -and $smokeReport.bundled_bots -eq 48
        } catch { $smokeContract = $false; $smokeContractError = $_.Exception.Message }
        Write-ProofJson 'package-smoke-contract.json' ([ordered]@{ passed = $smokeContract; report = $smokeReport; error = $smokeContractError })
        Add-QualificationResult ([ordered]@{ step = 'package-smoke-contract'; exit_code = $(if ($smokeContract) { 0 } else { 1 }); log = 'package-smoke-contract.json' })
        if ($desktopBuilt -eq 0) {
            if (Test-Path -LiteralPath (Join-Path $proof 'desktop.trx')) { throw 'Do not reuse a Desktop TRX.' }
            $null = Run-QualificationStep 'packaged-native-desktop' 'dotnet.exe' (@(
                'test', $testProject, '-c', 'Release', '--no-build', '--no-restore', '--blame-hang-timeout', '10m',
                '--blame-hang-dump-type', 'none', '--logger', 'trx;LogFileName=desktop.trx', '--results-directory', $proof) + $dotnetOptions)
            $trxError = $null
            try {
                $trx = [xml](Get-Content -LiteralPath (Join-Path $proof 'desktop.trx') -Raw)
                $counts = $trx.TestRun.ResultSummary.Counters
                $trxResults = @($trx.TestRun.Results.UnitTestResult)
                $complete = [int]$counts.total -eq 189 -and [int]$counts.executed -eq 189 -and
                    [int]$counts.passed -eq 189 -and [int]$counts.failed -eq 0 -and [int]$counts.notExecuted -eq 0 -and
                    $trxResults.Count -eq 189 -and @($trxResults | Where-Object { $_.outcome -cne 'Passed' }).Count -eq 0
                $desktopCounts = [ordered]@{ total = [int]$counts.total; executed = [int]$counts.executed; passed = [int]$counts.passed; failed = [int]$counts.failed; skipped = [int]$counts.notExecuted }
            } catch { $complete = $false; $trxError = $_.Exception.Message; $desktopCounts = $null }
            Write-ProofJson 'desktop-trx-counts.json' ([ordered]@{ complete189 = $complete; counts = $desktopCounts; error = $trxError })
            Add-QualificationResult ([ordered]@{ step = 'desktop-trx-completeness'; exit_code = $(if ($complete) { 0 } else { 1 }); log = 'desktop-trx-counts.json' })
        } else {
            Add-DependentResult 'packaged-native-desktop' 'Fresh Desktop test build failed.'
            Add-DependentResult 'desktop-trx-completeness' 'Fresh Desktop test build failed.'
        }
        $dllAfter = Get-NativeDllIdentity ($desktopBuilt -eq 0)
        Write-ProofJson 'native-dll-identity.json' $dllAfter
        if ($dllAfter.target_release -ne $dllBefore.target_release -or $dllAfter.package -ne $dllBefore.package -or
            $dllAfter.desktop -ne $dllBefore.desktop) {
            throw 'Qualified native DLL changed during package smoke or Desktop tests'
        }
        $assets = @(Get-ChildItem -LiteralPath (Join-Path $repo 'Installer/bots') -File -Filter '*.txt' -Recurse)
        $assetRows = @(foreach ($asset in $assets) {
            $relative = [IO.Path]::GetRelativePath((Join-Path $repo 'Installer/bots'), $asset.FullName)
            $target = if ($relative -in @('Animal_Minimalis.txt', 'Alga_Minimalis_Chloroplastus.txt')) {
                Join-Path (Join-Path $package 'Bots') $relative
            } else { Join-Path (Join-Path $package 'Bots/Historical') $relative }
            $expectedHash = (Get-FileHash -LiteralPath $asset.FullName -Algorithm SHA256).Hash
            $actualHash = if (Test-Path -LiteralPath $target -PathType Leaf) { (Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash } else { $null }
            [ordered]@{ source = $relative.Replace('\', '/'); packaged = [IO.Path]::GetRelativePath($package, $target).Replace('\', '/')
                source_sha256 = $expectedHash; package_sha256 = $actualHash; exact_bytes = $expectedHash -eq $actualHash }
        })
        $assetMismatches = @($assetRows | Where-Object { !$_.exact_bytes })
        $packagedBotCount = if (Test-Path -LiteralPath (Join-Path $package 'Bots') -PathType Container) {
            @(Get-ChildItem -LiteralPath (Join-Path $package 'Bots') -File -Filter '*.txt' -Recurse).Count
        } else { 0 }
        Write-ProofJson 'package-assets.json' ([ordered]@{
            bots = $assets.Count; packaged_bots = $packagedBotCount; asset_mismatches = $assetMismatches; files = $assetRows
        })
        Add-QualificationResult ([ordered]@{
            step = 'package-assets'; exit_code = $(if ($assets.Count -eq 48 -and $packagedBotCount -eq 48 -and $assetMismatches.Count -eq 0) { 0 } else { 1 })
            bots = $assets.Count; packaged_bots = $packagedBotCount; log = 'package-assets.json'
        })
    } else {
        foreach ($label in @('package-smoke', 'package-smoke-contract', 'packaged-native-desktop', 'desktop-trx-completeness', 'package-assets')) {
            Add-DependentResult $label 'Fresh54 publish failed; no previous package or stale Desktop result used.'
        }
    }
    Record-PreexistingImage
    Check-QualificationSource
    $failed = @($results | Where-Object exit_code -ne 0)
    Save-QualificationState $(if ($failed.Count) { 'needs_repair' } else { 'completed' }) 'all' @{
        failed_steps = @($failed | ForEach-Object step); all_ordinary_outcomes_collected = $true
        fresh_scope = 'Desktop test build/full 189 tests, Windows publish, CPU 13-tick smoke, 48 exact assets, final source/PNG guards.'
        package_fresh_publish_invocation = $true; product_fresh_build = $false; product_build_origin_source = $comparisonBase
        fresh_full_cpu_workspace_invocation = $false
        retention = 'Root separately seals exact source-equivalent 53 Rust/Core/CLI/zero and 21 candidate evidence. This runner does not produce or relabel a fresh full CPU result.'
    }
    $qualificationExit = if ($failed.Count) { 1 } else { 0 }
    ConvertTo-Json -InputObject ($results.ToArray()) -Depth 8
} catch {
    Save-QualificationState 'stopped' 'qualification' @{ error = $_.Exception.Message; all_ordinary_outcomes_collected = $false }
    throw
} finally {
    try { Record-PreexistingImage } catch { Write-Host ('Could not record final preexisting PNG identity: ' + $_.Exception.Message) }
    $runLock.Dispose()
}
exit $qualificationExit
