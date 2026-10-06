"""Network portability checks in a recorded disposable DB. No account, order or gateway."""
import json,sys,time,uuid,math,hashlib,os,copy,subprocess,urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
import phase2_acceptance as p
M=ROOT/'.local/training-transfer-test.json';F=ROOT/'.local/training-transfer-test-data';E=ROOT/'evidence/training-transfer-integration.json'
CAMPAIGN='ac'*16
def manifest():
 m=json.loads(M.read_text());assert m['database'].startswith('okx_training_transfer_test_')and m['container'].startswith('okx-training-transfer-test-');return m
def quote(v):return "'"+json.dumps(v,separators=(',',':')).replace("'","''")+"'::jsonb"
def fixture(generation):
 return {'schema':1,'feature_version':'causal-ohlcv-v1','weights':[math.sin(i*.773916936745491)*7.9 for i in range(340)],'parameters':{'fast':9,'slow':40,'lookback':20,'rsi_low':30.,'bb_z':2.,'volume_ratio':1.2,'min_atr':.0005,'stop_bps':100.,'target_bps':200.,'max_hold':48,'cooldown':5,'confidence':.3},'normalizer':{'mean':[.12345678912345678]*16,'scale':[.03124395937294501]*16,'start_ms':1700000000000,'end_ms':1710000000000},'laya':False,'laya_checkpoint':'','laya_runtime':'','instrument':'BTC-EUR','bar':'5m','dataset_hash':'ab'*32,'seed':17,'generation':generation}
def provision():
 assert not M.exists(),'Recorded test already exists';F.mkdir(exist_ok=True);os.chmod(F,0o777)
 suffix=uuid.uuid4().hex[:10];db='okx_training_transfer_test_'+suffix;name='okx-training-transfer-test-'+suffix
 p.sql('postgres',f'CREATE DATABASE {db}');M.write_text(json.dumps({'database':db,'container':name}))
 p.run('docker','create','--name',name,'--network','okx-tradingterminal_market','--read-only','--cap-drop=ALL','--security-opt=no-new-privileges:true','--memory=512m','--cpus=2','--add-host','host.docker.internal:host-gateway','-e','EXECUTION_MODE=PAPER_LOCAL','-e','DATABASE_HOST='+p.DB_CONTAINER,'-e','DATABASE_NAME='+db,'-e','DATABASE_PASSWORD_FILE=/run/secrets/db_password','-e','TRAINING_DIR=/training','-e','LAYA_URL=http://host.docker.internal:8092','-e','RUST_LOG=warn','-v',str(ROOT/'.local/db_password')+':/run/secrets/db_password:ro','-v',str(F)+':/training','-p','127.0.0.1:4781:8080','okx-tradingterminal:0.4.0')
 p.run('docker','network','connect','okx-tradingterminal_storage',name);p.run('docker','start',name);p.wait(lambda:p.api('/api/ready')[0]==200,45)
 print('Disposable network transfer test ready',flush=True)
