#!/usr/bin/env python3
"""Destructive tests ONLY in a freshly created, recorded test database/container.
Never initializes, resets or trades the user's paper account on port 4780.
Uses public OKX data; no exchange API credentials or exchange order requests.
"""
import json,sys,time,subprocess,uuid,urllib.request,urllib.error,statistics
from pathlib import Path
from decimal import Decimal,ROUND_DOWN
from concurrent.futures import ThreadPoolExecutor
ROOT=Path(__file__).resolve().parent.parent
MANIFEST=ROOT/'.local/phase2-test.json'
EVIDENCE=ROOT/'evidence/phase2-integration.json'
BASE='http://127.0.0.1:4781'
DB_CONTAINER='okx-tradingterminal-db-1'
IMAGE='okx-tradingterminal:0.2.0'

def run(*args,check=True):
 r=subprocess.run(['rtk','proxy',*args],capture_output=True,text=True,cwd=ROOT)
 if check and r.returncode: raise RuntimeError(' '.join(args[:4])+': '+r.stderr[:1000])
 return r

def sql(db,statement):return run('docker','exec',DB_CONTAINER,'psql','-U','terminal','-d',db,'-At','-v','ON_ERROR_STOP=1','-c',statement).stdout.strip()

def api(path,body=None,headers=None):
 h={'Origin':BASE,'X-Terminal-Request':'paper-v2','Content-Type':'application/json'}
 if headers is not None:h=headers
 req=urllib.request.Request(BASE+path,data=None if body is None else json.dumps(body).encode(),headers=h)
 start=time.perf_counter()
 try:
  with urllib.request.urlopen(req,timeout=15) as r:raw=r.read();code=r.status
 except urllib.error.HTTPError as e:raw=e.read();code=e.code
 try:v=json.loads(raw)
 except Exception:v={'raw':raw.decode()[:200]}
 return code,v,(time.perf_counter()-start)*1000

def wait(predicate,seconds=25):
 end=time.monotonic()+seconds;last=None
 while time.monotonic()<end:
  try:
   last=predicate()
   if last:return last
  except Exception as e:last=str(e)
  time.sleep(.3)
 raise AssertionError(f'Wait timed out: {last}')

def state():
 c,v,_=api('/api/paper');assert c==200,v;return v

def provision():
 if MANIFEST.exists():raise RuntimeError('Testmanifest bestaat; eerst de opgenomen testomgeving opruimen')
 suffix=uuid.uuid4().hex[:10];db='okx_phase2_test_'+suffix;name='okx-phase2-test-'+suffix
 sql('postgres',f'CREATE DATABASE {db}')
 m={'database':db,'container':name,'second':name+'-second','created':time.time()};MANIFEST.write_text(json.dumps(m))
 create(name,db,'4781')
 print(json.dumps(m),flush=True)
 wait(lambda:api('/api/ready')[0]==200,45)
 print('Testomgeving gereed op '+BASE,flush=True)

def create(name,db,port=None):
 args=['docker','create','--name',name,'--network','okx-tradingterminal_market','--read-only','--cap-drop=ALL','--security-opt=no-new-privileges:true','--memory=512m','--cpus=2','-e','EXECUTION_MODE=PAPER_LOCAL','-e','DATABASE_HOST='+DB_CONTAINER,'-e','DATABASE_NAME='+db,'-e','DATABASE_PASSWORD_FILE=/run/secrets/db_password','-e','RUST_LOG=warn','-v',str(ROOT/'.local/db_password')+':/run/secrets/db_password:ro']
 if port:args+=['-p','127.0.0.1:'+port+':8080']
 run(*args,IMAGE)
 run('docker','network','connect','okx-tradingterminal_storage',name)
 run('docker','start',name)

