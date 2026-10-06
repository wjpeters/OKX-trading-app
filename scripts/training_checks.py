"""Synthetic pipeline and disposable database checks. Never touches account setup/orders."""
import json,sys,time,uuid,math,hashlib,subprocess,os
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'trainer'));sys.path.insert(0,str(ROOT/'scripts'))
import worker
import phase2_acceptance as p
p.IMAGE='okx-tradingterminal:0.4.0'
MANIFEST=ROOT/'.local/training-test.json'
FOLDER=ROOT/'.local/training-test-data'
EVIDENCE=ROOT/'evidence/training-integration.json'
def save(checks,**more):EVIDENCE.write_text(json.dumps({'scope':'synthetic fixtures; disposable database on port 4781; no orders or account setup','at':time.time(),'checks':checks,**more},indent=2))
def pipeline():
 import pyarrow as pa,pyarrow.parquet as pq
 FOLDER.mkdir(exist_ok=True);os.chmod(FOLDER,0o777)
 ts=1700000040000;ts-=ts%900000
 rows=[]
 for i in range(3600):
  price=100+8*math.sin(i/18)+.002*i
  rows.append({'ts':ts+i*60000,'open':str(price),'high':str(price+1),'low':str(price-1),'close':str(price+.2*math.sin(i)),'volume':'10000'})
 path=FOLDER/'synthetic.parquet';pq.write_table(pa.Table.from_pylist(rows),path,compression='snappy')
 worker.ROOT=FOLDER;worker.BUDGET=worker.Budget();os.environ['EVALUATOR']=str(ROOT/'backend/target/debug/okx-evaluator')
 ds={'id':'synthetic','instrument':'BTC-EUR','path':str(path),'hash':worker.digest(path),'start_ms':ts,'end_ms':ts+len(rows)*60000,'costs':{}}
 config={'laya':False,'laya_runtime':'','seeds':[17,41,73],'workers':2,'population':8,'generations':2,'patience':30,'daily_hours':8}
 worker.atomic(FOLDER/'control.json',{'desired':'RUNNING'})
 with worker.Evaluator(str(path),'1m')as e:
  norm=e.call({'command':'fit','start_ms':ts,'end_ms':ts+2400*60000})
  progress=worker.read(FOLDER/'synthetic-progress.json',{'history':[]});result=worker.optimize(config,ds,'1m',FOLDER,progress,'synthetic-only',ts+200*60000,ts+2400*60000,norm,None)
  assert len(result)==3 and len(progress['history'])==6
  art=result[0][2];report=worker.evaluate(e,art,ds,ts+2400*60000,ds['end_ms'])
  worker.atomic(FOLDER/'synthetic-artifact.json',art)
  worker.atomic(FOLDER/'synthetic-report.json',report)
  worker.atomic(FOLDER/'synthetic-progress.json',progress)
  pvalue,ci=worker.bootstrap_p(report['daily_returns'],91271)
  assert pvalue==1 and ci is None
  again=worker.optimize(config,ds,'1m',FOLDER,progress,'synthetic-only',ts+200*60000,ts+2400*60000,norm,None)
  assert [x[1]for x in again]==[x[1]for x in result]
 save([{'check':'real CMA-ES / two Rust evaluator workers / three seeds / six generations','passed':True},{'check':'resume returns identical finalized candidates','passed':True},{'check':'short synthetic results rejected as insufficient evidence','passed':True}],synthetic={'evaluations':progress['evaluated'],'roundtrips':report['roundtrips'],'artifact':'test fixture, not a qualified trading bot'})
 print('Synthetic training pipeline passed',flush=True)
def provision():
 assert not MANIFEST.exists(),'Existing test manifest; clean that recorded test first'
 FOLDER.mkdir(exist_ok=True);os.chmod(FOLDER,0o777)
 suffix=uuid.uuid4().hex[:10];db='okx_training_test_'+suffix;name='okx-training-test-'+suffix
 p.sql('postgres',f'CREATE DATABASE {db}')
 m={'database':db,'container':name};MANIFEST.write_text(json.dumps(m))
 p.run('docker','create','--name',name,'--network','okx-tradingterminal_market','--read-only','--cap-drop=ALL','--security-opt=no-new-privileges:true','--memory=512m','--cpus=2','--add-host','host.docker.internal:host-gateway','-e','EXECUTION_MODE=PAPER_LOCAL','-e','DATABASE_HOST='+p.DB_CONTAINER,'-e','DATABASE_NAME='+db,'-e','DATABASE_PASSWORD_FILE=/run/secrets/db_password','-e','TRAINING_DIR=/training','-e','LAYA_URL=http://host.docker.internal:8092','-e','RUST_LOG=warn','-v',str(ROOT/'.local/db_password')+':/run/secrets/db_password:ro','-v',str(FOLDER)+':/training','-p','127.0.0.1:4781:8080',p.IMAGE)
 p.run('docker','network','connect','okx-tradingterminal_storage',name);p.run('docker','start',name)
 p.wait(lambda:p.api('/api/ready')[0]==200,45)
 print('Disposable Training Lab ready',flush=True)
