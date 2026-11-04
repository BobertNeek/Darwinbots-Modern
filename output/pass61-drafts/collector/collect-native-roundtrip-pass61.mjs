import fs from 'node:fs/promises';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';

// Unexecuted authoring candidate. Run only after the sole qualifier has exited.
// Invocation: node.exe THIS_FILE --repo ABSOLUTE_WORKTREE --runner-exit-code 0|1
const EXPECTED_HEAD = '8a42380d2096bb97d5028a0c3cfc03b96420fdcd';
const EXPECTED_MANIFEST = '4AD7E8553AAF90EBDE8C5B73172F38B67D97C76B8A4D239F99074E2C580408A0';
const EXPECTED_PNG = '44B7419EC76B825F1C307A284BD14A607DC975C1C410857F41A6A66C1E57FAE9';
const EXPECTED_STEPS = ["format-check","strict-clippy","cpu-rust-workspace","cli-build","original-zero-cycle-replay","original-zero-cycle-raw-file-preservation","original-zero-cycle-compare","original-zero-cycle-field-coverage","candidate-movement-1","candidate-movement-10","candidate-movement-100","candidate-fixed-1","candidate-fixed-10","candidate-fixed-100","candidate-contacts-1","candidate-contacts-10","candidate-contacts-100","candidate-body-1","candidate-body-10","candidate-body-100","candidate-ties-1","candidate-ties-10","candidate-ties-100","candidate-feeding-1","candidate-feeding-10","candidate-feeding-100","candidate-reproduction-1","candidate-reproduction-10","candidate-reproduction-100","candidate-checkpoints-completeness","candidate-movement-102","genuine-movement-1-compare","genuine-movement-1-field-coverage","genuine-movement-10-compare","genuine-movement-10-field-coverage","genuine-movement-100-compare","genuine-movement-100-field-coverage","genuine-movement-102-compare","genuine-movement-102-field-coverage","genuine-movement-checkpoints-completeness","candidate-chloroplast-1","candidate-chloroplast-10","candidate-contacts-11","genuine-additional-candidate-checkpoints-completeness","genuine-fixed-1-compare","genuine-fixed-1-field-coverage","genuine-body-1-compare","genuine-body-1-field-coverage","genuine-chloroplast-1-compare","genuine-chloroplast-1-field-coverage","genuine-chloroplast-10-compare","genuine-chloroplast-10-field-coverage","genuine-feeding-1-compare","genuine-feeding-1-field-coverage","genuine-feeding-10-compare","genuine-feeding-10-field-coverage","genuine-ties-1-compare","genuine-ties-1-field-coverage","genuine-contacts-1-compare","genuine-contacts-1-field-coverage","genuine-contacts-11-compare","genuine-contacts-11-field-coverage","genuine-contacts-100-compare","genuine-contacts-100-field-coverage","genuine-reproduction-1-compare","genuine-reproduction-1-field-coverage","genuine-reproduction-10-compare","genuine-reproduction-10-field-coverage","genuine-fixed-10-compare","genuine-fixed-10-field-coverage","genuine-fixed-100-compare","genuine-fixed-100-field-coverage","genuine-body-10-compare","genuine-body-10-field-coverage","genuine-body-100-compare","genuine-body-100-field-coverage","genuine-contacts-10-compare","genuine-contacts-10-field-coverage","genuine-ties-10-compare","genuine-ties-10-field-coverage","genuine-ties-100-compare","genuine-ties-100-field-coverage","genuine-feeding-100-compare","genuine-feeding-100-field-coverage","genuine-reproduction-100-compare","genuine-reproduction-100-field-coverage","genuine-additional-checkpoints-completeness","desktop-core","desktop-test-build","windows-publish","package-smoke","packaged-native-desktop","package-assets","preexisting-wip-image"];
const SOURCE = 'output/native-roundtrip-pass61-source.json';
const PROOF = 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass61';
const PACKAGE = 'artifacts/win-x64-native-roundtrip-pass61';
const PNG = 'docs/parity/receipts/db24832-world06-shape-editor.png';
const RUST_RECEIPT = PROOF + '/rust-target-results.json';
const SUMMARY = PROOF + '/qualification-summary.json';
const issues = [];
const fail = message => { throw new Error(message); };
const must = (condition, message) => { if (!condition) fail(message); };
const note = (condition, message) => { if (!condition) issues.push(message); };
const sha = data => createHash('sha256').update(data).digest('hex').toUpperCase();
const integer = value => typeof value === 'number' && Number.isSafeInteger(value);
const count = value => integer(value) && value >= 0;
const jsonBytes = value => Buffer.from(JSON.stringify(value, null, 2) + '\n');
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
const args = process.argv.slice(2);
must(args.length === 4 && args[0] === '--repo' && args[2] === '--runner-exit-code' &&
  /^(0|1)$/.test(args[3]), 'Pass --repo ABSOLUTE_WORKTREE --runner-exit-code ACTUAL_0_OR_1.');
