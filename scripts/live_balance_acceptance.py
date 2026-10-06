#!/usr/bin/env python3
"""New balance/release checks only in the disposable orders-test database.
No credentials are mounted and no private exchange executor runs here.
"""
import importlib.util,json,time,sys
from pathlib import Path
from decimal import Decimal,ROUND_DOWN
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('native',ROOT/'scripts/order_acceptance.py')
n=importlib.util.module_from_spec(spec);spec.loader.exec_module(n)
p=n.p
def check():
    m=json.loads(n.MANIFEST.read_text());db=m['database']
    assert db.startswith('okx_orders_test_') and m['container'].startswith('okx-orders-test-')
    def sql(s):return p.sql(db,s)
    def update(table,mode,fields):
        values=','.join(k+"='"+json.dumps(v).replace("'","''")+"'::jsonb" if k in ['snapshot','policy','live_release'] else k+'='+str(v) for k,v in fields.items())
        sql('UPDATE execution.'+table+' SET '+values+" WHERE mode='"+mode+"'")
    checks=[]
    def ok(label,test,detail=None):
        checks.append({'check':label,'passed':bool(test),'detail':detail})
        (ROOT/'evidence/live-balances-integration.json').write_text(json.dumps({'at':time.time(),'scope':'new disposable DB; synthetic accounts; zero private calls','checks':checks},indent=2))
        print(('PASS ' if test else 'FAIL ')+label,flush=True);assert test,detail
    market=p.wait(lambda:(lambda r:r[1]['market'] if r[0]==200 and r[1]['market']['books'].get('BTC-EUR') else None)(p.api('/api/market')),45)
    meta=next(i for i in market['instruments'] if i['instId']=='BTC-EUR');book=market['books']['BTC-EUR'];lot=Decimal(meta['lotSz']);tick=Decimal(meta['tickSz']);bid=Decimal(book['bids'][0][0]);ask=Decimal(book['asks'][0][0])
    limit=str((ask/tick).to_integral_value(rounding=ROUND_DOWN)*tick)
    snapshot={'uid':'LIVE-FIXTURE-ONLY','mode':'LIVE','received_ms':0,'connected':True,'reconciled':True,'trade_permission':True,'valuation_complete':True,'equity_eur':'1000000','available':{'EUR':'100','BTC':'0.04'},'holdings':{'EUR':'100','BTC':'0.04'},'quotes':{'BTC-EUR':{'bid':str(bid),'ask':str(ask),'received_ms':0,'rules':{'tick':meta['tickSz'],'lot':meta['lotSz'],'minimum':meta['minSz'],'base':'BTC','quote':'EUR'}}},'tradable':['BTC-EUR'],'fee_rate':'0.001','daily_loss':'0','total_loss':'0','drawdown':'0','native_protection_verified':True,'live_acceptance_complete':False,'live_enabled':True}
    def refresh(**changes):
        snapshot.update(changes);snapshot['received_ms']=int(time.time()*1000);snapshot['quotes']['BTC-EUR']['received_ms']=snapshot['received_ms'];update('accounts','LIVE',{'snapshot':snapshot,'updated_ms':snapshot['received_ms']})
    wallet={'uid':snapshot['uid'],'mode':'LIVE','received_ms':int(time.time()*1000),'trading_received_ms':int(time.time()*1000),'trade_permission':False,'trading':[{'currency':'BTC','total':'1','available':'0.04','frozen':'0.96'},{'currency':'EUR','total':'100','available':'100','frozen':'0'},{'currency':'USDT','total':'7','available':'5','frozen':'2'}],'funding':[{'currency':'BTC','total':'1000','available':'1000','frozen':'0'}],'funding_reason':''}
    def wallet_refresh():
        wallet['received_ms']=wallet['trading_received_ms']=int(time.time()*1000);update('wallets','LIVE',{'snapshot':wallet,'updated_ms':wallet['received_ms']});sql("UPDATE execution.wallets SET status='BALANCES_CONNECTED' WHERE mode='LIVE'")
    limits={'account_uid':snapshot['uid'],'instruments':['BTC-EUR'],'capital':'1000000','max_order':'100000','max_position':'100000','trade_risk':'100000','daily_loss':'100000','total_loss':'100000','drawdown':'100000','max_orders':10,'orders_per_minute':60,'spread_bps':'500','confirmed':True}
    order={'mode':'LIVE','instrument':'BTC-EUR','side':'sell','quantity':'0.01','order':{'type':'limit','policy':'limit','price':limit},'protection':None}
    sized={'mode':'LIVE','instrument':'BTC-EUR','side':'sell','percent':50,'order':{'type':'market','slippage_bps':100}}
    release={'account_uid':snapshot['uid'],'policy_revision':0,'enabled':True,'confirmed':True}
    ok('Geen aansluiting geeft geen verzonnen saldo',p.api('/api/trading/balances?mode=LIVE')[1]['snapshot'] is None)
    refresh();wallet_refresh()
    b=p.api('/api/trading/balances?mode=LIVE')[1]
    ok('Alle munten en wallets leesbaar zonder handelsvrijgave',len(b['snapshot']['trading'])==3 and b['snapshot']['funding'][0]['available']=='1000')
    ok('Verkoopslider gebruikt vrije BTC en geen funding of frozen',Decimal(p.api('/api/trading/size',sized)[1]['quantity'])==Decimal('0.02'))
    c,v,_=p.api('/api/trading/size',{**sized,'side':'buy'})
    expected=(Decimal('50')/(Decimal(v.get('price_bound','1'))*Decimal('1.001'))/lot).to_integral_value(rounding=ROUND_DOWN)*lot
    ok('Koopslider gebruikt EUR met fees, slippage en lotafronding',c==200 and Decimal(v['quantity'])==expected,v if c!=200 else None)
    ok('Percentage groter dan honderd afgewezen',p.api('/api/trading/size',{**sized,'percent':101})[0]==422)
    ok('Caller kan geen size-machtiging invoeren',p.api('/api/trading/size',{**sized,'approved':True})[0]==422)
    ok('Live-vrijgave zonder limieten afgewezen',p.api('/api/trading/live',release)[0]==422)
    ok('Fake snapshot-vlag geeft geen Live-machtiging',not p.api('/api/trading/preview',order)[1]['can_submit'])
    ok('Live-limieten vragen persoonlijke bevestiging',p.api('/api/trading/limits',{'mode':'LIVE','limits':{**limits,'confirmed':False}})[0]==422)
    ok('Synthetische Live-limieten alleen in testdatabase bevestigd',p.api('/api/trading/limits',{'mode':'LIVE','limits':limits})[0]==200)
    revision=p.api('/api/trading?mode=LIVE')[1]['policy_revision'];release['policy_revision']=revision
    ok('Vrijgave zonder bevestiging afgewezen',p.api('/api/trading/live',{**release,'confirmed':False})[0]==422)
    ok('Vrijgave voor ander account afgewezen',p.api('/api/trading/live',{**release,'account_uid':'OTHER'})[0]==422)
    ok('Vrijgave voor oude limieten afgewezen',p.api('/api/trading/live',{**release,'policy_revision':revision-1})[0]==422)
    refresh(trade_permission=False);ok('Leesrechten geven geen handelsvrijgave',p.api('/api/trading/live',release)[0]==422)
    refresh(trade_permission=True,reconciled=False);ok('Onverklaarde accountstatus geeft geen vrijgave',p.api('/api/trading/live',release)[0]==422)
    refresh(reconciled=True);ok('Actueel bevestigd account kan persoonlijk worden vrijgegeven',p.api('/api/trading/live',release)[0]==200)
    c,v,_=p.api('/api/trading/preview',order);ok('Vrijgegeven Live maakt uitvoerbare exacte preview',c==200 and v['can_submit'],v)
    cid=v['client_id'];ok('Live-confirmatie bewaart intent zonder exchangefill',p.api('/api/trading/submit',{'preview_id':v['preview_id'],'confirmed':True})[0]==200 and sql("SELECT state FROM execution.orders WHERE client_id='"+cid+"'")=='QUEUED' and sql("SELECT count(*) FROM execution.fills WHERE mode='LIVE'")=='0')
    wallet_refresh();ok('Slider vermindert saldo met lokale orderclaims',Decimal(p.api('/api/trading/size',sized)[1]['quantity'])==Decimal('0.015'))
    fresh_preview=p.api('/api/trading/preview',order)[1]
    ok('Live pauzeren trekt vrijgave en niet-verzonden claims in',p.api('/api/trading/live',{**release,'enabled':False})[0]==200 and sql("SELECT (state='REJECTED' AND reserve=0)::text FROM execution.orders WHERE client_id='"+cid+"'")=='true')
    ok('Preview van voor de pauze kan geen nieuwe order indienen',p.api('/api/trading/submit',{'preview_id':fresh_preview['preview_id'],'confirmed':True})[0]==422)
    refresh();ok('Live opnieuw vrijgeven na nieuwe controle',p.api('/api/trading/live',release)[0]==200)
    ok('Limietwijziging trekt eerdere Live-vrijgave in',p.api('/api/trading/limits',{'mode':'LIVE','limits':limits})[0]==200 and p.api('/api/trading?mode=LIVE')[1]['live_release'] is None)
    wallet['trading_received_ms']=0;update('wallets','LIVE',{'snapshot':wallet});ok('Verouderd saldo mag geen hoeveelheid berekenen',p.api('/api/trading/size',sized)[0]==422)
    wallet_refresh();wallet['trading'][0]['available']=None;update('wallets','LIVE',{'snapshot':wallet});ok('Onbekend beschikbaar saldo wordt geen nul of totaal',p.api('/api/trading/size',sized)[0]==422)
    ok('Gebruikersrekening en botlimieten behouden',n.production_config()==m['production_config'])
    print(str(len(checks))+' saldo- en vrijgavecontroles geslaagd; nul private calls')
if __name__=='__main__':{'provision':n.provision,'check':check,'native':n.check,'cleanup':n.cleanup}[sys.argv[1]]()
