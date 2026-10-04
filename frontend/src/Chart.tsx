import { useEffect, useRef, useState } from "react";
import {
  createChart,
  CandlestickSeries,
  BarSeries,
  LineSeries,
  HistogramSeries,
  ColorType,
} from "lightweight-charts";
import type {
  IChartApi,
  UTCTimestamp,
  ISeriesApi,
  SeriesType,
} from "lightweight-charts";
import type { Candles, IndicatorKey } from "./types";
import Drawings from "./Drawings";
import { useQuery } from "@tanstack/react-query";
import { api } from "./types";
import { usePreferences } from "./store";
export default function Chart({
  data,
  tickSize,
}: {
  data: Candles;
  tickSize: string;
}) {
  const dataRef = useRef(data);
  dataRef.current = data;
  const updateData = useRef<((next: Candles) => void) | null>(null);
  const [axes, setAxes] = useState<{
    chart: IChartApi;
    series: ISeriesApi<SeriesType>;
  } | null>(null);
  const paper = useQuery({
    queryKey: ["chart-paper"],
    queryFn: () =>
      api<{
        account: {
          positions: Record<
            string,
            {
              instrument: string;
              quantity: string;
              cost: string;
              stop: string;
              target: string | null;
            }
          >;
          orders: Record<
            string,
            {
              intent: { instrument: string; limit: string | null };
              state: string;
            }
          >;
        } | null;
      }>("/api/paper"),
    refetchInterval: 2000,
  });
  const priceFormat = {
    type: "price" as const,
    precision: Math.min(12, (tickSize.split(".")[1] || "").length),
    minMove: Number(tickSize) || 0.01,
  };
  const host = useRef<HTMLDivElement>(null);
  const chart = useRef<IChartApi | null>(null);
  const view = useRef<{ from: number; to: number } | null>(null);
  const { chartType, overlay, oscillator, light, timezone } = usePreferences();
  useEffect(() => {
    view.current = null;
  }, [data.instrument, data.bar]);
  useEffect(() => {
    if (!host.current) return;
    const data = dataRef.current;
    const updates: ((next: Candles) => void)[] = [];
    const c = createChart(host.current, {
      autoSize: true,
      localization: {
        timeFormatter: (
          t: number | string | { year: number; month: number; day: number },
        ) =>
          new Date(
            typeof t === "number"
              ? t * 1000
              : typeof t === "string"
                ? t
                : Date.UTC(t.year, t.month - 1, t.day),
          ).toLocaleString("nl-NL", {
            timeZone: timezone || "UTC",
            month: "short",
            day: "numeric",
            hour: "2-digit",
            minute: "2-digit",
          }),
      },
      layout: {
        background: {
          type: ColorType.Solid,
          color: light ? "#ffffff" : "#11171c",
        },
        textColor: light ? "#5b6673" : "#8b99a6",
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
        fontSize: 11,
        attributionLogo: true,
        panes: {
          separatorColor: light ? "#e0e5e9" : "#252f38",
          separatorHoverColor: "#456255",
          enableResize: true,
        },
      },
      grid: {
        vertLines: { color: light ? "#f2f4f5" : "#1a222a" },
        horzLines: { color: light ? "#f2f4f5" : "#1a222a" },
      },
      rightPriceScale: { borderColor: "#2b353e" },
      timeScale: {
        borderColor: "#2b353e",
        timeVisible: true,
        secondsVisible: false,
        rightOffset: 5,
        tickMarkFormatter: (
          t: number | string | { year: number; month: number; day: number },
        ) =>
          new Date(
            typeof t === "number"
              ? t * 1000
              : typeof t === "string"
                ? t
                : Date.UTC(t.year, t.month - 1, t.day),
          ).toLocaleString("nl-NL", {
            timeZone: timezone || "UTC",
            day: "2-digit",
            month: "2-digit",
            hour: "2-digit",
            minute: "2-digit",
          }),
      },
      crosshair: {
        mode: 0,
        vertLine: { color: "#718174" },
        horzLine: { color: "#718174" },
      },
    });
    chart.current = c;
    const toRows = (next: Candles) => {
      let rows = next.candles.map((v) => ({
        time: (v.ts / 1000) as UTCTimestamp,
        open: Number(v.open),
        high: Number(v.high),
        low: Number(v.low),
        close: Number(v.close),
      }));
      if (chartType === "Heikin Ashi") {
        let previous: { open: number; close: number } | null = null;
        rows = rows.map((row) => {
          const close = (row.open + row.high + row.low + row.close) / 4;
          const open = previous
            ? (previous.open + previous.close) / 2
            : (row.open + row.close) / 2;
          previous = { open, close };
          return {
            ...row,
            open,
            close,
            high: Math.max(row.high, open, close),
            low: Math.min(row.low, open, close),
          };
        });
      }
      return rows;
    };
    let rows = toRows(data);
    let main: ISeriesApi<SeriesType>;
    if (chartType === "Lijn") {
      main = c.addSeries(LineSeries, {
        color: "#58cba8",
        lineWidth: 2,
        priceFormat,
      });
      main.setData(rows.map((x) => ({ time: x.time, value: x.close })));
    } else if (chartType === "OHLC") {
      main = c.addSeries(BarSeries, {
        upColor: "#45bd97",
        downColor: "#e6777e",
        priceFormat,
      });
      main.setData(rows);
    } else {
      main = c.addSeries(CandlestickSeries, {
        upColor: "#45bd97",
        downColor: "#e6777e",
        borderVisible: false,
        wickUpColor: "#45bd97",
        wickDownColor: "#e6777e",
        priceFormat,
      });
      main.setData(rows);
    }
    updates.push((next) => {
      rows = toRows(next);
      main.setData(
        chartType === "Lijn"
          ? rows.map((x) => ({ time: x.time, value: x.close }))
          : rows,
      );
    });
    setAxes({ chart: c, series: main });
    let syncing = false;
    const chartId = crypto.randomUUID();
    c.subscribeCrosshairMove((e) => {
      if (syncing || !e.time) return;
      window.dispatchEvent(
        new CustomEvent("okx-crosshair", {
          detail: { id: chartId, time: e.time, instrument: data.instrument },
        }),
      );
    });
    const sync = (event: Event) => {
      const e = (event as CustomEvent).detail;
      if (e.id === chartId || e.instrument !== data.instrument) return;
      const row = rows.find((r) => r.time === e.time);
      if (!row) return;
      syncing = true;
      c.setCrosshairPosition(row.close, row.time, main);
      syncing = false;
    };
    window.addEventListener("okx-crosshair", sync);

    const addLine = (key: IndicatorKey, color: string, pane = 0) => {
      const indicator = c.addSeries(
        LineSeries,
        {
          color,
          lineWidth: 1,
          priceLineVisible: false,
          lastValueVisible: true,
        },
        pane,
      );
      const update = (next: Candles) =>
        indicator.setData(
          next.indicators.flatMap((p) =>
            p[key] === null
              ? []
              : [{ time: (p.ts / 1000) as UTCTimestamp, value: p[key]! }],
          ),
        );
      update(data);
      updates.push(update);
    };
    if (overlay === "bb") {
      addLine("bb_upper", "#a893ce");
      addLine("bb_lower", "#a893ce");
      addLine("sma", "#d7b779");
    } else if (["sma", "ema", "vwap"].includes(overlay)) {
      addLine(overlay as IndicatorKey, "#d7b779");
    }
    const volume = c.addSeries(
      HistogramSeries,
      {
        priceFormat: { type: "volume" },
        priceLineVisible: false,
        lastValueVisible: false,
      },
      1,
    );
    const updateVolume = (next: Candles) =>
      volume.setData(
        next.candles.map((v) => ({
          time: (v.ts / 1000) as UTCTimestamp,
          value: Number(v.volume),
          color: Number(v.close) >= Number(v.open) ? "#285b4d" : "#6b3941",
        })),
      );
    updateVolume(data);
    updates.push(updateVolume);
    if (oscillator !== "none") {
      addLine(oscillator as IndicatorKey, "#a59aca", 2);
      if (oscillator === "macd") addLine("signal", "#d7b779", 2);
    }
    c.panes()[0]?.setStretchFactor(5);
    c.panes()[1]?.setStretchFactor(1);
    c.panes()[2]?.setStretchFactor(1.5);
    if (view.current) c.timeScale().setVisibleLogicalRange(view.current);
    else
      c.timeScale().setVisibleLogicalRange({
        from: Math.max(0, rows.length - 115),
        to: rows.length + 5,
      });
    c.timeScale().subscribeVisibleLogicalRangeChange((range) => {
      if (range) view.current = range;
    });
    updateData.current = (next) => updates.forEach((update) => update(next));
    return () => {
      updateData.current = null;
      window.removeEventListener("okx-crosshair", sync);
      setAxes(null);
      c.remove();
      chart.current = null;
    };
  }, [
    data.instrument,
    data.bar,
    chartType,
    overlay,
    oscillator,
    light,
    tickSize,
    timezone,
  ]);
  useEffect(() => {
    updateData.current?.(data);
  }, [data]);
  useEffect(() => {
    if (!axes) return;
    const lines: ReturnType<typeof axes.series.createPriceLine>[] = [];
    for (const p of Object.values(paper.data?.account?.positions || {})) {
      if (p.instrument !== data.instrument || +p.quantity <= 0) continue;
      for (const [price, title, color] of [
        [+p.cost / +p.quantity, "Positie", "#77b4d7"],
        [+p.stop, "Stop", "#e6777e"],
        [+(p.target || 0), "Target", "#58cba8"],
      ] as const) {
        if (price > 0)
          lines.push(
            axes.series.createPriceLine({
              price,
              title,
              color,
              lineWidth: 1,
              lineStyle: 2,
              axisLabelVisible: true,
            }),
          );
      }
    }
    return () => {
      for (const l of lines) {
        try {
          axes.series.removePriceLine(l);
        } catch {}
      }
    };
  }, [axes, paper.data, data.instrument]);
  return (
    <div className="chart-with-tools">
      <div
        className="chart-canvas"
        ref={host}
        aria-label={`${data.instrument} koersgrafiek met volume en indicator`}
      />
      {axes && (
        <Drawings
          key={data.instrument + data.bar}
          chart={axes.chart}
          series={axes.series}
          data={data}
          tickSize={tickSize}
        />
      )}
    </div>
  );
}