const repo = path.resolve(args[1]);
must(path.isAbsolute(args[1]), 'Repository argument must be absolute.');
const runnerExit = Number(args[3]);
const repoReal = await fs.realpath(repo);
const rootKey = repoReal.toLowerCase() + path.sep;
const owns = absolute => absolute.toLowerCase().startsWith(rootKey);
const absolute = file => path.resolve(repo, file);
const relative = file => path.relative(repo, file).replaceAll('\\', '/');
async function owned(file) {
  const resolved = absolute(file);
  must(owns(resolved), 'Path is outside the worktree: ' + file);
  const real = await fs.realpath(resolved);
  must(owns(real), 'Real path is outside the worktree: ' + file);
  return resolved;
}
async function raw(file) { return fs.readFile(await owned(file)); }
async function readJson(file) {
  return JSON.parse((await raw(file)).toString('utf8').replace(/^\uFEFF/, ''));
}
async function optionalJson(file) {
  try { return { value: await readJson(file), parse_error: null }; }
  catch (error) {
    if (error.code === 'ENOENT') return { value: null, parse_error: 'missing' };
    return { value: null, parse_error: error.message };
  }
}
async function fileInfo(file) {
  const data = await raw(file);
  return { file: relative(absolute(file)), sha256: sha(data), bytes: data.length };
}
async function exists(file) {
  try { await fs.stat(absolute(file)); return true; }
  catch (error) { if (error.code === 'ENOENT') return false; throw error; }
}
function currentHead() {
  const value = execFileSync('git.exe', ['-C', repo, 'rev-parse', 'HEAD'],
    { encoding: 'utf8', windowsHide: true }).trim();
  must(value === EXPECTED_HEAD, 'Current HEAD differs from the frozen source61 commit.');
  return value;
}
async function verifySource() {
  must(sha(await raw(SOURCE)) === EXPECTED_MANIFEST, 'Source manifest hash changed.');
  must(sha(await raw(PROOF + '/source.json')) === EXPECTED_MANIFEST, 'Proof manifest hash changed.');
  must(manifest.parent_head === EXPECTED_HEAD && manifest.source_file_count === 636 &&
    manifest.files.length === 636, 'Wrong frozen source/head/pin count.');
  const keys = new Set();
  for (const pin of manifest.files) {
    must(typeof pin.file === 'string' && !path.isAbsolute(pin.file) &&
      /^[0-9A-F]{64}$/i.test(pin.sha256), 'Invalid source pin.');
    const resolved = await owned(pin.file);
    const key = resolved.toLowerCase();
    must(!keys.has(key), 'Duplicate canonical source pin: ' + pin.file);
    keys.add(key);
    must(sha(await fs.readFile(resolved)) === pin.sha256.toUpperCase(),
      'Frozen input changed: ' + pin.file);
  }
  return { raw_pin_count: manifest.files.length, distinct_case_insensitive_pins: keys.size,
    raw_pin_mismatches: 0, invalid_or_outside_workspace_pins: 0,
    all_pins_reverified_utc: new Date().toISOString(), actual_head: currentHead(),
    source_and_proof_manifest_hashes_match: true };
}
must(!(await exists(RUST_RECEIPT)) && !(await exists(SUMMARY)),
  'Do not overwrite either existing evidence receipt.');
const manifest = await readJson(SOURCE);
must(manifest.qualification_state === 'all61_code_and_tests_authored_frozen' &&
  manifest.expected_ordinary_outcomes === 94 && manifest.expected_distinct_rust_targets === 68 &&
  manifest.expected_managed_test_counts?.core === 312 && manifest.expected_managed_test_counts?.desktop === 213,
  'Unexpected source61 qualification declaration.');
const state = await readJson(PROOF + '/state.json');
const results = await readJson(PROOF + '/results.json');
must(state.step === 'all' && ['completed', 'needs_repair'].includes(state.status),
  'The full qualifier has not reached an ordinary terminal state.');
must(state.source_parent === EXPECTED_HEAD && state.source_manifest_sha256 === EXPECTED_MANIFEST,
  'Terminal state belongs to a different source.');
must(integer(state.pid) && state.pid > 0 && state.pid <= 0xFFFFFFFF, 'Invalid runner PID.');
const pidPresent = execFileSync('pwsh.exe', ['-NoProfile', '-NonInteractive', '-Command',
  '@(Get-CimInstance Win32_Process -Filter "ProcessId = ' + state.pid + '" -ErrorAction Stop).Count'],
  { encoding: 'utf8', windowsHide: true }).trim();
must(pidPresent === '0', 'The original runner PID is present or its absence was not verified.');
must(Array.isArray(results) && results.length === 94, 'Expected all94 ordinary outcome records.');
const byStep = new Map();
for (const step of results) {
  must(typeof step.step === 'string' && integer(step.exit_code) && !byStep.has(step.step),
    'Duplicate/invalid ordinary outcome.');
  byStep.set(step.step, step);
}
must(EXPECTED_STEPS.length === 94 && EXPECTED_STEPS.every(name => byStep.has(name)),
  'The exact94 ordinary step identities do not match the runner convention.');
for (const item of results) {
  if (item.status === 'not_run_dependency') must(item.exit_code === 125, 'Invalid dependency result status.');
  if (item.log) {
    must(typeof item.log === 'string' && !path.isAbsolute(item.log), 'Invalid ordinary log identity.');
    await fileInfo(PROOF + '/' + item.log);
  }
  if (item.arguments?.[0] === 'native-compare') {
    must(item.stderr_log === item.step + '.stderr.log', 'Missing split comparison stderr identity.');
    await fileInfo(PROOF + '/' + item.stderr_log);
  }
}
const failedSteps = results.filter(step => step.exit_code !== 0);
must((state.status === 'completed') === (failedSteps.length === 0) &&
  runnerExit === (failedSteps.length ? 1 : 0), 'Actual runner exit/state/outcomes disagree.');
must(same([...(state.details.failed_steps ?? [])].sort(), failedSteps.map(x => x.step).sort()),
  'Terminal failed-step identities disagree with results.json.');
const step = name => byStep.get(name);
const dependency = item => item?.status === 'not_run_dependency';
const sourceValidationStart = await verifySource();
const identity = await readJson(PROOF + '/runner-identity.json');
must(identity.file === 'output/qualify-native-roundtrip-pass61.ps1' &&
  sha(await raw(identity.file)) === identity.sha256 &&
  identity.source_manifest_sha256 === EXPECTED_MANIFEST, 'Executed runner identity mismatch.');

async function proofEvidence() {
  const files = [];
  async function visit(directory) {
    for (const entry of await fs.readdir(await owned(directory), { withFileTypes: true })) {
      const file = directory + '/' + entry.name;
      if (entry.isDirectory()) await visit(file);
      else if (entry.isFile() && !['run.lock', 'qualification-summary.json', 'rust-target-results.json'].includes(entry.name))
        files.push(await fileInfo(file));
      else if (!entry.isFile()) fail('Unexpected nonregular proof entry: ' + file);
    }
  }
  await visit(PROOF);
  return files.sort((a, b) => a.file.localeCompare(b.file));
}
const initialEvidence = await proofEvidence();

