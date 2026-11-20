$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
[Diagnostics.Process]::GetCurrentProcess().PriorityClass = 'BelowNormal'
$env:CARGO_BUILD_JOBS='1'
$env:RUST_TEST_THREADS='1'
$env:RAYON_NUM_THREADS='1'
$repo=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Set-Location -LiteralPath $repo
$proof=Join-Path $repo 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass49'
[IO.Directory]::CreateDirectory($proof)|Out-Null
$manifest=Get-Content -LiteralPath (Join-Path $repo 'output/native-roundtrip-pass49-source.json') -Raw|ConvertFrom-Json
if(Test-Path -LiteralPath (Join-Path $proof 'state.json')){throw 'Existing batch49 state; do not duplicate or overwrite a run'}
$manifest|ConvertTo-Json -Depth 6|Set-Content -LiteralPath (Join-Path $proof 'source.json')
$results=[Collections.Generic.List[object]]::new()
$qualificationExit=1

$preservedImage=Join-Path $repo 'docs/parity/receipts/db24832-world06-shape-editor.png'
$preservedImageBackup=Join-Path $repo 'output/native-pass49-preserved-shape-editor.png'
$preservedImageHash=(Get-FileHash -LiteralPath $preservedImage).Hash
if(Test-Path -LiteralPath $preservedImageBackup){throw 'Do not overwrite an existing WIP image backup'}
Copy-Item -LiteralPath $preservedImage -Destination $preservedImageBackup
$script:preservedImageRestored=$false
function Restore-PreexistingImage {
 if($script:preservedImageRestored){return}
 $afterTests=(Get-FileHash -LiteralPath $preservedImage).Hash
 if($afterTests -ne $preservedImageHash){Copy-Item -LiteralPath $preservedImageBackup -Destination $preservedImage}
 $restored=(Get-FileHash -LiteralPath $preservedImage).Hash
 if($restored -ne $preservedImageHash){throw 'Preexisting WIP image was not restored'}
 [ordered]@{file='docs/parity/receipts/db24832-world06-shape-editor.png';before=$preservedImageHash;test_output=$afterTests;restored=$restored}|ConvertTo-Json|Set-Content -LiteralPath (Join-Path $proof 'preserved-wip-image.json')
 $script:preservedImageRestored=$true
}

function Save-QualificationState($status,$step,$details) {
 [ordered]@{status=$status;step=$step;pid=$PID;utc=[DateTime]::UtcNow.ToString('o');source_parent=$manifest.parent_head;details=$details}|ConvertTo-Json -Depth 7|Set-Content -LiteralPath (Join-Path $proof 'state.json')
}
function Check-QualificationSource {
 Check-RetainedPass48Proof
 if(Test-Path -LiteralPath (Join-Path $repo 'output/STOP-native-pass49')){throw 'Owned batch49 STOP marker present'}
 foreach($file in $manifest.files) {
  if((Get-FileHash -LiteralPath (Join-Path $repo $file.file) -Algorithm SHA256).Hash -ne $file.sha256){throw ('Qualified source changed: '+$file.file)}
 }
 $changed=@(git -C $repo diff --name-only $manifest.parent_head -- modern Cargo.toml Cargo.lock .cargo 2>> (Join-Path $proof "source-check.stderr.log"))
 foreach($file in $changed){if($file -notin $manifest.files.file){throw ('Unpinned source change: '+$file)}}
 $untracked=@(git -C $repo ls-files --others --exclude-standard -- modern Cargo.toml Cargo.lock .cargo)
 foreach($file in $untracked){if($file -notin $manifest.files.file){throw ('Unpinned untracked source: '+$file)}}
}
function Run-QualificationStep($label,$executable,[string[]]$arguments) {
 Check-QualificationSource
 $log=Join-Path $proof ($label+'.log')
 Save-QualificationState 'running' $label @{log=$log}
 $started=[DateTime]::UtcNow
 & $executable @arguments *> $log
 $exitCode=$LASTEXITCODE
 Check-QualificationSource
 $results.Add([ordered]@{step=$label;exit_code=$exitCode;seconds=([DateTime]::UtcNow-$started).TotalSeconds;log=($label+'.log')})
 $results|ConvertTo-Json -Depth 5|Set-Content -LiteralPath (Join-Path $proof 'results.json')
 return $exitCode
}

