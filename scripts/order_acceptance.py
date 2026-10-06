#!/usr/bin/env python3
"""Native order control-plane acceptance. Disposable database; never calls private OKX.
Synthetic account snapshots are injected only in the newly recorded test database.
"""
import importlib.util,json,time,uuid,sys,hashlib
from pathlib import Path
from decimal import Decimal,ROUND_DOWN
from concurrent.futures import ThreadPoolExecutor
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('isolated',ROOT/'scripts/phase2_acceptance.py')
p=importlib.util.module_from_spec(spec);spec.loader.exec_module(p)
p.BASE='http://127.0.0.1:4781';p.IMAGE='okx-tradingterminal:0.4.0'
MANIFEST=ROOT/'.local/orders-test.json';EVIDENCE=ROOT/'evidence/native-orders-integration.json'
def production_config():
    raw=p.run('curl','-fsS','http://127.0.0.1:4780/api/paper').stdout
    s=json.loads(raw)['account'];return hashlib.sha256(json.dumps(None if not s else {'limits':s['limits'],'bots':{k:{'limits':b['limits'],'instrument':b.get('instrument')} for k,b in s['bots'].items()}},sort_keys=True).encode()).hexdigest()
def provision():
    assert not MANIFEST.exists(),'Bestaande testomgeving eerst opruimen'
    suffix=uuid.uuid4().hex[:10];db='okx_orders_test_'+suffix;name='okx-orders-test-'+suffix
    p.sql('postgres',f'CREATE DATABASE {db}')
    MANIFEST.write_text(json.dumps({'database':db,'container':name,'created':time.time(),'production_config':production_config()}))
    p.create(name,db,'4781');p.wait(lambda:p.api('/api/ready')[0]==200,45);print('Geïsoleerde testomgeving gereed',flush=True)