def quote(v):return "'"+json.dumps(v,separators=(',',':')).replace("'","''")+"'"
def check():
 m=json.loads(MANIFEST.read_text());db=m['database'];checks=json.loads(EVIDENCE.read_text())['checks']
 def ok(label,condition,detail=None):
  checks.append({'check':label,'passed':bool(condition)});save(checks);print(('PASS 'if condition else'FAIL ')+label,flush=True);assert condition,detail
 code,state,_=p.api('/api/training');ok('training schema migrated and overview accessible',code==200,state)
 ok('user budget not initialized in test environment',p.api('/api/paper')[1]['account']is None)
 ok('default fixed network is 16 / 16 / 4',state['network']=={'inputs':16,'hidden':16,'outputs':4,'weights':340})
 ok('live release remains unavailable',state['release_ready']is False)
 ok('cross-origin mutations rejected',p.api('/api/training/campaigns',{},headers={'Origin':'https://invalid.example','Content-Type':'application/json','X-Terminal-Request':'paper-v2'})[0]==403)
 now=int(time.time()*1000);end=now-now%60000-60000;start=end-6*3600000
 code,imp,_=p.api('/api/training/datasets',{'instrument':'BTC-EUR','start_ms':start,'end_ms':end});ok('real public dataset import starts',code==200,imp)
 ds=p.wait(lambda:next((d for d in p.api('/api/training')[1]['datasets']if d['id']==imp['id']and d['status']in ('READY','INCOMPLETE','ERROR')),None),90)
 ok('public candles imported without gaps',ds['status']=='READY'and ds['manifest']['coverage']==1,ds)
 body={'name':'Synthetic guard test','dataset_ids':[imp['id']],'bars':['1m'],'laya':False,'costs':{}}
 ok('short datasets cannot start a campaign',p.api('/api/training/campaigns',body)[0]==422)
 ok('resource cap rejected before campaign creation',p.api('/api/training/campaigns',{**body,'memory_mb':8192})[0]==422)
 ok('duplicate timeframes rejected',p.api('/api/training/campaigns',{**body,'bars':['1m','1m']})[0]==422)
 # Synthetic immutable records test holdout reservation independently from search performance.
 art=json.loads((FOLDER/'synthetic-artifact.json').read_text());h=hashlib.sha256(json.dumps(art).encode()).hexdigest()
 config={'name':'TESTBEELD · kunstmatige data · geen gekwalificeerde bot','id':'ffffffffffffffffffffffffffffffff','datasets':[{'id':imp['id'],'instrument':'BTC-EUR','start_ms':start-365*worker.DAY,'end_ms':end,'holdout_start_ms':end-60*worker.DAY,'path':'/training/synthetic.parquet','hash':'fixture'}],'bars':['1m'],'laya':False,'daily_hours':8,'qualification':{'bootstrap_seed':91271}}
 p.sql(db,f"INSERT INTO training.campaigns(id,config,config_hash,desired,status,created_ms,updated_ms) VALUES('ffffffffffffffffffffffffffffffff',{quote(config)},'fixture','PAUSED','COMPLETED',{now},{now})")
 report=json.loads((FOLDER/'synthetic-report.json').read_text());summary={'development':report,'benchmarks':{},'activation':report['activation'],'curve':report['curve']}
 p.sql(db,f"UPDATE training.campaigns SET progress={quote(json.loads((FOLDER/'synthetic-progress.json').read_text()))} WHERE id='ffffffffffffffffffffffffffffffff'")
 p.sql(db,f"INSERT INTO training.artifacts(hash,campaign_id,instrument,bar,artifact,summary,created_ms) VALUES('{h}','ffffffffffffffffffffffffffffffff','BTC-EUR','1m',{quote(art)},{quote(summary)},{now})")
 imm=p.run('docker','exec',p.DB_CONTAINER,'psql','-U','terminal','-d',db,'-At','-v','ON_ERROR_STOP=1','-c',f"UPDATE training.artifacts SET bar='5m' WHERE hash='{h}'",check=False)
 ok('artifact mutation rejected by database',imm.returncode!=0)
 imm=p.run('docker','exec',p.DB_CONTAINER,'psql','-U','terminal','-d',db,'-At','-v','ON_ERROR_STOP=1','-c',"UPDATE training.campaigns SET config='{}' WHERE id='ffffffffffffffffffffffffffffffff'",check=False)
 ok('campaign profile mutation rejected by database',imm.returncode!=0)
 code,value,_=p.api(f'/api/training/artifacts/{h}/qualify',{});ok('predeclared holdout family reserved once',code==200,value)
 ok('second attempt on consumed holdout rejected',p.api(f'/api/training/artifacts/{h}/qualify',{})[0]==422)
 ok('unqualified forward rejected',p.api(f'/api/training/artifacts/{h}/forward',{})[0]==422)
 ok('live release rejected even with explicit test consent',p.api(f'/api/training/artifacts/{h}/release',{'account_uid':'synthetic','budget':'100','confirmed':True})[0]==422)
 ok('qualification never initializes a paper account',p.api('/api/paper')[1]['account']is None)
 save(checks,real_dataset={'instrument':'BTC-EUR','rows':ds['manifest']['rows'],'coverage':ds['manifest']['coverage']})
 print(f'{len(checks)} training checks passed',flush=True)
def cleanup():
 m=json.loads(MANIFEST.read_text());assert m['database'].startswith('okx_training_test_')and m['container'].startswith('okx-training-test-')
 p.run('docker','rm','-f',m['container']);p.sql('postgres',f"DROP DATABASE {m['database']} WITH (FORCE)");MANIFEST.unlink()
if __name__=='__main__':{'pipeline':pipeline,'provision':provision,'check':check,'cleanup':cleanup}[sys.argv[1]]()
