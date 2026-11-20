$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
[Diagnostics.Process]::GetCurrentProcess().PriorityClass = 'BelowNormal'
$env:CARGO_BUILD_JOBS = '1'
$env:RUST_TEST_THREADS = '1'
$env:RAYON_NUM_THREADS = '1'
$env:MSBUILDDISABLENODEREUSE = '1'
$env:DOTNET_CLI_USE_MSBUILD_SERVER = '0'

$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Set-Location -LiteralPath $repo
$manifestPath = Join-Path $repo 'output/native-roundtrip-pass53-source.json'
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ($manifest.qualification_state -ne 'all53_code_and_tests_authored_frozen') {
    throw 'Batch53 source has not been released for combined qualification'
}
$expectedParent = [string]$manifest.parent_head
$comparisonBase = [string]$manifest.comparison_base
if ($expectedParent -notmatch '^[0-9a-fA-F]{40}$' -or $comparisonBase -notmatch '^[0-9a-fA-F]{7,40}$') {
    throw 'Batch53 manifest requires its exact frozen parent HEAD and a separate comparison_base'
}
if ($comparisonBase -cne '0ea730616161a0fe25a8516a089a52cf516cc435' -or $expectedParent -ceq $comparisonBase) {
    throw 'Batch53 requires its new committed HEAD and the separate batch52 source comparison base 0ea730616161a0fe25a8516a089a52cf516cc435'
}
$manifestHash = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash
$runnerHash = (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash
$proof = Join-Path $repo 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass53'
$package = Join-Path $repo 'artifacts/win-x64-native-roundtrip-pass53'
$candidateDirectory = Join-Path $repo 'output/native-oracle-candidate53'
foreach ($freshPath in @($proof, $package, $candidateDirectory)) {
    if (Test-Path -LiteralPath $freshPath) { throw "Do not overwrite or merge an existing batch53 artifact: $freshPath" }
}
$pinnedNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($file in $manifest.files) {
    $relative = ([string]$file.file).Replace('\', '/')
    $absolute = [IO.Path]::GetFullPath((Join-Path $repo $relative))
    if (!$absolute.StartsWith($repo + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -or
        !$pinnedNames.Add($relative) -or [string]$file.sha256 -notmatch '^[0-9a-fA-F]{64}$') {
        throw "Invalid or duplicate batch53 source pin: $relative"
    }
}
if ($pinnedNames.Count -eq 0) { throw 'Batch53 source pins are missing' }
$guardedPaths = @('modern', 'Cargo.toml', 'Cargo.lock', '.cargo', 'Installer/bots',
    '.gitattributes', '.github/workflows/modern-cross-platform.yml')

# Preserve prepared39 input identities without retaining any earlier runtime outcomes.
$candidateInputDirectory = Join-Path $repo 'output/native-oracle-candidate39'
$candidateManifestPath = Join-Path $candidateInputDirectory 'manifest.json'
$candidateManifestHash = (Get-FileHash -LiteralPath $candidateManifestPath -Algorithm SHA256).Hash
$candidateManifest = Get-Content -LiteralPath $candidateManifestPath -Raw | ConvertFrom-Json
$previousInputReceiptPath = Join-Path $repo 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass49/candidate-checkpoints.json'
$previousInputReceiptHash = (Get-FileHash -LiteralPath $previousInputReceiptPath -Algorithm SHA256).Hash
$previousInputReceipt = Get-Content -LiteralPath $previousInputReceiptPath -Raw | ConvertFrom-Json
$expectedCases = @('body', 'contacts', 'feeding', 'fixed', 'movement', 'reproduction', 'ties')
if ($candidateManifest.source_version -ne '2.48.32' -or @($candidateManifest.cases).Count -ne 7 -or
    ((@($candidateManifest.cases.case | Sort-Object) -join '|') -ne ($expectedCases -join '|'))) {
    throw 'Candidate39 manifest requires the exact seven prepared source cases'
}
$candidateInputs = @(foreach ($inputCase in $candidateManifest.cases) {
    if ($inputCase.start_cycle -ne 0 -or (@($inputCase.checkpoint_cycles) -join '|') -ne '1|10|100' -or
        $inputCase.start_sim -cne ($inputCase.case + '-start.sim')) {
        throw 'Candidate39 start filenames/cycles/checkpoints changed'
    }
    $inputPath = Join-Path $candidateInputDirectory $inputCase.start_sim
    $inputHash = (Get-FileHash -LiteralPath $inputPath -Algorithm SHA256).Hash
    $previousHashes = @($previousInputReceipt.cases | Where-Object case -eq $inputCase.case |
        Select-Object -ExpandProperty input_sha256 -Unique)
    if ($previousHashes.Count -ne 1 -or $previousHashes[0] -ne $inputHash) {
        throw "Prepared39 input differs from its previous input identity: $($inputCase.case)"
    }
    [ordered]@{ case = $inputCase.case; file = $inputPath; sha256 = $inputHash; checkpoints = @(1, 10, 100) }
})

[IO.Directory]::CreateDirectory($proof) | Out-Null
$runLock = [IO.File]::Open((Join-Path $proof 'run.lock'), [IO.FileMode]::CreateNew,
    [IO.FileAccess]::ReadWrite, [IO.FileShare]::Read)
[IO.File]::Copy($manifestPath, (Join-Path $proof 'source.json'), $false)
$env:DARWINBOTS_GUI_AUDIT_DIR = Join-Path $proof 'gui'
$results = [Collections.Generic.List[object]]::new()
$qualificationExit = 1
$script:qualifiedCliHash = $null
$script:preservedImageRecorded = $false
$preservedImage = Join-Path $repo 'docs/parity/receipts/db24832-world06-shape-editor.png'

function Write-ProofJson($name, $value) {
    ConvertTo-Json -InputObject $value -Depth 24 |
        Set-Content -LiteralPath (Join-Path $proof $name) -Encoding utf8NoBOM
}
function Save-QualificationState($status, $step, $details) {
    Write-ProofJson 'state.json' ([ordered]@{
        status = $status; step = $step; pid = $PID; utc = [DateTime]::UtcNow.ToString('o')
        source_parent = $expectedParent; comparison_base = $comparisonBase
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
    if (Test-Path -LiteralPath (Join-Path $repo 'output/STOP-native-pass53')) { throw 'Owned batch53 STOP marker present' }
    if ((Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash -ne $manifestHash -or
        (Get-FileHash -LiteralPath (Join-Path $proof 'source.json') -Algorithm SHA256).Hash -ne $manifestHash) {
        throw 'Frozen batch53 source manifest changed'
    }
    if ((Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash -ne $runnerHash) { throw 'Batch53 runner changed' }
    foreach ($file in $manifest.files) {
        if ((Get-FileHash -LiteralPath (Join-Path $repo $file.file) -Algorithm SHA256).Hash -ne $file.sha256) {
            throw "Qualified source changed: $($file.file)"
        }
    }
    if ((Get-FileHash -LiteralPath $candidateManifestPath -Algorithm SHA256).Hash -ne $candidateManifestHash -or
        (Get-FileHash -LiteralPath $previousInputReceiptPath -Algorithm SHA256).Hash -ne $previousInputReceiptHash) {
        throw 'Candidate input provenance metadata changed'
    }
    foreach ($inputCase in $candidateInputs) {
        if ((Get-FileHash -LiteralPath $inputCase.file -Algorithm SHA256).Hash -ne $inputCase.sha256) {
            throw "Prepared39 candidate input changed: $($inputCase.case)"
        }
    }
    $currentHead = git -C $repo rev-parse HEAD 2>> (Join-Path $proof 'source-check.stderr.log')
    if ($LASTEXITCODE -ne 0 -or $currentHead -cne $expectedParent) { throw 'Qualification parent HEAD changed or could not be read' }
    $changed = @(git -C $repo -c core.quotepath=false diff --no-renames --name-only $comparisonBase -- @guardedPaths 2>> (Join-Path $proof 'source-check.stderr.log'))
    if ($LASTEXITCODE -ne 0) { throw 'Cannot audit tracked qualification inputs' }
    foreach ($file in $changed) {
        if (!$pinnedNames.Contains($file.Replace('\', '/'))) { throw "Unpinned source change: $file" }
    }
    $untracked = @(git -C $repo -c core.quotepath=false ls-files --others --exclude-standard -- @guardedPaths 2>> (Join-Path $proof 'source-check.stderr.log'))
    if ($LASTEXITCODE -ne 0) { throw 'Cannot audit untracked qualification inputs' }
    foreach ($file in $untracked) {
        if (!$pinnedNames.Contains($file.Replace('\', '/'))) { throw "Unpinned untracked source: $file" }
    }
    if ($script:qualifiedCliHash -and
        (Get-FileHash -LiteralPath (Join-Path $repo 'target/release/darwinbots-cli.exe') -Algorithm SHA256).Hash -ne $script:qualifiedCliHash) {
        throw 'Qualified CLI identity changed'
    }
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

Write-ProofJson 'runner-identity.json' ([ordered]@{
    file = 'output/qualify-native-roundtrip-pass53.ps1'; sha256 = $runnerHash
    source_manifest_sha256 = $manifestHash; protected_paths = $guardedPaths
    priority = 'BelowNormal'; cargo_build_jobs = 1; rust_test_threads = 1; rayon_threads = 1
    msbuild_node_reuse = $false; gui_audit_directory = $env:DARWINBOTS_GUI_AUDIT_DIR
    rust_runtime_scope = 'One fresh CPU darwinbots-cli library invocation; no fresh full batch53 Rust workspace run or equivalence projection produced by this runner.'
})
Write-ProofJson 'candidate-inputs.json' ([ordered]@{
    status = 'Prepared39 input identities only. No earlier runtime outcomes retained and no genuine original-runtime oracle claimed.'
    manifest = $candidateManifestPath; manifest_sha256 = $candidateManifestHash
    previous_input_identity_receipt = $previousInputReceiptPath; previous_input_identity_receipt_sha256 = $previousInputReceiptHash
    cases = $candidateInputs
})

try {
    Check-QualificationSource
    $null = Run-QualificationStep 'format-check' 'cargo.exe' @('fmt', '--all', '--', '--check')
    $null = Run-QualificationStep 'strict-clippy' 'cargo.exe' @('clippy', '--release', '--workspace', '--all-targets', '--', '-D', 'warnings')
    # This invocation proves only the fresh CLI library CPU result. Any full52
    # source-equivalence projection is sealed separately by root after collection.
    $null = Run-QualificationStep 'cpu-rust-cli' 'cargo.exe' @(
        'test', '--release', '-p', 'darwinbots-cli', '--lib', '--',
        '--skip', 'gpu', '--skip', 'switch', '--nocapture', '--test-threads=1')
    $cliBuilt = Run-QualificationStep 'cli-build' 'cargo.exe' @('build', '--release', '-p', 'darwinbots-cli')
    if ($cliBuilt -eq 0) {
        $script:qualifiedCliHash = (Get-FileHash -LiteralPath (Join-Path $repo 'target/release/darwinbots-cli.exe') -Algorithm SHA256).Hash
        Write-ProofJson 'cli-build-identity.json' ([ordered]@{
            cli_sha256 = $script:qualifiedCliHash; source = 'source.json'; build_exit_code = $cliBuilt
        })
        $zeroOutput = Join-Path $proof 'user-rust-cycle281.sim'
        if (Test-Path -LiteralPath $zeroOutput) { throw 'Do not reuse a zero-cycle replay output' }
        $replay = Run-QualificationStep 'original-zero-cycle-replay' './target/release/darwinbots-cli.exe' @(
            'native-replay', '--simulation', 'modern/engine/tests/fixtures/db24832-original-user.sim',
            '--until-cycle', '281', '--output-sim', $zeroOutput)
        if ($replay -eq 0 -and (Test-Path -LiteralPath $zeroOutput -PathType Leaf)) {
            $comparison = Run-QualificationStep 'original-zero-cycle-compare' './target/release/darwinbots-cli.exe' @(
                'native-compare', '--left', 'modern/engine/tests/fixtures/db24832-original-user.sim', '--right', $zeroOutput,
                '--absolute-tolerance', '0', '--relative-tolerance', '0', '--max-differences', '40')
            $coverage = $false
            $coverageError = $null
            try {
                $report = Get-Content -LiteralPath (Join-Path $proof 'original-zero-cycle-compare.log') -Raw | ConvertFrom-Json
                $coverage = $comparison -eq 0 -and $report.passed -and $report.compared_fields -eq 393316 -and
                    $report.difference_count -eq 0 -and $report.absolute_tolerance -eq 0 -and $report.relative_tolerance -eq 0
            } catch { $coverageError = $_.Exception.Message }
            Add-QualificationResult ([ordered]@{
                step = 'original-zero-cycle-field-coverage'; exit_code = $(if ($coverage) { 0 } else { 1 })
                expected_compared_fields = 393316; log = 'original-zero-cycle-compare.log'; parse_error = $coverageError
            })
        } else {
            Add-DependentResult 'original-zero-cycle-compare' 'Fresh zero-cycle replay did not produce a successful output.'
            Add-DependentResult 'original-zero-cycle-field-coverage' 'Fresh zero-cycle replay did not produce a successful output.'
        }
        if (Test-Path -LiteralPath $candidateDirectory) { throw 'Do not reuse an existing candidate53 directory' }
        [IO.Directory]::CreateDirectory($candidateDirectory) | Out-Null
        $candidateReceipts = @(foreach ($inputCase in $candidateInputs) {
            foreach ($cycle in $inputCase.checkpoints) {
                $output = Join-Path $candidateDirectory ($inputCase.case + '-rust-cycle' + $cycle + '.sim')
                if (Test-Path -LiteralPath $output) { throw 'Do not overwrite a candidate53 checkpoint' }
                $candidateExit = Run-QualificationStep ('candidate-' + $inputCase.case + '-' + $cycle) './target/release/darwinbots-cli.exe' @(
                    'native-replay', '--simulation', $inputCase.file, '--until-cycle', ([string]$cycle), '--output-sim', $output)
                [ordered]@{
                    case = $inputCase.case; cycle = $cycle; exit_code = $candidateExit
                    input = $inputCase.file; input_sha256 = $inputCase.sha256; output = $output
                    output_sha256 = if (Test-Path -LiteralPath $output -PathType Leaf) { (Get-FileHash -LiteralPath $output -Algorithm SHA256).Hash } else { $null }
                }
            }
        })
        Write-ProofJson 'candidate-checkpoints.json' ([ordered]@{
            source = 'native-roundtrip-pass53/source.json'; status = 'Fresh Rust candidates only; no original-runtime comparisons'
            cases = $candidateReceipts
        })
        $complete = $candidateReceipts.Count -eq 21 -and @($candidateReceipts | Where-Object {
            $_.exit_code -ne 0 -or !$_.output_sha256 -or !(Test-Path -LiteralPath $_.output -PathType Leaf)
        }).Count -eq 0
        Add-QualificationResult ([ordered]@{
            step = 'candidate-checkpoints-completeness'; exit_code = $(if ($complete) { 0 } else { 1 })
            cases = 7; expected_outputs = 21; actual_rows = $candidateReceipts.Count; log = 'candidate-checkpoints.json'
        })
    } else {
        Add-DependentResult 'original-zero-cycle-replay' 'Fresh CLI build failed.'
        Add-DependentResult 'original-zero-cycle-compare' 'Fresh CLI build failed.'
        Add-DependentResult 'original-zero-cycle-field-coverage' 'Fresh CLI build failed.'
        Add-DependentResult 'candidate-checkpoints-completeness' 'Fresh CLI build failed; no stale CLI or earlier candidate outputs used.'
    }

    $dotnetOptions = @('--disable-build-servers', '-m:1', '-nr:false', '-p:UseSharedCompilation=false')
    if (Test-Path -LiteralPath (Join-Path $proof 'core.trx')) { throw 'Do not reuse a Core TRX' }
    $null = Run-QualificationStep 'desktop-core' 'dotnet.exe' (@(
        'test', 'modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj', '-c', 'Release',
        '--logger', 'trx;LogFileName=core.trx', '--results-directory', $proof) + $dotnetOptions)
    $desktopBuilt = Run-QualificationStep 'desktop-test-build' 'dotnet.exe' (@(
        'build', 'modern/desktop/tests/Darwinbots.Desktop.Tests/Darwinbots.Desktop.Tests.csproj', '-c', 'Release') + $dotnetOptions)
    if (Test-Path -LiteralPath $package) { throw 'Package directory appeared before publish; do not overwrite it' }
    $published = Run-QualificationStep 'windows-publish' 'dotnet.exe' (@(
        'publish', 'modern/desktop/src/Darwinbots.Desktop/Darwinbots.Desktop.csproj', '-c', 'Release',
        '-r', 'win-x64', '--self-contained', 'false', '-o', $package) + $dotnetOptions)
    if ($published -eq 0) {
        Check-QualificationSource
        if ($desktopBuilt -eq 0) {
            Copy-Item -LiteralPath (Join-Path $package 'darwinbots_engine.dll') -Destination (
                Join-Path $repo 'modern/desktop/tests/Darwinbots.Desktop.Tests/bin/Release/net10.0/darwinbots_engine.dll')
        }
        $dllBefore = Get-NativeDllIdentity ($desktopBuilt -eq 0)
        Write-ProofJson 'native-dll-identity-before.json' $dllBefore
        if ($dllBefore.target_release -ne $dllBefore.package -or
            ($desktopBuilt -eq 0 -and $dllBefore.package -ne $dllBefore.desktop)) {
            throw 'Fresh target/package/Desktop native DLL identity mismatch'
        }
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
        if ($desktopBuilt -eq 0) {
            if (Test-Path -LiteralPath (Join-Path $proof 'desktop.trx')) { throw 'Do not reuse a Desktop TRX' }
            $null = Run-QualificationStep 'packaged-native-desktop' 'dotnet.exe' (@(
                'test', 'modern/desktop/tests/Darwinbots.Desktop.Tests/Darwinbots.Desktop.Tests.csproj',
                '-c', 'Release', '--no-build', '--blame-hang-timeout', '10m', '--blame-hang-dump-type', 'none',
                '--logger', 'trx;LogFileName=desktop.trx', '--results-directory', $proof) + $dotnetOptions)
        } else {
            Add-DependentResult 'packaged-native-desktop' 'Fresh Desktop test build failed.'
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
        Add-DependentResult 'package-smoke' 'Fresh package publish failed.'
        Add-DependentResult 'packaged-native-desktop' 'Fresh package publish failed; no earlier package/native DLL used.'
        Add-DependentResult 'package-assets' 'Fresh package publish failed.'
    }

    Record-PreexistingImage
    Check-QualificationSource
    Write-ProofJson 'results.json' ($results.ToArray())
    $failed = @($results | Where-Object exit_code -ne 0)
    Save-QualificationState $(if ($failed.Count) { 'needs_repair' } else { 'completed' }) 'all' @{
        failed_steps = @($failed | ForEach-Object step)
        scope = 'All batch53 ordinary runner outcomes collected; fresh Rust runtime scope is CLI library only. Root separately seals counts, summary, admission and any retained full52 source-equivalence projection.'
    }
    $qualificationExit = if ($failed.Count) { 1 } else { 0 }
    ConvertTo-Json -InputObject ($results.ToArray()) -Depth 8
} catch {
    Save-QualificationState 'stopped' 'qualification' @{ error = $_.Exception.Message }
    throw
} finally {
    try { Record-PreexistingImage }
    catch { Write-Host ('Could not record final preexisting PNG identity: ' + $_.Exception.Message) }
    $runLock.Dispose()
}

exit $qualificationExit
