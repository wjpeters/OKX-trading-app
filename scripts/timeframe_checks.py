"""Read-only acceptance for chart timeframes; no account, alert or order writes."""
import json
import datetime as dt
import urllib.request
from pathlib import Path
bars = ["1m","3m","5m","15m","30m","1H","2H","4H","6Hutc","12Hutc","1Dutc","1Wutc","1Mutc","3Mutc"]
results=[]
for bar in bars:
    with urllib.request.urlopen("http://127.0.0.1:4780/api/candles?instrument=BTC-EUR&bar="+bar,timeout=30) as r:
        data=json.load(r)
    candles=data["candles"]
    assert data["bar"] == bar and len(candles)>0, bar
    assert data["fresh"] and data["gaps"] == 0, (bar,data["fresh"],data["gaps"])
    assert len(data["indicators"]) == len(candles), bar
    for candle in candles:
        date=dt.datetime.fromtimestamp(candle["ts"]/1000,dt.timezone.utc)
        if bar=="1Wutc": assert date.weekday()==0 and date.hour==0, (bar,date)
        if bar in ["1Mutc","3Mutc"]: assert date.day==1 and date.hour==0, (bar,date)
        if bar=="3Mutc": assert date.month in [1,4,7,10], date
    if len(candles)>=25: assert data["indicators"][-1]["ema"] is not None, bar
    record={"bar":bar,"candles":len(candles),"gaps":data["gaps"],"fresh":data["fresh"],"first":candles[0]["ts"],"last":candles[-1]["ts"],"ema_ready":data["indicators"][-1]["ema"] is not None}
    results.append(record)
    print(json.dumps(record),flush=True)
# Calendar-aware historical pagination must return only older closed periods.
for bar in ["1Wutc","1Mutc"]:
    recent=next(r for r in results if r["bar"]==bar)
    cursor=recent["last"]
    with urllib.request.urlopen(f"http://127.0.0.1:4780/api/candles?instrument=BTC-EUR&bar={bar}&after={cursor}",timeout=30) as r:
        history=json.load(r)
    assert history["historical"] and not history["fresh"] and history["gaps"]==0,bar
    assert history["candles"] and max(c["ts"] for c in history["candles"])<cursor,bar
Path("evidence/timeframes-live-2026-10-04.json").write_text(json.dumps({"coverage":results,"historical_pagination":["1Wutc","1Mutc"],"source":"Local API using public OKX EEA data","writes":"market cache only"},indent=2)+"\n")
print("Alle 14 tijdframes en twee historische paginacontroles geslaagd.")