# Source49 and the explicit retained48 projection must remain frozen throughout this run.
$projection=Get-Content -LiteralPath (Join-Path $repo 'output/pass49-retained48-projection.json') -Raw|ConvertFrom-Json
function Check-RetainedPass48Proof {
 if($projection.qualification_state -ne 'all48_outcomes_collected_retained_targets_passed_and_final49_scope_verified'){throw 'Retained48 projection is not released'}
 foreach($retained in $projection.retained_file_hashes){
  if((Get-FileHash -LiteralPath (Join-Path $repo $retained.file)).Hash -ne $retained.sha256){throw ('Retained48 input changed: '+$retained.file)}
 }
 if((Get-FileHash -LiteralPath (Join-Path $repo 'output/native-roundtrip-pass49-source.json')).Hash -ne $projection.current_source_manifest_sha256){throw 'Source49 manifest changed'}
}
try {
 Check-RetainedPass48Proof
 $package=Join-Path $repo 'artifacts/win-x64-native-roundtrip-pass49'
 if(Test-Path -LiteralPath $package){throw 'Do not overwrite or merge an existing49 package'}
 if(Test-Path -LiteralPath (Join-Path $repo 'output/native-oracle-candidate49')){throw 'Do not reuse candidate49 outputs'}
 $candidateInputManifest=Get-Content -LiteralPath (Join-Path $repo 'output/native-oracle-candidate39/manifest.json') -Raw|ConvertFrom-Json
 $expectedCases=@('body','contacts','feeding','fixed','movement','reproduction','ties')
 if(@($candidateInputManifest.cases).Count -ne 7 -or ((@($candidateInputManifest.cases.case|Sort-Object) -join '|') -ne ($expectedCases -join '|'))){throw 'Candidate manifest requires exact seven source cases'}
 foreach($inputCase in $candidateInputManifest.cases){if((@($inputCase.checkpoint_cycles) -join '|') -ne '1|10|100'){throw 'Candidate checkpoints must be exactly1,10,100'};if(!(Test-Path -LiteralPath (Join-Path (Join-Path $repo 'output/native-oracle-candidate39') $inputCase.start_sim))){throw 'Candidate input missing'}}
 $format=Run-QualificationStep 'format-check' 'cargo.exe' @('fmt','--all','--','--check')

 $lint=Run-QualificationStep 'strict-clippy' 'cargo.exe' @('clippy','--release','--workspace','--all-targets','--','-D','warnings')

 Check-RetainedPass48Proof
 $null=Run-QualificationStep 'cpu-engine-lib' 'cargo.exe' @('test','--release','-p','darwinbots-engine','--lib','--no-fail-fast','--','--skip','gpu','--skip','switch','--nocapture','--test-threads=1')
 $null=Run-QualificationStep 'cpu-ffi-integrations' 'cargo.exe' @('test','--release','-p','darwinbots-engine','--test','core_state_contracts','--test','gpu_ffi','--test','native_random_lifecycle_248','--test','robot_tools','--no-fail-fast','--','--skip','gpu','--skip','switch','--nocapture','--test-threads=1')
 $null=Run-QualificationStep 'cpu-cli-lib' 'cargo.exe' @('test','--release','-p','darwinbots-cli','--lib','--no-fail-fast','--','--skip','gpu','--skip','switch','--nocapture','--test-threads=1')
 $null=Run-QualificationStep 'cpu-cli-integrations' 'cargo.exe' @('test','--release','-p','darwinbots-cli','--test','commands','--test','native_comparison','--no-fail-fast','--','--skip','gpu','--skip','switch','--nocapture','--test-threads=1')
 $null=Run-QualificationStep 'cpu-docs' 'cargo.exe' @('test','--release','--workspace','--doc','--no-fail-fast','--','--test-threads=1')
 Check-RetainedPass48Proof
 $cliBuilt=Run-QualificationStep 'cli-build' 'cargo.exe' @('build','--release','-p','darwinbots-cli')
 if($cliBuilt -eq 0) {
  $qualifiedCliHash=(Get-FileHash -LiteralPath './target/release/darwinbots-cli.exe').Hash
  $replay=Run-QualificationStep 'original-zero-cycle-replay' './target/release/darwinbots-cli.exe' @('native-replay','--simulation','modern/engine/tests/fixtures/db24832-original-user.sim','--until-cycle','281','--output-sim',(Join-Path $proof 'user-rust-cycle281.sim'))
  if($replay -eq 0){$null=Run-QualificationStep 'original-zero-cycle-compare' './target/release/darwinbots-cli.exe' @('native-compare','--left','modern/engine/tests/fixtures/db24832-original-user.sim','--right',(Join-Path $proof 'user-rust-cycle281.sim'),'--absolute-tolerance','0','--relative-tolerance','0','--max-differences','40')}
  [ordered]@{cli_sha256=(Get-FileHash -LiteralPath './target/release/darwinbots-cli.exe').Hash;source='source.json';build_exit_code=$cliBuilt}|ConvertTo-Json|Set-Content -LiteralPath (Join-Path $proof 'cli-build-identity.json')
  $candidate49=Join-Path $repo 'output/native-oracle-candidate49'
  if(Test-Path -LiteralPath $candidate49){throw 'Do not reuse an existing candidate49 directory'}
  [IO.Directory]::CreateDirectory($candidate49)|Out-Null
  $candidateInput44=Join-Path $repo 'output/native-oracle-candidate39'
  $candidateManifest44=Get-Content -LiteralPath (Join-Path $candidateInput44 'manifest.json') -Raw|ConvertFrom-Json
  $candidateReceipts44=@(foreach($case44 in $candidateManifest44.cases){
   $input44=Join-Path $candidateInput44 $case44.start_sim
   foreach($cycle44 in $case44.checkpoint_cycles){
    $output44=Join-Path $candidate49 ($case44.case+'-rust-cycle'+$cycle44+'.sim')
    $candidateExit49=Run-QualificationStep ('candidate-'+$case44.case+'-'+$cycle44) './target/release/darwinbots-cli.exe' @('native-replay','--simulation',$input44,'--until-cycle',([string]$cycle44),'--output-sim',$output44)
    [ordered]@{case=$case44.case;cycle=$cycle44;exit_code=$candidateExit49;input=$input44;input_sha256=(Get-FileHash -LiteralPath $input44).Hash;output=$output44;output_sha256=if(Test-Path -LiteralPath $output44){(Get-FileHash -LiteralPath $output44).Hash}else{$null}}
   }
  })
  [ordered]@{source='native-roundtrip-pass49/source.json';status='Rust candidate outputs only; no original-runtime comparisons';cases=$candidateReceipts44}|ConvertTo-Json -Depth 5|Set-Content -LiteralPath (Join-Path $proof 'candidate-checkpoints.json')
  $candidateRowsComplete=@($candidateReceipts44).Count -eq 21 -and @($candidateReceipts44|Where-Object { $_.exit_code -ne 0 -or !$_.output_sha256 -or !(Test-Path -LiteralPath $_.output) }).Count -eq 0
  $results.Add([ordered]@{step='candidate-checkpoints-completeness';exit_code=$(if($candidateRowsComplete){0}else{1});cases=7;expected_outputs=21;actual_rows=@($candidateReceipts44).Count})
 }
 $null=Run-QualificationStep 'desktop-core' 'dotnet.exe' @('test','modern/desktop/tests/Darwinbots.Desktop.Core.Tests/Darwinbots.Desktop.Core.Tests.csproj','-c','Release','--logger','trx;LogFileName=core.trx','--results-directory',$proof)
 $desktopBuilt=Run-QualificationStep 'desktop-test-build' 'dotnet.exe' @('build','modern/desktop/tests/Darwinbots.Desktop.Tests/Darwinbots.Desktop.Tests.csproj','-c','Release')
 $package=Join-Path $repo 'artifacts/win-x64-native-roundtrip-pass49'
 if(Test-Path -LiteralPath $package){throw 'Package directory appeared before publish; do not overwrite'}
 $published=Run-QualificationStep 'windows-publish' 'dotnet.exe' @('publish','modern/desktop/src/Darwinbots.Desktop/Darwinbots.Desktop.csproj','-c','Release','-r','win-x64','--self-contained','false','-o',$package)
 if($published -eq 0) {
  Check-QualificationSource
  Save-QualificationState 'running' 'package-smoke' @{}
  $smoke=Start-Process -FilePath (Join-Path $package 'Darwinbots.Desktop.exe') -ArgumentList '--smoke-test' -WorkingDirectory $package -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $proof 'package-smoke.json') -RedirectStandardError (Join-Path $proof 'package-smoke.stderr.log')
  $null=$smoke.Handle
  $smokeTimedOut=!$smoke.WaitForExit(120000)
  if($smokeTimedOut){$smoke.Kill($true);$smoke.WaitForExit()}
  $smoke.WaitForExit()
  Check-QualificationSource
  $results.Add([ordered]@{step='package-smoke';exit_code=$(if($smokeTimedOut){124}else{$smoke.ExitCode});timed_out=$smokeTimedOut;log='package-smoke.json'})
  if($desktopBuilt -eq 0) {
   $packageDll=Join-Path $package 'darwinbots_engine.dll'
   $testDll=Join-Path $repo 'modern/desktop/tests/Darwinbots.Desktop.Tests/bin/Release/net10.0/darwinbots_engine.dll'
   $targetDll=Join-Path $repo 'target/release/darwinbots_engine.dll'
   Copy-Item -LiteralPath $packageDll -Destination $testDll
   $dllBefore=[ordered]@{target_release=(Get-FileHash -LiteralPath $targetDll).Hash;package=(Get-FileHash -LiteralPath $packageDll).Hash;desktop=(Get-FileHash -LiteralPath $testDll).Hash}
   if($dllBefore.target_release -ne $dllBefore.package -or $dllBefore.package -ne $dllBefore.desktop){throw 'Fresh target/package/test DLL identity mismatch'}
   $dllBefore|ConvertTo-Json|Set-Content -LiteralPath (Join-Path $proof 'native-dll-identity-before.json')
   $env:DARWINBOTS_GUI_AUDIT_DIR=Join-Path $proof 'gui'
   $null=Run-QualificationStep 'packaged-native-desktop' 'dotnet.exe' @('test','modern/desktop/tests/Darwinbots.Desktop.Tests/Darwinbots.Desktop.Tests.csproj','-c','Release','--no-build','--blame-hang-timeout','10m','--blame-hang-dump-type','none','--logger','trx;LogFileName=desktop.trx','--results-directory',$proof)
   Restore-PreexistingImage
   $dllAfter=[ordered]@{target_release=(Get-FileHash -LiteralPath $targetDll).Hash;package=(Get-FileHash -LiteralPath $packageDll).Hash;desktop=(Get-FileHash -LiteralPath $testDll).Hash}
   if($dllAfter.target_release -ne $dllBefore.target_release -or $dllAfter.package -ne $dllBefore.package -or $dllAfter.desktop -ne $dllBefore.desktop){throw 'Qualified DLL changed during Desktop tests'}
   $dllAfter|ConvertTo-Json|Set-Content -LiteralPath (Join-Path $proof 'native-dll-identity.json')
  }
  $assets=@(Get-ChildItem Installer/bots -File -Filter *.txt -Recurse)
  $mismatches=@(foreach($asset in $assets){$relative=[IO.Path]::GetRelativePath((Join-Path $repo 'Installer/bots'),$asset.FullName);$target=if($asset.Name -in @('Animal_Minimalis.txt','Alga_Minimalis_Chloroplastus.txt')){Join-Path (Join-Path $package 'Bots') $asset.Name}else{Join-Path (Join-Path $package 'Bots/Historical') $relative};if(!(Test-Path -LiteralPath $target) -or (Get-FileHash -LiteralPath $target).Hash -ne (Get-FileHash -LiteralPath $asset.FullName).Hash){$relative}})
  [ordered]@{bots=$assets.Count;asset_mismatches=$mismatches}|ConvertTo-Json|Set-Content -LiteralPath (Join-Path $proof 'package-assets.json')
  $results.Add([ordered]@{step='package-assets';exit_code=$(if($assets.Count -eq 48 -and $mismatches.Count -eq 0){0}else{1});bots=$assets.Count})
 }

 Check-QualificationSource
 Check-RetainedPass48Proof
 if($cliBuilt -eq 0 -and (Get-FileHash -LiteralPath './target/release/darwinbots-cli.exe').Hash -ne $qualifiedCliHash){throw 'Qualified CLI changed after build'}
 $projection|ConvertTo-Json -Depth 12|Set-Content -LiteralPath (Join-Path $proof 'retained-pass48-projection.json')
 $results.Add([ordered]@{step='retained-pass48-targets';exit_code=0;retained_targets=$projection.retained_target_count;fresh_target_invocations=$projection.fresh_target_count;basis='exact source and reachability projection; not fresh49 runtime'})
 $results|ConvertTo-Json -Depth 5|Set-Content -LiteralPath (Join-Path $proof 'results.json')
 $failed=@($results|Where-Object exit_code -ne 0)
 Save-QualificationState $(if($failed.Count){'needs_repair'}else{'completed'}) 'all' @{failed_steps=@($failed | ForEach-Object { $_.step })}
 $qualificationExit=$(if($failed.Count){1}else{0})
 $results|ConvertTo-Json -Depth 5
} catch {
 Save-QualificationState 'stopped' 'qualification' @{error=$_.Exception.Message}
 throw
} finally {
 Restore-PreexistingImage
}

exit $qualificationExit
