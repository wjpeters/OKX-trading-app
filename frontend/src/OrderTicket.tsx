import { useState, useEffect, useRef } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "./types";
import { post } from "./studio-ui";
import type { Market } from "./types";
import { acceptsChartLimitPrice } from "./chart-price";
import type { ChartPricePick } from "./chart-price";
import "./orders.css";
import { limitFields, emptyLimitDraft, changeLimit, applyLimitSuggestions, suggestedLimits, draftFromSavedLimits } from "./limit-suggestions";
import type { LimitKey } from "./limit-suggestions";

type Mode = "OKX_DEMO" | "LIVE";
type Asset={currency:string;total:string;available:string|null;frozen:string|null};
type Balances={mode:Mode;fresh:boolean;configured:boolean;status:string;reason:string;snapshot?:{uid:string;trading_received_ms:number;trade_permission:boolean;trading:Asset[];funding:Asset[]|null;funding_reason:string}|null};
type Sized={account_uid:string;currency:string;available:string;selected:string;quantity:string;percent:number;below_minimum:boolean;wallet_received_ms:number};
type Kind = "market" | "limit" | "post_only" | "ioc" | "fok" | "trigger" | "tp_sl" | "trailing" | "iceberg" | "twap";
type Gateway = {
  credentials_configured: boolean; gateway_fresh: boolean; status: string; reason: string;
  limits?: Record<string, unknown> | null; policy_revision:number; live_release?:{account_uid:string;policy_revision:number}|null;
  snapshot?: { uid: string; live_enabled:boolean; connected:boolean; trade_permission: boolean; reconciled: boolean; equity_eur: string;
    available: Record<string, string>; quotes: Record<string, unknown>; tradable?: string[];
    native_protection_verified: boolean } | null;
};
type Preview = { preview_id: string; client_id: string; mode: Mode; can_submit: boolean;
  kind: string; price_bound: string; estimated_notional: string; blocked: string[]; expires_ms: number;
  warning: string; ticket: { instrument: string; side: string; quantity: string; order: Record<string,unknown> };
};
type NativeOrder = { client_id: string; state: string; exchange_id: string | null; quantity: string; filled: string;
  reason: string; created_ms: number; ticket: { side: string; instrument: string; order: { type: string } } };
type Activity = { fills:{bill_id:string; instrument:string; side:string; quantity:string; price:string; fee:string; fee_currency:string}[];protection:{client_id:string;quantity:string;state:string}[] };
const groups = [
  { name: "Basis", kinds: [["market", "Marktorder"], ["limit", "Limietorder"]] },
  { name: "Geavanceerd", kinds: [["post_only", "Post-only"], ["ioc", "Immediate or cancel"], ["fok", "Fill or kill"], ["trigger", "Trigger"], ["tp_sl", "TP/SL · OCO"], ["trailing", "Trailing stop"]] },
  { name: "Gesplitst", kinds: [["iceberg", "Iceberg"], ["twap", "TWAP"]] },
];
const help: Record<Kind,string> = {
  market: "Direct uitvoeren tegen beschikbare prijzen. De uiteindelijke koers kan afwijken.",
  limit: "Koop tot deze prijs of verkoop vanaf deze prijs. Uitvoering kan gedeeltelijk zijn.",
  post_only: "Alleen als maker in het orderboek. OKX annuleert als de order direct zou matchen.",
  ioc: "Voer meteen het beschikbare deel uit; annuleer het restant.",
  fok: "Voer de volledige hoeveelheid meteen uit of annuleer de hele order.",
  trigger: "OKX plaatst de gekozen order zodra de laatste handelsprijs de trigger bereikt.",
  tp_sl: "Eén doel geeft een conditional order. TP én SL samen worden als OCO geplaatst.",
  trailing: "De trigger volgt de gunstige prijsbeweging en activeert bij de gekozen terugval.",
  iceberg: "OKX verdeelt de totale hoeveelheid over kleinere zichtbare orders. De volledige opdracht telt mee voor je budget.",
  twap: "OKX verdeelt de totale hoeveelheid over kleinere opdrachten met een tijdsinterval.",
};
const label:Record<string,string> = { QUEUED:"In wachtrij",SUBMITTING:"Verzenden",ACCEPTED:"Bij OKX ontvangen",PARTIAL_FILL:"Gedeeltelijk uitgevoerd",FILLED:"Volledig uitgevoerd",CANCEL_PENDING:"Annulering wordt verwerkt",CANCELED:"Geannuleerd",REJECTED:"Geweigerd",UNKNOWN:"Status wordt uitgezocht" };
const fmt=(s:string | undefined)=>{if(s===undefined)return "—";if(!/^-?\d+(?:\.\d+)?$/.test(s))return s;const [whole,fraction=""]=s.split(".");const tail=fraction.replace(/0+$/,"");return whole.replace(/\B(?=(\d{3})+(?!\d))/g,".")+(tail?","+tail:"");};

