// A chart uses floating coordinates. Convert its displayed price to an exact
// multiple of the exchange tick before handing it to the order draft.
export type ChartPricePick = { instrument: string; price: string };

export function chartPriceAtTick(price: number, tickSize: string): string | null {
  if (!Number.isFinite(price) || price <= 0 || !/^\d+(?:\.\d{1,18})?$/.test(tickSize)) return null;
  const [tickWhole, tickFraction = ""] = tickSize.split(".");
  const tick = BigInt(tickWhole + tickFraction);
  if (tick <= 0n) return null;
  const [mantissa, exponent = "0"] = price.toString().toLowerCase().split("e");
  const [whole, fraction = ""] = mantissa.split(".");
  const scale = fraction.length - Number(exponent);
  const common = Math.max(0, scale, tickFraction.length);
  const raw = BigInt(whole + fraction) * 10n ** BigInt(common - scale);
  const step = tick * 10n ** BigInt(common - tickFraction.length);
  const units = (raw + step / 2n) / step;
  if (units === 0n) return null;
  const rounded = (units * tick).toString().padStart(tickFraction.length + 1, "0");
  if (!tickFraction.length) return rounded;
  return `${rounded.slice(0, -tickFraction.length)}.${rounded.slice(-tickFraction.length)}`;
}

export function acceptsChartLimitPrice(kind: string, execution: string): boolean {
  return ["limit", "post_only", "ioc", "fok"].includes(kind) ||
    (kind === "trigger" && ["limit", "ioc", "fok"].includes(execution));
}
