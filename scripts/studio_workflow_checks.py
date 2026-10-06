#!/usr/bin/env python3
"""Disposable integration tests for the unified studio. Never trades the user account."""
import phase3_acceptance as p
import json,sys,time,copy,urllib.request
b=p.b
b.MANIFEST=b.ROOT/'.local/studio-workflow-test.json'
b.IMAGE='okx-tradingterminal:0.4.0'
checks=[]
def ok(label,condition,detail=None):
 checks.append({'check':label,'passed':bool(condition),'detail':detail})
 (b.ROOT/'evidence/studio-workflow-integration.json').write_text(json.dumps({'scope':'disposable database on port 4781','checks':checks},indent=2))
 print(('PASS ' if condition else 'FAIL ')+label,flush=True)
 assert condition,detail
def api(path,body=None):
 c,r,_=b.api(path,body)
 return c,r
def save(name,g,version=0,kind='graph'):
 return api('/api/studio/documents',{'kind':kind,'name':name,'base_version':version,'payload':g})
def check():
 before=json.load(urllib.request.urlopen('http://127.0.0.1:4780/api/paper'))['account']
 state=b.wait(lambda:b.state() if b.state()['worker_alive'] else None,30)
 ok('Fresh isolated account',state['account'] is None)
 ok('Setup disposable paper account',api('/api/paper/setup',{'account':state['presets']['account'],'manual':state['presets']['manual'],'confirmed':True})[0]==200)
 limits=state['presets']['manual']
 config={'mode':'paper','bar':'1m','config':{'id':'workflow-bot','name':'Studio if-else','instrument':'BTC-EUR','limits':limits,'fast':9,'slow':21,'order_eur':'100','stop_bps':'100','target_bps':'200'}}
 g={'schema':1,'bot':config,'nodes':[{'id':'m','kind':'market','params':{}},{'id':'f','kind':'if_else','params':{'field':'rsi','comparison':'above','value':50}},{'id':'yes','kind':'action','params':{'action':'buy'}},{'id':'no','kind':'action','params':{'action':'sell'}},{'id':'join','kind':'or','params':{}},{'id':'i','kind':'intent','params':{}},{'id':'r','kind':'risk_gate','params':{}}],'edges':[{'source':'m','target':'f'},{'source':'f','target':'yes','sourceHandle':'true'},{'source':'f','target':'no','sourceHandle':'false'},{'source':'yes','target':'join'},{'source':'no','target':'join'},{'source':'join','target':'i'},{'source':'i','target':'r'}]}
 for i,n in enumerate(g['nodes']):n['position']={'x':i*220,'y':100}
 g['nodes'][2]['position']={'x':440,'y':10};g['nodes'][3]['position']={'x':440,'y':210}
 ok('If/else graph validates',api('/api/studio/graph/validate',g)[0]==200)
 bad=copy.deepcopy(g);bad['edges'][1].pop('sourceHandle');ok('Missing branch handle rejected',api('/api/studio/graph/validate',bad)[0]==422)
 ok('Bot settings saved with graph',save('Studio if-else',g)[0]==200)
 ok('Optimistic version conflict rejected',save('Studio if-else',g)[0]==422)
 c,rows=api('/api/studio/documents?kind=graph');ok('Reload preserves branch handles and settings',rows[0]['payload']==g)
 b.wait(lambda:api('/api/studio/install',{'name':'Studio if-else','version':1})[0]==200,30)
 bot=b.state()['account']['bots']['workflow-bot'];ok('Atomic install creates pinned paused bot',not bot['running'] and bot['version']=='graph:Studio if-else:1')
 live=copy.deepcopy(g);live['bot']['mode']='live';ok('Live design can be saved',save('Studio if-else',live,1)[0]==200)
 ok('Live installation blocked',api('/api/studio/install',{'name':'Studio if-else','version':2})[0]==422)
 ok('Legacy deployment cannot bypass saved mode',api('/api/studio/deploy',{'name':'Studio if-else','version':2,'bot_id':'workflow-bot'})[0]==422)
 c,r=api('/api/replay/graph',{'name':'Studio if-else','version':1,'spread_bps':'10','slippage_bps':'5'});ok('Full graph backtest executes and preserves limits',c==200 and r['report']['mode']=='GRAPH_REPLAY' and r['report']['limits']==limits,r.get('error'))
 ok('Backtest records both branches',set(d['signal'] for d in r['report']['decisions'])=={'BUY','SELL'})
 first=r['report']['first_ms'];ok('No fill before signal candle closes',all(f['ts']>=first+101*60000 for f in r['report']['fills']))
 ok('Backtest has no paper side effects',not b.state()['account']['orders'])
 c,reports=api('/api/replay/reports?mode=GRAPH_REPLAY');ok('Saved backtest reloads',c==200 and reports[0]['report']['graph']==g)
 source='//@version=6\nindicator("Studio indicator", overlay=true)\nn=input.int(3,"Periode",options=[3,5,10])\nplot(ta.sma(close,n),"Gemiddelde")\nplot(bar_index % 20 == 0 ? 1 : 0,"entry")\nplot(bar_index % 20 == 10 ? 1 : 0,"exit")'
 pine={'source':source,'inputs':{'Periode':5},'display':'overlay','scriptKind':'strategy'}
 c,r=api('/api/studio/pine',{'source':source,'inputs':pine['inputs'],'instrument':'BTC-EUR','bar':'1m'});ok('Pine numerical input and chart plot compute',c==200 and r.get('ok') and len(r['plots'])==3,r.get('diagnostics'))
 ok('Pine exact version saves',save('Studio Pine',pine,kind='pine')[0]==200)
 pg=copy.deepcopy(g);pg['nodes']=[pg['nodes'][0],{'id':'p','kind':'pine','params':{'name':'Studio Pine','version':1},'position':{'x':220,'y':100}},pg['nodes'][-2],pg['nodes'][-1]];pg['edges']=[{'source':'m','target':'p'},{'source':'p','target':'i'},{'source':'i','target':'r'}];pg['bot']['config']['name']='Studio Pine bot';pg['bot']['config']['id']='pine-workflow'
 ok('Pine graph saves',save('Studio Pine bot',pg)[0]==200)
 c,r=api('/api/replay/graph',{'name':'Studio Pine bot','version':1,'spread_bps':'10','slippage_bps':'5'});ok('Pine node backtest uses exact script and inputs',c==200 and r['report']['pine_versions'][0]['payload']==pine and r['report']['fill_count']>0,r.get('error'))
 c,r=api('/api/studio/install',{'name':'Studio Pine bot','version':1});ok('Pine bot compile-checked before installation',c==200,r)
 pg['nodes'][1]['params'].update(plot='Gemiddelde',comparison='above',value=0);ok('Pine indicator plot node saves',save('Studio Pine bot',pg,1)[0]==200)
 c,r=api('/api/replay/graph',{'name':'Studio Pine bot','version':2,'spread_bps':'10','slippage_bps':'5'});ok('Pine indicator output usable as node condition',c==200 and any(d['signal']=='BUY' for d in r['report']['decisions']),r.get('error'))
 overflow='//@version=6\nindicator("Overflow")\n'+'\n'.join('plot(close,"p'+str(i)+'")' for i in range(20))+'\nplot(1,"entry")'
 c,r=api('/api/studio/pine',{'source':overflow,'instrument':'BTC-EUR','bar':'1m'});ok('Excess plots fail instead of dropping signals',c==200 and not r['ok'] and not r['intents'])
 bad=copy.deepcopy(g);bad['bot']['config']['limits']['capital']='0';ok('Invalid budget rejected before saving',save('Invalid budget',bad)[0]==422)
 ok('User account remains unchanged',json.load(urllib.request.urlopen('http://127.0.0.1:4780/api/paper'))['account']==before)
 print(str(len(checks))+' checks passed',flush=True)
if __name__=='__main__':
 {'provision':b.provision,'check':check,'cleanup':p.cleanup}[sys.argv[1]]()