export default function OrderTicket({ market, symbol, chartPrice, openSettings }: { market:Market; symbol:string; chartPrice?:ChartPricePick; openSettings:()=>void }) {
  const qc=useQueryClient();
  const limitsRef=useRef<HTMLDetailsElement>(null);
  const consumedPrice=useRef(chartPrice);
  const [pickedPrice,setPickedPrice]=useState<string>();
  function configureLimits(){const target=limitsRef.current;if(target){target.open=true;target.scrollIntoView({block:"start",behavior:"smooth"});target.querySelector<HTMLInputElement>("input")?.focus({preventScroll:true});}}
  const [mode,setMode]=useState<Mode>("LIVE");
  const [kind,setKind]=useState<Kind>("market");
  const [side,setSide]=useState<"buy"|"sell">("buy");
  const [fields,setFields]=useState<Record<string,string>>({ slippage:"1",execution:"market",callbackUnit:"ratio",callback:"1",visible:"1",aggression:"conservative",start:"instant",condition:"cross_up",timeframe:"15m",threshold:"30",interval:"10",varianceUnit:"ratio",variance:"0.1",tpMethod:"market",slMethod:"market" });
  const [preview,setPreview]=useState<Preview>();
  const [percentage,setPercentage]=useState<number|null>(null);const [sized,setSized]=useState<Sized>();const [sizeBusy,setSizeBusy]=useState(false);const [sizeError,setSizeError]=useState("");
  const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");const [now,setNow]=useState(Date.now());
  useEffect(()=>{const t=setInterval(()=>setNow(Date.now()),1000);return()=>clearInterval(t);},[]);
  useEffect(()=>{setPreview(undefined);setMessage("");},[symbol,mode,kind,side,fields]);
  useEffect(()=>{setPickedPrice(undefined);},[symbol,mode,kind,side]);
  useEffect(()=>{
    if(consumedPrice.current===chartPrice)return;
    consumedPrice.current=chartPrice;
    if(!chartPrice||chartPrice.instrument!==symbol||busy||preview||!acceptsChartLimitPrice(kind,fields.execution))return;
    setPercentage(null);setSized(undefined);setSizeError("");
    setFields(old=>({...old,price:chartPrice.price}));
    setPickedPrice(chartPrice.price);
  },[chartPrice,symbol,mode,kind,side,fields.execution,busy,preview]);
  useEffect(()=>{setFields(old=>({...old,price:"",trigger:"",tpTrigger:"",tpLimit:"",slTrigger:"",slLimit:"",priceLimit:"",activation:"",startPrice:"",protectStop:"",protectTarget:"",...(old.callbackUnit==="price"?{callback:""}:{}),...(old.varianceUnit==="price"?{variance:""}:{})}));setPickedPrice(undefined);},[symbol]);
  const status=useQuery({queryKey:["gateway",mode,symbol],queryFn:()=>api<Gateway>(`/api/trading?mode=${mode}&instrument=${encodeURIComponent(symbol)}`),refetchInterval:3000});
  const balances=useQuery({queryKey:["okx-balances",mode],queryFn:()=>api<Balances>(`/api/trading/balances?mode=${mode}`),refetchInterval:3000});
  const orders=useQuery({queryKey:["exchange-orders",mode],queryFn:()=>api<{orders:NativeOrder[]}>(`/api/trading/orders?mode=${mode}`),refetchInterval:3000});
  const activity=useQuery({queryKey:["native-activity",mode],queryFn:()=>api<Activity>(`/api/trading/activity?mode=${mode}`),refetchInterval:3000});
  const meta=market.instruments.find(i=>i.instId===symbol);const base=meta?.baseCcy||symbol.split("-")[0];const quote=meta?.quoteCcy||symbol.split("-").at(-1)||"EUR";
  const supported=quote==="EUR"||quote==="USDC";
  const balanceAsset=balances.data?.snapshot?.trading.find(a=>a.currency===(side==="buy"?quote:base));
  const freeBalance=balances.data?.fresh?(balanceAsset?balanceAsset.available??undefined:"0"):undefined;
  const set=(name:string,value:string)=>{if(name==="price")setPickedPrice(undefined);if(name==="quantity"){setPercentage(null);setSized(undefined);}setFields(old=>({...old,[name]:value,...(name==="start"&&value==="price"&&! ["cross_up","cross_down"].includes(old.condition)?{condition:"cross_up"}:{})}));};
  const price = (name:string,title:string,required=true)=> <label key={name}>{title}<div className="order-input-unit"><input aria-label={title} type="number" step={meta?.tickSz||"any"} min={meta?.tickSz||"0.00000001"} required={required} value={fields[name]||""} onChange={e=>set(name,e.target.value)}/><span>{quote}</span></div></label>;
  const amount = (name:string,title:string)=> <label key={name}>{title}<div className="order-input-unit"><input aria-label={title} type="number" step={meta?.lotSz||"any"} min={meta?.minSz||"0.00000001"} required value={fields[name]||""} onChange={e=>set(name,e.target.value)}/><span>{base}</span></div></label>;
  function percentRatio(value:string){ if(!/^\d+(?:\.\d{1,4})?$/.test(value))throw Error("Ongeldig percentage");const [whole,fraction=""]=value.split(".");return `${whole.padStart(3,"0").slice(0,-2)}.${whole.padStart(2,"0").slice(-2)}${fraction}`; }
  function distance(unit:string,value:string) { return {unit,value:unit==="ratio" ? percentRatio(value):value}; }
  function orderSpec():Record<string,unknown> {
    if(["limit","post_only","ioc","fok"].includes(kind))return {type:"limit",price:fields.price,policy:kind};
    if(kind==="market")return {type:"market",slippage_bps:Math.round(Number(fields.slippage)*100)};
    if(kind==="trigger")return {type:"trigger",trigger:fields.trigger,execution:fields.execution==="market"?{type:"market"}:{type:"limit",price:fields.price,policy:fields.execution}};
    const leg=(key:string,method:string,limit:string)=>fields[key]?{trigger:fields[key],order_price:fields[method]==="market"?null:fields[limit]}:null;
    if(kind==="tp_sl")return {type:"tp_sl",take_profit:leg("tpTrigger","tpMethod","tpLimit"),stop_loss:leg("slTrigger","slMethod","slLimit")};
    if(kind==="trailing")return {type:"trailing",distance:distance(fields.callbackUnit,fields.callback),activation:fields.activation||null};
    if(kind==="iceberg")return {type:"iceberg",slice:fields.slice,visible_orders:Number(fields.visible),aggressiveness:fields.aggression,price_limit:fields.priceLimit,start:fields.start==="instant"?{type:"instant"}:fields.start==="price"?{type:"price",price:fields.startPrice,condition:fields.condition}:{type:"rsi",timeframe:fields.timeframe,threshold:Number(fields.threshold),condition:fields.condition}};
    return {type:"twap",slice:fields.slice,price_limit:fields.priceLimit,interval_seconds:Number(fields.interval),variance:distance(fields.varianceUnit,fields.variance)};
  }
  const uid=balances.data?.snapshot?.uid;
  useEffect(()=>{setFields(old=>({...old,quantity:""}));setPercentage(null);setSized(undefined);setSizeError("");},[mode,side,symbol,uid]);
  let specKey="";try{specKey=JSON.stringify(orderSpec());}catch{specKey="invalid";}
  useEffect(()=>{
    if(percentage===null){setSizeBusy(false);return;}
    let current=true;setSizeBusy(true);setSizeError("");
    const timer=setTimeout(async()=>{try{const result=await post<Sized>("/api/trading/size",{mode,instrument:symbol,side,percent:percentage,order:JSON.parse(specKey)});if(current){setSized(result);setFields(old=>({...old,quantity:result.quantity}));}}catch(e){if(current){setSized(undefined);setFields(old=>({...old,quantity:""}));setSizeError(String(e));}}finally{if(current)setSizeBusy(false);}},250);
    return()=>{current=false;clearTimeout(timer);};
  },[percentage,mode,side,symbol,specKey]);
  async function run(fn:()=>Promise<unknown>){setBusy(true);setMessage("");try{await fn();await qc.invalidateQueries({queryKey:["exchange-orders",mode]});}catch(e){setMessage(String(e));}finally{setBusy(false);}}
  async function review(e:React.FormEvent){e.preventDefault();await run(async()=>{let quantity=fields.quantity;const order=orderSpec();if(percentage!==null){const result=await post<Sized>("/api/trading/size",{mode,instrument:symbol,side,percent:percentage,order});if(result.below_minimum)throw Error("De hoeveelheid ligt onder het OKX-minimum. Kies meer saldo.");setSized(result);quantity=result.quantity;}const p=await post<Preview>("/api/trading/preview",{mode,instrument:symbol,side,quantity,order,protection:side==="buy"?{stop:fields.protectStop,target:fields.protectTarget||null}:null});setNow(Date.now());setPreview(p);});}
  const numeric=(name:string,title:string,min:number,max:number,step:number)=> <label>{title}<input aria-label={title} type="number" required min={min} max={max} step={step} value={fields[name]||""} onChange={e=>set(name,e.target.value)}/></label>;
  const select=(name:string,title:string,options:[string,string][])=> <label>{title}<select aria-label={title} value={fields[name]} onChange={e=>set(name,e.target.value)}>{options.map(([v,n])=><option key={v} value={v}>{n}</option>)}</select></label>;
  return <section className="order-ticket">
    <div className="panel-title"><strong>Order plaatsen</strong><span className={`order-mode ${mode==="LIVE"?"live":""}`}>{mode==="LIVE"?"Echt geld":"OKX Demo"}</span></div>
    <div className="order-body">
      <label>Rekening<select aria-label="Orderomgeving" value={mode} onChange={e=>setMode(e.target.value as Mode)}><option value="OKX_DEMO">OKX Demo · virtueel saldo bij OKX</option><option value="LIVE">OKX Live · echt geld</option></select></label>
      <p className="order-connection" role="status">{status.isError?String(status.error):status.data?.reason||"Aansluiting controleren…"}</p>
      {!status.data?.credentials_configured&&<button type="button" className="order-secondary" onClick={openSettings}>OKX aansluiten</button>}
      <WalletBalances data={balances.data} error={balances.isError?String(balances.error):""} onRefresh={()=>qc.invalidateQueries({queryKey:["okx-balances",mode]})}/>
      {mode==="LIVE"&&<LiveControl gateway={status.data} onConfigure={configureLimits} onChanged={()=>qc.invalidateQueries({queryKey:["gateway",mode]})}/>}
      <div className="order-market-access">{!supported?<p className="order-notice">Dit handelsprofiel ondersteunt EUR- en USDC-spotmarkten. Kies een van die betaalvaluta’s.</p>:!status.data?.gateway_fresh||!status.data?.snapshot?.connected?<p className="order-notice">De accountverbinding is niet actueel. Wacht op de controle van beschikbare markten.</p>:status.data?.snapshot?.tradable?.includes(symbol)?status.data?.limits&&Array.isArray(status.data.limits.instruments)&&status.data.limits.instruments.includes(symbol)?<small>Deze markt is beschikbaar bij OKX en bevestigd in je handelslimieten.</small>:<><p className="order-notice">{symbol} is beschikbaar bij OKX. Voeg deze markt toe aan je handelslimieten{mode==="LIVE"?" en bevestig daarna Live opnieuw":""}.</p><button type="button" className="order-secondary" onClick={configureLimits}>Markt in handelslimieten instellen</button></>:<p className="order-notice">{status.data?.snapshot?`OKX geeft ${symbol} momenteel niet vrij voor deze rekening.`:"Verbind je OKX-account om de beschikbare markten te controleren."}</p>}</div>
      {supported&&<form onSubmit={review}>
        <div className="order-side"><button type="button" className={side==="buy"?"buy active":"buy"} onClick={()=>setSide("buy")}>Kopen</button><button type="button" className={side==="sell"?"sell active":"sell"} onClick={()=>setSide("sell")}>Verkopen</button></div>
        <label>Ordertype<select aria-label="Ordertype" value={kind} onChange={e=>setKind(e.target.value as Kind)}>{groups.map(g=><optgroup key={g.name} label={g.name}>{g.kinds.map(([v,n])=><option key={v} value={v}>{n}</option>)}</optgroup>)}</select></label>
        <p className="order-help">{help[kind]}</p>
        <div className="order-market"><strong>{symbol}</strong><span>Spot · zonder hefboom</span></div>
        <p className="order-available">Beschikbaar bij OKX: {fmt(freeBalance)} {side==="buy"?quote:base}</p>
        {amount("quantity","Totale hoeveelheid")}
        <div className="order-percentage"><label htmlFor="order-percentage">Gebruik {percentage===null?"een percentage":`${percentage}%`} van beschikbaar {side==="buy"?quote:base}</label><input id="order-percentage" aria-label="Percentage van beschikbaar saldo" type="range" min="0" max="100" step="1" value={percentage??0} disabled={!balances.data?.fresh||busy} onChange={e=>setPercentage(Number(e.target.value))}/><div className="percentage-presets">{[0,25,50,75,100].map(p=><button type="button" key={p} className={percentage===p?"active":""} disabled={!balances.data?.fresh||busy} onClick={()=>setPercentage(p)}>{p}%</button>)}</div>{sizeBusy?<small>Hoeveelheid berekenen…</small>:sized&&percentage!==null?<small>{sized.percent}% van {fmt(sized.available)} {sized.currency} = {fmt(sized.quantity)} {base}.{side==="buy"?" Inclusief ruimte voor fees en je prijsgrens.":" Afgerond op de OKX-stapgrootte."}{sized.below_minimum&&" Onder het minimum voor deze markt."}</small>:<small>Kopen gebruikt {quote}. Verkopen gebruikt {base}. Funding-saldo wordt niet meegerekend.</small>}{sizeError&&<small className="order-error">{sizeError}</small>}</div>
        {["limit","post_only","ioc","fok"].includes(kind)&&<>{price("price","Limietprijs")}<small className="order-chart-price" role="status">{pickedPrice?`Grafiekprijs overgenomen: ${fmt(pickedPrice)} ${quote}`:"Klik in de koersgrafiek om je limietprijs te kiezen."}</small><button type="button" className="order-bbo" onClick={()=>set("price",String(market.books[symbol]?.[side==="buy"?"bids":"asks"][0][0]||""))}>Gebruik beste {side==="buy"?"bied":"laat"}prijs</button></>}
        {kind==="market"&&<>{side==="buy"?numeric("slippage","Maximale slippage (%)",0,5,0.01):<p className="order-help">Een marktverkoop heeft geen gegarandeerde minimumprijs. Kies een limietorder voor een vaste prijsgrens.</p>}</>}
        {kind==="trigger"&&<>{price("trigger","Triggerprijs")}{select("execution","Order na trigger",[["market","Marktorder"],["limit","Limietorder"],["ioc","IOC"],["fok","FOK"]])}{fields.execution!=="market"&&<>{price("price","Orderprijs na trigger")}<small className="order-chart-price" role="status">{pickedPrice?`Grafiekprijs overgenomen: ${fmt(pickedPrice)} ${quote}`:"Klik in de koersgrafiek om je orderprijs te kiezen."}</small></>}<p className="order-help">Triggerbron: laatste handelsprijs. Mark/index zijn alleen voor derivaten.</p></>}
        {kind==="tp_sl"&&<><fieldset><legend>Take-profit (optioneel)</legend>{price("tpTrigger","TP-triggerprijs",false)}{select("tpMethod","TP-uitvoering",[["market","Markt"],["limit","Limiet"]])}{fields.tpTrigger&&fields.tpMethod==="limit"&&price("tpLimit","TP-orderprijs")}</fieldset><fieldset><legend>Stop-loss (optioneel)</legend>{price("slTrigger","SL-triggerprijs",false)}{select("slMethod","SL-uitvoering",[["market","Markt"],["limit","Limiet"]])}{fields.slTrigger&&fields.slMethod==="limit"&&price("slLimit","SL-orderprijs")}</fieldset></>}
        {kind==="trailing"&&<>{select("callbackUnit","Trailing-afstand",[["ratio","Percentage"],["price",`Vaste prijsafstand (${quote})`]])}{fields.callbackUnit==="ratio"?numeric("callback","Terugval (%)",0.01,100,0.01):price("callback",`Terugval (${quote})`)}{price("activation","Activatieprijs (optioneel)",false)}</>}
        {(kind==="iceberg"||kind==="twap")&&<>{amount("slice","Hoeveelheid per deelorder")}{price("priceLimit",side==="buy"?"Maximale koopprijs":"Minimale verkoopprijs")}</>}
        {kind==="iceberg"&&<>{numeric("visible","Aantal zichtbare orders",1,100,1)}{select("aggression","Uitvoersnelheid",[["conservative","Passief in het orderboek"],["mid","Balans tussen prijs en snelheid"],["radical","Snelle uitvoering"]])}{select("start","Startvoorwaarde",[["instant","Direct starten"],["price","Bij een prijs"],["rsi","Bij RSI"]])}{fields.start==="price"&&price("startPrice","Startprijs")}{fields.start!=="instant"&&select("condition","Start wanneer",fields.start==="price"?[["cross_up","Prijs kruist omhoog"],["cross_down","Prijs kruist omlaag"]]:[["cross_up","RSI kruist omhoog"],["cross_down","RSI kruist omlaag"],["above","RSI is hoger"],["below","RSI is lager"],["cross","RSI kruist"]])}{fields.start==="rsi"&&<>{select("timeframe","RSI-interval",["3m","5m","15m","30m","1H","4H","1D"].map(v=>[v,v]))}{numeric("threshold","RSI-drempel",1,100,1)}<small>RSI-periode: 14 bij OKX.</small></>}</>}
        {kind==="twap"&&<>{numeric("interval","Interval tussen delen (seconden)",1,86400,1)}{select("varianceUnit","Prijsruimte per deel",[["ratio","Percentage"],["price",`Vaste prijsafstand (${quote})`]])}{fields.varianceUnit==="ratio"?numeric("variance","Prijsruimte (%)",0.01,1,0.01):price("variance",`Prijsruimte (${quote})`)}</>}
        {side==="buy"&&<fieldset><legend>Bescherming na aankoop</legend>{price("protectStop","Beschermende stopprijs")}{price("protectTarget","Beschermend koersdoel (optioneel)",false)}<small>De uitvoering moet ook gedeeltelijke aankopen aantoonbaar beschermen.</small></fieldset>}
        <button className={`order-review ${side}`} disabled={busy||sizeBusy||(percentage!==null&&(!sized||sized.below_minimum))} type="submit">{busy?"Controleren…":"Order controleren"}</button>
        <small className="order-help">Controleren maakt een preview. Het plaatst nog geen order bij OKX.</small>
      </form>}
      <details ref={limitsRef} className="order-account"><summary>Account en handelslimieten</summary><Limits mode={mode} gateway={status.data} instruments={(status.data?.snapshot?.tradable||[]).filter(id=>id.endsWith("-EUR")||id.endsWith("-USDC"))} onSaved={()=>qc.invalidateQueries({queryKey:["gateway",mode]})}/></details>
      <details className="order-history" open><summary>Mijn OKX-orders · {orders.data?.orders.length||0}</summary>{orders.isError&&<p className="order-error">{String(orders.error)}</p>}{orders.data?.orders.map(o=><div className="native-order" key={o.client_id}><strong>{o.ticket.side==="buy"?"Koop":"Verkoop"} {o.ticket.instrument}</strong><span>{label[o.state]||o.state}</span><small>{o.ticket.order.type} · {fmt(o.filled)} / {fmt(o.quantity)}</small>{o.reason&&<p>{o.reason}</p>}{!["FILLED","CANCELED","REJECTED"].includes(o.state)&&<button disabled={busy} onClick={()=>run(async()=>{await post("/api/trading/cancel",{mode,client_id:o.client_id});setMessage("Annulering aangevraagd.");})}>Annuleren</button>}</div>)}{orders.data&&!orders.data.orders.length&&<p className="order-help">Nog geen orders vanuit deze terminal in deze omgeving.</p>}</details>
      <details className="order-history"><summary>Uitvoeringen en fees · {activity.data?.fills.length||0}</summary>{activity.data?.fills.map(f=><div className="native-order" key={f.bill_id}><strong>{f.side==="buy"?"Koop":"Verkoop"} {f.instrument}</strong><small>{fmt(f.quantity)} tegen {fmt(f.price)} {f.instrument.split("-").at(-1)}</small><small>Fee / rebate: {fmt(f.fee)} {f.fee_currency}</small></div>)}{activity.data&&!activity.data.fills.length&&<p className="order-help">Nog geen bevestigde OKX-uitvoeringen.</p>}</details>
      <details className="order-history"><summary>Bescherming · {activity.data?.protection.length||0}</summary>{activity.data?.protection.map(p=><div className="native-order" key={p.client_id}><strong>{p.state==="ACTIVE"?"Actieve stop bij OKX":p.state}</strong><small>Hoeveelheid: {fmt(p.quantity)}</small></div>)}<p className="order-help">Een ontvangen aanvraag is nog geen actieve stop. Bij onzekere bescherming blokkeert de gateway nieuwe aankopen.</p></details>
      {message&&<p role="status" className="order-error">{message}</p>}
    </div>
    {preview&&<div className="order-modal-backdrop"><section className="order-modal" role="dialog" aria-modal="true" aria-label="Orderpreview"><h3>{preview.mode==="LIVE"?"Live-order controleren · echt geld":"OKX Demo-order controleren"}</h3><dl><dt>Instrument</dt><dd>{preview.ticket.instrument}</dd><dt>Richting</dt><dd>{preview.ticket.side==="buy"?"Kopen":"Verkopen"}</dd><dt>Hoeveelheid</dt><dd>{fmt(preview.ticket.quantity)} {base}</dd><dt>Ordertype</dt><dd>{preview.kind}</dd><dt>Geschatte waarde</dt><dd>{fmt(preview.estimated_notional)} {quote}</dd><dt>Prijsreferentie / limiet</dt><dd>{fmt(preview.price_bound)} {quote}</dd><dt>Geldig</dt><dd>{Math.max(0,Math.ceil((preview.expires_ms-now)/1000))} seconden</dd></dl><p>{preview.warning}</p>{preview.blocked.length>0&&<div className="order-notice"><strong>Nog nodig om te verzenden</strong><ul>{preview.blocked.map(v=><li key={v}>{v}</li>)}</ul></div>}<div className="order-modal-actions"><button onClick={()=>setPreview(undefined)} disabled={busy}>Terug naar order</button><button className="order-review" disabled={busy||!preview.can_submit||now>=preview.expires_ms} onClick={()=>run(async()=>{const r=await post<{message:string}>("/api/trading/submit",{preview_id:preview.preview_id,confirmed:true});setPreview(undefined);setMessage(r.message);})}>{preview.mode==="LIVE"?"Bevestigen en naar OKX Live":"Bevestigen en naar OKX Demo"}</button></div></section></div>}
  </section>;
}

