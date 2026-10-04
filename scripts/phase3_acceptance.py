#!/usr/bin/env python3
"""Isolated regressions for studio/research/history. Real public data, no actual account/trades."""
import phase2_acceptance as b
import sys,json,time,copy,urllib.request,urllib.error
b.MANIFEST=b.ROOT/'.local/phase3-test.json';b.EVIDENCE=b.ROOT/'evidence/phase3-regression.json';b.IMAGE='okx-tradingterminal:0.4.0'
def create(name,db,port=None):
 args=['docker','create','--name',name,'--network','okx-tradingterminal_market','--read-only','--cap-drop=ALL','--security-opt=no-new-privileges:true','--memory=512m','--cpus=2','-e','EXECUTION_MODE=PAPER_LOCAL','-e','DATABASE_HOST='+b.DB_CONTAINER,'-e','DATABASE_NAME='+db,'-e','DATABASE_PASSWORD_FILE=/run/secrets/db_password','-e','RUST_LOG=warn','-v',str(b.ROOT/'.local/db_password')+':/run/secrets/db_password:ro','-v',name+'-jobs:/jobs','-v',name+'-archives:/archives']
 if port:args+=['-p','127.0.0.1:'+port+':8080']
 b.run(*args,b.IMAGE);b.run('docker','network','connect','okx-tradingterminal_storage',name);b.run('docker','start',name)
 if port and b.run('docker','inspect',name+'-pine',check=False).returncode:b.run('docker','run','-d','--name',name+'-pine','--network','none','--read-only','--memory=256m','--cpus=.5','--pids-limit=32','--cap-drop=ALL','--security-opt=no-new-privileges:true','-v',name+'-jobs:/jobs','okx-pine-worker:0.3.0')
b.create=create
checks=[]
def ok(name,condition,detail=None):
 checks.append({'check':name,'passed':bool(condition),'detail':detail});(b.ROOT/'evidence/phase3-integration.json').write_text(json.dumps({'at':time.time(),'scope':'isolated local test database','checks':checks},indent=2));print(('PASS ' if condition else 'FAIL ')+name,flush=True);assert condition,detail

