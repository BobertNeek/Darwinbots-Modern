import fs from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
const dest='output/hosted-main-73e3502',runId=39220119048,source='73e350297f54e88a7a690c435517ebb13615f953';
const hash=b=>createHash('sha256').update(b).digest('hex').toUpperCase();
await fs.mkdir(dest,{recursive:true});
async function cached(name,args){const file=dest+'/'+name;try{return await fs.readFile(file);}catch(e){if(e.code!=='ENOENT')throw e;}
 const raw=execFileSync('gh.exe',args,{windowsHide:true,maxBuffer:16*1024*1024});await fs.writeFile(file,raw,{flag:'wx'});return raw;}
const run=JSON.parse(await cached('run.json',['api','repos/BobertNeek/Darwinbots-Modern/actions/runs/'+runId]));
const jobs=JSON.parse(await cached('jobs.json',['api','repos/BobertNeek/Darwinbots-Modern/actions/runs/'+runId+'/jobs?per_page=100'])).jobs;
if(run.head_sha!==source||run.status!=='completed'||jobs.length!==4||jobs.some(j=>j.status!=='completed'))throw Error('Current hosted run not terminal');
const platforms=[];
function managed(text,name,expected){
 const assembly=name==='core'?'Darwinbots.Desktop.Core.Tests.dll':'Darwinbots.Desktop.Tests.dll';
 const identified=text.split('\n').filter(l=>l.includes(assembly)).join('\n');
 const compact=[...identified.matchAll(/Passed!\s*-\s*Failed:\s*(\d+),\s*Passed:\s*(\d+),\s*Skipped:\s*(\d+),\s*Total:\s*(\d+)/g)];
 if(compact.length===1){const r=compact[0];return {passed:+r[2],failed:+r[1],skipped:+r[3],total:+r[4]};}
 const total=text.match(/Total tests:\s*(\d+)/),passed=text.match(/\bPassed:\s*(\d+)/);
 if(total&&passed&&text.includes('Test Run Successful.'))return {passed:+passed[1],failed:0,skipped:0,total:+total[1]};
 console.log(JSON.stringify({unparsed:name,expected,summary_lines:text.split('\n').filter(l=>/Passed|Failed|Skipped|Total tests|Test Run/.test(l)).slice(-12)},null,2));throw Error('Managed result not parsed');
}
for(const job of jobs){
 const raw=await cached('job-'+job.id+'.log',['run','view',String(runId),'--repo','BobertNeek/Darwinbots-Modern','--job',String(job.id),'--log']);
 const text=raw.toString('utf8').replace(/\x1B\[[0-?]*[ -/]*[@-~]/g,''),lines=text.split(/\r?\n/);
 const step=name=>{
  const stage=job.steps.find(s=>s.name===name);if(!stage)throw Error('Missing named job step: '+name);
  const begin=Date.parse(stage.started_at),end=Date.parse(stage.completed_at)+1000;
  return lines.filter(l=>{const stamp=l.match(/\d{4}-\d{2}-\d{2}T[0-9:.]+Z/);return stamp&&Date.parse(stamp[0])>=begin&&Date.parse(stamp[0])<end;}).join('\n');
 };
 const rustRows=[...step('Test Rust workspace').matchAll(/test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)];
 const rust={targets:rustRows.length,passed:rustRows.reduce((n,r)=>n+Number(r[1]),0),failed:rustRows.reduce((n,r)=>n+Number(r[2]),0),ignored:rustRows.reduce((n,r)=>n+Number(r[3]),0)};
 const core=managed(step('Test desktop core'),'core',314),desktop=managed(step('Test rendered desktop controls'),'desktop',215);
 const smoke=step('Smoke test published desktop package').split('\n').map(l=>{try{return JSON.parse(l.slice(l.indexOf('{')));}catch{return null;}}).find(v=>v?.status==='passed'&&v?.backend==='CPU');
 platforms.push({id:job.id,name:job.name,conclusion:job.conclusion,rust,core,desktop,smoke:smoke??null});
}
const passed=run.conclusion==='success'&&platforms.every(p=>p.conclusion==='success'&&p.rust.targets===68&&p.rust.passed===1064&&p.rust.failed===0&&p.rust.ignored===12&&
 p.core.passed===314&&p.core.failed===0&&p.core.skipped===0&&p.core.total===314&&p.desktop.passed===215&&p.desktop.failed===0&&p.desktop.skipped===0&&p.desktop.total===215&&
 p.smoke?.ticks===13&&p.smoke?.movement===true&&p.smoke?.save_resume===true&&p.smoke?.bundled_bots===48);
const evidence=[];for(const file of (await fs.readdir(dest)).filter(f=>f!=='summary.json')){const raw=await fs.readFile(dest+'/'+file);evidence.push({file:dest+'/'+file,sha256:hash(raw),bytes:raw.length});}
const summary={schema_version:1,source_head:source,run_id:runId,sealed_utc:new Date().toISOString(),all_four_jobs_terminal:true,qualification_passed:passed,platforms,evidence,
 boundary:'Actual current-source terminal job logs, four platforms. Ignored hardware tests are not physical GPU evidence. Hosted package asset byte hashes not separately recollected; smoke verifies48 count. Fresh local65 verifies all48 exact hashes.'};
await fs.writeFile(dest+'/summary.json',JSON.stringify(summary,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({qualification_passed:passed,platforms},null,2));if(!passed)process.exitCode=1;