function WalletBalances({data,error,onRefresh}:{data?:Balances;error:string;onRefresh:()=>Promise<unknown>}){
  const [wallet,setWallet]=useState<"trading"|"funding">("trading");
  const assets=data?.snapshot?.[wallet];
  return <details className="order-balances"><summary>Mijn OKX-saldi {data?.fresh?<span className="balance-fresh">Actueel</span>:<span>Wachten</span>}</summary><div className="balance-wallet"><button type="button" className={wallet==="trading"?"active":""} onClick={()=>setWallet("trading")}>Handelsrekening</button><button type="button" className={wallet==="funding"?"active":""} onClick={()=>setWallet("funding")}>Funding</button><button type="button" aria-label="Saldi vernieuwen" onClick={onRefresh}>↻</button></div>{error?<p className="order-error">{error}</p>:!data?.snapshot?<p className="order-help">{data?.reason||"Saldi ophalen…"}</p>:<><small>Account {data.snapshot.uid} · {new Date(data.snapshot.trading_received_ms).toLocaleTimeString("nl-NL")}{!data.fresh&&" · verouderd, niet gebruiken voor orders"}</small>{wallet==="funding"&&<p className="order-help">Funding is apart bij OKX. Alleen saldo op de handelsrekening kan hier worden verhandeld.</p>}{assets?<>{assets.length?<div className={`balance-list ${!data.fresh?"stale":""}`}>{assets.map(a=><div className="balance-row" key={a.currency}><strong>{a.currency}</strong><div><b>{fmt(a.total)}</b><small>Vrij {fmt(a.available??undefined)} · Vast {fmt(a.frozen??undefined)}</small></div></div>)}</div>:<p className="order-help">Geen muntsaldo op deze rekening.</p>}</>:<p className="order-help">{data.snapshot.funding_reason}</p>}{!data.fresh&&<p className="order-error">{data.reason}</p>}</>}</details>;
}
function LiveControl({gateway,onConfigure,onChanged}:{gateway?:Gateway;onConfigure:()=>void;onChanged:()=>Promise<unknown>}){
  const [confirmed,setConfirmed]=useState(false);const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");
  useEffect(()=>{setConfirmed(false);setMessage("");},[gateway?.snapshot?.uid,gateway?.policy_revision]);
  const enabled=!!gateway?.live_release;const s=gateway?.snapshot;const policy=gateway?.limits;
  const blockers:string[]=[];
  if(!policy)blockers.push("Je account, markten, handelsbudget en limieten zijn nog niet bevestigd.");
  else if(policy.confirmed!==true||policy.account_uid!==s?.uid)blockers.push("Bevestig de handelslimieten voor het actuele account opnieuw.");
  if(!gateway?.credentials_configured)blockers.push("Voeg je OKX Live-aansluiting toe onder Systeem.");
  else if(!gateway.gateway_fresh||!s?.connected)blockers.push("Wacht op een actuele private accountverbinding.");
  else {
    if(!s.reconciled)blockers.push("De orderadministratie wordt nog gesynchroniseerd.");
    if(!s.trade_permission)blockers.push("Je OKX-aansluiting heeft nog geen handelsrechten.");
    if(!s.native_protection_verified)blockers.push("Controleer eerst onzekere orders en hun bescherming.");
    if(policy&&Array.isArray(policy.instruments)&&policy.instruments.some(id=>typeof id!=="string"||!s.tradable?.includes(id)))blockers.push("Een bevestigde markt is niet beschikbaar voor dit account. Pas je marktkeuze aan.");
  }
  const ready=blockers.length===0;
  const needsLimits=!policy||policy.confirmed!==true||policy.account_uid!==s?.uid;
  async function change(value:boolean){setBusy(true);setMessage("");try{const r=await post<{message:string}>("/api/trading/live",{account_uid:s?.uid,policy_revision:gateway?.policy_revision,enabled:value,confirmed:true});setConfirmed(false);setMessage(r.message);await onChanged();}catch(e){setMessage(String(e));}finally{setBusy(false);}}
  return <div className="live-setup">{!enabled&&blockers.length>0&&<div className="order-notice" role="status"><strong>Nog nodig voor Live-handelen</strong><ul>{blockers.map(reason=><li key={reason}>{reason}</li>)}</ul>{needsLimits&&<button type="button" className="order-secondary" onClick={onConfigure}>Account en handelslimieten instellen</button>}</div>}<details className="live-control"><summary>{enabled?(ready?"Live-vrijgave actief":"Live-vrijgave wacht op verbinding"):"Live-handelen vrijgeven"}</summary>{enabled?<><strong>{s?.live_enabled&&ready?"Live-handelen vrijgegeven":"Live-vrijgave wacht op verbinding"}</strong><small>Account {s?.uid}. Elke order vraagt een aparte bevestiging.</small><button type="button" disabled={busy} onClick={()=>change(false)}>Nieuwe Live-orders pauzeren</button></>:<><strong>Live-handelen vrijgeven</strong><p className="order-help">{ready?"Je account en limieten zijn bevestigd. Vink hieronder je persoonlijke vrijgave aan en kies Live vrijgeven.":"De vrijgave blijft uitgeschakeld totdat de stappen hierboven klaar zijn."}</p><label className="order-check"><input type="checkbox" checked={confirmed} disabled={!ready||busy} onChange={e=>setConfirmed(e.target.checked)}/>Ik geef Live vrij voor account {s?.uid||"…"} en mijn bevestigde limieten. Elke order bevestig ik apart.</label><button type="button" disabled={busy||!ready||!confirmed} onClick={()=>change(true)}>Live vrijgeven</button></>}{message&&<p className="order-help" role="status">{message}</p>}</details></div>;
}

