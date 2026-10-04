import type { ReactNode } from "react";
import { labelFor, targetPrice } from "./drawing-model";
import type { Anchor, Drawing, Tool } from "./drawing-model";

export function ToolIcon({ kind }: { kind: Tool }) {
  const paths: Record<Tool, ReactNode> = {
    select: <path d="m5 3 13 9-6 1-3 7z" />,
    horizontal: (
      <>
        <path d="M3 12h18" />
        <circle cx="8" cy="12" r="2" />
      </>
    ),
    vertical: (
      <>
        <path d="M12 3v18" />
        <circle cx="12" cy="8" r="2" />
      </>
    ),
    trend: (
      <>
        <path d="m5 18 14-12" />
        <circle cx="5" cy="18" r="2" />
        <circle cx="19" cy="6" r="2" />
      </>
    ),
    channel: (
      <>
        <path d="M3 14 18 4M6 21 21 11M4.5 17.5 19.5 7.5" />
      </>
    ),
    fib: (
      <>
        <path d="M3 4h18M3 10h14M3 14h18M3 20h18M4 20 20 4" />
      </>
    ),
    rectangle: (
      <>
        <rect x="4" y="6" width="16" height="12" rx="1" />
        <path d="M4 4v4M20 16v4" />
      </>
    ),
    text: <path d="M4 6V4h16v2M12 4v16M8 20h8" />,
    measure: (
      <>
        <path d="m4 17 13-13 4 4-13 13zM9 12l3 3M13 8l3 3" />
      </>
    ),
    risk: (
      <>
        <rect x="4" y="3" width="16" height="18" rx="1" />
        <path d="M4 14h16M12 6v5m-2-2 2-3 2 3M10 17h4" />
      </>
    ),
  };
  return (
    <svg
      width="19"
      height="19"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {paths[kind]}
    </svg>
  );
}
export type Handle = "a" | "b" | "offset" | "target" | "move";
export type XY = { x: number; y: number };
export default function DrawingShape({
  drawing: d,
  project,
  width,
  height,
  selected,
  preview,
  format,
  onStart,
  onEdit,
}: {
  drawing: Drawing;
  project: (a: Anchor) => XY | null;
  width: number;
  height: number;
  selected: boolean;
  preview?: boolean;
  format: (price: number) => string;
  onStart: (
    e: React.PointerEvent<SVGGElement | SVGCircleElement>,
    handle: Handle,
  ) => void;
  onEdit: () => void;
}) {
  const a = project(d.a),
    b = project(d.b);
  if (!a || !b) return null;
  const color = d.color || "#89bfc1",
    x = Math.min(a.x, b.x),
    y = Math.min(a.y, b.y),
    w = Math.abs(b.x - a.x),
    h = Math.abs(b.y - a.y);
  const line = (
    x1: number,
    y1: number,
    x2: number,
    y2: number,
    stroke = color,
    dash?: string,
  ) => (
    <g>
      <line className="drawing-hit" x1={x1} y1={y1} x2={x2} y2={y2} />
      <line
        x1={x1}
        y1={y1}
        x2={x2}
        y2={y2}
        stroke={stroke}
        strokeWidth={selected ? 2 : 1.5}
        strokeDasharray={dash}
      />
    </g>
  );
  const text = (px: number, py: number, value: string) =>
    py < 10 || py > height - 2 ? null : (
      <text x={Math.max(4, Math.min(px, Math.max(4, width - 190)))} y={py}>
        {value}
      </text>
    );
  const handles: { key: Handle; p: XY }[] = [{ key: "a", p: a }];
  let shape: ReactNode;
  if (d.kind === "horizontal") {
    shape = (
      <>
        {line(0, a.y, width, a.y)}
        {text(8, a.y - 7, format(d.a.price))}
      </>
    );
    handles[0].p = { x: Math.max(12, Math.min(a.x, width - 12)), y: a.y };
  } else if (d.kind === "vertical") {
    shape = line(a.x, 0, a.x, height);
    handles[0].p = { x: a.x, y: Math.max(12, Math.min(a.y, height - 12)) };
  } else if (d.kind === "text")
    shape = (
      <text x={a.x} y={a.y} className="drawing-note" fill={color}>
        {d.text || "Notitie"}
      </text>
    );
  else {
    handles.push({ key: "b", p: b });
    if (d.kind === "rectangle")
      shape = (
        <rect
          x={x}
          y={y}
          width={w}
          height={h}
          fill={color + "18"}
          stroke={color}
          strokeWidth={selected ? 2 : 1.5}
        />
      );
    else if (d.kind === "fib")
      shape = (
        <>
          {[0, 0.236, 0.382, 0.5, 0.618, 0.786, 1].map((v) => (
            <g key={v}>
              {line(
                x,
                a.y + (b.y - a.y) * v,
                x + w,
                a.y + (b.y - a.y) * v,
                color,
                v === 0.5 ? "4 4" : undefined,
              )}
              {text(
                x + 5,
                a.y + (b.y - a.y) * v - 5,
                `${v} · ${format(d.a.price + (d.b.price - d.a.price) * v)}`,
              )}
            </g>
          ))}
        </>
      );
    else if (d.kind === "channel") {
      const c = project({ ...d.a, price: d.a.price - (d.offset ?? 0) }),
        e = project({ ...d.b, price: d.b.price - (d.offset ?? 0) });
      if (!c || !e) return null;
      handles.push({
        key: "offset",
        p: { x: (c.x + e.x) / 2, y: (c.y + e.y) / 2 },
      });
      shape = (
        <>
          <polygon
            points={`${a.x},${a.y} ${b.x},${b.y} ${e.x},${e.y} ${c.x},${c.y}`}
            fill={color + "12"}
          />
          {line(a.x, a.y, b.x, b.y)}
          {line(c.x, c.y, e.x, e.y)}
          {line(a.x, (a.y + c.y) / 2, b.x, (b.y + e.y) / 2, color, "4 4")}
        </>
      );
    } else if (d.kind === "risk") {
      const target = targetPrice(d),
        p = project({ time: d.b.time, price: target });
      if (!p) return null;
      const reward = Math.abs(target - d.a.price),
        risk = Math.abs(d.b.price - d.a.price);
      handles.push({ key: "target", p });
      shape = (
        <>
          <rect
            x={x}
            y={Math.min(a.y, b.y)}
            width={Math.max(w, 2)}
            height={h}
            fill="#e6777e26"
            stroke="#e6777e"
          />
          <rect
            x={x}
            y={Math.min(a.y, p.y)}
            width={Math.max(w, 2)}
            height={Math.abs(p.y - a.y)}
            fill="#58cba822"
            stroke="#58cba8"
          />
          {line(x, a.y, x + w, a.y)}
          {text(x + 5, p.y - 6, `Doel ${format(target)}`)}
          {text(
            x + 5,
            a.y - 6,
            `${d.b.price < d.a.price ? "Long" : "Short"} · ${risk ? (reward / risk).toFixed(2) : "0"}:1 · ${format(d.a.price)}`,
          )}
          {text(x + 5, b.y + 16, `Stop ${format(d.b.price)}`)}
        </>
      );
    } else
      shape = (
        <>
          {line(a.x, a.y, b.x, b.y)}
          {d.kind === "measure" && (
            <>
              <rect
                x={x}
                y={y}
                width={w}
                height={h}
                fill={color + "12"}
                stroke={color}
                strokeDasharray="3 4"
              />
              {text(
                x + 5,
                y - 10,
                `${((d.b.price / d.a.price - 1) * 100).toFixed(2)}% · ${format(d.b.price - d.a.price)} · ${Math.round(Math.abs(d.b.time - d.a.time) / 60000)} min`,
              )}
            </>
          )}
        </>
      );
  }
  return (
    <g
      data-drawing-id={d.id}
      data-kind={d.kind}
      className={`drawing-object ${preview ? "is-preview" : ""} ${selected ? "is-selected" : ""}`}
      style={{ cursor: d.locked ? "pointer" : "move" }}
      onPointerDown={(e) => onStart(e, "move")}
      onDoubleClick={onEdit}
    >
      <title>
        {labelFor(d.kind)}
        {d.locked ? " · vergrendeld" : " · sleep om te verplaatsen"}
      </title>
      {shape}
      {(selected || preview) &&
        handles.map(({ key, p }) => (
          <circle
            key={key}
            data-handle={key}
            cx={p.x}
            cy={p.y}
            r={selected ? 5 : 3}
            fill="var(--panel)"
            stroke={color}
            strokeWidth="2"
            className="drawing-handle"
            onPointerDown={(e) => {
              e.stopPropagation();
              onStart(e, key);
            }}
          />
        ))}
    </g>
  );
}
