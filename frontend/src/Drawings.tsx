import { useEffect, useId, useRef, useState } from "react";
import type {
  IChartApi,
  ISeriesApi,
  Logical,
  SeriesType,
} from "lightweight-charts";
import {
  ChevronDown,
  Magnet,
  Repeat2,
  Eye,
  EyeOff,
  Undo2,
  Redo2,
  LockKeyhole,
  UnlockKeyhole,
  Trash2,
  X,
  Settings2,
  Bell,
} from "lucide-react";
import type { Candles } from "./types";
import { post } from "./studio-ui";
import DrawingShape, { ToolIcon } from "./DrawingShape";
import type { Handle } from "./DrawingShape";
import {
  colors,
  draftDrawing,
  drawingTools,
  indexAt,
  labelFor,
  linePrice,
  parseDrawings,
  targetPrice,
  timeAt,
  translateDrawing,
} from "./drawing-model";
import type { Anchor, Drawing, Tool } from "./drawing-model";
import { chartPriceAtTick } from "./chart-price";
import type { ChartPricePick } from "./chart-price";
import "./drawings.css";
import { timeframeLabel } from "./timeframes";

type Drag = {
  original: Drawing;
  value: Drawing;
  start: Anchor;
  handle: Handle;
  pointerId: number;
  moved: boolean;
};
export default function Drawings({
  chart,
  series,
  data,
  tickSize,
  onPricePick,
}: {
  chart: IChartApi;
  series: ISeriesApi<SeriesType>;
  data: Candles;
  tickSize: string;
  onPricePick?: (pick: ChartPricePick) => void;
}) {
  const key = `okx-drawings-v1:${data.instrument}:${data.bar}`;
  const [loaded] = useState(() => {
    try {
      return {
        drawings: parseDrawings(localStorage.getItem(key)),
        valid: true,
      };
    } catch {
      return { drawings: [], valid: false };
    }
  });
  const [drawings, setDrawings] = useState<Drawing[]>(loaded.drawings);
  const [tool, setTool] = useState<Tool>("select");
  const [magnet, setMagnet] = useState(false);
  const [repeat, setRepeat] = useState(false);
  const [hidden, setHidden] = useState(false);
  const [selected, setSelected] = useState<string>();
  const [points, setPoints] = useState<Anchor[]>([]);
  const [cursor, setCursor] = useState<Anchor>();
  const [drag, setDrag] = useState<Drag>();
  const [history, setHistory] = useState<Drawing[][]>([]);
  const [future, setFuture] = useState<Drawing[][]>([]);
  const [palette, setPalette] = useState(false);
  const [editing, setEditing] = useState(false);
  const [tooltip, setTooltip] = useState<Tool>();
  const [color, setColor] = useState(colors[0]);
  const [message, setMessage] = useState(
    loaded.valid
      ? ""
      : "Opgeslagen tekeningen konden niet worden gelezen. Ze blijven bewaard; nieuwe tekeningen gelden alleen voor deze sessie.",
  );
  const [size, setSize] = useState({ w: 0, h: 0 });
  const [, redraw] = useState(0);
  const ref = useRef<HTMLDivElement>(null);
  const svg = useRef<SVGSVGElement>(null);
  const textInput = useRef<HTMLInputElement>(null);
  const pricePick = useRef(onPricePick);
  pricePick.current = onPricePick;
  const latest = useRef({ drawings, drag });
  latest.current = { drawings, drag };
  const clipId = useId().replace(/:/g, "");
  const definition = drawingTools.find((t) => t.kind === tool)!;
  const chosen = drawings.find((d) => d.id === selected);
  const precision = Math.min(12, (tickSize.split(".")[1] || "").length);
  const format = (p: number) =>
    p.toLocaleString("nl-NL", {
      minimumFractionDigits: precision,
      maximumFractionDigits: precision,
    });
  const commit = (next: Drawing[]) => {
    const previous = latest.current.drawings;
    setHistory((h) => [...h.slice(-49), previous]);
    setFuture([]);
    latest.current.drawings = next;
    setDrawings(next);
  };
  const cancel = () => {
    setPoints([]);
    setCursor(undefined);
    setTool("select");
    setDrag(undefined);
    setPalette(false);
    setEditing(false);
  };
  const choose = (kind: Tool) => {
    setTool(kind);
    setPoints([]);
    setCursor(undefined);
    setSelected(undefined);
    setPalette(false);
    setEditing(false);
    setTooltip(undefined);
    setHidden(false);
    ref.current?.focus({ preventScroll: true });
  };
  const undo = () => {
    if (!history.length) return;
    cancel();
    setFuture((f) => [drawings, ...f]);
    setDrawings(history.at(-1)!);
    setHistory((h) => h.slice(0, -1));
  };
  const redo = () => {
    if (!future.length) return;
    cancel();
    setHistory((h) => [...h, drawings]);
    setDrawings(future[0]);
    setFuture((f) => f.slice(1));
  };
  const remove = () => {
    if (chosen && !chosen.locked) {
      commit(drawings.filter((d) => d.id !== chosen.id));
      setSelected(undefined);
      setEditing(false);
    }
  };
  const patch = (change: Partial<Drawing>) => {
    if (chosen && !chosen.locked)
      commit(
        drawings.map((d) => (d.id === chosen.id ? { ...d, ...change } : d)),
      );
  };
  useEffect(() => {
    if (!loaded.valid) return;
    try {
      localStorage.setItem(key, JSON.stringify(drawings));
    } catch {
      setMessage(
        "Opslaan in deze browser lukt niet. Houd dit venster open om je tekeningen te behouden.",
      );
    }
  }, [drawings, key, loaded.valid]);
  useEffect(() => {
    let frame = 0;
    const refresh = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        setSize({
          w: chart.timeScale().width(),
          h: chart.panes()[0]?.getHeight() || 0,
        });
        redraw((n) => n + 1);
      });
    };
    const observer = new ResizeObserver(refresh);
    if (ref.current) observer.observe(ref.current);
    chart.timeScale().subscribeVisibleLogicalRangeChange(refresh);
    chart.subscribeCrosshairMove(refresh);
    // Price-axis dragging and pane resizing don't emit a public range event.
    const timer = window.setInterval(refresh, 200);
    refresh();
    return () => {
      clearInterval(timer);
      observer.disconnect();
      cancelAnimationFrame(frame);
      chart.timeScale().unsubscribeVisibleLogicalRangeChange(refresh);
      chart.unsubscribeCrosshairMove(refresh);
    };
  }, [chart]);
  useEffect(() => {
    const parent = ref.current?.parentElement;
    const focusChart = (event: PointerEvent) => {
      if ((event.target as Element).closest(".chart-canvas")) {
        ref.current?.focus({ preventScroll: true });
        setSelected(undefined);
        setPalette(false);
        setEditing(false);
      }
    };
    parent?.addEventListener("pointerdown", focusChart);
    return () => parent?.removeEventListener("pointerdown", focusChart);
  }, []);
  useEffect(() => {
    if (chosen?.kind === "text" && editing) {
      textInput.current?.focus();
      textInput.current?.select();
    }
  }, [selected, editing]);
  useEffect(() => {
    if (tool !== "select") return;
    const pick = (event: import("lightweight-charts").MouseEventParams) => {
      const point = event.point;
      if (!pricePick.current || latest.current.drag || event.paneIndex !== 0 || !point ||
          point.x < 0 || point.x >= chart.timeScale().width() ||
          point.y < 0 || point.y >= (chart.panes()[0]?.getHeight() || 0)) return;
      const value = series.coordinateToPrice(point.y);
      const price = value === null ? null : chartPriceAtTick(value, tickSize);
      if (price !== null) pricePick.current({ instrument: data.instrument, price });
    };
    chart.subscribeClick(pick);
    return () => chart.unsubscribeClick(pick);
  }, [chart, series, data.instrument, tickSize, tool]);
  const project = (a: Anchor) => {
    const x = chart
        .timeScale()
        .logicalToCoordinate(
          indexAt(data.candles, a.time, data.bar) as Logical,
        ),
      y = series.priceToCoordinate(a.price);
    return x === null || y === null ? null : { x, y };
  };
  const anchorAt = (
    event: React.PointerEvent,
    snap = true,
  ): Anchor | undefined => {
    const bounds = svg.current?.getBoundingClientRect();
    if (!bounds) return;
    const x = Math.max(0, Math.min(event.clientX - bounds.left, size.w));
    const y = Math.max(0, Math.min(event.clientY - bounds.top, size.h));
    const logical = chart.timeScale().coordinateToLogical(x),
      price = series.coordinateToPrice(y);
    if (logical === null || price === null) return;
    let a: Anchor = { time: timeAt(data.candles, logical, data.bar), price };
    if (snap && magnet) {
      const candle = data.candles[Math.round(logical)];
      if (candle) {
        const near = [candle.open, candle.high, candle.low, candle.close]
          .map(Number)
          .sort(
            (a, b) =>
              Math.abs((series.priceToCoordinate(a) ?? Infinity) - y) -
              Math.abs((series.priceToCoordinate(b) ?? Infinity) - y),
          )[0];
        const px = chart
          .timeScale()
          .logicalToCoordinate(Math.round(logical) as Logical);
        if (
          px !== null &&
          Math.hypot(px - x, (series.priceToCoordinate(near) ?? Infinity) - y) <
            12
        )
          a = { time: candle.ts, price: near };
      }
    }
    if (snap && Number(tickSize) > 0)
      a.price = Number(
        (Math.round(a.price / Number(tickSize)) * Number(tickSize)).toFixed(
          precision,
        ),
      );
    return Number.isFinite(a.time) && Number.isFinite(a.price) ? a : undefined;
  };
  const place = (event: React.PointerEvent<SVGRectElement>) => {
    if (event.button !== 0 || tool === "select") return;
    event.preventDefault();
    ref.current?.focus({ preventScroll: true });
    const a = anchorAt(event);
    if (!a) return;
    if (drawings.length >= 100) {
      setMessage(
        "Er staan 100 tekeningen op deze grafiek. Verwijder eerst een tekening.",
      );
      return;
    }
    if (
      points.length &&
      Math.abs(a.time - points[0].time) < 1 &&
      Math.abs(a.price - points[0].price) < Number(tickSize)
    )
      return;
    if (tool === "risk" && points.length === 1 && a.price === points[0].price) {
      setMessage("Kies een stopprijs boven of onder de instap.");
      return;
    }
    if (
      tool === "risk" &&
      points.length === 2 &&
      (a.price - points[0].price) * (points[1].price - points[0].price) >= 0
    ) {
      setMessage(
        "Plaats het koersdoel aan de andere kant van de instap dan de stop.",
      );
      return;
    }
    if (points.length + 1 < definition.steps.length) {
      setPoints([...points, a]);
      setCursor(a);
      return;
    }
    const d = {
      ...draftDrawing(tool, points, a, color),
      id: crypto.randomUUID(),
    };
    commit([...drawings, d]);
    setPoints([]);
    setCursor(undefined);
    setSelected(d.id);
    setMessage("");
    if (!repeat || tool === "text") setTool("select");
    setEditing(tool === "text");
  };
  const startDrag = (
    event: React.PointerEvent<SVGGElement | SVGCircleElement>,
    d: Drawing,
    handle: Handle,
  ) => {
    if (tool !== "select" || event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    ref.current?.focus({ preventScroll: true });
    setSelected(d.id);
    setEditing(false);
    setPalette(false);
    if (d.locked) return;
    const start = anchorAt(event, false);
    if (!start) return;
    svg.current?.setPointerCapture(event.pointerId);
    const next = {
      original: d,
      value: d,
      start,
      handle,
      pointerId: event.pointerId,
      moved: false,
    };
    latest.current.drag = next;
    setDrag(next);
  };
  const move = (event: React.PointerEvent<SVGSVGElement>) => {
    const current = latest.current.drag;
    const a = anchorAt(event, current?.handle !== "move");
    if (!a) return;
    if (!current) {
      if (tool !== "select") setCursor(a);
      return;
    }
    const from = project(current.start),
      to = project(a);
    if (
      !current.moved &&
      from &&
      to &&
      Math.hypot(to.x - from.x, to.y - from.y) < 3
    )
      return;
    const d = current.original;
    let value = d;
    if (current.handle === "move") {
      const dt = d.kind === "horizontal" ? 0 : a.time - current.start.time,
        dp = d.kind === "vertical" ? 0 : a.price - current.start.price;
      value = translateDrawing(d, dt, dp);
    } else if (current.handle === "offset")
      value = { ...d, offset: linePrice(d.a, d.b, a.time) - a.price };
    else if (current.handle === "target") value = { ...d, target: a.price };
    else value = { ...d, [current.handle]: a };
    if (
      d.kind === "risk" &&
      (targetPrice(value) - value.a.price) * (value.b.price - value.a.price) >=
        0
    )
      return;
    const next = { ...current, value, moved: true };
    latest.current.drag = next;
    setDrag(next);
  };
  const finish = (event: React.PointerEvent) => {
    const current = latest.current.drag;
    if (!current || current.pointerId !== event.pointerId) return;
    latest.current.drag = undefined;
    setDrag(undefined);
    if (current.moved)
      commit(
        drawings.map((d) => (d.id === current.original.id ? current.value : d)),
      );
  };
  const setPrice = (part: "a" | "b" | "target" | "offset", value: number) => {
    if (!chosen || !Number.isFinite(value)) return;
    const next =
      part === "a" || part === "b"
        ? { ...chosen, [part]: { ...chosen[part], price: value } }
        : { ...chosen, [part]: value };
    if (
      chosen.kind === "risk" &&
      (targetPrice(next) - next.a.price) * (next.b.price - next.a.price) >= 0
    ) {
      setMessage(
        "Stop en koersdoel moeten aan verschillende kanten van de instap liggen.",
      );
      return;
    }
    patch(next);
  };
  const alert = async () => {
    if (!chosen) return;
    const isLine = ["trend", "channel", "horizontal"].includes(chosen.kind);
    const [a, b] =
      chosen.a.time <= chosen.b.time
        ? [chosen.a, chosen.b]
        : [chosen.b, chosen.a];
    try {
      await post("/api/studio/alerts", {
        name: `Tekening ${data.instrument}`,
        instrument: data.instrument,
        bar: data.bar,
        field: isLine ? "drawing_line" : "price",
        comparison: "above",
        value: isLine ? 0 : chosen.a.price,
        drawing: isLine
          ? {
              first_ms: Math.round(a.time),
              second_ms: Math.round(b.time > a.time ? b.time : a.time + 60000),
              first_price: a.price,
              second_price: chosen.kind === "horizontal" ? a.price : b.price,
            }
          : null,
        trigger: "intrabar",
        cooldown_secs: 60,
      });
      setMessage(
        isLine
          ? "Alert bewaard voor deze lijn. Latere tekenwijzigingen wijzigen de alert niet."
          : "Alert bewaard boven de eerste ankerprijs.",
      );
    } catch (e) {
      setMessage(String(e));
    }
  };
  const preview =
    tool !== "select" && cursor
      ? draftDrawing(tool, points, cursor, color)
      : undefined;
  return (
    <div
      ref={ref}
      className="drawing-layer"
      tabIndex={-1}
      aria-label={`Tekenen op ${data.instrument} ${timeframeLabel(data.bar)}`}
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.preventDefault();
          e.stopPropagation();
          cancel();
          setSelected(undefined);
          latest.current.drag = undefined;
          return;
        }
        if (
          (e.target as Element).closest(
            "input,textarea,select,[contenteditable=true]",
          )
        )
          return;
        if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "z") {
          e.preventDefault();
          e.stopPropagation();
          e.shiftKey ? redo() : undo();
        }
        if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "y") {
          e.preventDefault();
          redo();
        }
        if (e.key === "Delete" || e.key === "Backspace") {
          e.preventDefault();
          remove();
        }
      }}
    >
      <div
        className="drawing-rail"
        role="toolbar"
        aria-label="Tekengereedschap"
      >
        {drawingTools.map((t) => (
          <button
            key={t.kind}
            type="button"
            aria-label={t.label}
            aria-pressed={tool === t.kind}
            className={tool === t.kind ? "active" : ""}
            onClick={() => choose(t.kind)}
            onMouseEnter={() => setTooltip(t.kind)}
            onMouseLeave={() => setTooltip(undefined)}
            onFocus={() => setTooltip(t.kind)}
            onBlur={() => setTooltip(undefined)}
          >
            <ToolIcon kind={t.kind} />
          </button>
        ))}
      </div>
      {tooltip && (
        <div className="drawing-tooltip">
          <strong>{labelFor(tooltip)}</strong>
          <span>{drawingTools.find((t) => t.kind === tooltip)!.hint}</span>
        </div>
      )}
      <div className="drawing-toolbar" role="toolbar" aria-label="Tekenopties">
        <button
          className="drawing-picker"
          aria-label="Alle tekentools en uitleg"
          aria-expanded={palette}
          onClick={() => {
            setPalette(!palette);
            setEditing(false);
            setTooltip(undefined);
          }}
        >
          <span>{tool === "select" ? "Tekentools" : labelFor(tool)}</span>
          <ChevronDown size={14} />
        </button>
        <span className="drawing-divider" />
        <button
          title="Ongedaan maken · ⌘/Ctrl Z"
          aria-label="Ongedaan maken"
          disabled={!history.length}
          onClick={undo}
        >
          <Undo2 size={16} />
        </button>
        <button
          title="Opnieuw · ⌘/Ctrl Shift Z"
          aria-label="Opnieuw"
          disabled={!future.length}
          onClick={redo}
        >
          <Redo2 size={16} />
        </button>
        <span className="drawing-spacer" />
        <button
          title="Magneet: klikpunten nabij een candle vastzetten op OHLC"
          aria-label="Magneet"
          aria-pressed={magnet}
          onClick={() => setMagnet(!magnet)}
        >
          <Magnet size={16} />
        </button>
        <button
          title="Blijf tekenen met dezelfde tool"
          aria-label="Blijf tekenen"
          aria-pressed={repeat}
          onClick={() => setRepeat(!repeat)}
        >
          <Repeat2 size={16} />
        </button>
        <button
          title={hidden ? "Toon tekeningen" : "Verberg tekeningen"}
          aria-label={hidden ? "Toon tekeningen" : "Verberg tekeningen"}
          aria-pressed={hidden}
          onClick={() => {
            setHidden(!hidden);
            cancel();
            setSelected(undefined);
          }}
        >
          {hidden ? <EyeOff size={16} /> : <Eye size={16} />}
        </button>
      </div>
      {palette && (
        <div
          className="drawing-catalog"
          role="dialog"
          aria-label="Kies een tekentool"
        >
          <header>
            <strong>Tekenen op de grafiek</strong>
            <button
              aria-label="Sluit tekentools"
              onClick={() => setPalette(false)}
            >
              <X size={16} />
            </button>
          </header>
          <p>
            Kies een tool en klik de punten aan. Je ziet vooraf het resultaat.
          </p>
          <div>
            {drawingTools
              .filter((t) => t.kind !== "select")
              .map((t) => (
                <button key={t.kind} onClick={() => choose(t.kind)}>
                  <ToolIcon kind={t.kind} />
                  <span>
                    <strong>{t.label}</strong>
                    <small>{t.hint}</small>
                  </span>
                  <em>
                    {t.steps.length} {t.steps.length === 1 ? "klik" : "klikken"}
                  </em>
                </button>
              ))}
          </div>
          <footer>Esc annuleert · Selecteer en sleep om te bewerken</footer>
        </div>
      )}
      <svg
        ref={svg}
        width="100%"
        height="100%"
        className={`drawing-svg ${tool !== "select" ? "is-drawing" : ""}`}
        onPointerMove={move}
        onPointerUp={finish}
        onPointerCancel={() => {
          latest.current.drag = undefined;
          setDrag(undefined);
        }}
        onLostPointerCapture={() => {
          latest.current.drag = undefined;
          setDrag(undefined);
        }}
        onPointerLeave={() => {
          if (!drag) setCursor(undefined);
        }}
        onContextMenu={(e) => {
          if (tool !== "select" || drag) {
            e.preventDefault();
            cancel();
            latest.current.drag = undefined;
          }
        }}
      >
        <defs>
          <clipPath id={clipId}>
            <rect width={size.w} height={size.h} />
          </clipPath>
        </defs>
        <g clipPath={`url(#${clipId})`}>
          {!hidden &&
            drawings.map((d) => (
              <DrawingShape
                key={d.id}
                drawing={drag?.original.id === d.id ? drag.value : d}
                project={project}
                width={size.w}
                height={size.h}
                selected={selected === d.id}
                format={format}
                onStart={(e, h) => startDrag(e, d, h)}
                onEdit={() => {
                  setSelected(d.id);
                  setEditing(true);
                }}
              />
            ))}
          {preview && (
            <DrawingShape
              drawing={preview}
              project={project}
              width={size.w}
              height={size.h}
              selected={false}
              preview
              format={format}
              onStart={() => {}}
              onEdit={() => {}}
            />
          )}
          {tool !== "select" && (
            <rect
              className="drawing-capture"
              width={size.w}
              height={size.h}
              fill="transparent"
              onPointerDown={place}
            />
          )}
        </g>
      </svg>
      {chosen && tool === "select" && !hidden && (
        <div
          className="drawing-selection"
          role="toolbar"
          aria-label="Geselecteerde tekening"
        >
          <span>{labelFor(chosen.kind)}</span>
          <button
            title="Eigenschappen en exacte prijzen"
            aria-label="Tekening bewerken"
            aria-expanded={editing}
            onClick={() => setEditing(!editing)}
          >
            <Settings2 size={16} />
          </button>
          <button
            title={chosen.locked ? "Ontgrendelen" : "Vergrendelen"}
            aria-label={
              chosen.locked ? "Ontgrendel tekening" : "Vergrendel tekening"
            }
            onClick={() =>
              commit(
                drawings.map((d) =>
                  d.id === chosen.id ? { ...d, locked: !d.locked } : d,
                ),
              )
            }
          >
            {chosen.locked ? (
              <LockKeyhole size={16} />
            ) : (
              <UnlockKeyhole size={16} />
            )}
          </button>
          <button
            title="Verwijder · Delete"
            aria-label="Verwijder tekening"
            disabled={chosen.locked}
            onClick={remove}
          >
            <Trash2 size={16} />
          </button>
          <button
            title="Deselecteren"
            aria-label="Deselecteer tekening"
            onClick={() => {
              setSelected(undefined);
              setEditing(false);
            }}
          >
            <X size={16} />
          </button>
        </div>
      )}
      {chosen && editing && (
        <div
          className="drawing-properties"
          role="dialog"
          aria-label="Eigenschappen tekening"
        >
          <header>
            <strong>{labelFor(chosen.kind)}</strong>
            <button
              aria-label="Sluit eigenschappen"
              onClick={() => setEditing(false)}
            >
              <X size={16} />
            </button>
          </header>
          {chosen.locked ? (
            <p>Ontgrendel deze tekening om haar aan te passen.</p>
          ) : (
            <>
              {chosen.kind === "text" && (
                <label>
                  Notitie
                  <input
                    key={chosen.id}
                    ref={textInput}
                    aria-label="Tekst op grafiek"
                    maxLength={100}
                    defaultValue={chosen.text}
                    onBlur={(e) => {
                      if (e.target.value !== chosen.text)
                        patch({ text: e.target.value || "Notitie" });
                    }}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        e.currentTarget.blur();
                        setEditing(false);
                      }
                    }}
                  />
                </label>
              )}
              <div
                className="drawing-colors"
                role="group"
                aria-label="Lijnkleur"
              >
                {colors.map((c) => (
                  <button
                    key={c}
                    aria-label={`Kleur ${c}`}
                    aria-pressed={(chosen.color || colors[0]) === c}
                    style={{ background: c }}
                    onClick={() => {
                      patch({ color: c });
                      setColor(c);
                    }}
                  />
                ))}
              </div>
              {chosen.kind !== "vertical" &&
                (
                  [
                    "a",
                    ...(["horizontal", "text"].includes(chosen.kind)
                      ? []
                      : ["b"]),
                    ...(chosen.kind === "risk" ? ["target"] : []),
                    ...(chosen.kind === "channel" ? ["offset"] : []),
                  ] as const
                ).map((part) => {
                  const k = part as "a" | "b" | "target" | "offset",
                    value =
                      k === "a" || k === "b"
                        ? chosen[k].price
                        : k === "target"
                          ? targetPrice(chosen)
                          : (chosen.offset ?? 0);
                  const label =
                    k === "target"
                      ? "Koersdoel"
                      : k === "offset"
                        ? "Kanaalafstand (+ omlaag)"
                        : chosen.kind === "risk"
                          ? k === "a"
                            ? "Instapprijs"
                            : "Stopprijs"
                          : `Prijs ${k.toUpperCase()}`;
                  return (
                    <label key={`${chosen.id}-${k}-${value}`}>
                      {label}
                      <input
                        aria-label={label}
                        type="number"
                        step={tickSize || "any"}
                        defaultValue={value}
                        onBlur={(e) => {
                          if (e.target.value && +e.target.value !== value)
                            setPrice(k, +e.target.value);
                        }}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") e.currentTarget.blur();
                        }}
                      />
                    </label>
                  );
                })}
            </>
          )}
          <button className="drawing-alert" onClick={alert}>
            <Bell size={14} />
            {["trend", "channel", "horizontal"].includes(chosen.kind)
              ? "Alert boven lijn"
              : "Alert boven ankerprijs"}
          </button>
          <small>Een alert bewaart de huidige lijn of prijs.</small>
        </div>
      )}
      <div className="drawing-status" role="status" aria-live="polite">
        <span>
          {tool === "select" ? (
            chosen ? (
              chosen.locked ? (
                "Vergrendeld · ontgrendel om te bewerken"
              ) : (
                "Sleep de vorm of de ronde punten"
              )
            ) : hidden ? (
              "Tekeningen verborgen"
            ) : (
              "Kies links een tekentool"
            )
          ) : (
            <>
              <b>
                {points.length + 1}/{definition.steps.length}
              </b>{" "}
              {definition.steps[points.length]}
            </>
          )}
        </span>
        {tool !== "select" ? (
          <button onClick={cancel}>
            Annuleren <kbd>Esc</kbd>
          </button>
        ) : (
          <small>
            {drawings.length}{" "}
            {drawings.length === 1 ? "tekening" : "tekeningen"}
          </small>
        )}
      </div>
      {message && (
        <div className="drawing-message" role="status">
          <span>{message}</span>
          <button aria-label="Sluit melding" onClick={() => setMessage("")}>
            <X size={14} />
          </button>
        </div>
      )}
    </div>
  );
}
