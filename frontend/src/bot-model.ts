import type { Limits } from "./Paper";
import { preset } from "./Paper";
export type Params = Record<string, string | number | boolean>;
export type BotSettings = {
  mode: "paper" | "live";
  ai_enabled?: boolean;
  bar: "1m" | "5m" | "15m";
  config: {
    id: string;
    name: string;
    instrument: string;
    limits: Limits;
    fast: number;
    slow: number;
    order_eur: string;
    stop_bps: string;
    target_bps: string;
  };
};
export type Graph = {
  schema: number;
  nodes: {
    id: string;
    kind: string;
    params: Params;
    position: { x: number; y: number };
  }[];
  edges: { source: string; target: string; sourceHandle?: string | null }[];
  bot?: BotSettings;
};
export const names: Record<string, string> = {
  market: "Gesloten candle",
  sma_cross: "SMA-kruising",
  ema_cross: "EMA-kruising",
  macd_cross: "MACD-kruising",
  bollinger: "Bollinger Bands",
  candle_stop: "Stop bij candle-daling",
  rsi_range: "RSI-zone",
  pine: "Mijn Pine-script",
  if_else: "Als / anders",
  condition: "Voorwaarde",
  and: "Alle voorwaarden",
  or: "Eén voorwaarde",
  not: "Omgekeerd",
  action: "Koop / verkoop / wacht",
  time_window: "Tijdvenster",
  cooldown: "Wachttijd",
  news: "Nieuwsfilter",
  jev: "Jev-filter",
  sizing: "Orderbedrag",
  intent: "Ordervoorstel",
  risk_gate: "Risicocontrole",
};
export const defaults: Record<string, Params> = {
  sma_cross: { fast: 9, slow: 21 },
  ema_cross: { fast: 9, slow: 21 },
  macd_cross: { fast: 12, slow: 26 },
  bollinger: { period: 20, deviations: 2 },
  candle_stop: { drop_pct: 1, action: "close_pause" },
  rsi_range: { low: 30, high: 70 },
  condition: { field: "rsi", comparison: "above", value: 50 },
  if_else: { field: "rsi", comparison: "above", value: 50 },
  time_window: { start_hour: 0, end_hour: 23 },
  cooldown: { bars: 5 },
  news: { quiet_minutes: 30 },
  action: { action: "hold" },
};
export function settings(instrument = "BTC-EUR"): BotSettings {
  return {
    mode: "paper",
    ai_enabled: false,
    bar: "1m",
    config: {
      id: "bot-" + crypto.randomUUID().slice(0, 8),
      name: "Mijn bot",
      instrument: instrument.endsWith("-EUR") ? instrument : "BTC-EUR",
      limits: preset(1000),
      fast: 9,
      slow: 21,
      order_eur: "100",
      stop_bps: "100",
      target_bps: "200",
    },
  };
}
export function template(type = "trend"): Graph {
  const kinds =
    type === "branch"
      ? [
          "market",
          "if_else",
          "action",
          "action",
          "or",
          "sizing",
          "intent",
          "risk_gate",
        ]
      : [
          "market",
          type === "range" ? "rsi_range" : "sma_cross",
          "sizing",
          "intent",
          "risk_gate",
        ];
  const nodes = kinds.map((kind, i) => ({
    id: "n" + i,
    kind,
    params: {
      ...defaults[kind],
      ...(type === "branch" && kind === "action"
        ? { action: i === 2 ? "buy" : "sell" }
        : {}),
    },
    position: { x: i * 220, y: 100 },
  }));
  if (type === "branch") {
    nodes[2].position = { x: 440, y: 20 };
    nodes[3].position = { x: 440, y: 200 };
    nodes.slice(4).forEach((n, i) => {
      n.position = { x: 660 + i * 220, y: 100 };
    });
    return {
      schema: 1,
      nodes,
      edges: [
        { source: "n0", target: "n1" },
        { source: "n1", target: "n2", sourceHandle: "true" },
        { source: "n1", target: "n3", sourceHandle: "false" },
        { source: "n2", target: "n4" },
        { source: "n3", target: "n4" },
        { source: "n4", target: "n5" },
        { source: "n5", target: "n6" },
        { source: "n6", target: "n7" },
      ],
    };
  }
  return {
    schema: 1,
    nodes,
    edges: kinds
      .slice(1)
      .map((_, i) => ({ source: "n" + i, target: "n" + (i + 1) })),
  };
}
export function navigate(page: string) {
  window.dispatchEvent(new CustomEvent("okx-navigate", { detail: page }));
}
export function readDraft<T>(key: string, fallback: T): T {
  try {
    return JSON.parse(localStorage.getItem(key) || "null") ?? fallback;
  } catch {
    return fallback;
  }
}

export function stableJSON(value: unknown): string {
  return JSON.stringify(value, (_key, item) =>
    item && typeof item === "object" && !Array.isArray(item)
      ? Object.fromEntries(
          Object.keys(item)
            .sort()
            .map((k) => [k, item[k]]),
        )
      : item,
  );
}
