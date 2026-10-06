import { PineTS, Indicator } from 'pinets';
let raw=''; for await (const chunk of process.stdin) {raw+=chunk;if(raw.length>1_000_000)throw Error('INPUT_LIMIT');}
const started=performance.now();
try {
 const job=JSON.parse(raw),source=job.source;
 if(typeof source!=='string'||source.length>12000||!/^\s*\/\/@version=[56]\s*\n/.test(source))throw Error('Regel 1: echte Pine v5/v6-bron met //@version=5 of 6 vereist.');
 // Tokenize strings/comments separately: this is a capability filter, never a transpiler.
 const code=source.replace(/"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|\/\/[^\n]*/g,' ');
 const denied=/\b(request|strategy|varip|import|export|process|globalThis|global|require|eval|Function|constructor|prototype|__proto__|fetch|WebSocket)\b/;
 const supportedTA=new Set(["tr", "bb", "median", "rci", "param", "pivot_point_levels", "vwma", "linreg", "valuewhen", "range", "rma", "vwap", "stdev", "iii", "min", "percentrank", "bbw", "max", "crossover", "cmo", "dev", "pvt", "hma", "sar", "highest", "percentile_nearest_rank", "rising", "mfi", "mom", "supertrend", "lowest", "lowestbars", "roc", "nvi", "kc", "pivothigh", "rsi", "mode", "accdist", "correlation", "wma", "highestbars", "stoch", "wpr", "obv", "atr", "percentile_linear_interpolation", "kcw", "swma", "pivotlow", "wvad", "variance", "barssince", "cross", "macd", "change", "sma", "falling", "cci", "wad", "cum", "ema", "tsi", "pvi", "cog", "dmi", "alma", "crossunder"]);
 for(const call of code.matchAll(/\bta\.(\w+)\s*\(/g)){if(!supportedTA.has(call[1]))throw Error(`Onbekende TA-functie: ta.${call[1]}`);}
 const hit=code.match(denied);
 if(hit){const line=source.slice(0,source.indexOf(hit[0])).split('\n').length;throw Error(`Regel ${line}: ${hit[0]} is niet vrijgegeven in closed-bar Pine-profiel 1. Gebruik entry/exit-plots voor voorstellen. Zie compatibiliteitsmatrix.`);}
 if(!Array.isArray(job.candles)||job.candles.length<2||job.candles.length>5000)throw Error('CANDLE_LIMIT');
 if(job.candles.some(c=>!c.closed))throw Error('Alleen definitief gesloten candles zijn in dit profiel toegestaan.');
 const candles=job.candles.map(c=>({openTime:c.ts,closeTime:c.ts+job.step-1,open:+c.open,high:+c.high,low:+c.low,close:+c.close,volume:+c.volume}));
 for(let i=1;i<candles.length;i++)if(candles[i].openTime-candles[i-1].openTime!==job.step)throw Error('Datagat: geen Pine-uitvoering op onvolledige historie.');
 const pine=new PineTS(candles,job.instrument,job.timeframe);pine.setMaxLoops(10000);pine.setAlertMode('all');
 const indicator=new Indicator(source);
 const inputs=indicator.getInputsMeta();
 if(inputs.length>40)throw Error('Maximaal 40 inputs.');
 const overrides=job.inputs||{};
 if(typeof overrides!=='object'||Array.isArray(overrides))throw Error('Ongeldige inputs');
 for(const [key,value] of Object.entries(overrides)){
   const m=inputs.find(m=>m.name===key||m.id===key||m.title===key);if(!m)throw Error(`Onbekende input: ${key}`);
   if(['int','float','price','time'].includes(m.type)){
     if(typeof value!=='number'||!Number.isFinite(value)||Math.abs(value)>1e15||(m.type==='int'&&!Number.isInteger(value))||(m.minval!==undefined&&value<m.minval)||(m.maxval!==undefined&&value>m.maxval))throw Error(`Input buiten bereik: ${key}`);
   }else if(m.type==='bool'){if(typeof value!=='boolean')throw Error(`Booleaanse input vereist: ${key}`);}
   else if(['string','color','session','symbol','timeframe','text_area','enum'].includes(m.type)){if(typeof value!=='string'||value.length>200)throw Error(`Tekstinput ongeldig: ${key}`);}
   else throw Error(`Inputoverride nog niet gevalideerd: ${m.type}`);
   if(m.options&&!m.options.includes(value))throw Error(`Input niet in opties: ${key}`);
   indicator.input[m.name]=value;
 }
 const ctx=await pine.run(indicator);
 if(Object.keys(ctx.plots||{}).filter(k=>!k.startsWith('__')).length>20)throw Error('Maximaal 20 plots; verminder het aantal reeksen. Signalen worden nooit stil afgekapt.');
 const plots=Object.entries(ctx.plots||{}).filter(([k])=>!k.startsWith('__')).slice(0,20).map(([title,p])=>({title:String(title).slice(0,120),data:(p.data||[]).slice(-5000).map(v=>({time:v.time,value:Number.isFinite(v.value)?v.value:null})),options:{color:typeof p.options?.color==='string'?p.options.color:null}}));
 const intents=plots.filter(p=>p.title==='entry'||p.title==='exit').flatMap(p=>p.data.filter(v=>v.value!==null&&v.value>0).map(v=>({bar_ms:v.time,action:p.title==='entry'?'CONSIDER_ENTRY':'CONSIDER_EXIT',authority:'PROPOSAL_ONLY'})));
 process.stdout.write(JSON.stringify({ok:true,plots,intents,inputs,runtime:'PineTS 0.11.0 / closed-bar profile 1',duration_ms:performance.now()-started,bars:candles.length,diagnostics:[]}));
} catch(e){process.stdout.write(JSON.stringify({ok:false,plots:[],intents:[],diagnostics:[{line:Number(String(e.message).match(/(?:line|Regel)\s*(\d+)/i)?.[1]||1),message:String(e.message).slice(0,1600)}],duration_ms:performance.now()-started}));process.exitCode=1;}
