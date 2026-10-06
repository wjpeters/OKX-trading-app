"""Synthetic proofs only in the recorded training test DB. No exchange gateway."""
import json,time,hashlib,sys
from pathlib import Path
from decimal import Decimal
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
import phase2_acceptance as p
M=ROOT/'.local/training-test.json';E=ROOT/'evidence/training-native-integration.json'
def quote(v):return "'"+json.dumps(v,separators=(',',':')).replace("'","''")+"'::jsonb"
def manifest():
 m=json.loads(M.read_text());assert m['container'].startswith('okx-training-test-')and m['database'].startswith('okx_training_test_');return m
def recreate():
 m=manifest();p.run('docker','rm','-f',m['container'])
 args=['docker','create','--name',m['container'],'--network','okx-tradingterminal_market','--read-only','--cap-drop=ALL','--security-opt=no-new-privileges:true','--memory=512m','--cpus=2','--add-host','host.docker.internal:host-gateway','-e','EXECUTION_MODE=PAPER_LOCAL','-e','DATABASE_HOST='+p.DB_CONTAINER,'-e','DATABASE_NAME='+m['database'],'-e','DATABASE_PASSWORD_FILE=/run/secrets/db_password','-e','TRAINING_DIR=/training','-e','LAYA_URL=http://host.docker.internal:8092','-e','RUST_LOG=warn','-v',str(ROOT/'.local/db_password')+':/run/secrets/db_password:ro','-v',str(ROOT/'.local/training-test-data')+':/training','-p','127.0.0.1:4781:8080','okx-tradingterminal:0.4.0']
 p.run(*args);p.run('docker','network','connect','okx-tradingterminal_storage',m['container']);p.run('docker','start',m['container']);p.wait(lambda:p.api('/api/ready')[0]==200,45)
 print('Only the recorded test container was updated',flush=True)