const metadata = await readJson(manifest.workspace_metadata);
const metadataInfo = await fileInfo(manifest.workspace_metadata);
const members = metadata.workspace_members;
must(Array.isArray(members) && new Set(members).size === members.length, 'Invalid workspace members.');
const packages = metadata.packages.filter(pkg => members.includes(pkg.id));
must(packages.length === members.length, 'Missing workspace package metadata.');
const catalogue = [];
for (const pkg of packages) {
  for (const target of pkg.targets) {
    const kinds = target.kind;
    let kind;
    if (kinds.includes('test')) kind = 'test';
    else if (kinds.includes('bin')) kind = 'bin';
    else if (kinds.some(k => ['lib', 'cdylib', 'rlib'].includes(k))) kind = 'lib';
    else { must(!target.test && !target.doctest, 'Unsupported test target kind.'); continue; }
    const record = { package: pkg.name, kind, name: target.name, cargo_kinds: kinds,
      source: relative(await owned(target.src_path)) };
    if (target.test) catalogue.push(record);
    if (target.doctest) catalogue.push({ ...record, kind: 'doctest' });
  }
}
must(catalogue.length === 68, 'Frozen Cargo metadata does not declare68 targets.');
const targetKey = row => row.package + '|' + row.kind + '|' + row.name;
must(new Set(catalogue.map(targetKey)).size === catalogue.length, 'Ambiguous Cargo target identities.');
function parseRust(log) {
  const lines = log.split(/\r?\n/);
  const rows = [], incomplete = [];
  let current = null;
  for (let index = 0; index < lines.length; index++) {
    const line = lines[index];
    const header = line.match(/^\s+Running (unittests|tests[\\/])(.+)$/);
    const doc = line.match(/^\s*Doc-tests\s+(\S+)/);
    if (header || doc) {
      if (current) incomplete.push(current);
      let choices;
      if (doc) choices = catalogue.filter(t => t.kind === 'doctest' && t.name.replaceAll('-', '_') === doc[1]);
      else if (header[1] === 'unittests') {
        const kind = /src[\\/]main\.rs/.test(line) ? 'bin' : 'lib';
        const binary = line.match(/[\\/]([^\\/]+)-[a-f0-9]+(?:\.exe)?\)\s*$/)?.[1];
        choices = catalogue.filter(t => t.kind === kind && t.name.replaceAll('-', '_') === binary);
      } else {
        const name = line.match(/Running tests[\\/]([^\\/]+)\.rs/)?.[1];
        choices = catalogue.filter(t => t.kind === 'test' && t.name === name);
      }
      must(choices.length === 1, 'Unmapped/ambiguous Cargo header at line' + (index + 1));
      current = { ...choices[0], header_line: index + 1, header: line.trim(), failed_test_rows: [] };
    }
    const failed = line.match(/^test (.+) \.\.\. FAILED$/);
    if (failed && current) current.failed_test_rows.push({ name: failed[1], log_line: index + 1 });
    const summary = line.match(/^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out; finished in ([\d.]+)s/);
    if (!summary) continue;
    must(current, 'Rust summary has no target header at line' + (index + 1));
    const running = lines.slice(current.header_line - 1, index + 1)
      .map(text => text.match(/^running (\d+) tests?$/)).find(Boolean);
    const row = { ...current, result: summary[1], passed: Number(summary[2]), failed: Number(summary[3]),
      ignored: Number(summary[4]), measured: Number(summary[5]), filtered_out: Number(summary[6]),
      seconds: Number(summary[7]), summary_line: index + 1, summary: line,
      running_tests: running ? Number(running[1]) : null };
    row.running_test_count_matches = row.running_tests === row.passed + row.failed + row.ignored;
    note(row.running_test_count_matches, 'Rust running/summary count mismatch: ' + targetKey(row));
    note(row.failed_test_rows.length === row.failed, 'Rust failed-row count mismatch: ' + targetKey(row));
    rows.push(row);
    current = null;
  }
  if (current) incomplete.push(current);
  const keys = rows.map(targetKey);
  must(new Set(keys).size === keys.length, 'Duplicate completed Rust target results.');
  const missing = catalogue.filter(t => !keys.includes(targetKey(t)));
  const totals = rows.reduce((sum, row) => {
    for (const key of ['passed', 'failed', 'ignored', 'measured', 'filtered_out']) sum[key] += row[key];
    return sum;
  }, { passed: 0, failed: 0, ignored: 0, measured: 0, filtered_out: 0 });
  const failedNames = new Set(rows.flatMap(t => t.failed_test_rows.map(x => x.name)));
  const diagnostics = [];
  for (let i = 0; i < lines.length; i++) {
    const panic = lines[i].match(/^thread '(.+)' .* panicked at (.+):$/);
    if (panic && failedNames.has(panic[1])) diagnostics.push({
      test_name: panic[1], source: panic[2], log_line: i + 1,
      excerpt: lines.slice(i, i + 8).map(text => text.slice(0, 1200)).join('\n')
    });
  }
  return { targets: rows, incomplete_targets: incomplete, missing_expected: missing,
    totals, diagnostics, complete: rows.length === 68 && !incomplete.length && !missing.length };
}
must(same(step('cpu-rust-workspace').arguments, ['test', '--release', '--workspace', '--no-fail-fast', '--',
  '--skip', 'gpu', '--skip', 'switch', '--nocapture', '--test-threads=1']),
  'Rust result is not the required full workspace/hardware-filtered invocation.');
const rustRaw = await raw(PROOF + '/cpu-rust-workspace.log');
const rust = parseRust(rustRaw.toString('utf8'));
note(rust.complete, 'Rust did not emit every expected completed target.');
note(step('cpu-rust-workspace').exit_code !== 0 || rust.totals.failed === 0,
  'Cargo exit0 disagrees with failed Rust summaries.');
