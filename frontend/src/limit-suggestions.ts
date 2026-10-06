export type LimitKey = "capital" | "max_order" | "max_position" | "trade_risk" | "daily_loss" | "total_loss" | "drawdown" | "max_orders" | "orders_per_minute" | "spread_bps";
export type SuggestedKey = Exclude<LimitKey, "capital">;
export type LimitValues = Partial<Record<LimitKey, string>>;
export type LimitDraft = { values: LimitValues; manual: Partial<Record<SuggestedKey, true>> };
type LimitField = { key: SuggestedKey; title: string; hint: string; kind: "money" | "integer" | "spread"; max?: number; basisPoints?: number; fixed?: string };

// A product example profile, not a statistical average or an execution policy.
export const limitFields: readonly LimitField[] = [
  {key:"max_order",title:"Maximaal bedrag per order (EUR)",hint:"10% van je handelsbudget · aankoopwaarde van één order",kind:"money",basisPoints:1000},
  {key:"max_position",title:"Maximale blootstelling per munt (EUR)",hint:"25% van je handelsbudget · bestaande posities tellen mee",kind:"money",basisPoints:2500},
  {key:"trade_risk",title:"Maximaal risico per order (EUR)",hint:"1% van je handelsbudget · geraamd verlies tot de stop, inclusief fees",kind:"money",basisPoints:100},
  {key:"daily_loss",title:"Dagverlieslimiet (EUR, UTC)",hint:"3% van je handelsbudget · per UTC-dag",kind:"money",basisPoints:300},
  {key:"total_loss",title:"Totaalverlieslimiet (EUR)",hint:"10% van je handelsbudget · cumulatief verlies",kind:"money",basisPoints:1000},
  {key:"drawdown",title:"Maximale drawdown (EUR)",hint:"10% van je handelsbudget · daling vanaf de hoogste rekeningwaarde",kind:"money",basisPoints:1000},
  {key:"max_orders",title:"Maximaal open orders",hint:"Startprofiel: maximaal 3 tegelijk",kind:"integer",max:100,fixed:"3"},
  {key:"orders_per_minute",title:"Maximaal orders per minuut",hint:"Startprofiel: maximaal 2 per minuut",kind:"integer",max:60,fixed:"2"},
  {key:"spread_bps",title:"Maximale spread (basispunten)",hint:"Startprofiel: 20 basispunten = 0,2% verschil tussen bied en laat",kind:"spread",max:500,fixed:"20"},
];

function parsedBudget(raw:string):{units:bigint;scale:number}|null {
  if(!/^\d{1,16}(?:\.\d{1,8})?$/.test(raw))return null;
  const [whole,fraction=""]=raw.split(".");const units=BigInt(whole+fraction);
  return units>0n?{units,scale:fraction.length}:null;
}
function decimal(units:bigint,scale:number):string {
  const digits=units.toString().padStart(scale+1,"0");
  const whole=digits.slice(0,-scale);const fraction=digits.slice(-scale).replace(/0+$/,"");
  return whole+(fraction?"."+fraction:"");
}
export function suggestedLimits(budget:string):Record<SuggestedKey,string>|null {
  const parsed=parsedBudget(budget);if(!parsed)return null;
  return Object.fromEntries(limitFields.map(f=>[f.key,f.fixed??decimal(parsed.units*BigInt(f.basisPoints!),parsed.scale+4)])) as Record<SuggestedKey,string>;
}
export function emptyLimitDraft():LimitDraft {return {values:{},manual:{}};}
export function changeLimit(draft:LimitDraft,key:LimitKey,value:string):LimitDraft {
  const values={...draft.values,[key]:value};
  if(key!=="capital")return {values,manual:{...draft.manual,[key]:true}};
  const suggested=suggestedLimits(value);
  for(const field of limitFields)if(!draft.manual[field.key])values[field.key]=suggested?.[field.key]??"";
  return {values,manual:{...draft.manual}};
}
export function applyLimitSuggestions(draft:LimitDraft):LimitDraft {
  const suggested=suggestedLimits(draft.values.capital??"");
  return suggested?{values:{...draft.values,...suggested},manual:{}}:draft;
}
export function draftFromSavedLimits(policy:Record<string,unknown>|null|undefined):LimitDraft {
  const draft=emptyLimitDraft();if(!policy)return draft;
  for(const key of ["capital",...limitFields.map(f=>f.key)] as LimitKey[]) {
    const value=policy[key];
    if(typeof value==="string"||typeof value==="number") {
      draft.values[key]=String(value);
      if(key!=="capital")draft.manual[key]=true;
    }
  }
  return draft;
}
