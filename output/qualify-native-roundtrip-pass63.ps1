# All batch63 code/tests authored; execute only against the reviewed frozen source63 manifest.
# Batch63 qualification runner; apply after every code and test candidate is authored.
# New native comparison stderr logs preserve machine-readable failure JSON.
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
$manifestPath = Join-Path $repo 'output/native-roundtrip-pass63-source.json'
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ($manifest.qualification_state -ne 'all63_code_and_tests_authored_frozen') {
    throw 'Batch63 source has not been released for combined qualification'
}
$expectedParent = [string]$manifest.parent_head
$comparisonBase = [string]$manifest.comparison_base
if ($expectedParent -notmatch '^[0-9a-fA-F]{40}$' -or $comparisonBase -notmatch '^[0-9a-fA-F]{7,40}$') {
    throw 'Batch63 manifest requires its exact frozen parent HEAD and a separate comparison_base'
}
$manifestHash = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash
$runnerHash = (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash
$proof = Join-Path $repo 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass63'
$package = Join-Path $repo 'artifacts/win-x64-native-roundtrip-pass63'
$candidateDirectory = Join-Path $repo 'output/native-oracle-candidate63'
foreach ($freshPath in @($proof, $package, $candidateDirectory)) {
    if (Test-Path -LiteralPath $freshPath) { throw "Do not overwrite or merge an existing batch63 artifact: $freshPath" }
}
$pinnedNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($file in $manifest.files) {
    $relative = ([string]$file.file).Replace('\', '/')
    $absolute = [IO.Path]::GetFullPath((Join-Path $repo $relative))
    if (!$absolute.StartsWith($repo + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -or
        !$pinnedNames.Add($relative) -or [string]$file.sha256 -notmatch '^[0-9a-fA-F]{64}$') {
        throw "Invalid or duplicate batch63 source pin: $relative"
    }
}
if ($pinnedNames.Count -eq 0) { throw 'Batch63 source pins are missing' }

# Managed expectations are declarations, not substitute execution evidence.
$expectedManagedTestCounts = [ordered]@{}
foreach ($name in @('core', 'desktop')) {
    $count = $manifest.expected_managed_test_counts.$name
    if ($null -eq $count -or $count -is [bool] -or $count -isnot [ValueType] -or
        ![double]::IsFinite([double]$count) -or $count -lt 1 -or $count -gt [int]::MaxValue -or
        [Math]::Truncate([double]$count) -ne $count) {
        throw ('Batch63 requires a positive manifest expected_managed_test_counts.' + $name)
    }
    $expectedManagedTestCounts[$name] = [int]$count
}

# The target census comes from this batch's pinned cargo metadata, not the previous suite count.
$workspaceMetadataRelative = ([string]$manifest.workspace_metadata).Replace('\', '/')
$workspaceMetadataPath = [IO.Path]::GetFullPath((Join-Path $repo $workspaceMetadataRelative))
$workspaceMetadataPin = @($manifest.files | Where-Object { ([string]$_.file).Replace('\', '/') -ceq $workspaceMetadataRelative })
if (!$workspaceMetadataPath.StartsWith($repo + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -or
    !$pinnedNames.Contains($workspaceMetadataRelative) -or $workspaceMetadataPin.Count -ne 1 -or
    [string]$manifest.expected_distinct_rust_targets -notmatch '^[1-9][0-9]*$') {
    throw 'Batch63 requires a pinned workspace metadata file and an explicit expected Rust target count.'
}
$workspaceMetadataHash = (Get-FileHash -LiteralPath $workspaceMetadataPath -Algorithm SHA256).Hash
if ($workspaceMetadataHash -ne $workspaceMetadataPin[0].sha256) { throw 'Frozen workspace metadata hash mismatch.' }
$workspaceMetadata = Get-Content -LiteralPath $workspaceMetadataPath -Raw | ConvertFrom-Json
$workspaceMembers = @($workspaceMetadata.workspace_members)
$workspacePackages = @($workspaceMetadata.packages | Where-Object { $_.id -cin $workspaceMembers })
if ($workspaceMembers.Count -eq 0 -or $workspacePackages.Count -ne $workspaceMembers.Count) {
    throw 'Workspace metadata must identify every workspace member exactly once.'
}
$workspaceTargetRows = @(foreach ($workspacePackage in $workspacePackages) {
    [ordered]@{
        package = $workspacePackage.name
        test_targets = @($workspacePackage.targets | Where-Object test -eq $true).Count
        doc_test_targets = @($workspacePackage.targets | Where-Object doctest -eq $true).Count
    }
})
$metadataRustTargetCount = 0
foreach ($row in $workspaceTargetRows) { $metadataRustTargetCount += $row.test_targets + $row.doc_test_targets }
$expectedRustTargets = [int]$manifest.expected_distinct_rust_targets
if ($metadataRustTargetCount -le 0 -or $metadataRustTargetCount -ne $expectedRustTargets) {
    throw 'Expected Rust target count must equal the pinned workspace metadata test/doctest census.'
}
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

$captureProvenanceRelative = 'modern/engine/tests/fixtures/db24832-genuine-capture-provenance.json'
$captureProvenancePin = @($manifest.files | Where-Object { ([string]$_.file).Replace('\', '/') -ceq $captureProvenanceRelative })
if (!$pinnedNames.Contains($captureProvenanceRelative) -or $captureProvenancePin.Count -ne 1) {
    throw 'Original input/capture provenance must be a frozen source input.'
}
$captureProvenancePath = Join-Path $repo $captureProvenanceRelative
$captureProvenanceHash = (Get-FileHash -LiteralPath $captureProvenancePath -Algorithm SHA256).Hash
if ($captureProvenanceHash -ne $captureProvenancePin[0].sha256) { throw 'Frozen original provenance changed.' }
$captureProvenance = Get-Content -LiteralPath $captureProvenancePath -Raw | ConvertFrom-Json
if ($captureProvenance.schema_version -ne 1 -or
    $captureProvenance.authority.executable_sha256 -ne '7F580D9CE8F1095A01FD162E9BCAC03FE437599E080786FD670B676671C31990') {
    throw 'Original provenance must identify the captured Darwinbots2.48.32 executable.'
}

$genuineMovementReferences = @(
    @{ cycle = 1; relative = 'modern/engine/tests/fixtures/db24832-genuine-movement-cycle1-20261016.sim'; sha256 = '766F386C392C59834A3390A2C726D3E2F8DB695FE5B05A932A593D0DCCD010E3' },
    @{ cycle = 10; relative = 'modern/engine/tests/fixtures/db24832-genuine-movement-cycle10-20261016.sim'; sha256 = 'DA23FCBE0B725FCF134F7FA25022951E600088603126BFB6E327027FF23465A2' },
    @{ cycle = 100; relative = 'modern/engine/tests/fixtures/db24832-genuine-movement-cycle100-20261017.sim'; sha256 = 'D1F078B57BC1DDF4690DBEF24CC5945FC925755293060A8561FB7E82A1100202' },
    @{ cycle = 102; relative = 'modern/engine/tests/fixtures/db24832-genuine-movement-cycle102-20261016.sim'; sha256 = '2BB48F9883CA05E5771989CB08056BAC737E1EE50F148A8B234FF894EAF37B5B' }
)
foreach ($reference in $genuineMovementReferences) {
    if (!$pinnedNames.Contains($reference.relative) -or
        (Get-FileHash -LiteralPath (Join-Path $repo $reference.relative) -Algorithm SHA256).Hash -ne $reference.sha256) {
        throw 'Genuine movement fixtures must be pinned source inputs with their captured original hashes.'
    }
}

$genuineControlInputs = @(
    @{ case = 'fixed'; relative = 'modern/engine/tests/fixtures/db24832-fixed-start-48032.sim'; sha256 = '94C32DA0BDB409DF42C7DF7CB43B59A3DEA70E08D0D53631B8FEE0FDEAD41BE6'; prepared39 = $true }
    @{ case = 'body'; relative = 'modern/engine/tests/fixtures/db24832-body-start-48032.sim'; sha256 = '0445D63EF499C785DB6A3678E03DD2F0C4B57D0CB2DDD3EC3BB9CA9DCB8565B5'; prepared39 = $true }
    @{ case = 'chloroplast'; relative = 'modern/engine/tests/fixtures/db24832-chloroplast-start-48032.sim'; sha256 = 'CB85F10215164C18EB8D8E73A98C2B365B91FE5F7F2BB52E4F2789B7D1E38859'; prepared39 = $false }
    @{ case = 'feeding'; relative = 'modern/engine/tests/fixtures/db24832-feeding-start-48032.sim'; sha256 = 'A0AE7DFE9C75F8BBC6B7482106666C529F1DA96D91F7EE414D9F2E914B87C03D'; prepared39 = $true }
    @{ case = 'ties'; relative = 'modern/engine/tests/fixtures/db24832-ties-start-48032.sim'; sha256 = '302DE657A38E7B88B1ABBB652ED567BCC3530345E16554AC2C63EAC5290D9FA3'; prepared39 = $true }
    @{ case = 'contacts'; relative = 'modern/engine/tests/fixtures/db24832-contacts-start-48032.sim'; sha256 = '4AF50D8B78BD75723DDF36E47C3A80B9DE64636FBC1A9501E16C7FCBFF9F0C72'; prepared39 = $true }
    @{ case = 'reproduction'; relative = 'modern/engine/tests/fixtures/db24832-reproduction-start-48032.sim'; sha256 = '60FD33C86E938B94D86C074AE9B8097B9D61159CCF5D35D066BAFFCB4BF18040'; prepared39 = $true }
)
$genuineControlReferences = @(
    @{ case = 'fixed'; cycle = 1; relative = 'modern/engine/tests/fixtures/db24832-genuine-fixed-cycle1-20261016.sim'; sha256 = '01360A500B63AAD15D96384A25E792AB7C50CAB630497017F7C1214D00535BF4'; expected_fields = 1385 }
    @{ case = 'body'; cycle = 1; relative = 'modern/engine/tests/fixtures/db24832-genuine-body-cycle1-20261016.sim'; sha256 = '0F6EBE616D67BA8FAA45372BE1F0DB0FACEC2994DCC1632953F31634E9A847E5'; expected_fields = 1379 }
    @{ case = 'chloroplast'; cycle = 1; relative = 'modern/engine/tests/fixtures/db24832-genuine-chloroplast-cycle1-20261016.sim'; sha256 = '50DD4B01A0E0E0A19D3E79896B85127A41897BCA8762F8EBF020C63A78DA3170'; expected_fields = 1379 }
    @{ case = 'chloroplast'; cycle = 10; relative = 'modern/engine/tests/fixtures/db24832-genuine-chloroplast-cycle10-20261016.sim'; sha256 = '569F1272D28A876B3F140420193E5A767B6E9B2B0853B5E36D39513655A4BCCE'; expected_fields = 1379 }
    @{ case = 'feeding'; cycle = 1; relative = 'modern/engine/tests/fixtures/db24832-genuine-feeding-cycle1-20261017.sim'; sha256 = '0D0655818BECF73E1722E0A56AEBE05C4D083C0CC60C2C7F6F6A0DFEAD844337'; expected_fields = 2814 }
    @{ case = 'feeding'; cycle = 10; relative = 'modern/engine/tests/fixtures/db24832-genuine-feeding-cycle10-20261017.sim'; sha256 = 'A82ABBC5FA3E851A5F4E7506B8497FABA985ABDE24509DC6905B2F2D1F3C79CF'; expected_fields = 3090 }
    @{ case = 'ties'; cycle = 1; relative = 'modern/engine/tests/fixtures/db24832-genuine-ties-cycle1-20261017.sim'; sha256 = 'B1E4223FEF34CDA7FC1C7E1661B086751D83515AB387C0DF596AA2728CA6501D'; expected_fields = 2768 }
    @{ case = 'contacts'; cycle = 1; relative = 'modern/engine/tests/fixtures/db24832-genuine-contacts-cycle1-20261017.sim'; sha256 = 'F7DA4A17D59977FC65BB32CC6BCE2AE4C8E28087672F997CE9629CFB20D202D0'; expected_fields = 4115 }
    @{ case = 'contacts'; cycle = 11; relative = 'modern/engine/tests/fixtures/db24832-genuine-contacts-cycle11-20261017.sim'; sha256 = '8821C6D5EB2077EFAE7597FB1D4EB8ADEE3DE687275528BAF7A79C0DC5AF05BB'; expected_fields = 4115 }
    @{ case = 'contacts'; cycle = 100; relative = 'modern/engine/tests/fixtures/db24832-genuine-contacts-cycle100-20261017.sim'; sha256 = 'C6AB7E5578162E576F37F3F1CCF738985015D9FD9C3AF155D5E561F02F9F378E'; expected_fields = 4115 }
    @{ case = 'reproduction'; cycle = 1; relative = 'modern/engine/tests/fixtures/db24832-genuine-reproduction-cycle1-20261017.sim'; sha256 = 'A6F9DC7F78280AF622D35401ECF65CB1E6ADE344D2D150342E89B5556DA0E03A'; expected_fields = 1387 }
    @{ case = 'reproduction'; cycle = 10; relative = 'modern/engine/tests/fixtures/db24832-genuine-reproduction-cycle10-20261017.sim'; sha256 = '4BCD08A85E121216B1BF49C4315FACEA7C985CCF691C147306A479BACE437B69'; expected_fields = 2772 }
    @{ case = 'fixed'; cycle = 10; relative = 'modern/engine/tests/fixtures/db24832-genuine-fixed-cycle10-20261017.sim'; sha256 = '051F845860338E6D1B21572A73EE60867D1AD6EC2981016A1E3E6BEC11450A44'; expected_fields = 1385 }
    @{ case = 'fixed'; cycle = 100; relative = 'modern/engine/tests/fixtures/db24832-genuine-fixed-cycle100-20261017.sim'; sha256 = '792BE4607BA33883184F260092646E673FFBED263AFDD1915C045B34E71C1ACA'; expected_fields = 1385 }
    @{ case = 'body'; cycle = 10; relative = 'modern/engine/tests/fixtures/db24832-genuine-body-cycle10-20261017.sim'; sha256 = 'B3373F593FBFB6C60D7A0F9BDB2D3F0936C274567655E8F0029F809189246E95'; expected_fields = 1379 }
    @{ case = 'body'; cycle = 100; relative = 'modern/engine/tests/fixtures/db24832-genuine-body-cycle100-20261017.sim'; sha256 = '9657E8D3917AAFD3DE87E3B409D90EBEFA1F811356EF0DA9225BAEFD18CB89C9'; expected_fields = 1379 }
    @{ case = 'contacts'; cycle = 10; relative = 'modern/engine/tests/fixtures/db24832-genuine-contacts-cycle10-20261017.sim'; sha256 = 'DE73B60672D17B3C5C8FADCF1DD37F2C917B9D63C3F161BA26978FC989249453'; expected_fields = 4115 }
    @{ case = 'ties'; cycle = 10; relative = 'modern/engine/tests/fixtures/db24832-genuine-ties-cycle10-20261017.sim'; sha256 = '1065E95C9F8B3BDE99773ABB90AD8C37D5DE3D8B56351C32EAE531A7F5959AD0'; expected_fields = 2768 }
    @{ case = 'ties'; cycle = 100; relative = 'modern/engine/tests/fixtures/db24832-genuine-ties-cycle100-20261017.sim'; sha256 = '6AC8CA1E1ACC3FBE443DC6D9006BAA3A615D402362D1495CC57E37EEC3EE5D00'; expected_fields = 2768 }
    @{ case = 'feeding'; cycle = 100; relative = 'modern/engine/tests/fixtures/db24832-genuine-feeding-cycle100-20261017.sim'; sha256 = '32537482FDB46360B18219BECDB33D99D02B2ACEFDC41E68A9E929A3C5B2CB4F'; expected_fields = 1684 }
    @{ case = 'reproduction'; cycle = 100; relative = 'modern/engine/tests/fixtures/db24832-genuine-reproduction-cycle100-20261017.sim'; sha256 = 'DEFBB6F511E8753F44B4FD5BC6D1629FFFCA3CEBEA1E6D7BA81D175AEC3BA234'; expected_fields = 2772 }
)
foreach ($control in @($genuineControlInputs) + @($genuineControlReferences)) {
    if (!$pinnedNames.Contains($control.relative) -or
        (Get-FileHash -LiteralPath (Join-Path $repo $control.relative) -Algorithm SHA256).Hash -ne $control.sha256) {
        throw 'Every additional genuine control input/reference must be frozen with its exact captured hash.'
    }
}
foreach ($control in $genuineControlInputs | Where-Object prepared39 -eq $true) {
    $prepared = @($candidateInputs | Where-Object case -ceq $control.case)
    if ($prepared.Count -ne 1 -or $prepared[0].sha256 -ne $control.sha256) {
        throw "Genuine control does not identify its exact prepared39 input: $($control.case)"
    }
}

# Bind every captured identity and actual saved counter to the pinned original metadata.
$allGenuineReferences = @(
    foreach ($reference in $genuineMovementReferences) {
        @{ case = 'movement'; cycle = $reference.cycle; relative = $reference.relative; sha256 = $reference.sha256; expected_fields = 1379 }
    }
) + @($genuineControlReferences)
if (@($captureProvenance.captures).Count -ne $allGenuineReferences.Count) {
    throw 'Frozen original capture metadata must account for exactly this batch comparison set.'
}
foreach ($reference in $allGenuineReferences) {
    $metadataCapture = @($captureProvenance.captures | Where-Object {
        $_.case -ceq $reference.case -and $_.cycle -eq $reference.cycle -and $_.file -ceq $reference.relative
    })
    if ($metadataCapture.Count -ne 1 -or $metadataCapture[0].sha256 -ne $reference.sha256 -or
        $metadataCapture[0].seed -ne 48032 -or
        ($null -ne $metadataCapture[0].expected_compared_fields -and
            $metadataCapture[0].expected_compared_fields -ne $reference.expected_fields) -or
        (Get-Item -LiteralPath (Join-Path $repo $reference.relative)).Length -ne $metadataCapture[0].bytes) {
        throw 'A genuine reference differs from its pinned captured hash, actual counter, seed or byte length.'
    }
}
foreach ($metadataInput in @($captureProvenance.inputs)) {
    $relative = ([string]$metadataInput.file).Replace('\', '/')
    if (!$pinnedNames.Contains($relative) -or $metadataInput.cycle -ne 0 -or $metadataInput.seed -ne 48032 -or
        (Get-FileHash -LiteralPath (Join-Path $repo $relative) -Algorithm SHA256).Hash -ne $metadataInput.sha256 -or
        (Get-Item -LiteralPath (Join-Path $repo $relative)).Length -ne $metadataInput.bytes) {
        throw 'Frozen original starting-input metadata changed.'
    }
}
foreach ($control in $genuineControlInputs) {
    $metadataInput = @($captureProvenance.inputs | Where-Object { $_.case -ceq $control.case -and $_.file -ceq $control.relative })
    if ($metadataInput.Count -ne 1 -or $metadataInput[0].sha256 -ne $control.sha256) {
        throw 'Every genuine control must identify its exact pinned source input in original metadata.'
    }
}
$movementMetadata = @($captureProvenance.inputs | Where-Object case -ceq 'movement')
$movementPrepared = @($candidateInputs | Where-Object case -ceq 'movement')
if ($movementMetadata.Count -ne 1 -or $movementPrepared.Count -ne 1 -or
    $movementMetadata[0].sha256 -ne $movementPrepared[0].sha256) {
    throw 'Movement metadata must retain the exact prepared source input.'
}

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
        source_manifest_sha256 = $manifestHash; expected_managed_test_counts = $expectedManagedTestCounts; details = $details
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
    if (Test-Path -LiteralPath (Join-Path $repo 'output/STOP-native-pass63')) { throw 'Owned batch63 STOP marker present' }
    if ((Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash -ne $manifestHash -or
        (Get-FileHash -LiteralPath (Join-Path $proof 'source.json') -Algorithm SHA256).Hash -ne $manifestHash) {
        throw 'Frozen batch63 source manifest changed'
    }
    if ((Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash -ne $runnerHash) { throw 'Batch63 runner changed' }
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
    $jsonComparison = $arguments.Count -gt 0 -and $arguments[0] -ceq 'native-compare'
    $stderrLog = Join-Path $proof ($label + '.stderr.log')
    if ($jsonComparison -and (Test-Path -LiteralPath $stderrLog)) {
        throw "Refusing to overwrite a native comparison stderr log: $label"
    }
    Save-QualificationState 'running' $label @{ log = $log }
    $started = [DateTime]::UtcNow
    $invocationError = $null
    try {
        if ($jsonComparison) {
            # The CLI writes a valid JSON report before its nonzero-exit stderr footer.
            & $executable @arguments 1> $log 2> $stderrLog
        } else {
            & $executable @arguments *> $log
        }
        $exitCode = if ($null -eq $LASTEXITCODE) { 1 } else { [int]$LASTEXITCODE }
    } catch {
        $exitCode = 1
        $invocationError = $_.Exception.Message
        $errorLog = if ($jsonComparison) { $stderrLog } else { $log }
        $_ | Out-String | Add-Content -LiteralPath $errorLog
    }
    Add-QualificationResult ([ordered]@{
        step = $label; exit_code = $exitCode; seconds = ([DateTime]::UtcNow - $started).TotalSeconds
        executable = $executable; arguments = $arguments; log = $label + '.log'; invocation_error = $invocationError
        stderr_log = $(if ($jsonComparison) { $label + '.stderr.log' } else { $null })
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
    file = 'output/qualify-native-roundtrip-pass63.ps1'; sha256 = $runnerHash
    source_manifest_sha256 = $manifestHash; protected_paths = $guardedPaths
    priority = 'BelowNormal'; cargo_build_jobs = 1; rust_test_threads = 1; rayon_threads = 1
    msbuild_node_reuse = $false; gui_audit_directory = $env:DARWINBOTS_GUI_AUDIT_DIR
})
Write-ProofJson 'workspace-target-expectations.json' ([ordered]@{
    source_head = $expectedParent; source_manifest_sha256 = $manifestHash
    metadata = $workspaceMetadataRelative; metadata_sha256 = $workspaceMetadataHash
    expected_distinct_rust_targets = $expectedRustTargets; metadata_distinct_rust_targets = $metadataRustTargetCount
    expected_managed_test_counts = $expectedManagedTestCounts
    packages = $workspaceTargetRows
    scope = 'Expected targets derived from pinned metadata. Actual emitted results are sealed independently after the fresh no-fail-fast run.'
})
Write-ProofJson 'genuine-reference-provenance.json' ([ordered]@{
    source_head = $expectedParent; source_manifest_sha256 = $manifestHash
    metadata = $captureProvenanceRelative; metadata_sha256 = $captureProvenanceHash
    inputs = $captureProvenance.inputs; captures = $captureProvenance.captures
    scope = 'Original input and saved-counter identities only. Runtime comparisons execute afresh below.'
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
    $rustWorkspaceExit = Run-QualificationStep 'cpu-rust-workspace' 'cargo.exe' @(
        'test', '--release', '--workspace', '--no-fail-fast', '--',
        '--skip', 'gpu', '--skip', 'switch', '--nocapture', '--test-threads=1')
    $cliBuilt = Run-QualificationStep 'cli-build' 'cargo.exe' @('build', '--release', '-p', 'darwinbots-cli')
    if ($cliBuilt -eq 0) {
        $script:qualifiedCliHash = (Get-FileHash -LiteralPath (Join-Path $repo 'target/release/darwinbots-cli.exe') -Algorithm SHA256).Hash
        Write-ProofJson 'cli-build-identity.json' ([ordered]@{
            cli_sha256 = $script:qualifiedCliHash; source = 'source.json'; build_exit_code = $cliBuilt
        })
        # This is validated raw file preservation at zero steps, not runtime parity.
        # Actual native-loader normalization is tested independently in the fresh CLI/Rust suite.
        $zeroInputRelative = 'modern/engine/tests/fixtures/db24832-original-user.sim'
        if (!$pinnedNames.Contains($zeroInputRelative)) { throw 'Zero-step raw input must be frozen.' }
        $zeroInput = Join-Path $repo $zeroInputRelative
        $zeroInputHash = (Get-FileHash -LiteralPath $zeroInput -Algorithm SHA256).Hash
        $zeroOutput = Join-Path $proof 'user-rust-cycle281.sim'
        if (Test-Path -LiteralPath $zeroOutput) { throw 'Do not reuse a zero-cycle replay output' }
        $replay = Run-QualificationStep 'original-zero-cycle-replay' './target/release/darwinbots-cli.exe' @(
            'native-replay', '--simulation', 'modern/engine/tests/fixtures/db24832-original-user.sim',
            '--until-cycle', '281', '--output-sim', $zeroOutput)
        if ($replay -eq 0 -and (Test-Path -LiteralPath $zeroOutput -PathType Leaf)) {
            $zeroOutputHash = (Get-FileHash -LiteralPath $zeroOutput -Algorithm SHA256).Hash
            $rawPreserved = $zeroInputHash -eq $zeroOutputHash -and
                (Get-Item -LiteralPath $zeroInput).Length -eq (Get-Item -LiteralPath $zeroOutput).Length
            Write-ProofJson 'original-zero-cycle-raw-preservation.json' ([ordered]@{
                schema_version = 1; batch = 63; source_head = $expectedParent; source_manifest_sha256 = $manifestHash
                input = $zeroInputRelative; input_sha256 = $zeroInputHash; output = $zeroOutput
                output_sha256 = $zeroOutputHash; exact_raw_bytes = $rawPreserved
                boundary = 'Validated native input preserved without simulation steps. No runtime loader parity or original trajectory agreement is established by this raw copy.'
                loader_normalization = 'Covered independently by repaired CLI tests in the fresh full Rust invocation.'
                fresh_rust_workspace_exit_code = $rustWorkspaceExit
            })
            Add-QualificationResult ([ordered]@{
                step = 'original-zero-cycle-raw-file-preservation'; exit_code = $(if ($rawPreserved) { 0 } else { 1 })
                log = 'original-zero-cycle-raw-preservation.json'
                scope = 'Validated raw file preservation only; runtime normalization is a separate CLI-test assertion.'
            })
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
                scope = 'Raw zero-step file preservation field coverage; not runtime parity.'
            })
        } else {
            Add-DependentResult 'original-zero-cycle-raw-file-preservation' 'Fresh validated zero-step preservation did not produce an output.'
            Add-DependentResult 'original-zero-cycle-compare' 'Fresh zero-cycle replay did not produce a successful output.'
            Add-DependentResult 'original-zero-cycle-field-coverage' 'Fresh zero-cycle replay did not produce a successful output.'
        }
        if (Test-Path -LiteralPath $candidateDirectory) { throw 'Do not reuse an existing candidate63 directory' }
        [IO.Directory]::CreateDirectory($candidateDirectory) | Out-Null
        $candidateReceipts = @(foreach ($inputCase in $candidateInputs) {
            foreach ($cycle in $inputCase.checkpoints) {
                $output = Join-Path $candidateDirectory ($inputCase.case + '-rust-cycle' + $cycle + '.sim')
                if (Test-Path -LiteralPath $output) { throw 'Do not overwrite a candidate63 checkpoint' }
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
            source = 'native-roundtrip-pass63/source.json'; status = 'Fresh seven-case/21-output Rust candidates; genuine movement and additional control comparisons are recorded separately.'
            cases = $candidateReceipts
        })
        $complete = $candidateReceipts.Count -eq 21 -and @($candidateReceipts | Where-Object {
            $_.exit_code -ne 0 -or !$_.output_sha256 -or !(Test-Path -LiteralPath $_.output -PathType Leaf)
        }).Count -eq 0
        Add-QualificationResult ([ordered]@{
            step = 'candidate-checkpoints-completeness'; exit_code = $(if ($complete) { 0 } else { 1 })
            cases = 7; expected_outputs = 21; actual_rows = $candidateReceipts.Count; log = 'candidate-checkpoints.json'
        })
        # Keep the seven-case/21-output plan intact. C102 is an additional
        # movement checkpoint because the original actually saved C102, not C100.
        $movementInput = @($candidateInputs | Where-Object case -ceq 'movement')
        if ($movementInput.Count -ne 1) { throw 'Exactly one prepared movement input is required.' }
        $movement102 = Join-Path $candidateDirectory 'movement-rust-cycle102.sim'
        if (Test-Path -LiteralPath $movement102) { throw 'Do not reuse a C102 movement output.' }
        $movement102Exit = Run-QualificationStep 'candidate-movement-102' './target/release/darwinbots-cli.exe' @(
            'native-replay', '--simulation', $movementInput[0].file, '--until-cycle', '102', '--output-sim', $movement102)
        $genuineRows = @(foreach ($reference in $genuineMovementReferences) {
            $cycle = [int]$reference.cycle
            $rustOutput = Join-Path $candidateDirectory ('movement-rust-cycle' + $cycle + '.sim')
            $candidateExit = if ($cycle -eq 102) { $movement102Exit } else {
                $row = @($candidateReceipts | Where-Object { $_.case -ceq 'movement' -and $_.cycle -eq $cycle })
                if ($row.Count -eq 1) { $row[0].exit_code } else { 125 }
            }
            $comparisonLabel = 'genuine-movement-' + $cycle + '-compare'
            $coverageLabel = 'genuine-movement-' + $cycle + '-field-coverage'
            $originalPath = Join-Path $repo $reference.relative
            $comparison = 125
            $coverage = $false
            $report = $null
            $parseError = $null
            if ($candidateExit -eq 0 -and (Test-Path -LiteralPath $rustOutput -PathType Leaf)) {
                $comparison = Run-QualificationStep $comparisonLabel './target/release/darwinbots-cli.exe' @(
                    'native-compare', '--left', $originalPath, '--right', $rustOutput,
                    '--absolute-tolerance', '0', '--relative-tolerance', '0', '--max-differences', '100')
                try {
                    $report = Get-Content -LiteralPath (Join-Path $proof ($comparisonLabel + '.log')) -Raw | ConvertFrom-Json
                    $coverage = $comparison -eq 0 -and $report.passed -is [bool] -and $report.passed -eq $true -and
                        $report.compared_fields -eq 1379 -and $report.difference_count -eq 0 -and
                        $report.absolute_tolerance -eq 0 -and $report.relative_tolerance -eq 0
                } catch { $parseError = $_.Exception.Message }
                Add-QualificationResult ([ordered]@{
                    step = $coverageLabel; exit_code = $(if ($coverage) { 0 } else { 1 })
                    expected_compared_fields = 1379; log = $comparisonLabel + '.log'; parse_error = $parseError
                })
            } else {
                Add-DependentResult $comparisonLabel 'Its fresh Rust movement checkpoint failed or is missing.'
                Add-DependentResult $coverageLabel 'Its fresh Rust movement checkpoint failed or is missing.'
            }
            [ordered]@{
                cycle = $cycle; candidate_exit_code = $candidateExit; compare_exit_code = $comparison; passed = $coverage
                original = $reference.relative; original_sha256 = $reference.sha256; rust_output = $rustOutput
                rust_output_sha256 = if (Test-Path -LiteralPath $rustOutput -PathType Leaf) { (Get-FileHash -LiteralPath $rustOutput -Algorithm SHA256).Hash } else { $null }
                compared_fields = if ($null -ne $report) { $report.compared_fields } else { $null }
                difference_count = if ($null -ne $report) { $report.difference_count } else { $null }
                absolute_tolerance = 0; relative_tolerance = 0; log = $comparisonLabel + '.log'; parse_error = $parseError
            }
        })
        $genuinePassed = $genuineRows.Count -eq 4 -and @($genuineRows | Where-Object { !$_.passed }).Count -eq 0
        Write-ProofJson 'genuine-movement-comparisons.json' ([ordered]@{
            schema_version = 1; batch = 63; source_head = $expectedParent; source_manifest_sha256 = $manifestHash
            source_version = '2.48.32'; expected_cycles = @(1, 10, 100, 102); passed = $genuinePassed; checkpoints = $genuineRows
            boundary = 'Actual original fresh-load C1/C10/C100 and separate C102 saves. All counters are read from the original bytes. Other seven-case checkpoints are outside this movement receipt; additional genuine comparisons are recorded separately. SIM omits arbitrary historical Rnd/Gauss state.'
        })
        Add-QualificationResult ([ordered]@{
            step = 'genuine-movement-checkpoints-completeness'; exit_code = $(if ($genuinePassed) { 0 } else { 1 })
            expected_outputs = 4; actual_rows = $genuineRows.Count; log = 'genuine-movement-comparisons.json'
        })

        # Plant controls use a separately pinned source input. Keep the original
        # seven-case/21-output candidate receipt intact and record these two fresh
        # outputs independently before comparing all additional original captures.
        $plantInput = @($genuineControlInputs | Where-Object case -ceq 'chloroplast')
        if ($plantInput.Count -ne 1) { throw 'Exactly one pinned plant input is required.' }
        $additionalCandidateReceipts = @(foreach ($cycle in @(1, 10)) {
            $output = Join-Path $candidateDirectory ('chloroplast-rust-cycle' + $cycle + '.sim')
            if (Test-Path -LiteralPath $output) { throw 'Do not reuse a plant candidate63 checkpoint.' }
            $candidateExit = Run-QualificationStep ('candidate-chloroplast-' + $cycle) './target/release/darwinbots-cli.exe' @(
                'native-replay', '--simulation', (Join-Path $repo $plantInput[0].relative),
                '--until-cycle', ([string]$cycle), '--output-sim', $output)
            [ordered]@{
                case = 'chloroplast'; cycle = $cycle; exit_code = $candidateExit
                input = $plantInput[0].relative; input_sha256 = $plantInput[0].sha256; output = $output
                output_sha256 = if (Test-Path -LiteralPath $output -PathType Leaf) { (Get-FileHash -LiteralPath $output -Algorithm SHA256).Hash } else { $null }
            }
        })
        # C11 is an additional actual contacts checkpoint; C100 already has a
        # fresh output in the unchanged seven-case/21-output candidate plan.
        $contactsInput = @($genuineControlInputs | Where-Object case -ceq 'contacts')
        if ($contactsInput.Count -ne 1) { throw 'Exactly one pinned contacts input is required.' }
        $contacts11 = Join-Path $candidateDirectory 'contacts-rust-cycle11.sim'
        if (Test-Path -LiteralPath $contacts11) { throw 'Do not reuse a C11 contacts output.' }
        $contacts11Exit = Run-QualificationStep 'candidate-contacts-11' './target/release/darwinbots-cli.exe' @(
            'native-replay', '--simulation', (Join-Path $repo $contactsInput[0].relative),
            '--until-cycle', '11', '--output-sim', $contacts11)
        $additionalCandidateReceipts += [ordered]@{
            case = 'contacts'; cycle = 11; exit_code = $contacts11Exit
            input = $contactsInput[0].relative; input_sha256 = $contactsInput[0].sha256; output = $contacts11
            output_sha256 = if (Test-Path -LiteralPath $contacts11 -PathType Leaf) { (Get-FileHash -LiteralPath $contacts11 -Algorithm SHA256).Hash } else { $null }
        }
        Write-ProofJson 'genuine-additional-candidate-checkpoints.json' ([ordered]@{
            source_head = $expectedParent; source_manifest_sha256 = $manifestHash
            status = 'Two fresh plant outputs and one actual-C11 contacts output from pinned inputs; runtime outcomes are not inherited.'
            cases = $additionalCandidateReceipts
        })
        $additionalCandidatesPassed = $additionalCandidateReceipts.Count -eq 3 -and @($additionalCandidateReceipts | Where-Object {
            $_.exit_code -ne 0 -or !$_.output_sha256 -or !(Test-Path -LiteralPath $_.output -PathType Leaf)
        }).Count -eq 0
        Add-QualificationResult ([ordered]@{
            step = 'genuine-additional-candidate-checkpoints-completeness'; exit_code = $(if ($additionalCandidatesPassed) { 0 } else { 1 })
            expected_outputs = 3; actual_rows = $additionalCandidateReceipts.Count; log = 'genuine-additional-candidate-checkpoints.json'
        })
        $allControlCandidates = @($candidateReceipts) + @($additionalCandidateReceipts)
        $additionalGenuineRows = @(foreach ($reference in $genuineControlReferences) {
            $row = @($allControlCandidates | Where-Object { $_.case -ceq $reference.case -and $_.cycle -eq $reference.cycle })
            $rustOutput = Join-Path $candidateDirectory ($reference.case + '-rust-cycle' + $reference.cycle + '.sim')
            $candidateExit = if ($row.Count -eq 1) { $row[0].exit_code } else { 125 }
            $comparisonLabel = 'genuine-' + $reference.case + '-' + $reference.cycle + '-compare'
            $coverageLabel = 'genuine-' + $reference.case + '-' + $reference.cycle + '-field-coverage'
            $comparison = 125
            $coverage = $false
            $report = $null
            $parseError = $null
            if ($candidateExit -eq 0 -and (Test-Path -LiteralPath $rustOutput -PathType Leaf)) {
                $comparison = Run-QualificationStep $comparisonLabel './target/release/darwinbots-cli.exe' @(
                    'native-compare', '--left', (Join-Path $repo $reference.relative), '--right', $rustOutput,
                    '--absolute-tolerance', '0', '--relative-tolerance', '0', '--max-differences', '100')
                try {
                    $report = Get-Content -LiteralPath (Join-Path $proof ($comparisonLabel + '.log')) -Raw | ConvertFrom-Json
                    $coverage = $comparison -eq 0 -and $report.passed -is [bool] -and $report.passed -eq $true -and
                        $report.compared_fields -eq $reference.expected_fields -and $report.difference_count -eq 0 -and
                        $report.absolute_tolerance -eq 0 -and $report.relative_tolerance -eq 0
                } catch { $parseError = $_.Exception.Message }
                Add-QualificationResult ([ordered]@{
                    step = $coverageLabel; exit_code = $(if ($coverage) { 0 } else { 1 })
                    expected_compared_fields = $reference.expected_fields; log = $comparisonLabel + '.log'; parse_error = $parseError
                })
            } else {
                Add-DependentResult $comparisonLabel 'Its fresh pinned Rust checkpoint failed or is missing.'
                Add-DependentResult $coverageLabel 'Its fresh pinned Rust checkpoint failed or is missing.'
            }
            [ordered]@{
                case = $reference.case; cycle = $reference.cycle; candidate_exit_code = $candidateExit; compare_exit_code = $comparison; passed = $coverage
                original = $reference.relative; original_sha256 = $reference.sha256; rust_output = $rustOutput
                rust_output_sha256 = if (Test-Path -LiteralPath $rustOutput -PathType Leaf) { (Get-FileHash -LiteralPath $rustOutput -Algorithm SHA256).Hash } else { $null }
                expected_compared_fields = $reference.expected_fields
                compared_fields = if ($null -ne $report) { $report.compared_fields } else { $null }
                difference_count = if ($null -ne $report) { $report.difference_count } else { $null }
                absolute_tolerance = 0; relative_tolerance = 0; log = $comparisonLabel + '.log'; parse_error = $parseError
            }
        })
        $additionalGenuinePassed = $additionalGenuineRows.Count -eq $genuineControlReferences.Count -and @($additionalGenuineRows | Where-Object { !$_.passed }).Count -eq 0
        Write-ProofJson 'genuine-additional-comparisons.json' ([ordered]@{
            schema_version = 1; batch = 63; source_head = $expectedParent; source_manifest_sha256 = $manifestHash
            source_version = '2.48.32'; passed = $additionalGenuinePassed; checkpoints = $additionalGenuineRows
            boundary = 'Actual original fixed/body/ties/feeding/reproduction C1/C10/C100, contacts C1/C10/C11/C100 and chloroplast C1/C10 saves. Contacts C11 is a separate extra counter. Reproduction C10 independently proves the first birth at C2; C100 is compared against its actual surviving records. Exact compared fields only; SIM omits arbitrary historical Rnd/Gauss. Movement retains its separate1379-field receipt.'
        })
        Add-QualificationResult ([ordered]@{
            step = 'genuine-additional-checkpoints-completeness'; exit_code = $(if ($additionalGenuinePassed) { 0 } else { 1 })
            expected_outputs = $genuineControlReferences.Count; actual_rows = $additionalGenuineRows.Count; log = 'genuine-additional-comparisons.json'
        })

    } else {
        Add-DependentResult 'original-zero-cycle-replay' 'Fresh CLI build failed.'
        Add-DependentResult 'original-zero-cycle-raw-file-preservation' 'Fresh CLI build failed.'
        Add-DependentResult 'original-zero-cycle-compare' 'Fresh CLI build failed.'
        Add-DependentResult 'original-zero-cycle-field-coverage' 'Fresh CLI build failed.'
        Add-DependentResult 'candidate-checkpoints-completeness' 'Fresh CLI build failed; no stale CLI or earlier candidate outputs used.'
        Add-DependentResult 'genuine-movement-checkpoints-completeness' 'Fresh CLI build failed; no stale Rust movement outputs used.'
        Add-DependentResult 'genuine-additional-candidate-checkpoints-completeness' 'Fresh CLI build failed; no plant or contacts C11 outputs reused.'
        Add-DependentResult 'genuine-additional-checkpoints-completeness' 'Fresh CLI build failed; no original comparison outcomes inherited.'
        foreach ($reference in $genuineControlReferences) {
            Add-DependentResult ('genuine-' + $reference.case + '-' + $reference.cycle + '-compare') 'Fresh CLI build failed.'
            Add-DependentResult ('genuine-' + $reference.case + '-' + $reference.cycle + '-field-coverage') 'Fresh CLI build failed.'
        }
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
        scope = 'All available ordinary CPU outcomes collected; root seals counts, summary and admission separately.'
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
