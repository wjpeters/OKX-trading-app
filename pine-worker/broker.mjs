import { readdir, readFile, writeFile, rename, unlink, stat } from 'node:fs/promises';
import { spawn } from 'node:child_process';
const dir='/jobs';
async function execute(input){return new Promise(resolve=>{
 const p=spawn(process.execPath,['--max-old-space-size=128','--permission','--allow-fs-read=/app','/app/run.mjs'],{cwd:'/app',env:{NODE_ENV:'production'},stdio:['pipe','pipe','pipe']});
 let out='',done=false;const finish=v=>{if(done)return;done=true;clearTimeout(timer);resolve(v)};
 const timer=setTimeout(()=>{p.kill('SIGKILL');finish({ok:false,diagnostics:[{line:1,message:'Script overschrijdt looptijd van 3 seconden.'}],plots:[],intents:[]})},3000);
 p.stdout.on('data',c=>{out+=c;if(out.length>1_000_000){p.kill('SIGKILL');out='';finish({ok:false,diagnostics:[{line:1,message:'Scriptuitvoer te groot.'}],plots:[],intents:[]})}});
 p.stderr.resume();p.on('error',()=>finish({ok:false,diagnostics:[{line:1,message:'Geïsoleerde runtime niet beschikbaar.'}]}));
 p.on('close',()=>{try{finish(JSON.parse(out))}catch{finish({ok:false,diagnostics:[{line:1,message:'Runtime beëindigd: geheugenlimiet of ongeldige uitvoer.'}],plots:[],intents:[]})}});
 p.stdin.on('error',()=>{});p.stdin.end(input);
})}
for(;;){
 for(const f of (await readdir(dir)).filter(x=>/^[a-f0-9]{32}\.job$/.test(x)).slice(0,4)){
  const path=dir+'/'+f;try{const s=await stat(path);if(s.size>1_000_000||Date.now()-s.mtimeMs>15000){await unlink(path);continue;}
   const input=await readFile(path,'utf8');await unlink(path);const result=await execute(input);
   await writeFile(path+'.tmp',JSON.stringify(result),{mode:0o600});await rename(path+'.tmp',path+'.result');
  }catch{/* deleted by timeout or another broker; never access paths from a script */}
 }
 for(const f of await readdir(dir)){if(!/^[a-f0-9]{32}\.job(\.result|\.tmp)?$/.test(f))continue;try{if(Date.now()-(await stat(dir+'/'+f)).mtimeMs>30000)await unlink(dir+'/'+f)}catch{}}
 await new Promise(r=>setTimeout(r,100));
}
