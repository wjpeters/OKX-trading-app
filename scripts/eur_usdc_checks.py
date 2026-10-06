"""EUR/USDC acceptance. Uses only the disposable environment recorded by order_acceptance.py.
Run order_acceptance.py provision, this file, then order_acceptance.py cleanup.
Synthetic account and local intents only. Never mounts private OKX credentials.
"""
from pathlib import Path
import importlib.util,json,time
from decimal import Decimal,ROUND_DOWN
from concurrent.futures import ThreadPoolExecutor
root=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('orders',root/'scripts/order_acceptance.py');n=importlib.util.module_from_spec(spec);spec.loader.exec_module(n);p=n.p
m=json.loads(n.MANIFEST.read_text());db=m['database'];assert db.startswith('okx_orders_test_') and m['container'].startswith('okx-orders-test-')
checks=[]
def sql(s):return p.sql(db,s)
def ok(name,condition,detail=None):
 checks.append({'check':name,'passed':bool(condition),'detail':detail});(root/'evidence/eur-usdc-integration.json').write_text(json.dumps({'at':time.time(),'scope':'Disposable DB with synthetic account; public quotes; no private keys/gateway/order calls','checks':checks},indent=2));print(('PASS ' if condition else 'FAIL ')+name,flush=True);assert condition,detail
# Clear only fixture rows from the explicitly recorded disposable database.
sql("TRUNCATE execution.protection,execution.children,execution.fills,execution.orders,execution.previews,execution.events RESTART IDENTITY CASCADE")
market=p.wait(lambda:(lambda r:r[1]['market'] if r[0]==200 and r[1]['market']['books'].get('BTC-USDC') else None)(p.api('/api/market')),45)
code,catalogue,_=p.api('/api/markets');assert code==200
meta={v['instId']:v for v in catalogue['instruments']};pair='BTC-USDC';q=meta[pair];book=market['books'][pair];tick=Decimal(q['tickSz']);lot=Decimal(q['lotSz']);bid=Decimal(book['bids'][0][0]);ask=Decimal(book['asks'][0][0])
def floor(v,step):return (Decimal(v)/step).to_integral_value(rounding=ROUND_DOWN)*step
limit=str(floor(ask,tick));stop=str(floor(bid*Decimal('.9'),tick));qty=str(floor(Decimal('.03'),lot))
def ticket(order,side='sell'):
 return {'mode':'LIVE','instrument':pair,'side':side,'quantity':qty,'order':order,'protection':{'stop':stop,'target':None} if side=='buy' else None}
methods=[('market',{'type':'market','slippage_bps':100}),*[(v,{'type':'limit','policy':v,'price':limit}) for v in ['limit','post_only','ioc','fok']],('trigger',{'type':'trigger','trigger':str(floor(ask*Decimal('1.01'),tick)),'execution':{'type':'limit','price':limit,'policy':'ioc'}}),('oco',{'type':'tp_sl','take_profit':{'trigger':str(floor(ask*Decimal('1.1'),tick)),'order_price':None},'stop_loss':{'trigger':stop,'order_price':None}}),('move_order_stop',{'type':'trailing','distance':{'unit':'ratio','value':'0.01'},'activation':None}),('smart_iceberg',{'type':'iceberg','slice':'0.001','visible_orders':2,'aggressiveness':'mid','price_limit':limit,'start':{'type':'instant'}}),('twap',{'type':'twap','slice':'0.001','price_limit':limit,'interval_seconds':10,'variance':{'unit':'ratio','value':'0.001'}})]
for name,order in methods:
 c,v,_=p.api('/api/trading/preview',ticket(order));ok('USDC native contract '+name,c==200 and v['payload']['instId']==pair and v['payload']['tdMode']=='cash' and v['payload']['ordType']==name and not v['can_submit'],v if c!=200 else None)
