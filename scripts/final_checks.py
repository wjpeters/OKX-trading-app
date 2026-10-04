import phase3_acceptance as p,json,time
b=p.b;checks=[]
def ok(label,condition,detail=None):
 checks.append({'check':label,'passed':bool(condition),'detail':detail});print(('PASS ' if condition else 'FAIL ')+label,flush=True)
 (b.ROOT/'evidence/final-integration.json').write_text(json.dumps({'at':time.time(),'scope':'isolated disposable test database','checks':checks},indent=2));assert condition,detail
b.wait(lambda:b.api('/api/health')[0]==200,30)
source='//@version=6\nindicator("Risk test")\nn=input.int(3,"Period",minval=2,maxval=10)\nplot(ta.sma(close,n),"sma")\nplot(bar_index == 40 ? 1 : 0,"entry")\nplot(bar_index == 50 ? 1 : 0,"exit")'
c,r,_=b.api('/api/studio/pine',{'source':source,'inputs':{'Period':5},'instrument':'BTC-EUR','bar':'15m'});ok('AST-backed Pine input override through API',c==200 and r['ok'] and r['inputs'][0]['type']=='int',r.get('diagnostics'))
c,r,_=b.api('/api/studio/pine',{'source':source,'inputs':{'Period':1},'instrument':'BTC-EUR','bar':'15m'});ok('Pine input outside allowed range fails closed',c==200 and not r['ok'] and not r['intents'])
config={'instrument':'BTC-EUR','bar':'15m','fast':9,'slow':21,'capital':'1000','order_eur':'100','stop_bps':'100','target_bps':'200','spread_bps':'10','slippage_bps':'5'}
before=b.state()['account']['ledger']['cash']
c,r,t=b.api('/api/replay/pine',{'source':source,'inputs':{'Period':5},'config':config});ok('Pine replay reads imported data and same runtime',c==200 and r['report']['bars']>500 and r['report']['mode']=='PINE_REPLAY',r.get('error'))
report=r['report'];first=report['first_ms'];entries=[f for f in report['fills'] if f['side']=='BUY'];ok('Pine signal cannot fill its own candle',entries and all(f['ts']>=first+41*900000 for f in entries),{'fills':len(report['fills']),'bars':report['bars'],'runtime_ms':report['pine']['duration_ms'],'http_ms':t})
ok('Pine replay preserves paper balance',b.state()['account']['ledger']['cash']==before)
c,r,_=b.api('/api/replay/pine',{'source':'//@version=6\nindicator("bad")\nta.not_real(close)','config':config});ok('Invalid Pine cannot enter replay risk engine',c==422)
c,r,_=b.api('/api/research/compare',{'with_jev':'testbot','without_jev':'testbot'});ok('Same-bot false comparison refused',c==422,r.get('error'))
ok('Operations reports local isolation',b.api('/api/operations')[1]['authentication']['binding']=='127.0.0.1 only')

c,r,_=b.api('/api/replay/reports?mode=PINE_REPLAY');ok('Pine replay report remains retrievable',c==200 and any(x['report']['mode']=='PINE_REPLAY' for x in r))
c,r,_=b.api('/api/replay/reports?mode=WALK_FORWARD');ok('Walk-forward report remains retrievable',c==200 and any(x['report']['mode']=='WALK_FORWARD' for x in r))
now=int(time.time()*1000)
c,r,_=b.api('/api/studio/alerts',{'name':'Frozen drawing-line alert','instrument':'BTC-EUR','bar':'1m','field':'drawing_line','comparison':'above','value':0,'trigger':'intrabar','cooldown_secs':60,'drawing':{'first_ms':now-60000,'second_ms':now,'first_price':1.,'second_price':1.}});ok('Drawing-line alert stored with immutable price/time anchors',c==200,r);aid=r['id']
b.wait(lambda:any(e['alert_id']==aid for e in b.api('/api/studio/alerts')[1]['events']),20);ok('Drawing-line alert delivered server-side',True);b.api('/api/studio/alerts/control',{'id':aid,'enabled':False})
c,r,_=b.api('/api/studio/alerts',{'name':'Paused bot alert','instrument':'BTC-EUR','bar':'1m','field':'bot_running','comparison':'below','value':0.5,'trigger':'intrabar','cooldown_secs':60,'bot_id':'testbot'});ok('Bot condition alert stored',c==200,r);aid=r['id']
b.wait(lambda:any(e['alert_id']==aid for e in b.api('/api/studio/alerts')[1]['events']),20);ok('Bot condition delivered without browser',True);b.api('/api/studio/alerts/control',{'id':aid,'enabled':False})
