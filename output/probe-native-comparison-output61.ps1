param([string]$ReceiptPath = (Join-Path $PSScriptRoot 'pass61-native-comparison-probe.json'))
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
if (Test-Path -LiteralPath $ReceiptPath) { throw 'Refusing to overwrite an isolated comparison probe receipt.' }
$runner = Join-Path $PSScriptRoot 'qualify-native-roundtrip-pass61.ps1'
$tokens = $null
$parseErrors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($runner, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count) { throw 'Qualification runner has PowerShell syntax errors.' }
$functions = @($ast.FindAll({ param($node)
    $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -ceq 'Run-QualificationStep'
}, $true))
if ($functions.Count -ne 1) { throw 'Expected exactly one actual Run-QualificationStep implementation.' }
$nodeCommand = Get-Command node.exe -ErrorAction Stop
$scratch = Join-Path ([IO.Path]::GetTempPath()) ('db61-comparison-probe-' + [guid]::NewGuid())
[IO.Directory]::CreateDirectory($scratch) | Out-Null
$proof = $scratch
$script:probeResults = [Collections.Generic.List[object]]::new()
$caseRows = [Collections.Generic.List[object]]::new()
# Mock only the surrounding runner bookkeeping. Exercise its actual process/redirection function.
function Check-QualificationSource {}
function Save-QualificationState($status, $step, $details) {}
function Add-QualificationResult($result) { $script:probeResults.Add($result) }
function Assert-Probe([bool]$condition, [string]$message) { if (!$condition) { throw $message } }
. ([scriptblock]::Create($functions[0].Extent.Text))
$program = @'
const mode = process.argv[2];
if (mode === 'ordinary') {
    process.stdout.write('ordinary stdout\n');
    process.stderr.write('ordinary stderr\n');
    process.exit(0);
}
if (mode === 'invalid') {
    process.stdout.write('not a JSON report\n');
    process.stderr.write('actual invocation failure\n');
    process.exit(2);
}
const passed = mode === 'ok';
process.stdout.write(JSON.stringify({passed, compared_fields:passed ? 1684 : 1983, difference_count:passed ? 0 : 598}) + '\n');
if (!passed) process.stderr.write('native states differ: 598 fields\n');
process.exit(passed ? 0 : 1);
'@
[IO.File]::WriteAllText((Join-Path $scratch 'native-compare'), $program)
[IO.File]::WriteAllText((Join-Path $scratch 'ordinary-native'), $program)
Push-Location -LiteralPath $scratch
try {
    foreach ($case in @(
        @{ name = 'passed-json'; mode = 'ok'; exit = 0; fields = 1684; differences = 0 },
        @{ name = 'failed-json'; mode = 'different'; exit = 1; fields = 1983; differences = 598 }
    )) {
        $exit = Run-QualificationStep $case.name $nodeCommand.Source @('native-compare', $case.mode)
        Assert-Probe ($exit -eq $case.exit) ('Actual exit code was lost: ' + $case.name)
        $report = Get-Content -LiteralPath (Join-Path $proof ($case.name + '.log')) -Raw | ConvertFrom-Json
        Assert-Probe ($report.compared_fields -eq $case.fields -and $report.difference_count -eq $case.differences) 'JSON field counts must survive both outcomes.'
        Assert-Probe ($report.passed -eq ($case.exit -eq 0)) 'A nonzero comparison must remain failed.'
        $row = $script:probeResults[$script:probeResults.Count - 1]
        Assert-Probe ($row.exit_code -eq $case.exit -and $row.stderr_log -ceq ($case.name + '.stderr.log')) 'Comparison receipt lost status or stderr identity.'
        $stderr = [IO.File]::ReadAllText((Join-Path $proof $row.stderr_log))
        Assert-Probe $(if ($case.exit -eq 0) { $stderr.Length -eq 0 } else { $stderr.Contains('native states differ: 598 fields') }) 'Comparison stderr was not preserved separately.'
        $caseRows.Add([ordered]@{ case = $case.name; passed = $true; observed_exit = $exit; fields = $report.compared_fields; differences = $report.difference_count })
    }
    $exit = Run-QualificationStep 'invalid-json' $nodeCommand.Source @('native-compare', 'invalid')
    Assert-Probe ($exit -eq 2) 'Unexpected invocation failure must retain its actual status.'
    $rejected = $false
    try { $null = Get-Content -LiteralPath (Join-Path $proof 'invalid-json.log') -Raw | ConvertFrom-Json }
    catch { $rejected = $true }
    Assert-Probe $rejected 'Malformed stdout must remain unparseable.'
    Assert-Probe ([IO.File]::ReadAllText((Join-Path $proof 'invalid-json.stderr.log')).Contains('actual invocation failure')) 'Invocation error evidence was lost.'
    $caseRows.Add([ordered]@{ case = 'invalid-json'; passed = $true; observed_exit = $exit; invalid_stdout_rejected = $true })
    $exit = Run-QualificationStep 'ordinary' $nodeCommand.Source @('ordinary-native', 'ordinary')
    $ordinary = [IO.File]::ReadAllText((Join-Path $proof 'ordinary.log'))
    Assert-Probe ($exit -eq 0 -and $ordinary.Contains('ordinary stdout') -and $ordinary.Contains('ordinary stderr')) 'Ordinary Rust/managed-style merged logging changed.'
    Assert-Probe ($null -eq $script:probeResults[$script:probeResults.Count - 1].stderr_log) 'Ordinary output unexpectedly uses comparison logging.'
    $caseRows.Add([ordered]@{ case = 'ordinary'; passed = $true; observed_exit = $exit; merged_streams_preserved = $true })
} finally { Pop-Location }
$receipt = [ordered]@{
    schema_version = 1; batch = 61; status = 'passed'; cases_passed = $caseRows.Count
    created_utc = [DateTime]::UtcNow.ToString('o'); runner_sha256 = (Get-FileHash -LiteralPath $runner -Algorithm SHA256).Hash
    runner = $runner; scratch = $scratch; cases = $caseRows.ToArray(); command_results = $script:probeResults.ToArray()
    boundary = 'Isolated tests of actual runner stdout/stderr separation and failure retention. Stub Node processes only; no port tests, original oracle, package qualification or admission claimed.'
}
ConvertTo-Json -InputObject $receipt -Depth 12 | Set-Content -LiteralPath $ReceiptPath -Encoding utf8NoBOM
ConvertTo-Json -InputObject $receipt -Depth 12