# Synthetic Live state is confined to this fresh DB; no actual gateway or credentials.
ids=[pair,'BTC-EUR','USDC-EUR']+[v['instId'] for v in catalogue['instruments'] if v['quoteCcy'] in ['EUR','USDC'] and v['instId'] not in [pair,'BTC-EUR','USDC-EUR']][:15]
snapshot={'uid':'EUR-USDC-FIXTURE','mode':'LIVE','received_ms':0,'connected':True,'reconciled':True,'trade_permission':True,'valuation_complete':True,'equity_eur':'1000000','available':{'EUR':'1000000','USDC':'1000000','BTC':'0.04'},'holdings':{'EUR':'1000000'},'quotes':{},'tradable':ids,'fee_rate':'0.001','fee_rates':{pair:'0.001','BTC-EUR':'0.001','USDC-EUR':'0.001'},'daily_loss':'0','total_loss':'0','drawdown':'0','native_protection_verified':True,'live_acceptance_complete':False,'live_enabled':True}
def quote(id,b,a):
 v=meta[id];return {'bid':str(b),'ask':str(a),'received_ms':0,'rules':{'tick':v['tickSz'],'lot':v['lotSz'],'minimum':v['minSz'],'base':v['baseCcy'],'quote':v['quoteCcy']}}
snapshot['quotes'][pair]=quote(pair,bid,ask);b=market['books']['BTC-EUR'];snapshot['quotes']['BTC-EUR']=quote('BTC-EUR',b['bids'][0][0],b['asks'][0][0]);snapshot['quotes']['USDC-EUR']=quote('USDC-EUR','.8','.8')
def update(table,fields):
 sets=','.join(k+"='"+json.dumps(v).replace("'","''")+"'::jsonb" if k in ['snapshot','policy','live_release'] else k+'='+str(v) for k,v in fields.items());sql('UPDATE execution.'+table+' SET '+sets+" WHERE mode='LIVE'")
def refresh(stale_fx=False):
 now=int(time.time()*1000);snapshot['received_ms']=now
 for q in snapshot['quotes'].values():q['received_ms']=now
 if stale_fx:snapshot['quotes']['USDC-EUR']['received_ms']=0
 update('accounts',{'snapshot':snapshot,'updated_ms':now})
limits={'account_uid':snapshot['uid'],'instruments':ids,'capital':'1000000','max_order':'100000','max_position':'100000','trade_risk':'100000','daily_loss':'100000','total_loss':'100000','drawdown':'100000','max_orders':10,'orders_per_minute':60,'spread_bps':'500','confirmed':True}
refresh();c,v,_=p.api('/api/trading/limits',{'mode':'LIVE','limits':limits});ok('More than twelve account-allowed EUR/USDC markets can be confirmed',c==200,v if c!=200 else None)
c,v,_=p.api('/api/trading/limits',{'mode':'LIVE','limits':{**limits,'instruments':ids+['BTC-USDT']}});ok('Other quote currency cannot be confirmed',c==422)
revision=p.api('/api/trading?mode=LIVE')[1]['policy_revision'];release={'account_uid':snapshot['uid'],'policy_revision':revision,'confirmed':True,'enabled':True}
ok('Scope change still requires personal Live release',not p.api('/api/trading/preview',ticket(methods[1][1],'buy'))[1]['can_submit'])
ok('Synthetic personal release accepted only in disposable DB',p.api('/api/trading/live',release)[0]==200)
refresh();c,v,_=p.api('/api/trading/preview',ticket(methods[1][1],'buy'));ok('USDC buy produces a USDC reserve with current EUR conversion',c==200 and v['can_submit'] and v['plan']['reserve_currency']=='USDC',v if c!=200 or not v.get('can_submit') else None)
native=Decimal(v['estimated_notional']);small={**limits,'max_order':str(native*Decimal('.85'))};ok('EUR budget threshold accepts native amount only after FX',p.api('/api/trading/limits',{'mode':'LIVE','limits':small})[0]==200)
release['policy_revision']=p.api('/api/trading?mode=LIVE')[1]['policy_revision'];p.api('/api/trading/live',release);refresh();c,v,_=p.api('/api/trading/preview',ticket(methods[1][1],'buy'));ok('EUR-valued limit is distinct from USDC notional',c==200 and v['can_submit'],v if c!=200 or not v.get('can_submit') else None)
refresh(stale_fx=True);c,v,_=p.api('/api/trading/preview',ticket(methods[1][1],'buy'));ok('Stale FX prevents USDC purchase',c==200 and not v['can_submit'])
refresh();snapshot['fee_rates'].pop(pair);refresh();c,v,_=p.api('/api/trading/preview',ticket(methods[1][1],'buy'));ok('Missing per-market fee prevents USDC purchase',c==200 and not v['can_submit']);snapshot['fee_rates'][pair]='0.001';refresh()
wallet={'uid':snapshot['uid'],'mode':'LIVE','received_ms':0,'trading_received_ms':0,'trade_permission':True,'trading':[{'currency':'USDC','total':'100','available':'100','frozen':'0'},{'currency':'EUR','total':'1000000','available':'1000000','frozen':'0'},{'currency':'BTC','total':'.04','available':'.04','frozen':'0'}],'funding':[],'funding_reason':''}
def wallet_refresh():
 wallet['received_ms']=wallet['trading_received_ms']=int(time.time()*1000);update('wallets',{'snapshot':wallet,'updated_ms':wallet['received_ms']});sql("UPDATE execution.wallets SET status='BALANCES_CONNECTED' WHERE mode='LIVE'")
