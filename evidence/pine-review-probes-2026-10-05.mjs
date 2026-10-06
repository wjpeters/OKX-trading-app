import {spawnSync} from 'node:child_process';
const candles=Array.from({length:600},(_,i)=>({ts:1700000040000+i*60000,open:String(100+i),high:String(102+i),low:String(99+i),close:String(101+i),volume:'10',closed:true}));
function run(body,extra={}) {
 const start=performance.now();
 const p=spawnSync(process.execPath,['--max-old-space-size=128','--permission','--allow-fs-read=/app','/app/run.mjs'],{input:JSON.stringify({instrument:'BTC-EUR',timeframe:'1m',step:60000,candles:candles.slice(0,300),source:'//@version=6\nindicator("Review")\n'+body,...extra}),encoding:'utf8',timeout:5000,maxBuffer:1200000});
 let r;try{r=JSON.parse(p.stdout)}catch{r={ok:false,error:p.error?.message,stderr:p.stderr?.slice(0,200)}};
 return {ok:r.ok,wall_ms:performance.now()-start,duration_ms:r.duration_ms,diagnostics:r.diagnostics,plots:r.plots?.map(p=>({title:p.title,first:p.data.slice(0,3),last:p.data.slice(-3)})),intent_count:r.intents?.length,first_intent:r.intents?.[0],last_intent:r.intents?.at(-1),inputs:r.inputs};
}
const output=[];
function probe(name,body,extra){output.push({name,...run(body,extra)});}
probe('core_ema_rsi_atr','plot(ta.ema(close,3),"ema")\nplot(ta.rsi(close,3),"rsi")\nplot(ta.atr(3),"atr")');
probe('custom_function_array','f(x) =>\n    x * 2\na = array.from(1.0, 2.0, 3.0)\nplot(f(array.avg(a)),"result")');
probe('negative_history','plot(close[-1],"future")');
probe('var_600_full','var float count=0\ncount := count + 1\nplot(count,"count")\nplot(count > 350 ? 1 : 0,"entry")',{candles});
probe('var_300_tail','var float count=0\ncount := count + 1\nplot(count,"count")\nplot(count > 350 ? 1 : 0,"entry")',{candles:candles.slice(-300)});
for(const [timeframe,step] of [['1m',60000],['15m',900000],['1H',3600000],['1D',86400000],['1W',604800000]]){
 probe('timeframe_'+timeframe,'plot(timeframe.in_seconds(),"seconds")\nplot(timeframe.multiplier,"multiplier")\nplot(timeframe.isminutes ? 1 : 0,"minutes")',{timeframe,step,candles:candles.slice(0,40).map((c,i)=>({...c,ts:1700000040000+i*step}))});
}
probe('metadata','plot(syminfo.mintick,"mintick")\nplot(syminfo.pointvalue,"pointvalue")');
probe('offset_negative','plot(close > 390 ? 1 : 0,"entry",offset=-10)');
probe('offset_positive','plot(close > 390 ? 1 : 0,"entry",offset=10)');
probe('twenty_plots_then_entry',Array.from({length:20},(_,i)=>`plot(close,"p${i}")`).join('\n')+'\nplot(1,"entry")');
probe('alerts','alertcondition(close>390,"signal","test")\nplot(close,"close")');
probe('numeric_options','n = input.int(3,"Period",options=[3,5,10])\nplot(ta.sma(close,n),"sma")');
probe('strategy_rejected','strategy.entry("L",strategy.long)');
probe('security_rejected','plot(request.security("BTC-EUR","D",close))');
probe('barstate_closed','plot(barstate.isconfirmed ? 1 : 0,"confirmed")\nplot(barstate.isrealtime ? 1 : 0,"realtime")');
for(const n of [300,5000]){
 const cs=Array.from({length:n},(_,i)=>({...candles[i%600],ts:1700000040000+i*60000}));
 for(let j=0;j<5;j++) probe(`benchmark_${n}_${j}`,'f=ta.sma(close,9)\ns=ta.sma(close,21)\nplot(f,"fast")\nplot(s,"slow")\nplot(ta.crossover(f,s) ? 1 : 0,"entry")\nplot(ta.crossunder(f,s) ? 1 : 0,"exit")',{candles:cs});
}
console.log(JSON.stringify({scope:'synthetic probes, same running network-none worker and permission-limited child; wall includes child startup but not broker/API; no orders or database writes',output},null,2));
