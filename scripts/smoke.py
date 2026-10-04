import urllib.request,urllib.error,json,time,platform,subprocess
from pathlib import Path
base='http://127.0.0.1:4780'
def request(path,headers=None,method='GET'):
 req=urllib.request.Request(base+path,headers=headers or {},method=method)
 try:
  with urllib.request.urlopen(req,timeout=15) as r:return r.status,r.read(),dict(r.headers)
 except urllib.error.HTTPError as e:return e.code,e.read(),dict(e.headers)
def get(path):
 code,body,_=request(path);assert code==200,(path,code,body[:100]);return json.loads(body)
checks=[]
def check(name,condition):
 assert condition,name
 checks.append(name)
s=get('/api/status');check('read-only no execution',s['mode']=='READ_ONLY' and not s['execution_enabled']);check('Jev disabled proposals-only',s['jev']['state']=='DISABLED' and s['jev']['authority']=='PROPOSALS_ONLY');check('account unverified',s['account_region']=='UNVERIFIED');check('DB and market connected',s['database'] and s['public_ws'])
m=get('/api/market')['market'];check('12 real instruments and books',len(m['instruments'])==12 and len(m['books'])==12)
coverage=[]
for bar in ['1m','5m','15m','1H','4H','1Dutc']:
 d=get('/api/candles?instrument=BTC-EUR&bar='+bar);check('real candles '+bar,len(d['candles'])==300 and d['gaps']==0 and d['fresh']);check('decimal strings '+bar,isinstance(d['candles'][0]['close'],str));coverage.append({'bar':bar,'count':len(d['candles']),'gaps':d['gaps'],'first_ms':d['candles'][0]['ts'],'last_ms':d['candles'][-1]['ts'],'calculation_ms':d['calculation_ms'],'rest_roundtrip_ms':d['rest_roundtrip_ms']})
 if bar=='15m':cursor=d['candles'][0]['ts']
history=get('/api/candles?instrument=BTC-EUR&bar=15m&after='+str(cursor));check('historical cursor older only',history['historical'] and not history['fresh'] and max(c['ts'] for c in history['candles'])<cursor)
check('order route absent',request('/api/trade/order',method='POST')[0]==404)
check('paper order route absent',request('/api/paper/order',method='POST')[0]==404)
check('read API refuses writes',request('/api/status',method='POST')[0]==405)
check('evil origin blocked',request('/api/status',{'Origin':'https://evil.example'})[0]==403)
check('DNS rebinding host blocked',request('/api/status',{'Host':'evil.example'})[0]==403)
check('unknown instrument rejected',request('/api/candles?instrument=NOT-REAL&bar=1m')[0]==503)
check('Pine cannot reach gateway',request('/api/pine/execute',method='POST')[0]==404)
check('CSP set',"frame-ancestors 'none'" in request('/')[2].get('content-security-policy',''))
samples=[]
for _ in range(50):
 start=time.perf_counter();get('/api/status');samples.append((time.perf_counter()-start)*1000)
samples.sort()
report={'timestamp_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'checks_passed':len(checks),'checks':checks,'coverage':coverage,'status':get('/api/status'),'rest_status_roundtrip_ms':{'p50':samples[24],'p95':samples[47],'p99':samples[49],'samples':50},'host':{'platform':platform.platform(),'cpu':subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip()},'note':'Read-only checks. No order execution, account access, AI call, or strategy benchmark.'}
Path('evidence/smoke.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'checks_passed':len(checks),'coverage':[(x['bar'],x['count']) for x in coverage],'processing_ms':report['status']['processing_ms']},indent=2))