def check():
 m=manifest();db=m['database'];checks=[]
 def sql(s):return p.sql(db,s)
 def ok(label,test,detail=None):
  checks.append({'check':label,'passed':bool(test)});E.write_text(json.dumps({'at':time.time(),'scope':'synthetic network weights; disposable database; no account, order or private gateway','checks':checks},indent=2));print(('PASS 'if test else'FAIL ')+label,flush=True);assert test,detail
 ok('empty network archive available',p.api('/api/training/networks')[1]=={'networks':[]})
 config={'id':CAMPAIGN,'name':'TEST · synthetisch netwerk · training niet afgerond','bars':['5m'],'laya':False,'datasets':[]};now=int(time.time()*1000)
 sql(f"INSERT INTO training.campaigns(id,config,config_hash,desired,status,created_ms,updated_ms) VALUES('{CAMPAIGN}',{quote(config)},'fixture','RUNNING','LAYA_FEATURES',{now},{now})")
 dir=F/'campaigns'/CAMPAIGN;dir.mkdir(parents=True,exist_ok=True);(dir/'generations').mkdir(exist_ok=True)
 (dir/'progress.json').write_text(json.dumps({'status':'LAYA_FEATURES'}))
 ok('no invented trained weights before first generation',p.api(f'/api/training/campaigns/{CAMPAIGN}/export')[0]==422)
 art=fixture(3);progress={'status':'TRAINING','generation':3,'evaluated':24,'phase':'synthetic-fold-0','best_artifact':art,'history':[{'generation':2,'snapshot':'fold-0-17-2','net_return':0,'drawdown':0}]};(dir/'progress.json').write_text(json.dumps(progress));sql(f"UPDATE training.campaigns SET progress={quote(progress)},status='TRAINING' WHERE id='{CAMPAIGN}'")
 old=fixture(2);(dir/'generations/fold-0-17-2.json').write_text(json.dumps({'artifact':old,'metrics':{'phase':'synthetic-fold-0'}}))
 code,bundle,_=p.api(f'/api/training/campaigns/{CAMPAIGN}/export');ok('in-progress export includes complete weights and normalizer',code==200 and bundle['artifact']==art and bundle['source']['kind']=='in_progress',bundle)
 code,older,_=p.api(f'/api/training/campaigns/{CAMPAIGN}/export?checkpoint=fold-0-17-2');ok('explicit earlier generation exports exact checkpoint',code==200 and older['artifact']==old and older['source']['kind']=='checkpoint',older)
 ok('checkpoint path traversal blocked',p.api(f'/api/training/campaigns/{CAMPAIGN}/export?checkpoint=..%2Frequest')[0]==422)
 code,saved,_=p.api('/api/training/networks/import',{'name':'TEST · tussentijds netwerk','bundle':bundle});ok('mid-training network imports into persistent research archive',code==200 and saved['research_only']and saved['weights']==340,saved);sid=saved['id']
 code,restored,_=p.api('/api/training/networks/'+sid);ok('all weights and settings survive database roundtrip exactly',code==200 and restored==bundle,restored)
 ok('floating weight bits preserved across JSON and PostgreSQL',all(x.hex()==y.hex()for x,y in zip(art['weights'],restored['artifact']['weights'])))
 again=p.api('/api/training/networks/import',{'name':'Different filename','bundle':restored})[1];ok('repeated import reuses exact stored network',again['id']==sid and again['duplicate'])
 raw=b' '*20000+json.dumps({'name':'Whitespace formatting','bundle':bundle}).encode();req=urllib.request.Request(p.BASE+'/api/training/networks/import',data=raw,headers={'Origin':p.BASE,'Content-Type':'application/json','X-Terminal-Request':'paper-v2'})
 with urllib.request.urlopen(req,timeout=15)as response:larger=json.load(response)
 ok('network route accepts valid formatted files above global body limit',larger['id']==sid and larger['duplicate'])
 listing=p.api('/api/training/networks')[1]['networks'];ok('archive lists imported generation and provenance',len(listing)==1 and listing[0]['generation']==3 and listing[0]['source']['kind']=='in_progress')
 changed=copy.deepcopy(bundle);changed['artifact']['weights'][0]=.25;ok('modified weights rejected by content hash',p.api('/api/training/networks/import',{'name':'Changed','bundle':changed})[0]==422)
 changed=copy.deepcopy(bundle);changed['qualification']={'passed':True};ok('claimed qualification cannot be imported',p.api('/api/training/networks/import',{'name':'Claimed proof','bundle':changed})[0]==422)
 changed=copy.deepcopy(bundle);changed['artifact']['live_release']={'confirmed':True};ok('trading permission fields rejected',p.api('/api/training/networks/import',{'name':'Permission','bundle':changed})[0]==422)
 changed=copy.deepcopy(bundle);changed['version']=2;ok('unsupported portable file version rejected',p.api('/api/training/networks/import',{'name':'Version','bundle':changed})[0]==422)
 ok('oversized upload rejected before persistence',p.api('/api/training/networks/import',{'name':'Large','bundle':bundle,'padding':'x'*130000})[0]==413)
 ok('cross-origin import rejected',p.api('/api/training/networks/import',{'name':'Origin','bundle':bundle},headers={'Origin':'https://invalid.example','Content-Type':'application/json','X-Terminal-Request':'paper-v2'})[0]==403)
 mutation=p.run('docker','exec',p.DB_CONTAINER,'psql','-U','terminal','-d',db,'-At','-v','ON_ERROR_STOP=1','-c',f"UPDATE training.network_imports SET name='changed' WHERE id='{sid}'",check=False);ok('stored imported content remains immutable',mutation.returncode!=0)
 final=fixture(5);h=hashlib.sha256(json.dumps(final).encode()).hexdigest();summary={'development':{'net_return':0,'net_pnl':'0','fees':'0','execution_costs':'0','ai_costs':'0','max_drawdown':0,'exposure':0,'profit_factor':None,'roundtrips':0,'valid':False},'benchmarks':{},'validation':[],'curve':[],'local_compute_seconds':0};sql(f"INSERT INTO training.artifacts(hash,campaign_id,instrument,bar,artifact,summary,created_ms) VALUES('{h}','{CAMPAIGN}','BTC-EUR','5m',{quote(final)},{quote(summary)},{now})")
 code,final_bundle,_=p.api('/api/training/artifacts/'+h+'/export');ok('immutable candidate export supported without campaign completion',code==200 and final_bundle['artifact']==final and final_bundle['source']['original_artifact_hash']==h,final_bundle)
 ok('import does not create account, orders, evidence or trading release',p.api('/api/paper')[1]['account']is None and sql('SELECT count(*) FROM execution.orders')=='0'and sql('SELECT count(*) FROM training.releases')=='0'and sql('SELECT count(*) FROM training.holdouts')=='0'and sql('SELECT count(*) FROM training.forward_runs')=='0')
 changed=copy.deepcopy(bundle);changed['artifact']['weights'][0]=.25
 (F/'test-network-export.json').write_text(json.dumps(bundle,indent=2));(F/'test-network-corrupted.json').write_text(json.dumps(changed,indent=2));M.write_text(json.dumps({**m,'campaign':CAMPAIGN,'candidate_hash':h,'saved_id':sid}))
 print(f'{len(checks)} portability checks passed; synthetic browser fixture retained',flush=True)
def cleanup():
 m=manifest();p.run('docker','rm','-f',m['container']);p.sql('postgres',f"DROP DATABASE {m['database']} WITH (FORCE)");M.unlink();print('Only recorded transfer test container and database removed')
if __name__=='__main__':{'provision':provision,'check':check,'cleanup':cleanup}[sys.argv[1]]()