const rustPassed = rust.complete && rust.totals.failed === 0 && step('cpu-rust-workspace').exit_code === 0 &&
  rust.targets.every(row => row.running_test_count_matches && row.failed_test_rows.length === row.failed);

const xmlDecode = text => text.replace(/&#x([0-9a-f]+);|&#(\d+);|&(quot|apos|lt|gt|amp);/gi,
  (_, hex, dec, named) => hex ? String.fromCodePoint(parseInt(hex, 16)) : dec ? String.fromCodePoint(Number(dec)) :
    ({ quot: '"', apos: "'", lt: '<', gt: '>', amp: '&' })[named.toLowerCase()]);
function attributes(text) {
  return Object.fromEntries([...text.matchAll(/([A-Za-z_:][\w:.-]*)="([^"]*)"/g)]
    .map(match => [match[1], xmlDecode(match[2])]));
}
function trx(text) {
  must(!/<!DOCTYPE|<!ENTITY/i.test(text), 'Unexpected TRX document declaration.');
  const counters = [...text.matchAll(/<Counters\b([^>]*?)\/?\s*>/g)];
  must(counters.length === 1, 'TRX must contain exactly one counter record.');
  const counter = attributes(counters[0][1]);
  const rows = [...text.matchAll(/<UnitTestResult\b([^>]*?)(?:\/>|>([\s\S]*?)<\/UnitTestResult>)/g)].map(match => {
    const attrs = attributes(match[1]);
    const body = match[2] ?? '';
    return { test_name: attrs.testName, outcome: attrs.outcome, duration: attrs.duration,
      message: xmlDecode(body.match(/<Message>([\s\S]*?)<\/Message>/)?.[1] ?? '').slice(0, 3000),
      stack_trace: xmlDecode(body.match(/<StackTrace>([\s\S]*?)<\/StackTrace>/)?.[1] ?? '').slice(0, 3000) };
  });
  for (const key of ['passed', 'failed', 'total', 'executed', 'notExecuted'])
    must(/^\d+$/.test(counter[key] ?? ''), 'Missing/invalid TRX counter: ' + key);
  const result = { passed: Number(counter.passed), failed: Number(counter.failed), total: Number(counter.total),
    executed: Number(counter.executed), skipped: Number(counter.notExecuted), result_rows: rows.length,
    failed_tests: rows.filter(row => row.outcome !== 'Passed') };
  result.counter_arithmetic_consistent = result.executed + result.skipped === result.total &&
    result.total >= result.passed + result.failed + result.skipped;
  result.result_rows_match_counters = result.counter_arithmetic_consistent && rows.length === result.total &&
    rows.filter(row => row.outcome === 'Passed').length === result.passed &&
    rows.filter(row => row.outcome === 'Failed').length === result.failed &&
    rows.filter(row => row.outcome === 'NotExecuted').length === result.skipped;
  return result;
}
async function managed(name, stepName, expected) {
  const invocation = step(stepName), file = PROOF + '/' + name + '.trx';
  const present = await exists(file);
  if (dependency(invocation)) {
    must(!present, 'Dependent-not-run suite has an unexpected TRX: ' + name);
    return { status: 'not_run_dependency', expected_source_test_count: expected, discovered: 0, executed: 0,
      passed: null, failed: null, total: null, skipped: null, result_rows: 0, trx: null, reason: invocation.reason };
  }
  if (!present) {
    note(invocation.exit_code !== 0, 'Successful managed invocation has no TRX: ' + name);
    return { status: 'no_terminal_trx', expected_source_test_count: expected, discovered: null, executed: null,
      passed: null, failed: null, total: null, skipped: null, result_rows: null, trx: null,
      invocation_exit_code: invocation.exit_code };
  }
  const data = await raw(file), parsed = trx(data.toString('utf8'));
  note(parsed.result_rows_match_counters, 'Managed TRX rows/counters disagree: ' + name);
  note(parsed.total === expected, 'Managed TRX total differs from frozen declaration: ' + name);
  note(invocation.exit_code !== 0 || parsed.failed === 0, 'Managed exit0 has failed tests: ' + name);
  return { status: 'executed', ...parsed, expected_source_test_count: expected,
    expected_row_count_matches: parsed.total === expected, invocation_exit_code: invocation.exit_code,
    trx: file, trx_sha256: sha(data) };
}
const core = await managed('core', 'desktop-core', 312);
const desktop = await managed('desktop', 'packaged-native-desktop', 213);
const buildText = await fs.readFile(absolute(PROOF + '/desktop-test-build.log'), 'utf8').catch(error => {
  if (error.code === 'ENOENT' && dependency(step('desktop-test-build'))) return ''; throw error;
});
desktop.build_exit_code = step('desktop-test-build').exit_code;
desktop.compile_diagnostics = [...new Set(buildText.split(/\r?\n/).filter(line => /error\s+[A-Z]+\d+/.test(line)))];

async function verifyIdentity(file, expectedHash) {
  must(typeof expectedHash === 'string' && /^[0-9A-F]{64}$/i.test(expectedHash), 'Missing artifact hash: ' + file);
  const actual = await fileInfo(file);
  must(actual.sha256 === expectedHash.toUpperCase(), 'Artifact/receipt identity changed: ' + file);
  return actual;
}
const provenance = await readJson(PROOF + '/genuine-reference-provenance.json');
must(provenance.source_head === EXPECTED_HEAD && provenance.source_manifest_sha256 === EXPECTED_MANIFEST &&
  Array.isArray(provenance.captures) && provenance.captures.length === 25, 'Wrong genuine-reference provenance.');
must(provenance.metadata === 'modern/engine/tests/fixtures/db24832-genuine-capture-provenance.json',
  'Unexpected source capture metadata path.');
