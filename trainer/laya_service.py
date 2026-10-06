"""Fixed local Laya checkpoint, small causal market payloads, no trading tools."""
import os,json,time,hashlib,threading,subprocess
from pathlib import Path
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
PIN='7b928d828b7b0e022f929d9bd2e44165aa270148'
EXPECTED={'model.safetensors':'891102d372688fc2a094dac56a384bc537b87c63f21f9f3dac0be2b7cbc8d86c','rl_agent_config.json':'ae287b56bbcf5f8c4f4541ae9dfd00c914c4c48b940b8398c3058af37ba92bbd','encoder/config.json':'bf3ab80598fdccf414855a2ce80f22859e4492d06ca8a62ddd1cfb63972f8979','tokenizer/tokenizer.json':'6c8aaa9a542084f2457eab775d4eeb51f92a70c0fd9de28d5edb0ddec3c08d30','tokenizer/tokenizer_config.json':'50044de60daaa73df97d262e15a40d4faf0160e7d742df64b377877a1320dd12'}
STATUS={'ready':False,'checkpoint':PIN,'reason':'Checkpoint wordt lokaal geladen'}
AGENT=None
LOCK=threading.Lock()
QUESTIONS={'regime':{'type':'choice','instructions':'Classify the current market regime.','criteria':{'trend':'Persistent directional momentum.','range':'Oscillating prices, no persistent direction.','volatile':'Unpredictable changes, high volatility.'}}}
def load():
 global AGENT
 try:
  import torch
  torch.set_num_threads(2);torch.set_num_interop_threads(1)
  from huggingface_hub import snapshot_download
  from laya import Agent
  snapshot=snapshot_download('convaiinnovations/laya',revision=PIN,cache_dir=os.environ.get('LAYA_MODEL_DIR','/models'),allow_patterns=['rl_agent_config.json','model.safetensors','encoder/config.json','tokenizer/*'])
  hashes={}
  for rel in ('model.safetensors','rl_agent_config.json','encoder/config.json','tokenizer/tokenizer.json','tokenizer/tokenizer_config.json'):
   h=hashlib.sha256()
   with open(Path(snapshot)/rel,'rb')as f:
    while b:=f.read(1024*1024):h.update(b)
   hashes[rel]=h.hexdigest()
  device='mps' if torch.backends.mps.is_available() else 'cpu'
  if device=='mps':torch.mps.set_per_process_memory_fraction(min(1.,2.3*1024**3/torch.mps.recommended_max_memory()))
  if hashes!=EXPECTED:raise RuntimeError('Checkpoint file hash mismatch')
  AGENT=Agent(snapshot,device=device,backend='eager',expected_sha256=EXPECTED)
  device=AGENT.device.type
  AGENT.amp_enabled=False
  # Warm before reporting readiness. No generative text or order capabilities.
  sample=state({'features':[.001,.003,.004,.001,.5,1.1,.3,.004,0,0,0,.001,0,0,0,0]},'BTC-EUR','1m')
  for batch in (1,8):AGENT.predict_batch([sample]*batch,QUESTIONS)
  STATUS.update(ready=True,reason=None,hashes=hashes,model='Laya english',device=device,feature_prompt='market-regime-v1',runtime=f'torch2.8-eager-{device}-fp32-v1',paid_calls=0)
 except Exception as e:STATUS.update(ready=False,reason=str(e))
def state(row,instrument,bar):
 f=row['features']
 return f'{instrument} {bar}; r1={f[0]*100:.2f}%; r5={f[1]*100:.2f}%; vol={f[2]*100:.2f}%; trend={f[3]*100:.2f}%; RSI={f[4]*100:.0f}; volume={f[5]:.1f}; z={f[6]:.1f}; ATR={f[7]*100:.2f}%.'
def memory_guard():
 # MPS allocations have their own hard 2.3 GiB ceiling. RSS + driver are monitored
 # every 500 ms; crossing 2.5 GiB terminates this credential-free service.
 while True:
  time.sleep(.5)
  if not STATUS['ready']:continue
  try:
   import torch
   rss=int(subprocess.check_output(['/bin/ps','-o','rss=','-p',str(os.getpid())]))*1024 if __import__('sys').platform=='darwin'else int(Path('/proc/self/statm').read_text().split()[1])*os.sysconf('SC_PAGE_SIZE')
   gpu=torch.mps.driver_allocated_memory()if AGENT.device.type=='mps'else 0
   STATUS['memory_mb']=(rss+gpu)/1024**2;STATUS['memory_limit_mb']=2560
   if rss+gpu>2.5*1024**3:print('Laya memory budget exceeded',flush=True);os._exit(137)
  except (OSError,ValueError):STATUS['memory_monitor']='unavailable'
class Handler(BaseHTTPRequestHandler):
 def log_message(self,*args):pass
 def respond(self,code,value):
  data=json.dumps(value,allow_nan=False).encode();self.send_response(code);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data)
 def do_GET(self):self.respond(200,STATUS if self.path=='/health'else{'error':'unknown route'})
 def do_POST(self):
  if self.path!='/regime':return self.respond(404,{'error':'unknown route'})
  if not STATUS['ready']:return self.respond(503,STATUS)
  try:
   n=int(self.headers.get('Content-Length','0'))
   if not 0<n<=128000:raise ValueError('Body too large')
   req=json.loads(self.rfile.read(n));rows=req['rows']
   if req['checkpoint']!=PIN or not 1<=len(rows)<=8:raise ValueError('Checkpoint or batch invalid')
   texts=[]
   for row in rows:
    if len(row['features'])!=16 or any(not isinstance(x,(int,float))or not __import__('math').isfinite(x)for x in row['features']):raise ValueError('Non-finite feature')
    texts.append(state(row,req['instrument'],req['bar']))
   started=time.monotonic()
   with LOCK:results=AGENT.predict_batch(texts,QUESTIONS)
   output=[]
   for row,result in zip(rows,results):
    answer=result['answers']['regime'];probs=answer.get('probabilities',answer.get('probs'))
    if isinstance(probs,list):probs=dict(zip(('trend','range','volatile'),probs))
    if not isinstance(probs,dict):raise ValueError('Laya returned no regime probability distribution')
    vals=[float(probs[k])for k in ('trend','range','volatile')];vals.append(float(answer.get('answer_confidence',max(vals))))
    output.append({'ts':row['ts'],'values':vals})
   self.respond(200,{'checkpoint':PIN,'rows':output,'duration_ms':(time.monotonic()-started)*1000,'hashes':STATUS['hashes'],'prompt_version':'market-regime-v1','runtime':STATUS['runtime']})
  except Exception as e:self.respond(422,{'error':str(e),'checkpoint':PIN})
if __name__=='__main__':
 threading.Thread(target=load,daemon=True).start();threading.Thread(target=memory_guard,daemon=True).start();ThreadingHTTPServer((os.environ.get('LAYA_BIND','0.0.0.0'),8092),Handler).serve_forever()