function Limits({mode,gateway,instruments,onSaved}:{mode:Mode;gateway?:Gateway;instruments:string[];onSaved:()=>Promise<unknown>}){
  const [marketSearch,setMarketSearch]=useState("");
  const [draft,setDraft]=useState(emptyLimitDraft);const [chosen,setChosen]=useState<string[]>([]);const [confirmed,setConfirmed]=useState(false);const [message,setMessage]=useState("");const [busy,setBusy]=useState(false);
  useEffect(()=>{setDraft(draftFromSavedLimits(gateway?.limits));setChosen(Array.isArray(gateway?.limits?.instruments)?gateway.limits.instruments.filter((id):id is string=>typeof id==="string"):[]);setConfirmed(false);setMessage("");},[mode,gateway?.snapshot?.uid,gateway?.policy_revision]);
  const values=draft.values;const suggestions=suggestedLimits(values.capital??"");
  const custom=Object.keys(draft.manual).length>0;
  function change(key:LimitKey,value:string){setDraft(old=>changeLimit(old,key,value));setConfirmed(false);setMessage("");}
  return <form onSubmit={async e=>{e.preventDefault();setBusy(true);setMessage("");try{const numbers=Object.fromEntries(Object.entries(values).map(([k,v])=>[k,["max_orders","orders_per_minute"].includes(k)?Number(v):v]));await post("/api/trading/limits",{mode,limits:{...numbers,account_uid:gateway?.snapshot?.uid,instruments:chosen,confirmed}});await onSaved();setMessage("Account, markten en limieten bevestigd.");}catch(e){setMessage(String(e));}finally{setBusy(false);}}}>
    <p className="order-help">Account: {gateway?.snapshot?.uid||"Nog niet verbonden"}. De voorbeeldwaarden worden pas opgeslagen als je zelf bevestigt.</p>
    <label>Totaal handelsbudget (EUR)<input aria-label="Totaal handelsbudget (EUR)" type="number" min="0.000001" step="any" required disabled={busy} placeholder="Bijvoorbeeld 500" value={values.capital||""} onChange={e=>change("capital",e.target.value)}/></label>
    <div className="limits-profile"><strong>{custom?"Startprofiel met eigen aanpassingen":"Startprofiel · automatisch invullen"}</strong><p>Vul je handelsbudget in. De overige waarden rekenen mee. Zelf aangepaste velden blijven behouden als je het budget wijzigt.</p>{suggestions?<p className="limits-summary">Order: € {fmt(values.max_order)} · Risico: € {fmt(values.trade_risk)} · Dagverlies: € {fmt(values.daily_loss)}</p>:<p className="order-help">Gebruik een positief bedrag zoals 500 of 500,50 om de voorbeeldwaarden te berekenen.</p>}<button type="button" className="order-secondary" disabled={busy||!suggestions} onClick={()=>{setDraft(old=>applyLimitSuggestions(old));setConfirmed(false);setMessage("");}}>Startprofiel opnieuw invullen</button><small>Dit zijn aanpasbare voorbeeldwaarden voor je handelsplan.</small></div>
    {limitFields.map(field=><label key={field.key}>{field.title}<input aria-label={field.title} type="number" min={field.kind==="integer"?"1":"0.00000001"} max={field.kind==="money"?values.capital:field.max} step={field.kind==="integer"?"1":"any"} required disabled={busy} value={values[field.key]||""} onChange={e=>change(field.key,e.target.value)}/><small className={`limits-field-hint ${draft.manual[field.key]?"custom":""}`}>{draft.manual[field.key]?"Zelf aangepast · blijft behouden bij budgetwijziging":field.hint}</small></label>)}
    <fieldset><legend>Bevestigde spotmarkten · {chosen.length} van {instruments.length}</legend><input aria-label="Zoek markt in handelslimieten" placeholder="Bijvoorbeeld BTC of USDC" value={marketSearch} onChange={e=>setMarketSearch(e.target.value)}/><div className="market-selection-actions"><button type="button" disabled={busy||!instruments.length} onClick={()=>{setChosen([...instruments]);setConfirmed(false);}}>Alle beschikbare spotmarkten kiezen</button><button type="button" disabled={busy} onClick={()=>{setChosen([]);setConfirmed(false);}}>Selectie wissen</button></div>{instruments.filter(id=>id.toLowerCase().includes(marketSearch.toLowerCase().replaceAll("/","-"))).slice(0,60).map(id=><label className="order-check" key={id}><input type="checkbox" disabled={busy} checked={chosen.includes(id)} onChange={e=>{setChosen(e.target.checked?[...chosen,id]:chosen.filter(v=>v!==id));setConfirmed(false);}}/>{id}</label>)}{instruments.length>60&&<small>Zoek om andere markten te kiezen. Alle kiezen omvat de volledige accountlijst; je bevestigt de keuze hieronder.</small>}{!chosen.length&&<small>Kies ten minste één markt om je handelslimieten te bevestigen.</small>}</fieldset>
    <label className="order-check"><input type="checkbox" disabled={busy} checked={confirmed} onChange={e=>setConfirmed(e.target.checked)}/>Ik bevestig dit account, deze markten, dit budget en alle limieten voor {mode==="LIVE"?"echt geld":"OKX Demo"}.</label>
    <button type="submit" disabled={busy||!confirmed||!chosen.length||!gateway?.snapshot?.uid}>Limieten bevestigen</button>{message&&<p role="status">{message}</p>}
  </form>;
}