const capturePin = manifest.files.find(pin => pin.file.replaceAll('\\', '/') === provenance.metadata);
must(capturePin && capturePin.sha256 === provenance.metadata_sha256, 'Capture metadata is not source-pinned.');
await verifyIdentity(provenance.metadata, provenance.metadata_sha256);
const immutableCaptures = (await readJson(provenance.metadata)).captures;
must(Array.isArray(immutableCaptures) && immutableCaptures.length === 25, 'Pinned capture metadata must list25references.');
const referenceMap = new Map(immutableCaptures.map(row => [row.case + '|' + row.cycle, row]));
const referenceKeys = new Set(referenceMap.keys());
must(referenceKeys.size === 25, 'Duplicate genuine-reference case/cycle.');
for (const ref of provenance.captures) {
  const pinned = referenceMap.get(ref.case + '|' + ref.cycle);
  must(pinned && pinned.file === ref.file && pinned.sha256 === ref.sha256 && pinned.cycle === ref.cycle,
    'Proof capture identity differs from pinned source metadata.');
  await verifyIdentity(ref.file, ref.sha256);
}
const genuineRows = [];
for (const [receiptFile, fallbackCase] of [
  [PROOF + '/genuine-movement-comparisons.json', 'movement'],
  [PROOF + '/genuine-additional-comparisons.json', null]
]) {
  const item = await optionalJson(receiptFile);
  if (!item.value) {
    note(step('cli-build').exit_code !== 0, 'Missing comparison receipt despite successful CLI build: ' + receiptFile);
    continue;
  }
  must(item.value.source_head === EXPECTED_HEAD && item.value.source_manifest_sha256 === EXPECTED_MANIFEST,
    'Comparison receipt source mismatch.');
  for (const originalRow of item.value.checkpoints) {
    const row = { ...(fallbackCase ? { case: fallbackCase } : {}), ...originalRow };
    const key = row.case + '|' + row.cycle;
    must(referenceKeys.has(key) && !genuineRows.some(x => x.case + '|' + x.cycle === key),
      'Unexpected/duplicate genuine control: ' + key);
    const pinnedReference = referenceMap.get(key);
    must(row.original === pinnedReference.file && row.original_sha256 === pinnedReference.sha256,
      'Comparison selected the wrong original reference: ' + key);
    const expectedOutput = absolute('output/native-oracle-candidate61/' + row.case + '-rust-cycle' + row.cycle + '.sim');
    must(absolute(row.rust_output).toLowerCase() === expectedOutput.toLowerCase(),
      'Comparison did not use its fresh61 candidate path: ' + key);
    await verifyIdentity(row.original, row.original_sha256);
    if (row.rust_output_sha256) await verifyIdentity(row.rust_output, row.rust_output_sha256);
    const label = 'genuine-' + row.case + '-' + row.cycle + '-compare';
    const invocation = step(label), coverage = step(label.replace(/-compare$/, '-field-coverage'));
    must(invocation && coverage, 'Missing ordinary comparison identities: ' + key);
    let parsed = null, parseError = null, comparisonInfo = null, stderrInfo = null;
    if (!dependency(invocation)) {
      must(invocation.stderr_log === label + '.stderr.log', 'Native comparison lacks split stderr identity.');
      comparisonInfo = await fileInfo(PROOF + '/' + invocation.log);
      stderrInfo = await fileInfo(PROOF + '/' + invocation.stderr_log);
      try {
        parsed = await readJson(PROOF + '/' + invocation.log);
        must(typeof parsed.passed === 'boolean' && count(parsed.compared_fields) &&
          count(parsed.difference_count) && typeof parsed.absolute_tolerance === 'number' &&
          typeof parsed.relative_tolerance === 'number' && Array.isArray(parsed.differences),
          'Invalid native comparison report schema.');
      } catch (error) { parsed = null; parseError = error.message; }
    }
    note(invocation.exit_code !== 0 || (parsed?.passed === true && parseError === null),
      'Successful comparison has missing/failed JSON: ' + key);
    const fieldMatch = parsed && count(coverage.expected_compared_fields) &&
      parsed.compared_fields === coverage.expected_compared_fields;
    const passed = row.passed === true && invocation.exit_code === 0 && coverage.exit_code === 0 &&
      parsed?.passed === true && fieldMatch && parsed.difference_count === 0 &&
      parsed.absolute_tolerance === 0 && parsed.relative_tolerance === 0;
    note(row.passed !== true || passed, 'Runner claimed passing control without matching evidence: ' + key);
    genuineRows.push({ case: row.case, cycle: row.cycle, runner_receipt: originalRow,
      execution_status: dependency(invocation) ? 'not_run_dependency' : 'executed',
      collector_parsed_report: parsed ? { passed: parsed.passed, compared_fields: parsed.compared_fields,
        difference_count: parsed.difference_count, absolute_tolerance: parsed.absolute_tolerance,
        relative_tolerance: parsed.relative_tolerance, retained_difference_rows: parsed.differences.length,
        truncated: parsed.truncated, parse_boundary: 'entire_separate_stdout_json' } : null,
      collector_parse_error: parseError, collector_passed: passed,
      original_hash_reverified: true, output_hash_reverified: Boolean(row.rust_output_sha256),
      comparison_log: comparisonInfo, stderr_log: stderrInfo });
  }
}
const performed = genuineRows.filter(row => row.collector_parsed_report);
const genuinePassed = genuineRows.filter(row => row.collector_passed);
note(step('cli-build').exit_code !== 0 || genuineRows.length === 25, 'Expected all25 genuine checkpoint rows.');
const allGenuinePassed = genuineRows.length === 25 && genuinePassed.length === 25;
const actualUnionFields = performed.reduce((sum, row) => sum + row.collector_parsed_report.compared_fields, 0);
if (allGenuinePassed) note(actualUnionFields === 55849, 'Passing25-control field total is not55849.');

