#!/usr/bin/env python3
"""System bot acceptance against a disposable database only."""
import phase3_acceptance as p
import json,sys,copy,urllib.request
b=p.b
b.MANIFEST=b.ROOT/'.local/system-bots-test.json'
b.IMAGE='okx-tradingterminal:0.4.0'
checks=[]
def api(path,body=None):
 c,r,_=b.api(path,body);return c,r
def ok(label,result):
 checks.append({'check':label,'passed':bool(result)})
 (b.ROOT/'evidence/system-bots-integration.json').write_text(json.dumps(checks,indent=2))
 print(('PASS ' if result else 'FAIL ')+label,flush=True);assert result,label
def configuration(account):
 if account is None:return None
 return {'limits':account['limits'],'bots':{k:{f:v for f,v in bot.items() if f not in ['ledger','last_bar','last_signal','running']} for k,bot in account['bots'].items()}}
def check():
 before=json.load(urllib.request.urlopen('http://127.0.0.1:4780/api/paper'))['account']
 c,catalog=api('/api/studio/system-bots');ok('Five built-in system bots available',c==200 and len(catalog)==5)
 s=b.state();ok('Examples need no user account',s['account'] is None)
 for item in catalog:
  g=copy.deepcopy(item['graph']);g['bot']={'mode':'paper','ai_enabled':False,'bar':'1m','config':{'id':'system-'+item['id'],'name':item['name'],'instrument':'BTC-EUR','limits':s['presets']['manual'],'fast':9,'slow':21,'order_eur':'100','stop_bps':'100','target_bps':'200'}}
  ok(item['name']+' validates without AI',api('/api/studio/graph/validate',g)[0]==200)
  ok(item['name']+' saved as editable copy',api('/api/studio/documents',{'kind':'graph','name':item['name'],'base_version':0,'payload':g})[0]==200)
  c,r=api('/api/replay/graph',{'name':item['name'],'version':1,'spread_bps':'10','slippage_bps':'5'})
  ok(item['name']+' backtests actual saved graph',c==200 and r['report']['graph']==g and r['report']['bars']>100)
  if item['id']=='sma':sample=g
 bad=copy.deepcopy(sample);bad['nodes'][1]['kind']='jev';bad['nodes'][1]['params']={}
 c,r=api('/api/studio/graph/validate',bad);ok('Jev rejected when AI is off',c==422 and 'AI staat uit' in str(r))
 bad['bot']['ai_enabled']=True;ok('Jev only allowed with explicit opt-in',api('/api/studio/graph/validate',bad)[0]==200)
 bad=copy.deepcopy(sample);bad['nodes'][2]['params']['drop_pct']=0;ok('Zero decline threshold rejected',api('/api/studio/graph/validate',bad)[0]==422)
 for action,signal in [('close_pause','STOP_CLOSE'),('pause','STOP_PAUSE'),('close','EXIT_NOW')]:
  guard=copy.deepcopy(sample);guard['nodes'][1]['kind']='action';guard['nodes'][1]['params']={'action':'buy'};guard['nodes'][2]['params']={'drop_pct':0.01,'action':action}
  name='Guard '+action
  ok(name+' saved',api('/api/studio/documents',{'kind':'graph','name':name,'base_version':0,'payload':guard})[0]==200)
  c,r=api('/api/replay/graph',{'name':name,'version':1,'spread_bps':'10','slippage_bps':'5'})
  ds=r.get('report',{}).get('decisions',[])
  stops=[d for d in ds if d['signal']==signal]
  ok(name+' fires on actual historical candle',c==200 and len(stops)>0)
  if action!='close':ok(name+' remains paused after trigger',not any(d['ts']>stops[0]['ts'] and d['signal']=='BUY' for d in ds))
 ok('Immutable catalog unchanged after edits' ,api('/api/studio/system-bots')[1]==catalog)
 c,research=api('/api/research');ok('No AI configured or spent',c==200 and not research['jev']['configured'] and research['jev']['spent_usd']=='0')
 ok('Backtests did not initialize or trade account',b.state()['account'] is None)
 ok('User configuration untouched',configuration(json.load(urllib.request.urlopen('http://127.0.0.1:4780/api/paper'))['account'])==configuration(before))
 print(str(len(checks))+' checks passed',flush=True)
if __name__=='__main__':{'provision':b.provision,'check':check,'cleanup':p.cleanup}[sys.argv[1]]()
