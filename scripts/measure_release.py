"""Small explicitly scoped measurements, never orders or model calls."""
import json,time,platform,subprocess,urllib.request,concurrent.futures
from pathlib import Path
r=Path(__file__).resolve().parents[1]
def get(path,port=4780):
 with urllib.request.urlopen(f'http://127.0.0.1:{port}'+path,timeout=15) as f:return json.load(f)
def pct(xs):
 xs=sorted(xs)
 return {f'p{p}':xs[round((len(xs)-1)*p/100)] for p in [50,95,99]} if xs else {}
def stats():
 p=subprocess.run(['rtk','proxy','docker','stats','--no-stream','--format','{{json .}}','okx-tradingterminal-terminal-1','okx-tradingterminal-pine-1','okx-tradingterminal-db-1'],capture_output=True,text=True,check=True)
 return [json.loads(x) for x in p.stdout.splitlines()]
start=get('/api/status');mem_before=stats();t=time.perf_counter();samples=[];errors=0
for i in range(30):
 t0=time.perf_counter()
 try:get('/api/status');samples.append((time.perf_counter()-t0)*1000)
 except Exception:errors+=1
 time.sleep(1)
end=get('/api/status');elapsed=time.perf_counter()-t;mem_after=stats()
def probe(_):
 t0=time.perf_counter()
 try:get('/api/status',4781);return ((time.perf_counter()-t0)*1000,True)
 except Exception:return ((time.perf_counter()-t0)*1000,False)
t0=time.perf_counter()
with concurrent.futures.ThreadPoolExecutor(max_workers=12) as pool:burst=list(pool.map(probe,range(120)))
result={'at_ms':int(time.time()*1000),'hardware':subprocess.run(['rtk','proxy','sysctl','-n','machdep.cpu.brand_string'],capture_output=True,text=True).stdout.strip(),'host':platform.platform(),'container_arch':'linux/arm64','instruments':end['instruments'],'active_bots':end['active_bots'],'indicator_load':'10 native indicators per requested candle series; no continuously active indicator bots','scope':'30s public feed observation; HTTP status query normal/burst. Not event-to-intent or exchange latency, not long soak. No orders or Jev calls. Other local containers active.','duration_s':elapsed,'validated_ws_messages':end['validated_messages']-start['validated_messages'],'validated_ws_messages_per_s':(end['validated_messages']-start['validated_messages'])/elapsed,'ws_processing_ms':end['processing_ms'],'http_ms':pct(samples),'http_samples':len(samples),'http_errors':errors,'burst_test_database':{'requests':len(burst),'concurrency':12,'duration_s':time.perf_counter()-t0,'errors':sum(not ok for _,ok in burst),'http_ms':pct([v for v,ok in burst if ok])},'container_stats_before':mem_before,'container_stats_after':mem_after,'paid_jev_calls':0}
(r/'evidence/phase4-benchmark.json').write_text(json.dumps(result,indent=2));print(json.dumps(result))
