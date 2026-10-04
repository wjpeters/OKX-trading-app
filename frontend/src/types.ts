export interface Instrument {
  instId: string;
  baseCcy: string;
  quoteCcy: string;
  tickSz: string;
  lotSz: string;
  minSz: string;
  state: string;
}
export interface Ticker {
  last: string;
  open24h: string;
  high24h: string;
  low24h: string;
  vol24h: string;
  ts: string;
  received_at_ms: number;
}
export interface Book {
  asks: string[][];
  bids: string[][];
  ts: string;
  received_at_ms: number;
  seqId: number;
}
export interface Trade {
  tradeId: string;
  px: string;
  sz: string;
  side: "buy" | "sell";
  ts: string;
  received_at_ms: number;
}
export interface Market {
  instruments: Instrument[];
  tickers: Record<string, Ticker>;
  books: Record<string, Book>;
  trades: Record<string, Trade[]>;
  feed_connected: boolean;
  feed_error: string | null;
  last_message_ms: number | null;
  reconnects: number;
}
export interface Snapshot {
  market: Market;
  server_time_ms: number;
}
export interface Candle {
  ts: number;
  open: string;
  high: string;
  low: string;
  close: string;
  volume: string;
  closed: boolean;
}
export type IndicatorKey =
  | "sma"
  | "ema"
  | "rsi"
  | "macd"
  | "signal"
  | "atr"
  | "bb_upper"
  | "bb_lower"
  | "vwap"
  | "stochastic"
  | "adx"
  | "obv";
export type Point = { ts: number } & Record<IndicatorKey, number | null>;
export interface Candles {
  instrument: string;
  bar: string;
  source: string;
  received_at_ms: number;
  candles: Candle[];
  indicators: Point[];
  gaps: number;
  fresh: boolean;
  historical: boolean;
  rest_roundtrip_ms: number;
  calculation_ms: number;
}
export interface Status {
  active_bots: number;
  version: string;
  mode: string;
  risk_state: string;
  risk_reason: string;
  account: string;
  account_region: string;
  public_region: string;
  database: boolean;
  public_ws: boolean;
  last_message_ms: number | null;
  feed_error: string | null;
  reconnects: number;
  instruments: number;
  clock_offset_ms: number | null;
  clock_rtt_ms: number | null;
  processing_ms: {
    p50: number | null;
    p95: number | null;
    p99: number | null;
    samples: number;
    scope: string;
  };
  server_time_ms: number;
}
export async function api<T>(path: string): Promise<T> {
  const r = await fetch(path);
  if (!r.ok) {
    let message = "Verbinding met de terminal mislukt";
    try {
      const body = await r.json();
      message = body.error || message;
    } catch {}
    throw new Error(message);
  }
  return r.json() as Promise<T>;
}