def check():
    m=json.loads(MANIFEST.read_text());db=m['database'];checks=[]
    def sql(s):return p.sql(db,s)
    def ok(label,test,detail=None):
        checks.append({'check':label,'passed':bool(test),'detail':detail});EVIDENCE.write_text(json.dumps({'at':time.time(),'scope':'disposable database; synthetic account; real public OKX quotes; zero private exchange calls','checks':checks},indent=2));print(('PASS ' if test else 'FAIL ')+label,flush=True);assert test,detail
    market=p.wait(lambda:(lambda r:r[1]['market'] if r[0]==200 and r[1]['market']['books'].get('BTC-EUR') else None)(p.api('/api/market')),45)
    meta=next(i for i in market['instruments'] if i['instId']=='BTC-EUR');book=market['books']['BTC-EUR'];tick=Decimal(meta['tickSz']);lot=Decimal(meta['lotSz']);bid=Decimal(book['bids'][0][0]);ask=Decimal(book['asks'][0][0])
    def price(v):return str((Decimal(v)/tick).to_integral_value(rounding=ROUND_DOWN)*tick)
    quantity=str((Decimal('.03')/lot).to_integral_value(rounding=ROUND_DOWN)*lot)
    stop=price(bid*Decimal('.9'));limit=price(ask)
    def ticket(order,side='sell',mode='OKX_DEMO'):
        return {'mode':mode,'instrument':'BTC-EUR','side':side,'quantity':quantity,'order':order,'protection':None if side=='sell' else {'stop':stop,'target':None}}
    methods=[('market',{'type':'market','slippage_bps':100}),*[(k,{'type':'limit','policy':k,'price':limit}) for k in ['limit','post_only','ioc','fok']],('trigger',{'type':'trigger','trigger':price(ask*Decimal('1.01')),'execution':{'type':'limit','price':price(ask*Decimal('1.02')),'policy':'ioc'}}),('oco',{'type':'tp_sl','take_profit':{'trigger':price(ask*Decimal('1.1')),'order_price':None},'stop_loss':{'trigger':stop,'order_price':None}}),('move_order_stop',{'type':'trailing','distance':{'unit':'ratio','value':'0.01'},'activation':None}),('smart_iceberg',{'type':'iceberg','slice':'0.001','visible_orders':2,'aggressiveness':'mid','price_limit':limit,'start':{'type':'instant'}}),('twap',{'type':'twap','slice':'0.001','price_limit':limit,'interval_seconds':10,'variance':{'unit':'ratio','value':'0.001'}})]
    ok('Geen keys: ordergateway meldt ontbrekende aansluiting',p.api('/api/trading?mode=OKX_DEMO')[1]['credentials_configured'] is False)
    for kind,order in methods:
        c,v,_=p.api('/api/trading/preview',ticket(order));ok('Native contract '+kind,c==200 and v['payload']['ordType']==kind and v['can_submit'] is False,v if c!=200 else None)
    c,v,_=p.api('/api/trading/preview',ticket({'type':'limit','policy':'limit','price':limit},mode='LIVE'));ok('Live-preview zonder account blijft geblokkeerd',c==200 and not v['can_submit'])
    ok('Directe Live-submit kan de blokkade niet omzeilen',p.api('/api/trading/submit',{'preview_id':v['preview_id'],'confirmed':True})[0]==422)
    ok('Caller mag geen risicomachtiging meesturen',p.api('/api/trading/preview',{**ticket(methods[0][1]),'approved':True})[0]==422)
    ok('Onjuiste prijsstap afgewezen',p.api('/api/trading/preview',ticket({'type':'limit','policy':'limit','price':str(Decimal(limit)+tick/2)}))[0]==422)
    ok('Deel groter dan totaal afgewezen',p.api('/api/trading/preview',ticket({**methods[-2][1],'slice':quantity}))[0]==422)
    ok('Ontbrekende Origin geblokkeerd',p.api('/api/trading/submit',{},headers={'Content-Type':'application/json','X-Terminal-Request':'paper-v2'})[0]==403)
    snapshot={'uid':'SYNTHETIC-ONLY','mode':'OKX_DEMO','received_ms':0,'connected':True,'reconciled':True,'trade_permission':True,'valuation_complete':True,'equity_eur':'1000000','available':{'EUR':'1000000','BTC':'0.04'},'holdings':{'EUR':'1000000'},'quotes':{'BTC-EUR':{'bid':str(bid),'ask':str(ask),'received_ms':0,'rules':{'tick':meta['tickSz'],'lot':meta['lotSz'],'minimum':meta['minSz'],'base':'BTC','quote':'EUR'}}},'tradable':['BTC-EUR'],'fee_rate':'0.001','daily_loss':'0','total_loss':'0','drawdown':'0','native_protection_verified':True,'live_acceptance_complete':False}
    def refresh(**changes):
        now=int(time.time()*1000);snapshot.update(changes);snapshot['received_ms']=now;snapshot['quotes']['BTC-EUR']['received_ms']=now
        sql("UPDATE execution.accounts SET snapshot='"+json.dumps(snapshot)+"'::jsonb,status='ACCOUNT_CONNECTED',updated_ms="+str(now)+" WHERE mode='OKX_DEMO'")
    refresh()
    limits={'account_uid':'SYNTHETIC-ONLY','instruments':['BTC-EUR'],'capital':'1000000','max_order':'100000','max_position':'100000','trade_risk':'100000','daily_loss':'100000','total_loss':'100000','drawdown':'100000','max_orders':10,'orders_per_minute':60,'spread_bps':'500','confirmed':True}
    ok('Limieten vereisen persoonlijke bevestiging',p.api('/api/trading/limits',{'mode':'OKX_DEMO','limits':{**limits,'confirmed':False}})[0]==422)
    ok('Synthetische accountlimieten in testdatabase geaccepteerd',p.api('/api/trading/limits',{'mode':'OKX_DEMO','limits':limits})[0]==200)
    refresh();order=ticket({'type':'limit','policy':'limit','price':limit});previews=[p.api('/api/trading/preview',order)[1] for _ in range(2)];ok('Previews reserveren geen saldo',sql('SELECT count(*) FROM execution.orders')=='0')
    with ThreadPoolExecutor(2) as pool:results=list(pool.map(lambda v:p.api('/api/trading/submit',{'preview_id':v['preview_id'],'confirmed':True}),previews))
    winners=[v for c,v,_ in results if c==200];ok('Twee verkopen kunnen inventaris niet dubbel reserveren',len(winners)==1,results if len(winners)!=1 else None)
    winner=winners[0]['client_id'];selected=next(v for v in previews if v['client_id']==winner)
    c,v,_=p.api('/api/trading/submit',{'preview_id':selected['preview_id'],'confirmed':True});ok('Herhaalde bevestiging krijgt dezelfde order',c==200 and v['duplicate'] and v['client_id']==winner)
    ok('Geen fill bij duurzaam opgeslagen intent',sql('SELECT count(*) FROM execution.fills')=='0' and sql("SELECT state FROM execution.orders WHERE client_id='"+winner+"'")=='QUEUED')
    p.run('docker','restart',m['container']);p.wait(lambda:p.api('/api/ready')[0]==200,45)
    ok('Order en reservering overleven serverherstart',sql("SELECT (state='QUEUED' AND reserve>0)::text FROM execution.orders WHERE client_id='"+winner+"'")=='true')
    ok('Annuleren vóór verzending veilig lokaal afgehandeld',p.api('/api/trading/cancel',{'mode':'OKX_DEMO','client_id':winner})[0]==200 and sql("SELECT (state='CANCELED' AND reserve=0)::text FROM execution.orders WHERE client_id='"+winner+"'")=='true')
    refresh();c,v,_=p.api('/api/trading/preview',order);sql("UPDATE execution.previews SET expires_ms=0 WHERE id='"+v['preview_id']+"'");ok('Verlopen preview kan niet verzonden worden',p.api('/api/trading/submit',{'preview_id':v['preview_id'],'confirmed':True})[0]==422)
    refresh(connected=False);c,v,_=p.api('/api/trading/preview',order);ok('Onderbroken private verbinding blokkeert nieuwe order',c==200 and not v['can_submit']);refresh(connected=True)
    # Real DB uniqueness, rather than a simulated in-memory ledger.
    statement="INSERT INTO execution.fills(mode,bill_id,parent_id,exchange_id,instrument,side,quantity,price,fee,fee_currency,fill_ms,detail) VALUES('OKX_DEMO','testbill','"+winner+"','synthetic-native','BTC-EUR','buy',1,100,-0.001,'BTC',1,'{}') ON CONFLICT DO NOTHING"
    sql(statement);sql(statement);ok('Dubbele fill boekt exact eenmaal met basefee',sql("SELECT (count(*)=1 AND sum(quantity+fee)=0.999)::text FROM execution.fills WHERE bill_id='testbill'")=='true')
    sql("INSERT INTO execution.protection(mode,parent_id,client_id,quantity,state,created_ms) VALUES('OKX_DEMO','"+winner+"','synthetic-stop',0.03,'ACTIVE',1)")
    refresh();c,v,_=p.api('/api/trading/preview',order);ok('Beschermde inventaris kan niet opnieuw verkocht worden',c==200 and not v['can_submit'])
    ok('Bevestigde fills en bescherming leesbaar',p.api('/api/trading/activity?mode=OKX_DEMO')[0]==200 and len(p.api('/api/trading/activity?mode=OKX_DEMO')[1]['fills'])==1)
    ok('Demo- en Live-administratie gescheiden',sql("SELECT count(*) FROM execution.orders WHERE mode='LIVE'")=='0' and p.api('/api/trading/activity?mode=LIVE')[1]['fills']==[])
    ok('Gebruikersrekening en botlimieten behouden',production_config()==m['production_config'])
    print(f'{len(checks)} controles geslaagd; nul private OKX-calls',flush=True)
def cleanup():
    m=json.loads(MANIFEST.read_text());assert m['database'].startswith('okx_orders_test_') and m['container'].startswith('okx-orders-test-')
    p.run('docker','rm','-f',m['container'],check=False);p.sql('postgres','DROP DATABASE '+m['database']+' WITH (FORCE)');MANIFEST.unlink();print('Uitsluitend opgenomen testomgeving opgeruimd')
if __name__=='__main__':{'provision':provision,'check':check,'cleanup':cleanup}[sys.argv[1]]()