wallet_refresh();size={'mode':'LIVE','instrument':pair,'side':'buy','percent':50,'order':methods[1][1]};c,v,_=p.api('/api/trading/size',size);expected=floor(Decimal('50')/(Decimal(limit)*Decimal('1.001')),lot);ok('Slider spends USDC rather than the larger EUR balance',c==200 and v['currency']=='USDC' and Decimal(v['quantity'])==expected,v if c!=200 else None)
p.api('/api/trading?mode=LIVE&instrument=BTC-USDC');ok('Selected EUR/USDC market requests an isolated refresh',sql("SELECT count(*) FROM execution.market_interest WHERE mode='LIVE' AND instrument='BTC-USDC'")=='1')
# Currency-level concurrency: sell USDC/EUR and buy BTC/USDC must share free USDC.
p.api('/api/markets/watch',{'instrument':'USDC-EUR'});p.wait(lambda:p.api('/api/market')[1]['market']['books'].get('USDC-EUR'),30)
snapshot['available']['USDC']='100';refresh();p.api('/api/trading/limits',{'mode':'LIVE','limits':limits});release['policy_revision']=p.api('/api/trading?mode=LIVE')[1]['policy_revision'];p.api('/api/trading/live',release)
buy=ticket(methods[1][1],'buy');buy['quantity']=str(floor(Decimal('60')/(Decimal(limit)*Decimal('1.001')),lot))
fxbook=p.api('/api/market')[1]['market']['books']['USDC-EUR'];sell={'mode':'LIVE','instrument':'USDC-EUR','side':'sell','quantity':'60','order':{'type':'limit','policy':'limit','price':fxbook['bids'][0][0]},'protection':None}
refresh();previews=[p.api('/api/trading/preview',v)[1] for v in [buy,sell]];ok('Both currency-sharing drafts are individually valid',all(v.get('can_submit') for v in previews),previews if not all(v.get('can_submit') for v in previews) else None)
with ThreadPoolExecutor(2) as pool:results=list(pool.map(lambda v:p.api('/api/trading/submit',{'preview_id':v['preview_id'],'confirmed':True}),previews))
ok('Concurrent buy and sale cannot double spend USDC',sum(c==200 for c,v,d in results)==1,results if sum(c==200 for c,v,d in results)!=1 else None)
ok('Fixture intents use native reserve currencies without exchange fills',sql("SELECT count(*) FROM execution.orders WHERE reserve_currency='USDC' AND state='QUEUED'")=='1' and sql('SELECT count(*) FROM execution.fills')=='0')
ok('User account and paper-bot limits remain unchanged',n.production_config()==m['production_config'])
print(str(len(checks))+' EUR/USDC integration checks passed',flush=True)
