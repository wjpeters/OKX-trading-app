import {spawnSync} from 'node:child_process';import assert from 'node:assert/strict';
const candles=Array.from({length:40},(_,i)=>({ts:1700000000000+i*60000,open:String(i+1),high:String(i+2),low:String(i+.5),close:String(i+1),volume:'10',closed:true}));
const base={instrument:'BTC-EUR',timeframe:'1',step:60000,candles};
function run(source,extra={}){const r=spawnSync(process.execPath,['--max-old-space-size=128','--permission','--allow-fs-read=/app','/app/run.mjs'],{input:JSON.stringify({...base,...extra,source}),encoding:'utf8',timeout:5000,maxBuffer:1000000});if(r.error)throw r.error;return JSON.parse(r.stdout)}
const cases=[];function test(name,fn){fn();cases.push({name,passed:true});}
test('real Pine v6 SMA warm-up and values',()=>{const r=run('//@version=6\nindicator("SMA")\nplot(ta.sma(close, 3), "sma")');assert.equal(r.ok,true);const a=r.plots[0].data.map(x=>x.value);assert.deepEqual(a.slice(0,4),[null,null,2,3]);assert.equal(a.at(-1),39)});
test('history never reads future bar',()=>{const r=run('//@version=5\nindicator("History")\nplot(close[1], "previous")');assert.equal(r.ok,true);assert.equal(r.plots[0].data.at(-1).value,39)});
test('var persists on closed bars',()=>{const r=run('//@version=6\nindicator("Var")\nvar float count = 0\ncount := count + 1\nplot(count, "count")');assert.equal(r.ok,true);assert.equal(r.plots[0].data.at(-1).value,40)});
test('unknown TA is rejected even in dead branch',()=>{const r=run('//@version=6\nindicator("Bad")\nif false\n    ta.not_a_function(close)\nplot(close)');assert.equal(r.ok,false);assert.equal(r.intents.length,0)});
test('invalid Pine has no executable proposals',()=>{const r=run('//@version=6\nindicator("Bad")\nplot(close + )');assert.equal(r.ok,false);assert.equal(r.intents.length,0)});
test('open bars excluded by profile',()=>{const r=run('//@version=6\nindicator("Open")\nplot(close)',{candles:[...candles.slice(0,-1),{...candles.at(-1),closed:false}]});assert.equal(r.ok,false)});
test('future security and intrabar state explicitly rejected',()=>{for(const call of ['request.security("BTC-EUR","D",close)','varip float x = 0'])assert.equal(run('//@version=6\nindicator("Forbidden")\n'+call).ok,false)});
test('data gaps rejected',()=>{assert.equal(run('//@version=6\nindicator("Gap")\nplot(close)',{candles:candles.filter((_,i)=>i!==5)}).ok,false)});
test('unbounded loop rejected',()=>{assert.equal(run('//@version=6\nindicator("Loop")\nx = 0\nwhile true\n    x := x + 1\nplot(x)').ok,false)});
test('entry and exit are untrusted proposals only',()=>{const r=run('//@version=6\nindicator("Intent")\nplot(close > 35 ? 1 : 0,"entry")');assert.equal(r.intents.length,5);assert(r.intents.every(i=>i.authority==='PROPOSAL_ONLY'))});
test('AST input metadata and overrides',()=>{const src='//@version=6\nindicator("Inputs")\nn = input.int(3, "Periode", minval=2, maxval=10)\nplot(ta.sma(close,n),"sma")';const r=run(src,{inputs:{Periode:5}});assert.equal(r.ok,true,JSON.stringify(r));assert.equal(r.plots[0].data.at(-1).value,38);assert.equal(r.inputs[0].type,'int');assert.equal(run(src,{inputs:{Periode:1}}).ok,false);assert.equal(run(src,{inputs:{Missing:3}}).ok,false)});
console.log(JSON.stringify({runtime:'PineTS 0.11.0',scope:'synthetic deterministic fixtures in network-none container',checks:cases},null,2));