def check():
 m=json.loads(MANIFEST.read_text());db=m['database'];name=m['container'];checks=[];timings=[]
 def ok(label,condition,detail=None):
  checks.append({'check':label,'passed':bool(condition),'detail':detail})
  EVIDENCE.write_text(json.dumps({'at':time.time(),'scope':'isolated local test database; real public OKX data','checks':checks,'durability_ms':timings},indent=2))
  print(('PASS ' if condition else 'FAIL ')+label,flush=True)
  assert condition,detail
 s=wait(lambda:(lambda x:x if x['worker_alive'] else None)(state()))
 ok('User account untouched',json.loads(run('curl','-fsS','http://127.0.0.1:4780/api/paper').stdout)['account'] is None)
 ok('Fresh test database',s['account'] is None)
 for label,headers in [('Missing Origin',{'Content-Type':'application/json','X-Terminal-Request':'paper-v2'}),('Cross Origin',{'Origin':'https://evil.example','Content-Type':'application/json','X-Terminal-Request':'paper-v2'}),('Missing custom header',{'Origin':BASE,'Content-Type':'application/json'})]:
  ok(label+' blocked',api('/api/paper/setup',{},headers)[0]==403)
 a=dict(s['presets']['account']);manual=dict(s['presets']['manual'])
 for l,cap in [(a,'1000'),(manual,'500')]:l.update(capital=cap,trade_risk='100',daily_loss=cap,total_loss=cap,drawdown=cap,position=cap,orders=40,orders_per_minute=60,spread_bps='200',slippage_bps='100',data_age_ms=15000)
 a['position']='600'
 body={'account':a,'manual':manual,'confirmed':True}
 ok('Unconfirmed setup rejected',api('/api/paper/setup',{**body,'confirmed':False})[0]==422)
 ok('Explicit paper setup succeeds',api('/api/paper/setup',body)[0]==200)
 ok('Existing balance cannot be reset',api('/api/paper/setup',body)[0]==422)
 ok('Untrusted model fields rejected',api('/api/paper/preview',{'risk_approved':True,'model':'jev'})[0]==422)
 def market():
  c,v,_=api('/api/market');assert c==200
  meta=next(i for i in v['market']['instruments'] if i['instId']=='BTC-EUR');book=v['market']['books']['BTC-EUR'];return meta,book
 meta,book=wait(lambda:market())
 tick=Decimal(meta['tickSz']);lot=Decimal(meta['lotSz'])
 def floor(x,step):return (x/step).to_integral_value(rounding=ROUND_DOWN)*step
 def intent(tag,kind='LIMIT',eur='350',bot='manual'):
  _,b=market();ask=Decimal(b['asks'][0][0]);bid=Decimal(b['bids'][0][0]);limit=floor(ask*Decimal('.98'),tick)
  qty=floor(Decimal(eur)/(limit if kind=='LIMIT' else ask*Decimal('1.011')),lot)
  return {'client_id':tag,'bot_id':bot,'instrument':'BTC-EUR','side':'BUY','kind':kind,'quantity':str(qty),'limit':str(limit) if kind=='LIMIT' else None,'stop':str(floor(bid*Decimal('.96'),tick)),'target':None,'position_id':None}
 c,p1,_=api('/api/paper/preview',intent('race1'));ok('Valid limit preview',c==200,p1)
 c,p2,_=api('/api/paper/preview',intent('race2'));ok('Second preview does not reserve capital',c==200 and not state()['account']['orders'],p2)
 with ThreadPoolExecutor(2) as pool:responses=list(pool.map(lambda p:api('/api/paper/submit',{'preview_id':p['preview_id'],'confirmed':True}),[p1,p2]))
 winners=[v for c,v,_ in responses if c==200];ok('Concurrent budget claims: exactly one order',len(winners)==1,responses)
 winner=winners[0]['order_id'];pwin=p1 if winner.endswith('race1') else p2
 c,duplicate,_=api('/api/paper/submit',{'preview_id':pwin['preview_id'],'confirmed':True});ok('Submission retry returns same order',c==200 and duplicate['order_id']==winner and duplicate['duplicate'])
 snapshot=state()['account'];reserve=snapshot['orders'][winner]['reserve'];ok('Reserve durable and no fictitious fill',Decimal(reserve)>300 and not snapshot['fills'])
 run('docker','kill',name);run('docker','start',name)
 wait(lambda:state()['worker_alive'] and api('/api/status')[1]['public_ws'],45)
 recovered=state()['account'];ok('Crash recovery preserves reserve and high-water mark',recovered['orders'][winner]['reserve']==reserve and recovered['ledger']['high_water']==snapshot['ledger']['high_water'])
 ok('Cancel request accepted',api('/api/paper/cancel',{'order_id':winner})[0]==200)
 wait(lambda:state()['account']['orders'][winner]['state']=='CANCELED')
 ok('Cancel acknowledgement releases reservation',state()['account']['orders'][winner]['reserve']=='0')
 botlimits=dict(manual);botlimits['position']='400'
 bot={'id':'testbot','name':'Acceptance SMA','instrument':'BTC-EUR','limits':botlimits,'fast':2,'slow':3,'order_eur':'20','stop_bps':'100','target_bps':'200'}
 c,v,_=api('/api/paper/bots',bot);ok('Separate bounded bot created',c==200,v)
 ok('Bot allocation cannot exceed account',api('/api/paper/bots',{**bot,'id':'too-much'})[0]==422)
 ok('Server bot starts explicitly',api('/api/paper/bots/control',{'bot_id':'testbot','running':True})[0]==200)
 b=wait(lambda:(lambda x:x if x['last_bar']>0 else None)(state()['account']['bots']['testbot']),30)
 ok('Bot processes closed candle without browser',b['last_bar']>0,b['last_signal'])
 ok('Bot pause keeps protection',api('/api/paper/bots/control',{'bot_id':'testbot','running':False})[0]==200)
 # Public-book fill path. Preview may expire due to price movement: re-preview deliberately.
 submitted=None
 for n in range(5):
  c,p,_=api('/api/paper/preview',intent('fill'+str(n),'MARKET','20'))
  if c==200:
   c,v,t=api('/api/paper/submit',{'preview_id':p['preview_id'],'confirmed':True})
   if c==200:submitted=v['order_id'];timings.append({'http_ms':t,'durable_ms':v['durable_ms']});break
  time.sleep(.4)
 ok('Public-book market intent persisted',submitted is not None)
 wait(lambda:Decimal(state()['account']['orders'][submitted]['filled'])>0,45)
 ok('Fill from real public book recorded with fee',any(f['order_id']==submitted and Decimal(f['fee'])>0 for f in state()['account']['fills']))
 ok('Pause entries explicit',api('/api/paper/control',{'action':'PAUSE_ENTRIES','confirmed':True})[0]==200)
 ok('Entry paused in risk engine',api('/api/paper/preview',intent('paused'))[0]==422)
 # No browser is involved. Isolated DB disconnect, leaving unrelated databases available.
 before=state()['account'];sql('postgres',f'ALTER DATABASE {db} ALLOW_CONNECTIONS false');sql('postgres',f"SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname='{db}'")
 try:
  ok('Database failure blocks mutations',api('/api/paper/cancel',{'order_id':submitted})[0]!=200)
  ok('User terminal remains reachable',run('curl','-fsS','http://127.0.0.1:4780/api/health').returncode==0)
 finally:sql('postgres',f'ALTER DATABASE {db} ALLOW_CONNECTIONS true')
 wait(lambda:state()['worker_alive'],45)
 after=state()['account'];ok('DB recovery preserves loss baseline and pause',after['ledger']['baseline']==before['ledger']['baseline'] and after['ledger']['high_water']==before['ledger']['high_water'] and after['ledger']['paused'])
 # A second process cannot acquire the execution lease in the same database.
 create(m['second'],db)
 time.sleep(3)
 raw=run('docker','exec',m['second'],'wget','-q','-O','-','--header','Host: localhost:8080','http://127.0.0.1:8080/api/paper').stdout
 ok('Second executor does not obtain lease',not json.loads(raw)['worker_alive'])
 run('docker','rm','-f',m['second'])
 for mode in ['LIVE','OKX_DEMO']:
  r=run('docker','run','--rm','--network','none','-e','EXECUTION_MODE='+mode,IMAGE,check=False)
  ok(mode+' build mode rejected before secret access',r.returncode!=0 and 'PAPER_LOCAL' in r.stderr+r.stdout)
 for path in ['/api/orders','/api/live/orders','/api/jev/execute']:
  ok('No bypass route '+path,api(path,{})[0]==404)
 req={'instrument':'BTC-EUR','bar':'15m','fast':5,'slow':20,'capital':'1000','order_eur':'100','stop_bps':'100','target_bps':'200','spread_bps':'10','slippage_bps':'5'}
 before=state()['account']['ledger']['cash'];c,replay,_=api('/api/replay',req)
 ok('Real OHLC replay persisted separately',c==200 and replay['report']['mode']=='REPLAY' and replay['report']['bars']>200,replay.get('error'))
 ok('Replay does not change paper cash',state()['account']['ledger']['cash']==before)
 EVIDENCE.write_text(json.dumps({'at':time.time(),'scope':'isolated local test database; public OKX data','checks':checks,'durability_ms':timings,'replay':replay['report']},indent=2))
 print(f'{len(checks)} checks passed. Test service retained for browser verification.',flush=True)

def cleanup():
 m=json.loads(MANIFEST.read_text());assert m['database'].startswith('okx_phase2_test_') and m['container'].startswith('okx-phase2-test-')
 for name in [m['second'],m['container']]:run('docker','rm','-f',name,check=False)
 sql('postgres',f"DROP DATABASE {m['database']} WITH (FORCE)")
 MANIFEST.unlink();print('Only recorded disposable test service/database removed.')

if __name__=='__main__':
 {'provision':provision,'check':check,'cleanup':cleanup}[sys.argv[1]]()