def studio():
 user_before=json.load(urllib.request.urlopen('http://127.0.0.1:4780/api/paper'))['account']
 source='//@version=6\nindicator("Acceptance")\nplot(ta.sma(close, 3), "SMA")\nplot(1,"entry")'
 c,r,t=b.api('/api/studio/pine',{'source':source,'instrument':'BTC-EUR','bar':'15m'});ok('Pine isolated real-candle preview',c==200 and r.get('ok') and r['bars']>200,{'duration_ms':r.get('duration_ms'),'http_ms':t,'bars':r.get('bars'),'diagnostics':r.get('diagnostics')})
 c,r,_=b.api('/api/studio/pine',{'source':'//@version=6\nindicator("Bad")\nif false\n    ta.not_real(close)','instrument':'BTC-EUR','bar':'15m'});ok('Invalid Pine has no intents',c==200 and not r['ok'] and not r['intents'])
 doc={'kind':'pine','name':'Acceptance Pine','base_version':0,'payload':{'source':source}};c,r,_=b.api('/api/studio/documents',doc);ok('Immutable Pine version stored',c==200 and r['version']==1,r);ok('Concurrent old version rejected',b.api('/api/studio/documents',doc)[0]==422)
 graph={'schema':1,'nodes':[{'id':'m','kind':'market','params':{}},{'id':'p','kind':'pine','params':{'name':'Acceptance Pine','version':1}},{'id':'i','kind':'intent','params':{}},{'id':'r','kind':'risk_gate','params':{}}],'edges':[{'source':'m','target':'p'},{'source':'p','target':'i'},{'source':'i','target':'r'}]}
 ok('Valid Pine graph compiles',b.api('/api/studio/graph/validate',graph)[0]==200)
 bad=copy.deepcopy(graph);bad['edges'].append({'source':'i','target':'m'});ok('Graph cycle refused',b.api('/api/studio/graph/validate',bad)[0]==422)
 bad=copy.deepcopy(graph);bad['nodes'][-1]['kind']='sizing';ok('Risk gate cannot be removed',b.api('/api/studio/graph/validate',bad)[0]==422)
 c,r,_=b.api('/api/studio/documents',{'kind':'graph','name':'Acceptance graph','base_version':0,'payload':graph});ok('Graph version stored',c==200,r)
 c,r,_=b.api('/api/studio/alerts',{'name':'Isolated price alert','instrument':'BTC-EUR','bar':'1m','field':'price','comparison':'above','value':1,'trigger':'intrabar','cooldown_secs':60});ok('Server alert created',c==200,r);aid=r['id']
 events=b.wait(lambda:(lambda x:x['events'] if any(e['alert_id']==aid for e in x['events']) else None)(b.api('/api/studio/alerts')[1]),20)
 ok('Server alert delivered without browser',any(e['alert_id']==aid for e in events));time.sleep(4)
 ok('Alert deduplicated within cooldown',len([e for e in b.api('/api/studio/alerts')[1]['events'] if e['alert_id']==aid])==1)
 ok('Alert pause',b.api('/api/studio/alerts/control',{'id':aid,'enabled':False})[0]==200)
 c,r,_=b.api('/api/studio/indicators',{'instrument':'BTC-EUR','bar':'15m','settings':{'ma':10,'oscillator':7,'macd_fast':5,'macd_slow':15,'macd_signal':5,'bb_multiplier':2.5}});ok('Configurable native indicators',c==200 and r['indicator_parameters']['ma']==10,r.get('error'))
 req={'instrument':'BTC-EUR','bar':'15m','fast':9,'slow':21,'capital':'1000','order_eur':'100','stop_bps':'100','target_bps':'200','spread_bps':'10','slippage_bps':'5'}
 c,r,_=b.api('/api/replay/evaluate',req);ok('Walk-forward separate OOS windows',c==200 and len(r['report']['folds'])==2,r.get('error'));f=r['report']['folds'];ok('OOS windows do not overlap',f[0]['out_of_sample']['last_ms']<f[1]['out_of_sample']['first_ms'])
 c,r,_=b.api('/api/history',{'instrument':'BTC-EUR','bar':'15m','pages':1});ok('Compressed Parquet history import',c==200 and len(r['parts'])==1,r)
 part=b.api('/api/history')[1][0];c,r,_=b.api('/api/history',{'instrument':'BTC-EUR','bar':'15m','pages':1});ok('History resume uses older cursor',c==200 and b.api('/api/history')[1][0]['last_ms']<part['first_ms'],r)
 blob=urllib.request.urlopen(b.BASE+'/api/history/export?id='+str(part['id'])).read();ok('Real Parquet file, not renamed JSON',blob[:4]==b'PAR1' and blob[-4:]==b'PAR1',{'bytes':len(blob)})
 c,r,_=b.api('/api/replay/evaluate',req);ok('Walk-forward reads imported Parquet data',c==200 and r['report']['bars']>=500,r.get('error'))
 ok('Jev no-key precondition',b.api('/api/jev/propose',{'instrument':'BTC-EUR','bar':'1m'})[0]==422)
 c,r,_=b.api('/api/research');ok('Provider status exposes missing keys honestly',c==200 and not r['jev']['configured'] and r['jev']['spent_usd']=='0')
 # Deploy a pinned script only after clearing the disposable bot's exposure.
 state=b.state()['account']
 for pos in state['positions'].values():
  if pos['bot_id']=='testbot' and float(pos['quantity'])>0:
   req={'client_id':'clearbot'+str(int(time.time()*1000)),'bot_id':'testbot','instrument':pos['instrument'],'side':'SELL','kind':'MARKET','quantity':pos['quantity'],'limit':None,'stop':None,'target':None,'position_id':pos['id']}
   c,r,_=b.api('/api/paper/preview',req);assert c==200,r;c,r,_=b.api('/api/paper/submit',{'preview_id':r['preview_id'],'confirmed':True});assert c==200,r
 b.wait(lambda:not any(p['bot_id']=='testbot' and float(p['quantity'])>0 for p in b.state()['account']['positions'].values()),30)
 c,r,_=b.api('/api/studio/deploy',{'bot_id':'testbot','name':'Acceptance graph','version':1});ok('Pinned graph deployment through central kernel',c==200,r)
 c,r,_=b.api('/api/paper/control',{'action':'RESUME_ENTRIES','confirmed':True});ok('Explicit resume for graph test',c==200,r)
 c,r,_=b.api('/api/paper/bots/control',{'bot_id':'testbot','running':True});ok('Graph bot starts with explicit limits',c==200,r)
 initial=b.state()['account']['bots']['testbot']['last_bar'];b.wait(lambda:b.state()['account']['bots']['testbot']['last_bar']>initial,95)
 state=b.state()['account'];ok('Pine bot executes server-side through fixed risk',any(o['intent']['bot_id']=='testbot' and o['created_ms']>=initial for o in state['orders'].values()),state['bots']['testbot']['last_signal'])
 b.api('/api/paper/bots/control',{'bot_id':'testbot','running':False})
 ok('User account remains untouched',json.load(urllib.request.urlopen('http://127.0.0.1:4780/api/paper'))['account']==user_before)
 print(str(len(checks))+' studio checks passed',flush=True)
def cleanup():
 m=json.loads(b.MANIFEST.read_text());b.run('docker','rm','-f',m['container']+'-pine',check=False);b.cleanup()
 for n in [m['container'],m['second']]:
  for suffix in ['-jobs','-archives']:b.run('docker','volume','rm',n+suffix,check=False)
if __name__=='__main__':{'provision':b.provision,'regression':b.check,'studio':studio,'cleanup':cleanup}[sys.argv[1]]()