async function candidates(file) {
  const item = await optionalJson(file);
  if (!item.value) return { rows: null, parse_error: item.parse_error };
  for (const row of item.value.cases) {
    await verifyIdentity(row.input, row.input_sha256);
    const expectedOutput = absolute('output/native-oracle-candidate61/' + row.case + '-rust-cycle' + row.cycle + '.sim');
    must(absolute(row.output).toLowerCase() === expectedOutput.toLowerCase(), 'Wrong fresh candidate output path.');
    if (row.output_sha256) await verifyIdentity(row.output, row.output_sha256);
  }
  return { rows: item.value.cases, parse_error: null };
}
const freshCandidates = await candidates(PROOF + '/candidate-checkpoints.json');
note(step('candidate-checkpoints-completeness').exit_code !== 0 || Boolean(freshCandidates.rows),
  'Successful candidate completeness has no parseable receipt.');
if (freshCandidates.rows) {
  const canonicalKeys = ['movement', 'fixed', 'body', 'contacts', 'feeding', 'ties', 'reproduction']
    .flatMap(name => [1, 10, 100].map(cycle => name + '|' + cycle));
  must(freshCandidates.rows.every(row => canonicalKeys.includes(row.case + '|' + row.cycle)) &&
    freshCandidates.rows.length === 21 &&
    new Set(freshCandidates.rows.map(row => row.case + '|' + row.cycle)).size === 21 &&
    freshCandidates.rows.every(row => [1, 10, 100].includes(row.cycle)), 'Unexpected seven-case candidate identities.');
  note(step('candidate-checkpoints-completeness').exit_code !== 0 ||
    freshCandidates.rows.every(row => row.exit_code === 0 && row.output_sha256),
    'Passing candidate completeness lacks successful fresh outputs.');
}
const extraCandidates = await candidates(PROOF + '/genuine-additional-candidate-checkpoints.json');
note(step('genuine-additional-candidate-checkpoints-completeness').exit_code !== 0 ||
  (extraCandidates.rows?.length === 3 &&
   new Set(extraCandidates.rows.map(row => row.case + '|' + row.cycle)).size === 3 &&
   extraCandidates.rows.every(row => row.exit_code === 0 && row.output_sha256)),
  'Successful extra-candidate completeness lacks three fresh outputs.');
const rawZeroItem = await optionalJson(PROOF + '/original-zero-cycle-raw-preservation.json');
const rawCompareItem = await optionalJson(PROOF + '/original-zero-cycle-compare.log');
let rawZero = { status: 'not_executed_or_no_terminal_receipt', compared_fields: null, differences: null,
  passed: null, exact_raw_bytes: null };
note(step('original-zero-cycle-raw-file-preservation').exit_code !== 0 || Boolean(rawZeroItem.value),
  'Successful raw zero preservation lacks its receipt.');
if (rawZeroItem.value) {
  const row = rawZeroItem.value;
  await verifyIdentity(row.input, row.input_sha256);
  await verifyIdentity(row.output, row.output_sha256);
  const exact = (await raw(row.input)).equals(await raw(row.output));
  const report = rawCompareItem.value;
  rawZero = { status: 'executed', compared_fields: report?.compared_fields ?? null,
    differences: report?.difference_count ?? null, passed: step('original-zero-cycle-compare').exit_code === 0 &&
      report?.passed === true && report.compared_fields === 393316 && report.difference_count === 0 &&
      report.absolute_tolerance === 0 && report.relative_tolerance === 0 && exact,
    exact_raw_bytes: exact, input_sha256: row.input_sha256, output_sha256: row.output_sha256,
    parse_error: rawCompareItem.parse_error,
    comparison_log: !dependency(step('original-zero-cycle-compare')) ?
      await fileInfo(PROOF + '/original-zero-cycle-compare.log') : null,
    stderr_log: !dependency(step('original-zero-cycle-compare')) ?
      await fileInfo(PROOF + '/original-zero-cycle-compare.stderr.log') : null,
    boundary: 'Validated raw file preservation only; native Engine load normalization is separate.' };
  note(step('original-zero-cycle-field-coverage').exit_code !== 0 || rawZero.passed,
    'Passing raw zero coverage disagrees with bytes/report.');
}

const publish = step('windows-publish');
const smokeItem = await optionalJson(PROOF + '/package-smoke.json');
const assetsItem = await optionalJson(PROOF + '/package-assets.json');
const dllBeforeItem = await optionalJson(PROOF + '/native-dll-identity-before.json');
const dllAfterItem = await optionalJson(PROOF + '/native-dll-identity.json');
const artifactRows = [];
const cliItem = await optionalJson(PROOF + '/cli-build-identity.json');
if (step('cli-build').exit_code === 0) {
  must(cliItem.value, 'Successful CLI build has no identity receipt.');
  artifactRows.push(await verifyIdentity('target/release/darwinbots-cli.exe', cliItem.value.cli_sha256));
}
let assetRows = null;
if (publish.exit_code === 0) {
  must(dllBeforeItem.value && dllAfterItem.value, 'Successful package lacks native DLL identities.');
  for (const file of ['target/release/darwinbots_engine.dll', PACKAGE + '/darwinbots_engine.dll'])
    artifactRows.push(await verifyIdentity(file, dllAfterItem.value.target_release));
  must(dllBeforeItem.value.target_release === dllBeforeItem.value.package &&
    dllAfterItem.value.target_release === dllAfterItem.value.package &&
    dllBeforeItem.value.target_release === dllAfterItem.value.target_release, 'Native DLL identity changed.');
  if (!dependency(step('packaged-native-desktop'))) {
    must(dllAfterItem.value.desktop === dllAfterItem.value.target_release &&
      dllBeforeItem.value.desktop === dllAfterItem.value.desktop, 'Desktop native DLL identity mismatch.');
    artifactRows.push(await verifyIdentity(
      'modern/desktop/tests/Darwinbots.Desktop.Tests/bin/Release/net10.0/darwinbots_engine.dll',
      dllAfterItem.value.desktop));
  }
  if (assetsItem.value) {
    assetRows = [];
    for (const row of assetsItem.value.files) {
      const sourceInfo = await verifyIdentity('Installer/bots/' + row.source, row.source_sha256);
      const packageInfo = await verifyIdentity(PACKAGE + '/' + row.packaged, row.package_sha256);
      assetRows.push({ source: row.source, matches: sourceInfo.sha256 === packageInfo.sha256 });
    }
    note(step('package-assets').exit_code !== 0 ||
      (assetRows.length === 48 && assetsItem.value.bots === 48 && assetsItem.value.packaged_bots === 48 &&
       assetRows.every(row => row.matches)), 'Passing asset step lacks48 exact source/package matches.');
  } else note(step('package-assets').exit_code !== 0, 'Successful asset step lacks receipt.');
}
if (step('package-smoke').exit_code === 0) {
  const smoke = smokeItem.value;
  note(smoke?.status === 'passed' && smoke.backend === 'CPU' && smoke.ticks === 13 &&
    smoke.population === 1 && smoke.movement === true && smoke.save_resume === true && smoke.bundled_bots === 48,
    'Successful package smoke lacks the complete13-tick CPU contract.');
}
const image = await fileInfo(PNG).catch(error => {
  if (error.code === 'ENOENT') return { file: PNG, sha256: null, bytes: null }; throw error;
});
const imageItem = await readJson(PROOF + '/preserved-wip-image.json');
const imageUnchanged = image.sha256 === EXPECTED_PNG && imageItem.unchanged === true &&
  imageItem.before?.sha256 === EXPECTED_PNG && imageItem.after?.sha256 === image.sha256;
