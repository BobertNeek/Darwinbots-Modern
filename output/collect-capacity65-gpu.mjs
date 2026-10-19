import fs from 'node:fs/promises';
import {createHash} from 'node:crypto';
const sha=b=>createHash('sha256').update(b).digest('hex').toUpperCase();
const read=async f=>JSON.parse((await fs.readFile(f,'utf8')).replace(/^\uFEFF/,''));
const pin=async file=>({file,sha256:sha(await fs.readFile(file))});
const must=(v,s)=>{if(!v)throw Error(s)};
const proof='docs/parity/evidence/batch-2026-10-14/capacity-gpu-controlled64-73e350297f54-20261019T120003941-0b70a90e';
const source='73e350297f54e88a7a690c435517ebb13615f953';
const cli='571D35B61F7F86816C1A45240F1FB44E604023EE99C7BC3AADE88FE7488CE89F';
const summary=await read(proof+'/summary.json'),exit=await read('output/capacity65-controlled-gpu-retry1-observed-exit.json');
must(summary.source_commit===source&&summary.status==='passed'&&summary.backend_requested==='Gpu'&&summary.tasks.length===4&&!summary.failed_or_insufficient_tasks.length,'Capacity plan incomplete');
must(exit.source_head===source&&exit.pid===48360&&exit.exit_code===0&&exit.exit_code_observed===true&&exit.yield_reason===null,'Actual outer exit not successful');
must(summary.cli_sha256===cli&&(await pin('target/release/darwinbots-cli.exe')).sha256===cli,'Capacity CLI changed');
must((await pin('output/capacity65-cli-qualified.json')).sha256==='09B68FE0CEB9FF17E3038F21E0AB71CDDA4732C4E6E1EDC3341D87248E12A426','Fresh CLI qualification changed');
must(summary.cpu_ready_sha256==='84799C3A07568BB00C8B4B164D43B46466D1BD5B4BABA0E61319D490BBF15486'&&(await pin(summary.cpu_ready)).sha256===summary.cpu_ready_sha256,'CPU admission changed');
const populations=[1000,4000,8000,32000],ticks=[2000,300,100,30];
for(let i=0;i<4;i++){
  const row=summary.tasks[i],r=row.report,a=row.actual_gpu_adapter;
  must(row.source_commit===source&&row.cli_sha256===cli&&row.task==='gpu-'+populations[i]&&row.status==='passed'&&row.held_population===true&&row.duration_status==='sustained'&&row.exit_code===0&&!row.error,'Task failed');
  must(row.actual_backend==='Gpu'&&row.adapter_status==='verified_physical_gpu'&&row.gpu_adapter_field_present===true&&a?.name==='NVIDIA GeForce RTX 3050'&&a.backend==='Vulkan'&&a.device_type==='DiscreteGpu'&&JSON.stringify(a)===JSON.stringify(r.gpu_adapter),'Physical adapter invalid');
  must(Number.isFinite(r.elapsed_seconds)&&r.elapsed_seconds>=60&&r.ticks===ticks[i]&&r.tick===ticks[i]&&r.intentional_movement_events>0,'Duration/tick/movement contract failed');
  for(const f of ['starting_population','candidate_starting_population','final_population','candidate_final_population','population'])must(r[f]===populations[i],'Population changed');
  for(const f of ['births','deaths','candidate_births','candidate_deaths'])must(r[f]===0,'Births or deaths');
  must(row.idle_before.idle_seconds>=300&&row.idle_before.last_input_tick===row.idle_after.last_input_tick&&row.rayon_num_threads===1&&row.priority==='BelowNormal','Execution guards missing');
  must(JSON.stringify(await read(proof+'/'+row.stdout))===JSON.stringify(r),'Raw report differs');
  must(JSON.stringify(await read(proof+'/'+row.task+'.receipt.json'))===JSON.stringify(row),'Per-task receipt differs');
}
must((await pin('output/qualify-capacity-controlled64.ps1')).sha256==='1F6FCCD1B9B52C8B118826F2FBA19D42811435A3096E89665948B7FC5DC7A708','Runner changed');
must((await pin('output/capacity64-controlled-mover.txt')).sha256==='4DCEDDD9F57C07036071A2E939EB94968E345686CA094718F5360C2AF0777149','Fixture changed');
const receipt={schema_version:1,source_head:source,sealed_utc:new Date().toISOString(),status:'passed',backend:'Gpu',cli_sha256:cli,physical_adapter:summary.tasks[0].actual_gpu_adapter,
  scope:'Sustained controlled-mover headless whole-probe throughput with one worker. No rendered-frame, ecology or speedup claim; prior-source CPU timings are not a current-source performance comparison.',
  tasks:summary.tasks.map(r=>({population:r.requested_population,ticks:r.requested_ticks,elapsed_seconds:r.report.elapsed_seconds,ticks_per_second:r.whole_probe_ticks_per_second,population_held:true,births:0,deaths:0,movement_events:r.report.intentional_movement_events})),
  evidence:await Promise.all([...(await fs.readdir(proof)).map(f=>proof+'/'+f),'output/capacity65-controlled-gpu-retry1-observed-exit.json','output/capacity65-controlled-gpu-retry1-start.json','output/capacity65-cli-qualified.json','output/capacity65-cli-restoration.json','output/capacity65-controlled-gpu-alife-release-observation.json'].map(pin))};
await fs.writeFile('output/capacity65-controlled-gpu-collected.json',JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({receipt:await pin('output/capacity65-controlled-gpu-collected.json'),tasks:receipt.tasks},null,2));
