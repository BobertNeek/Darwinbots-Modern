$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
[Diagnostics.Process]::GetCurrentProcess().PriorityClass = 'BelowNormal'
$env:CARGO_BUILD_JOBS = '1'
$env:RUST_TEST_THREADS = '1'
$env:RAYON_NUM_THREADS = '1'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Set-Location -LiteralPath $repo
$proof = Join-Path $repo 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass43'
if (Test-Path -LiteralPath $proof) { throw 'Use a fresh qualification directory.' }
[IO.Directory]::CreateDirectory($proof) | Out-Null
$parent = (git rev-parse HEAD).Trim()
$baseline = Get-Content -LiteralPath (Join-Path $repo 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass42/source.json') -Raw | ConvertFrom-Json
$allowed = @($baseline.files.file) + 'modern/engine/tests/native_continuation_qualification.rs'
$results = [Collections.Generic.List[object]]::new()
$image = Join-Path $repo 'docs/parity/receipts/db24832-world06-shape-editor.png'
$imageHash = (Get-FileHash -LiteralPath $image).Hash
function State($status,$step,$details) {
 [ordered]@{status=$status;step=$step;pid=$PID;utc=[DateTime]::UtcNow.ToString('o');source_parent=$parent;details=$details} | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $proof 'state.json')
}
function Check-Source {
 foreach($baseFile in $baseline.files) { if((Get-FileHash -LiteralPath (Join-Path $repo $baseFile.file)).Hash -ne $baseFile.sha256) { throw ('Inherited42 source changed: '+$baseFile.file) } }
 $dirty = @(git diff --name-only $parent -- modern Cargo.toml Cargo.lock .cargo)
 if($LASTEXITCODE -ne 0) { throw 'Source comparison failed.' }
 foreach($file in $dirty) { if($file -notin $allowed) { throw ('Unexpected source change: '+$file) } }
 $new = @(git ls-files --others --exclude-standard -- modern Cargo.toml Cargo.lock .cargo)
 if($LASTEXITCODE -ne 0 -or $new.Count) { throw 'Unexpected untracked source.' }
 if($script:pins) { foreach($file in $script:pins) { if((Get-FileHash -LiteralPath (Join-Path $repo $file.file)).Hash -ne $file.sha256) { throw ('Frozen source changed: '+$file.file) } } }
}
function Run-Step($label,$executable,[string[]]$arguments) {
 Check-Source
 State 'running' $label @{}
 $started = [DateTime]::UtcNow
 & $executable @arguments *> (Join-Path $proof ($label+'.log'))
 $results.Add([ordered]@{step=$label;exit_code=$LASTEXITCODE;seconds=([DateTime]::UtcNow-$started).TotalSeconds;log=($label+'.log')})
 $results | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $proof 'results.json')
}
try {
 Check-Source
 & cargo.exe fmt --all *> (Join-Path $proof 'format.log')
 if($LASTEXITCODE -ne 0) { throw 'Formatting failed.' }
 Check-Source
 $script:pins = @(foreach($file in $allowed) { [ordered]@{file=$file;sha256=(Get-FileHash -LiteralPath (Join-Path $repo $file)).Hash} })
 [ordered]@{parent_head=$parent;files=$pins;scope='Physical continuation assertions and GPU runner admission only; all ordinary CPU functions and engine/Core/desktop/CLI production unchanged';ordinary_cpu_baseline='../native-roundtrip-pass42/summary.json';physical_runtime_not_executed=$true} | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $proof 'source.json')
 Run-Step 'format-check' 'cargo.exe' @('fmt','--all','--','--check')
 Run-Step 'physical-target-clippy' 'cargo.exe' @('clippy','--release','-p','darwinbots-engine','--test','native_continuation_qualification','--','-D','warnings')
 Run-Step 'physical-target-link' 'cargo.exe' @('test','--release','-p','darwinbots-engine','--test','native_continuation_qualification','--no-run')
 Check-Source
 $afterImage = (Get-FileHash -LiteralPath $image).Hash
 if($afterImage -ne $imageHash) { throw 'Unrelated WIP image changed.' }
 [ordered]@{before=$imageHash;after=$afterImage;unchanged=$true} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $proof 'preserved-wip-image.json')
 $failed = @($results | Where-Object { $_.exit_code -ne 0 })
 State $(if($failed.Count){'needs_repair'}else{'completed'}) 'all' @{failed_steps=@($failed | ForEach-Object {$_.step});physical_tests_executed=0}
 $results | ConvertTo-Json -Depth 5
} catch { State 'stopped' 'qualification' @{error=$_.Exception.Message}; throw }