note(imageUnchanged, 'Protected PNG differs from the preserved source61 input/receipt; it was not altered or restored.');
const finalEvidence = await proofEvidence();
must(same(initialEvidence, finalEvidence), 'Proof files changed while collecting.');
const sourceValidation = await verifySource();
sourceValidation.preexisting_wip_image_unchanged = imageUnchanged;
sourceValidation.png_sha256 = image.sha256;
sourceValidation.png_bytes = image.bytes;
const sealedUtc = new Date().toISOString();
const allPassed = failedSteps.length === 0 && issues.length === 0 && rustPassed &&
  core.status === 'executed' && core.result_rows_match_counters && core.passed === 312 &&
  core.executed === 312 && core.skipped === 0 &&
  desktop.status === 'executed' && desktop.result_rows_match_counters && desktop.passed === 213 &&
  desktop.executed === 213 && desktop.skipped === 0 &&
  allGenuinePassed && rawZero.passed === true;
const rustReceipt = { schema_version: 2, batch: 61, sealed_utc: sealedUtc,
  state: rustPassed ? 'completed_passed' : 'completed_with_failures_or_incomplete_target_evidence',
  source_head: EXPECTED_HEAD, source_manifest: { file: SOURCE, sha256: EXPECTED_MANIFEST },
  workspace_metadata: metadataInfo,
  command: { executable: step('cpu-rust-workspace').executable, arguments: step('cpu-rust-workspace').arguments },
  invocation: { fresh_full_workspace_invocation: !step('cpu-rust-workspace').invocation_error, hardware_filtered: true,
    exit_code: step('cpu-rust-workspace').exit_code, seconds: step('cpu-rust-workspace').seconds,
    counts_inherited: false }, full_workspace_passed: rustPassed,
  expected_distinct_targets: 68, total_distinct_targets: rust.targets.length,
  all_expected_targets_emitted: rust.complete, counts: rust.totals,
  failed_target_count: rust.targets.filter(row => row.failed).length,
  failed_targets: rust.targets.filter(row => row.failed), failure_diagnostics: rust.diagnostics,
  all_targets: rust.targets, zero_case_targets: rust.targets.filter(row => row.running_tests === 0),
  doc_targets: rust.targets.filter(row => row.kind === 'doctest'), log: await fileInfo(PROOF + '/cpu-rust-workspace.log'),
  runner_results: await fileInfo(PROOF + '/results.json'), runner_state_at_collection: state,
  validation: { target_identity_includes_package_kind_name: true, missing_expected_targets: rust.missing_expected,
    incomplete_targets: rust.incomplete_targets, all_running_counts_match: rust.targets.every(row => row.running_test_count_matches) },
  proof_scope: 'Fresh local61 emitted results only. Missing summaries are explicit; no historical/hosted counts or physical-GPU result is inherited.' };
