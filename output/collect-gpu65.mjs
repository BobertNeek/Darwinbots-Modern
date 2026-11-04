import fs from 'node:fs/promises';
import {createHash} from 'node:crypto';
const hash=b=>createHash('sha256').update(b).digest('hex').toUpperCase();
const read=async f=>JSON.parse((await fs.readFile(f,'utf8')).replace(/^\uFEFF/,''));
const pin=async file=>({file,sha256:hash(await fs.readFile(file))});
const must=(v,m)=>{if(!v)throw Error(m)};
const source='73e350297f54e88a7a690c435517ebb13615f953';
const run='docs/parity/evidence/batch-2026-10-14/gpu-73e350297f54/run-20261019T100108423-b5b797d347204ba68665776596a19da1';
const summary=await read(run+'/summary.json'),outer=await read('output/gpu65-observed-exit.json');
must(summary.source_head===source&&summary.status==='completed'&&summary.planned_groups===6&&summary.outcome_count===6&&summary.groups.length===6&&!summary.guard_failure&&!summary.failed_groups.length,'GPU plan incomplete');
must(outer.source_head===source&&outer.pid===41944&&outer.exit_code===0&&outer.exit_code_observed===true&&outer.yield_reason===null,'Actual outer exit not successful');
must(summary.cpu_ready_sha256==='84799C3A07568BB00C8B4B164D43B46466D1BD5B4BABA0E61319D490BBF15486'&&hash(await fs.readFile(summary.cpu_ready))===summary.cpu_ready_sha256,'CPU admission changed');
const names=['physical-library','physical-cli','physical-integration','physical-native-continuation','portable-gpu','backend-switches'];
const counts=[7,1,3,1,14,1],harnesses=[1,1,3,1,66,66],groups=[];
for(const [i,row] of summary.groups.entries()){
  must(row.source_head===source&&row.task===names[i]&&row.status==='passed'&&row.exit_code===0&&!row.timed_out&&row.interrupted_by_input===false&&row.stop_reason==='process_exit','Failed or interrupted group');
  must(!row.guard_error&&!row.execution_error&&!row.binary_error&&!row.error&&['cleanup_errors','log_errors','input_observation_errors','state_errors'].every(k=>Array.isArray(row[k])&&!row[k].length),'Group guard/custody error');
  must(row.input_tick_before===row.input_tick_after&&row.idle_before_ms>=300000&&row.priority==='BelowNormal'&&row.rayon_threads===1,'Execution guards missing');
  const sealed=await read(run+'/'+row.task+'.json');
  must(JSON.stringify(sealed)===JSON.stringify(row),'Group summary differs from sealed receipt');
  let text='';
  for(const kind of ['stdout','stderr']){
    const raw=await fs.readFile(row[kind]);
    must(hash(raw)===row[kind+'_sha256']&&row[kind+'_capture'].writer_quiescent===true&&row[kind+'_capture'].status==='captured'&&row[kind+'_capture'].bytes===raw.length,'Log hash/custody failed');
    text+='\n'+raw.toString('utf8');
  }
  const rows=[...text.matchAll(/test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;/g)];
  const passed=rows.reduce((n,r)=>n+Number(r[1]),0);
  must(rows.length===harnesses[i]&&rows.length===row.terminal_harness_summary_count&&row.terminal_harness_summary_present===true&&rows.every(r=>Number(r[2])===0)&&passed===row.passed_cases&&passed===counts[i],'Terminal counts disagree');
  if(i<4)must(text.includes('NVIDIA GeForce RTX 3050')&&text.includes('Vulkan')&&text.includes('DiscreteGpu'),'Physical adapter evidence missing');
  if(i===3)must(text.includes('checkpoint cycle=10281')&&/physical_gpu_original_world_long_continuation_matches_cpu_and_repeated_save_resume/.test(text),'Strict 10k continuation missing');
  groups.push({task:row.task,status:'passed',passed_cases:passed,failed_cases:0,ignored_cases:rows.reduce((n,r)=>n+Number(r[3]),0),terminal_summaries:rows.length,elapsed_process_seconds:row.elapsed_process_seconds});
}
const evidence=await Promise.all((await fs.readdir(run)).filter(f=>f!=='STOP').map(f=>pin(run+'/'+f)));
for(const f of ['output/gpu65-start.json','output/gpu65-observed-exit.json','output/gpu65-alife-release-observation.json'])evidence.push(await pin(f));
const result={schema_version:1,source_head:source,collected_utc:new Date().toISOString(),status:'completed_passed',all_six_groups_passed:true,strict_original_world_10000_cycle_gpu_cpu_and_repeated_save_resume_passed:true,physical_adapter:{name:'NVIDIA GeForce RTX 3050',backend:'Vulkan',device_type:'DiscreteGpu'},groups,boundary:'Group selections can overlap; these counts are not summed as unique tests. Capacity and native visible walkthrough remain separate. Original source992 failed receipts remain unchanged.',evidence};
await fs.writeFile('output/gpu65-collected.json',JSON.stringify(result,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({receipt:await pin('output/gpu65-collected.json'),groups},null,2));