def check():
 m=manifest();db=m['database'];checks=[]
 def sql(s):return p.sql(db,s)
 def ok(label,test,detail=None):
  checks.append({'check':label,'passed':bool(test)});E.write_text(json.dumps({'scope':'synthetic qualification/account/candle fixtures only; disposable DB; zero private calls; not real qualification evidence','checks':checks,'at':time.time()},indent=2));print(('PASS 'if test else'FAIL ')+label,flush=True);assert test,detail
 ok('native schema and routes migrated',p.api('/api/training/native')[0]==200)
 ok('cross-origin native activation rejected',p.api('/api/training/artifacts/fake/native-release',{},headers={'Content-Type':'application/json','Origin':'https://invalid.example','X-Terminal-Request':'paper-v2'})[0]==403)
 art=json.loads((ROOT/'.local/training-test-data/synthetic-artifact.json').read_text());art['weights']=[0.]*340;art['weights'][337]=8.;art['generation']=99;art['parameters'].update(lookback=10,volume_ratio=.5,min_atr=0.)
 raw=json.dumps(art).encode();h=hashlib.sha256(raw).hexdigest();now=int(time.time()*1000)
 sys.path.insert(0,str(ROOT/'trainer'));import worker,os
 os.environ['EVALUATOR']=str(ROOT/'backend/target/debug/okx-evaluator')
 with worker.Evaluator(str(ROOT/'.local/training-test-data/synthetic.parquet'),'1m')as evaluator:
  report=evaluator.call({'command':'evaluate','artifact':art,'costs':{},'start_ms':art['normalizer']['start_ms'],'end_ms':art['normalizer']['end_ms'],'mode':'neural'})
 summary={'development':report,'benchmarks':{},'activation':report['activation'],'curve':report['curve'],'local_compute_seconds':0,'validation':[]}
 campaign='dddddddddddddddddddddddddddddddd'
 config={'name':'TEST NATIVE · kunstmatige bewijzen · geen echte kwalificatie','id':campaign,'costs':{},'bars':['1m'],'laya':False,'population':8,'generations':2,'daily_hours':8}
 sql(f"INSERT INTO training.campaigns(id,config,config_hash,desired,status,created_ms,updated_ms) VALUES('{campaign}',{quote(config)},'synthetic-native','PAUSED','COMPLETED',{now},{now}) ON CONFLICT DO NOTHING")
 sql(f"INSERT INTO training.artifacts(hash,campaign_id,instrument,bar,artifact,summary,created_ms) VALUES('{h}','{campaign}','BTC-EUR','1m',{quote(art)},{quote(summary)},{now}) ON CONFLICT DO NOTHING")
 sql(f"DELETE FROM training.holdouts WHERE artifact_hash='{h}'; DELETE FROM training.forward_runs WHERE artifact_hash='{h}'")
 bot={'account_uid':'SYNTHETIC-NATIVE-ONLY','instruments':['BTC-EUR'],'capital':'500','max_order':'50','max_position':'100','trade_risk':'5','daily_loss':'10','total_loss':'20','drawdown':'25','max_orders':3,'orders_per_minute':2,'spread_bps':'50','confirmed':True}
 body={'mode':'OKX_DEMO','limits':bot,'confirmed':True};path=f'/api/training/artifacts/{h}/native-release'
 ok('no native activation without real historical gate',p.api(path,body)[0]==422)
 sql(f"INSERT INTO training.holdouts(instrument,start_ms,end_ms,artifact_hash,campaign_id,status,report,created_ms) VALUES('BTC-EUR',1,2,'{h}','{campaign}','PASSED',{quote({'passed':True,'family_complete':True,'scope':'SYNTHETIC TEST ONLY'})},{now}) ON CONFLICT DO NOTHING")
 ok('no native activation without 30d forward gate',p.api(path,body)[0]==422)
 sql(f"INSERT INTO training.forward_runs(id,artifact_hash,status,state,report,started_ms,updated_ms) VALUES('{h[:32]}','{h}','PAUSED','{{}}',{quote({'passed':True,'elapsed_days':30,'roundtrips':100,'scope':'SYNTHETIC TEST ONLY'})},{now},{now}) ON CONFLICT DO NOTHING")
 market=p.wait(lambda:(lambda v:v[1]['market']if v[0]==200 and v[1]['market']['books'].get('BTC-EUR')else None)(p.api('/api/market')),40)
 meta=next(x for x in market['instruments']if x['instId']=='BTC-EUR');book=market['books']['BTC-EUR'];bid=book['bids'][0][0];ask=book['asks'][0][0]
 snap={'uid':bot['account_uid'],'mode':'OKX_DEMO','received_ms':now,'connected':True,'reconciled':True,'trade_permission':True,'valuation_complete':True,'equity_eur':'10000','available':{'EUR':'10000','BTC':'1'},'holdings':{'EUR':'10000'},'quotes':{'BTC-EUR':{'bid':bid,'ask':ask,'received_ms':now,'rules':{'tick':meta['tickSz'],'lot':meta['lotSz'],'minimum':meta['minSz'],'base':'BTC','quote':'EUR'}}},'tradable':['BTC-EUR'],'fee_rate':'0.001','fee_rates':{'BTC-EUR':'0.001'},'daily_loss':'0','total_loss':'0','drawdown':'0','native_protection_verified':True,'live_acceptance_complete':False,'live_enabled':True}
 account={**bot,'capital':'10000','max_order':'1000','max_position':'2000','trade_risk':'100','daily_loss':'500','total_loss':'1000','drawdown':'1000','max_orders':10,'orders_per_minute':60,'spread_bps':'500'}
 def refresh(mode='OKX_DEMO'):
  n=int(time.time()*1000);s={**snap,'mode':mode,'received_ms':n,'quotes':{'BTC-EUR':{**snap['quotes']['BTC-EUR'],'received_ms':n}}};sql(f"UPDATE execution.accounts SET snapshot={quote(s)},policy={quote(account)},policy_revision=4,status='ACCOUNT_CONNECTED',updated_ms={n},live_release={quote({'account_uid':bot['account_uid'],'policy_revision':4,'confirmed_ms':n})} WHERE mode='{mode}'")
 refresh('LIVE');code_version=p.api('/api/training/native')[1]['code_version']
 for mode in ['LIVE','OKX_DEMO']:sql(f"INSERT INTO execution.events(mode,kind,detail,created_ms) VALUES('{mode}','GATEWAY_STARTED',{quote({'code_version':code_version,'scope':'SYNTHETIC TEST ONLY; no gateway running'})},{int(time.time()*1000)})")
 c,v,_=p.api(path,{**body,'mode':'LIVE'});ok('manual Live permission cannot substitute for actual Demo proof',c==422 and 'Demo' in v.get('error',''),v)
 refresh();ok('unconfirmed exact bot release rejected',p.api(path,{**body,'confirmed':False})[0]==422)
 ok('bot drawdown cannot exceed 5%',p.api(path,{**body,'limits':{**bot,'drawdown':'26'}})[0]==422)
 ok('bot cannot enlarge account limits',p.api(path,{**body,'limits':{**bot,'max_order':'2000'}})[0]==422)
 # The test server has no credentials/gateway. It can only persist a proposal.
 while int(time.time())%60>8:time.sleep(.4)
 refresh();end=int(time.time()*1000)//60000*60000;sql("DELETE FROM training.live_candles WHERE instrument='BTC-EUR'")
 candles=[]
 for i in range(400):
  px=Decimal('100')+Decimal(i%17)/100
  if i==399:px=Decimal('105')
  ts=end-(400-i)*60000;c={'ts':ts,'open':str(px),'high':str(px+Decimal('.01')),'low':str(px-Decimal('.01')),'close':str(px),'volume':'1000','closed':True};candles.append(f"('BTC-EUR',{ts},{quote(c)},{int(time.time()*1000)})")
 sql('INSERT INTO training.live_candles(instrument,ts,payload,received_ms) VALUES'+','.join(candles))
 c,v,_=p.api(path,body);ok('explicit synthetic Demo binding accepted only in test DB',c==200,v);rid=v['id']
 queued=p.wait(lambda:sql(f"SELECT client_id FROM execution.orders WHERE bot_release_id='{rid}' AND state='QUEUED'"),8);ok('one bounded IOC proposal is durable without fictitious fills',bool(queued)and sql(f"SELECT count(*) FROM execution.fills f JOIN execution.orders o ON o.mode=f.mode AND o.client_id=f.parent_id WHERE o.bot_release_id='{rid}'")=='0')
 native=p.api('/api/training/native')[1];r=next(x for x in native['runs']if x['release']['id']==rid);ok('durable proposal latency below two seconds on fixture',r['report'].get('proposal_latency_ms',999999)<2000,r)
 time.sleep(.8);ok('duplicate same-candle proposals suppressed',sql(f"SELECT count(*) FROM execution.orders WHERE bot_release_id='{rid}'")=='1')
 c,v,_=p.api(f'/api/training/native/{rid}/pause',{});ok('pause cancels unsubmitted owned entry',c==200 and sql(f"SELECT state FROM execution.orders WHERE bot_release_id='{rid}'")=='REJECTED',v)
 ok('Demo acceptance cannot be fabricated from a queued proposal',p.api(f'/api/training/native/{rid}/accept-demo',{'confirmed':True})[0]==422)
 bad=p.run('docker','exec',p.DB_CONTAINER,'psql','-U','terminal','-d',db,'-At','-v','ON_ERROR_STOP=1','-c',f"UPDATE training.releases SET artifact_hash='invalid' WHERE id='{rid}'",check=False);ok('exact release identity is immutable in database',bad.returncode!=0)
 refresh();ok('revoke exact release succeeds',p.api(f'/api/training/native/{rid}/revoke',{})[0]==200)
 ok('revoked release cannot resume',p.api(f'/api/training/native/{rid}/resume',{})[0]==422)
 ok('no paper account or exchange fills initialized',p.api('/api/paper')[1]['account']is None and sql('SELECT count(*) FROM execution.fills')=='0')
 sql("UPDATE execution.accounts SET snapshot=NULL,policy=NULL,live_release=NULL,status='OFFLINE' WHERE mode IN ('OKX_DEMO','LIVE')")
 print(f'{len(checks)} native Training Lab checks passed',flush=True)
if __name__=='__main__':{'recreate':recreate,'check':check}[sys.argv[1]]()
