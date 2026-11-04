import fs from 'node:fs/promises';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
const repo = await fs.realpath(process.cwd());
const head = '73e350297f54e88a7a690c435517ebb13615f953';
const proof = 'docs/parity/evidence/batch-2026-10-14/native-roundtrip-pass65/';
const sourceFile = 'output/native-roundtrip-pass65-source.json';
const markerFile = 'docs/parity/evidence/batch-2026-10-14/gpu-73e350297f54/cpu-ready.json';
const sha = bytes => createHash('sha256').update(bytes).digest('hex').toUpperCase();
const must = (v, message) => { if (!v) throw Error(message); };
const key = file => path.resolve(repo, file).toLowerCase();
async function bytes(file) {
  must(!path.isAbsolute(file), 'Relative evidence path required');
  const real = await fs.realpath(path.resolve(repo, file));
  must(real.toLowerCase().startsWith(repo.toLowerCase() + path.sep), 'Evidence escapes checkout: ' + file);
  return fs.readFile(real);
}
async function pin(file) { const raw = await bytes(file); return { file, sha256: sha(raw), bytes: raw.length }; }
async function json(file) { return JSON.parse((await bytes(file)).toString('utf8').replace(/^\uFEFF/, '')); }
const source = await json(sourceFile);
const sourcePin = await pin(sourceFile);
must(sourcePin.sha256 === 'C50D046FA1C5D160C4A83639C7A6E2EA9E6ECB577315E7A70867BFCA4342B037', 'Manifest changed');
must(execFileSync('git.exe', ['rev-parse', 'HEAD'], {encoding:'utf8', windowsHide:true}).trim() === head, 'HEAD changed');
const sourceKeys = new Set();
for (const p of source.files) {
  must(!sourceKeys.has(key(p.file)), 'Duplicate source'); sourceKeys.add(key(p.file));
  must((await pin(p.file)).sha256 === p.sha256, 'Source changed: ' + p.file);
}
must(sourceKeys.size === 688 && source.parent_head === head, 'Wrong source census');
const audit = await json(proof + 'qualification-summary.json');
const rust = await json(proof + 'rust-target-results.json');
const hosted = await json('output/hosted-main-73e3502/summary.json');
const results = await json(proof + 'results.json');
must(audit.state === 'completed' && audit.source_head === head && audit.all_ordinary_outcomes_passed === true &&
  audit.ordinary_steps.total === 94 && audit.ordinary_steps.passed === 94 && !audit.collector_validation_issues.length &&
  audit.runner_completion.exit_code === 0 && audit.runner_completion.exit_code_observed === true, 'CPU audit incomplete');
must(audit.genuine.passed === 25 && audit.genuine.matched_controls_compared_fields === 55849 &&
  audit.genuine.differences_parseable_controls === 0 && audit.raw_zero.passed === true && audit.raw_zero.exact_raw_bytes === true,
  'Original controls incomplete');
must(hosted.source_head === head && hosted.qualification_passed === true && hosted.all_four_jobs_terminal === true &&
  hosted.platforms.length === 4 && hosted.platforms.every(p => p.conclusion === 'success' && p.core.passed === 314 &&
  p.core.failed === 0 && p.desktop.passed === 215 && p.desktop.failed === 0), 'Current hosted checks incomplete');
must(rust.state === 'passed' && rust.full_workspace_passed === true && rust.total_distinct_targets === 68, 'Rust incomplete');
const result = name => { const rows = results.filter(r => r.step === name); must(rows.length === 1 && rows[0].exit_code === 0, 'Missing successful step ' + name); return rows[0]; };
const evidence = new Map();
async function add(p) {
  const actual = await pin(p.file);
  must(actual.sha256 === p.sha256.toUpperCase(), 'Changed evidence: ' + p.file);
  if (sourceKeys.has(key(p.file))) return;
  const prior = evidence.get(key(p.file));
  must(!prior || prior.sha256 === actual.sha256, 'Conflicting evidence');
  evidence.set(key(p.file), actual);
}
for (const p of [...audit.evidence, ...hosted.evidence, sourcePin, await pin(proof + 'qualification-summary.json'),
  await pin(proof + 'rust-target-results.json'), await pin('output/hosted-main-73e3502/summary.json')]) await add(p);
const rustCpu = {...rust.counts, expected_distinct_targets:rust.expected_distinct_targets,
  total_distinct_targets:rust.total_distinct_targets, fresh_full_workspace_invocation:true, hardware_filtered:true};
const managed = name => {
  const row = audit.managed[name];
  must(row.status === 'executed' && row.result_rows_match_counters === true && row.failed === 0 && row.skipped === 0,
    'Unexecuted managed tests');
  return {source_head:head, fresh_invocation:true, passed:row.passed, failed:row.failed, skipped:row.skipped, total:row.total,
    trx:{file:row.trx, sha256:row.trx_sha256}, ...(name === 'desktop' ? {exact_package_native_dll:audit.package.desktop_native_dll_qualified} : {})};
};
const core = managed('core'), desktop = managed('desktop');
const summary = {schema_version:1, batch:65, sealed_utc:new Date().toISOString(), source_head:head, source_manifest:sourcePin,
  all_ordinary_local_outcomes_collected:true, local_cpu_passed:true, source_inputs_unchanged:true,
  format_exit_code:result('format-check').exit_code, strict_clippy_exit_code:result('strict-clippy').exit_code,
  rust_cpu:rustCpu, core, desktop,
  package:{fresh_invocation:true, publish_exit_code:result('windows-publish').exit_code, assets:audit.package.assets,
    asset_mismatches:audit.package.asset_mismatches.length, smoke:audit.package.smoke, native_dll_identity:audit.package.dll_identity_after},
  ordinary_steps:audit.ordinary_steps, genuine_controls:{passed:25, compared_fields:55849, differences:0},
  raw_zero:audit.raw_zero, hosted_all_four_passed:true, audit:await pin(proof + 'qualification-summary.json'),
  evidence:[...evidence.values()].sort((a,b)=>a.file.localeCompare(b.file)),
  boundary:'Fresh exact-source CPU admission only; physical GPU, capacity and visible native walkthrough remain separate.'};
await fs.writeFile(path.resolve(repo, proof + 'summary.json'), JSON.stringify(summary,null,2)+'\n', {flag:'wx'});
const summaryPin = await pin(proof + 'summary.json'); await add(summaryPin);
const marker = {schema_version:5, batch:65, source_head:head, sealed_utc:new Date().toISOString(),
  qualification_method:'fresh_full_cpu_workspace_and_core_desktop_package',
  cpu_workspace_passed:true, core_passed:true, desktop_passed:true, package_passed:true, source_qualification_passed:true,
  core_fresh_invocation:true, desktop_fresh_invocation:true, package_fresh_invocation:true, source_qualification_fresh_invocation:true,
  source_file_count:source.files.length, source_files:source.files, source_manifest:sourcePin,
  rust_result:await pin(proof+'rust-target-results.json'), summary:summaryPin,
  genuine_movement:await pin(proof+'genuine-movement-comparisons.json'), rust_cpu:rustCpu, core, desktop,
  evidence:[...evidence.values()].sort((a,b)=>a.file.localeCompare(b.file))};
await fs.mkdir(path.dirname(path.resolve(repo,markerFile)),{recursive:true});
await fs.writeFile(path.resolve(repo,markerFile),JSON.stringify(marker,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({summary:summaryPin, marker:await pin(markerFile), sources:sourceKeys.size, evidence:evidence.size},null,2));