const rustBytes = jsonBytes(rustReceipt);
const summary = { schema_version: 3, batch: 61, sealed_utc: sealedUtc, state: allPassed ? 'completed' : 'needs_repair',
  ordinary_outcomes_complete: true, all_ordinary_outcomes_passed: allPassed,
  all_available_ordinary_outcomes_collected: true,
  all_ordinary_product_invocations_executed: !results.some(dependency) && !results.some(row => row.invocation_error),
  runner_terminal_utc: state.utc, runner_completion: { pid: state.pid, process_verified_absent: true,
    exit_code: runnerExit, exit_code_provenance: 'Explicit actual closed-session exit supplied by root; terminal state/outcomes independently agree.' },
  source_head: EXPECTED_HEAD, source_manifest_sha256: EXPECTED_MANIFEST, source_validation: sourceValidation,
  source_validation_at_collection_start: sourceValidationStart,
  ordinary_steps: { expected: 94, total: results.length, distinct: byStep.size,
    passed: results.filter(row => row.exit_code === 0).length, nonzero: failedSteps.length,
    nonzero_non_dependency: failedSteps.filter(row => !dependency(row)).length,
    dependency_not_run: failedSteps.filter(dependency).length },
  format_passed: step('format-check').exit_code === 0, strict_clippy_passed: step('strict-clippy').exit_code === 0,
  strict_clippy: step('strict-clippy'), rust: rust.totals, rust_targets: rust.targets.length,
  rust_counts_complete: rust.complete, rust_failed_targets: rustReceipt.failed_targets,
  rust_failed_test_names: rust.targets.flatMap(row => row.failed_test_rows.map(test => test.name)),
  rust_failure_diagnostics: rust.diagnostics, rust_invocation: rustReceipt.invocation,
  rust_receipt: RUST_RECEIPT, rust_receipt_sha256: sha(rustBytes), managed: { core, desktop },
  genuine: { expected_controls: 25, controls: genuineRows.length, actual_rows: genuineRows.length,
    controls_with_parseable_reports: performed.length, passed: genuinePassed.length,
    failed: genuineRows.filter(row => row.execution_status === 'executed' && !row.collector_passed).length,
    failed_or_unproven: 25 - genuinePassed.length,
    expected_original_only_projected_fields: 55849,
    matched_controls_compared_fields: genuinePassed.reduce((sum, row) => sum + row.collector_parsed_report.compared_fields, 0),
    actual_union_compared_fields_parseable_controls: actualUnionFields,
    actual_union_compared_fields_all_controls: performed.length === 25 ? actualUnionFields : null,
    compared_fields_sum: performed.length === 25 ? actualUnionFields : null,
    canonical_matrix_captures: 21, canonical_matrix_total: 21,
    extra_controls: ['movementC102', 'contactsC11', 'chloroplastC1', 'chloroplastC10'],
    all_original_and_fresh_output_hashes_reverified: genuineRows.length === 25 &&
      genuineRows.every(row => row.original_hash_reverified && row.output_hash_reverified),
    differences_parseable_controls: performed.reduce((sum, row) => sum + row.collector_parsed_report.difference_count, 0),
    absolute_tolerance: 0, relative_tolerance: 0, checkpoints: genuineRows,
    missing_reference_keys: [...referenceKeys].filter(key => !genuineRows.some(row => row.case + '|' + row.cycle === key)),
    boundary: 'Counts come from separate stdout JSON even on nonzero comparison exits. Raw runner rows and separate stderr hashes remain preserved.55849 is the original-only expectation, never substituted for observed union fields.' },
  fresh_seven_case_candidates: { cases: freshCandidates.rows ? new Set(freshCandidates.rows.map(row => row.case)).size : null,
    expected_outputs: 21, actual_outputs: freshCandidates.rows?.length ?? null,
    distinct_case_cycle_keys: freshCandidates.rows ? new Set(freshCandidates.rows.map(row => row.case + '|' + row.cycle)).size : null,
    all_exit_codes_zero: Boolean(freshCandidates.rows?.every(row => row.exit_code === 0)),
    all_fresh_output_hashes_reverified: Boolean(freshCandidates.rows?.every(row => row.output_sha256)), rows: freshCandidates.rows,
    parse_error: freshCandidates.parse_error },
  additional_candidates: extraCandidates, raw_zero: rawZero,
  package: { published: publish.exit_code === 0, smoke: smokeItem.value, smoke_parse_error: smokeItem.parse_error,
    assets: assetsItem.value?.bots ?? null, packaged_assets: assetsItem.value?.packaged_bots ?? null,
    asset_mismatches: assetsItem.value?.asset_mismatches ?? null,
    all48_packaged_asset_hashes_reverified: assetRows?.length === 48 && assetRows.every(row => row.matches),
    target_package_native_dll_identity_matched: publish.exit_code === 0 &&
      dllBeforeItem.value?.target_release === dllAfterItem.value?.target_release &&
      dllAfterItem.value?.target_release === dllAfterItem.value?.package,
    native_dll_identity_matched: publish.exit_code === 0 && !dependency(step('packaged-native-desktop')) &&
      dllAfterItem.value?.desktop === dllAfterItem.value?.target_release,
    native_dll_sha256: dllAfterItem.value?.target_release ?? null,
    production_native_build_completed: step('cli-build').exit_code === 0,
    all_packaged_asset_hashes_reverified: assetRows?.length === 48 && assetRows.every(row => row.matches),
    desktop_native_dll_qualified: !dependency(step('packaged-native-desktop')) && Boolean(dllAfterItem.value?.desktop),
    dll_identity_before: dllBeforeItem.value, dll_identity_after: dllAfterItem.value,
    final_artifact_hashes: artifactRows },
  failed_steps: failedSteps, collector_validation_issues: issues, repair_drafts_authored_by_collector: false,
  admission_created: false, gpu_or_capacity_executed_by_this_cpu_runner: false,
  evidence: [...finalEvidence, { file: RUST_RECEIPT, sha256: sha(rustBytes), bytes: rustBytes.length }]
    .sort((a, b) => a.file.localeCompare(b.file)),
  boundary: 'Source61 ordinary CPU evidence only. Dependency/no-terminal-TRX counts remain null. No canonical summary.json, CPU-ready/hardware admission, physical GPU proof or port-completion claim is created.' };
const summaryBytes = jsonBytes(summary);
must(!(await exists(RUST_RECEIPT)) && !(await exists(SUMMARY)), 'Evidence receipt appeared during collection.');
await fs.writeFile(absolute(RUST_RECEIPT), rustBytes, { flag: 'wx' });
await fs.writeFile(absolute(SUMMARY), summaryBytes, { flag: 'wx' });
must(sha(await raw(RUST_RECEIPT)) === sha(rustBytes) && sha(await raw(SUMMARY)) === sha(summaryBytes),
  'New receipt readback mismatch.');
console.log(JSON.stringify({ collector_status: 'sealed', qualification_state: summary.state,
  summary: { file: SUMMARY, sha256: sha(summaryBytes), bytes: summaryBytes.length },
  rust_receipt: { file: RUST_RECEIPT, sha256: sha(rustBytes), bytes: rustBytes.length },
  ordinary: summary.ordinary_steps, rust: summary.rust, managed: {
    core: { status: core.status, passed: core.passed, failed: core.failed, total: core.total, skipped: core.skipped },
    desktop: { status: desktop.status, passed: desktop.passed, failed: desktop.failed, total: desktop.total, skipped: desktop.skipped }
  },
  genuine: { passed: genuinePassed.length, expected: 25, observed_union_fields: actualUnionFields },
  collector_validation_issues: issues, admission_created: false }, null, 2));
