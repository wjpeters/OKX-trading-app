raise SystemExit("Historische fase-1-runner uitgeschakeld: gebruik phase3_acceptance.py met een afzonderlijke testdatabase; draai geen storingstest op de gebruikersdatabase.")

import subprocess,json,urllib.request,urllib.error,time
from pathlib import Path
results=[]
def run(*args):return subprocess.run(args,capture_output=True,text=True,check=True).stdout.strip()
def get(path):
 try:
  with urllib.request.urlopen('http://127.0.0.1:4780'+path,timeout=8) as r:return r.status,json.load(r)
 except urllib.error.HTTPError as e:return e.code,None
 except (urllib.error.URLError,ConnectionError):return 0,None
def wait_ready():
 for _ in range(40):
  code,s=get('/api/status')
  if code==200 and s['database'] and s['public_ws']:return s
  time.sleep(.5)
 raise AssertionError('Runtime did not recover')
def count():return int(run('docker','compose','exec','-T','db','psql','-U','terminal','-d','terminal','-Atc','SELECT count(*) FROM market_cache'))
initial=count();assert initial>0
try:
 run('docker','compose','stop','db')
 code,s=get('/api/status');assert code==200 and not s['database'] and not s['execution_enabled'];results.append('Database outage explicitly reported; execution stays off')
 code,_=get('/api/candles?instrument=BTC-EUR&bar=1m');assert code==503;results.append('Database outage fails candle request without stale success')
finally:
 run('docker','compose','start','db')
wait_ready();assert count()>=initial;results.append('Database recovered and cache rows preserved')
run('docker','compose','restart','terminal');s=wait_ready();assert not s['execution_enabled'] and s['risk_state']=='PAUSE_ENTRIES';assert count()>=initial;results.append('Process restart resubscribes, preserves storage and keeps gate closed')
p=subprocess.run(['docker','run','--rm','-e','EXECUTION_MODE=LIVE','okx-tradingterminal:0.1.0'],capture_output=True,text=True)
assert p.returncode!=0 and 'READ_ONLY' in p.stderr;results.append('LIVE environment override refuses startup before loading credentials')
report={'timestamp_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'checks':results,'cache_rows_before':initial,'cache_rows_after':count(),'status_after':s}
Path('evidence/runtime.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(results,indent=2))
