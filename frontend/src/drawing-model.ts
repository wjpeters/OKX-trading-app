import type { Candle } from "./types";

export const drawingTools = [
  {
    kind: "select",
    label: "Selecteren",
    hint: "Klik op een tekening. Sleep de vorm of de ronde punten.",
    steps: [],
  },
  {
    kind: "horizontal",
    label: "Horizontale lijn",
    hint: "Markeer een steun- of weerstandsniveau.",
    steps: ["Klik op het prijsniveau"],
  },
  {
    kind: "vertical",
    label: "Verticale lijn",
    hint: "Markeer een moment op de tijdlijn.",
    steps: ["Klik op het tijdstip"],
  },
  {
    kind: "trend",
    label: "Trendlijn",
    hint: "Verbind twee punten in de koers.",
    steps: ["Klik op het beginpunt", "Klik op het eindpunt"],
  },
  {
    kind: "channel",
    label: "Parallel kanaal",
    hint: "Teken een trendlijn en bepaal daarna de breedte.",
    steps: [
      "Klik op het beginpunt",
      "Klik op het eindpunt",
      "Klik om de kanaalbreedte te bepalen",
    ],
  },
  {
    kind: "fib",
    label: "Fibonacci",
    hint: "Verbind een bodem en top om de niveaus te zien.",
    steps: ["Klik op het eerste uiterste", "Klik op het tweede uiterste"],
  },
  {
    kind: "rectangle",
    label: "Rechthoek",
    hint: "Markeer een prijszone tussen twee hoeken.",
    steps: ["Klik op de eerste hoek", "Klik op de tegenoverliggende hoek"],
  },
  {
    kind: "text",
    label: "Tekst",
    hint: "Plaats een notitie en typ direct je tekst.",
    steps: ["Klik waar je notitie moet staan"],
  },
  {
    kind: "measure",
    label: "Meten",
    hint: "Meet prijsverschil, percentage en verstreken tijd.",
    steps: ["Klik op het beginpunt", "Klik op het eindpunt"],
  },
  {
    kind: "risk",
    label: "Risk/reward",
    hint: "Kies instap, stop en doel voor een long- of shortscenario.",
    steps: [
      "Klik op de instapprijs",
      "Klik op de stopprijs en bepaal de breedte",
      "Klik op het koersdoel, tegenover de stop",
    ],
  },
] as const;
export type Tool = (typeof drawingTools)[number]["kind"];
export type Anchor = { time: number; price: number };
export type Drawing = {
  id: string;
  kind: Exclude<Tool, "select">;
  a: Anchor;
  b: Anchor;
  locked: boolean;
  text: string;
  offset?: number;
  target?: number;
  color?: string;
};
export const colors = ["#89bfc1", "#68d2ae", "#e4bd78", "#c3a2ef", "#ee9198"];
export const labelFor = (kind: Tool) =>
  drawingTools.find((t) => t.kind === kind)!.label;
export const linePrice = (a: Anchor, b: Anchor, time: number) =>
  a.price +
  (b.time === a.time
    ? 0
    : ((b.price - a.price) * (time - a.time)) / (b.time - a.time));
export const targetPrice = (d: Drawing) =>
  d.target ?? d.a.price + (d.a.price - d.b.price) * 2;
export function draftDrawing(
  kind: Drawing["kind"],
  points: Anchor[],
  cursor: Anchor,
  color: string,
): Drawing {
  const a = points[0] ?? cursor,
    b = points[1] ?? cursor;
  return {
    id: "preview",
    kind,
    a,
    b,
    locked: false,
    text: kind === "text" ? "Notitie" : "",
    color,
    offset:
      kind === "channel" && points.length > 1
        ? linePrice(a, b, cursor.time) - cursor.price
        : 0,
    target:
      kind === "risk"
        ? points.length > 1
          ? cursor.price
          : a.price + (a.price - b.price) * 2
        : undefined,
  };
}
export function translateDrawing(d: Drawing, dt: number, dp: number): Drawing {
  return {
    ...d,
    a: { time: d.a.time + dt, price: d.a.price + dp },
    b: { time: d.b.time + dt, price: d.b.price + dp },
    target: d.target === undefined ? undefined : d.target + dp,
  };
}
// Interpolate logical bar positions, including the empty space next to the candles.
export function timeAt(candles: Candle[], index: number): number {
  if (!candles.length) return NaN;
  const i = Math.max(0, Math.min(candles.length - 2, Math.floor(index)));
  const step = candles[i + 1] ? candles[i + 1].ts - candles[i].ts : 60000;
  return candles[i].ts + (index - i) * step;
}
export function indexAt(candles: Candle[], time: number): number {
  if (!candles.length) return NaN;
  let lo = 0,
    hi = candles.length - 1;
  while (lo < hi) {
    const m = Math.ceil((lo + hi) / 2);
    if (candles[m].ts <= time) lo = m;
    else hi = m - 1;
  }
  const i = Math.min(lo, Math.max(0, candles.length - 2));
  const step = candles[i + 1] ? candles[i + 1].ts - candles[i].ts : 60000;
  return i + (time - candles[i].ts) / step;
}
export function parseDrawings(raw: string | null): Drawing[] {
  const value: unknown = JSON.parse(raw || "[]");
  if (!Array.isArray(value)) throw new Error("Ongeldige tekeningen");
  const anchor = (a: Anchor) =>
    a && Number.isFinite(a.time) && Number.isFinite(a.price);
  if (
    !value.every(
      (d) =>
        d &&
        typeof d.id === "string" &&
        drawingTools.some((t) => t.kind !== "select" && t.kind === d.kind) &&
        anchor(d.a) &&
        anchor(d.b) &&
        typeof d.text === "string" &&
        [d.offset, d.target].every(
          (v) => v === undefined || Number.isFinite(v),
        ),
    )
  )
    throw new Error("Ongeldige tekening");
  return value.map((d) => ({
    ...d,
    color: colors.includes(d.color) ? d.color : colors[0],
  }));
}
