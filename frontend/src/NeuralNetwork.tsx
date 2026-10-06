import { useEffect, useRef, useState } from "react";
import type { NeuralArtifact, Activation } from "./training-types";
const labels = ["Rendement 1", "Rendement 5", "Volatiliteit", "Trend", "RSI", "Volume", "Bollinger z", "ATR", "Positie", "Open resultaat", "Houdduur", "Kosten", "Laya trend", "Laya range", "Laya volatiel", "Laya zekerheid"];
const outputs = ["Trend", "Breakout", "Mean reversion", "Wachten"];
type Dot = { x: number; y: number; z: number; layer: number; index: number; value: number };
export default function NeuralNetwork({ artifact, activation }: { artifact?: NeuralArtifact; activation?: Activation }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const angle = useRef({ yaw: -.12, pitch: .12 });
  const [mode, setMode] = useState("network");
  const [running, setRunning] = useState(true);
  const [selected, setSelected] = useState<{ layer: number; index: number }>();
  const hit = useRef<{ x: number; y: number; dot: Dot }[]>([]);
  const weights = artifact?.weights;
  const matrix = mode === "matrix";
  const chosen = activation?.selected ?? -1;
  useEffect(() => {
    const el = canvas.current;
    if (!el || matrix) return;
    const ctx = el.getContext("2d");
    if (!ctx) return;
    const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
    let raf = 0;
    let width = 900, height = 560;
    const resize = () => { const r = el.getBoundingClientRect(); width = r.width; height = r.height; el.width = width * devicePixelRatio; el.height = height * devicePixelRatio; };
    const observer = new ResizeObserver(resize); observer.observe(el); resize();
    function render(t: number) {
      if (!ctx) return;
      ctx.setTransform(devicePixelRatio, 0, 0, devicePixelRatio, 0, 0); ctx.clearRect(0, 0, width, height);
      const scale = Math.min(width / 11.5, height / 7.4);
      const { yaw, pitch } = angle.current;
      const project = (d: Dot) => {
        const x = d.x * Math.cos(yaw) + d.z * Math.sin(yaw);
        const z = -d.x * Math.sin(yaw) + d.z * Math.cos(yaw);
        const y = d.y * Math.cos(pitch) - z * Math.sin(pitch);
        const q = 11 / (11 + z * Math.cos(pitch) + d.y * Math.sin(pitch));
        return { x: width / 2 + x * scale * q, y: height / 2 + y * scale * q, q };
      };
      const dots: Dot[] = [];
      for (let layer = 0; layer < 3; layer++) {
        const count = layer === 2 ? 4 : 16;
        for (let i = 0; i < count; i++) {
          const ring = i / count * Math.PI * 2;
          dots.push({ layer, index: i, x: (layer - 1) * 3.35, y: layer === 2 ? (i - 1.5) * 1.15 : (i - 7.5) * .36, z: layer === 1 ? Math.sin(ring) * .9 : Math.cos(ring) * .38, value: layer === 0 ? activation?.inputs[i] ?? 0 : layer === 1 ? activation?.hidden[i] ?? 0 : activation?.outputs[i] ?? 0 });
        }
      }
      const coords = dots.map(project);
      hit.current = dots.map((dot, i) => ({ dot, x: coords[i].x, y: coords[i].y }));
      // Deterministic background particles belong to the visual, not the model.
      for (let i = 0; i < 65; i++) {ctx.fillStyle = `rgba(151,195,218,${.06 + (i % 4) * .035})`;ctx.beginPath();ctx.arc((i * 137.73) % width, (i * 83.39) % height, .7, 0, Math.PI * 2);ctx.fill();}
      for (let layer = 0; layer < 2; layer++) {
        const from = layer === 0 ? 0 : 16, to = layer === 0 ? 16 : 32;
        const n = layer === 0 ? 16 : 4;
        for (let j = 0; j < n; j++) for (let i = 0; i < 16; i++) {
          const w = weights?.[(layer === 0 ? 0 : 272) + j * 16 + i] ?? 0;
          const a = coords[from + i], b = coords[to + j];
          const focused = !selected || (selected.layer === layer && selected.index === i) || (selected.layer === layer + 1 && selected.index === j);
          const strength = Math.min(Math.abs(w) / 2, 1);
          const color = w < 0 ? "251,128,151" : "68,219,224";
          ctx.strokeStyle = `rgba(${color},${focused ? .035 + strength * .28 : .012})`;ctx.lineWidth = .45 + strength * 1.5;
          ctx.beginPath();ctx.moveTo(a.x, a.y);ctx.bezierCurveTo(a.x + (b.x - a.x) * .44, a.y, b.x - (b.x - a.x) * .44, b.y, b.x, b.y);ctx.stroke();
          if (running && !reduce && weights && focused && Math.abs(w) > .6 && (i + j) % 5 === 0) {
            const p = (t / 2600 + i * .137 + j * .23) % 1;
            ctx.fillStyle = `rgba(${color},${.25 + strength * .5})`;ctx.beginPath();ctx.arc(a.x + (b.x - a.x) * p, a.y + (b.y - a.y) * (p * p * (3 - 2 * p)), 1.6, 0, Math.PI * 2);ctx.fill();
          }
        }
      }
      dots.forEach((d, i) => {
        const p = coords[i];const focus = selected?.layer === d.layer && selected.index === d.index;const winning = d.layer === 2 && d.index === chosen;
        const color = d.layer === 1 ? "151,134,255" : d.layer === 2 ? winning ? "75,227,185" : "104,143,167" : "77,207,230";
        const r = (d.layer === 2 ? 11 : 6) * p.q + Math.min(Math.abs(d.value), 1) * 2;
        const glow = ctx.createRadialGradient(p.x, p.y, 0, p.x, p.y, r * 4);glow.addColorStop(0, `rgba(${color},${focus || winning ? .3 : .12})`);glow.addColorStop(1, `rgba(${color},0)`);ctx.fillStyle = glow;ctx.beginPath();ctx.arc(p.x, p.y, r * 4, 0, Math.PI * 2);ctx.fill();
        ctx.fillStyle = `rgba(${color},${.22 + Math.min(Math.abs(d.value), 1) * .65})`;ctx.strokeStyle = `rgba(${color},${focus || winning ? 1 : .6})`;ctx.lineWidth = focus ? 2 : 1;ctx.beginPath();ctx.arc(p.x, p.y, r, 0, Math.PI * 2);ctx.fill();ctx.stroke();
        ctx.font = `${d.layer === 2 ? 12 : 10}px Inter,system-ui`;ctx.textBaseline = "middle";ctx.fillStyle = d.layer === 2 ? "#d6e5f0" : "#92a8be";
        if (d.layer === 0) {ctx.textAlign = "right";ctx.fillText(labels[d.index], p.x - 14, p.y);}
        if (d.layer === 2) {ctx.textAlign = "left";ctx.fillText(`${outputs[d.index]}${activation ? ` · ${(d.value * 100).toFixed(1)}%` : ""}`, p.x + 20, p.y);}
      });
      raf = requestAnimationFrame(render);
    }
    raf = requestAnimationFrame(render);
    return () => {cancelAnimationFrame(raf);observer.disconnect();};
  }, [artifact, activation, matrix, running, selected]);
  const dragging = useRef<{ x: number; y: number; moved: boolean } | undefined>(undefined);
  return <div className="neural-observatory">
    <div className="neural-tools"><div><span className="lab-eyebrow">NEURAL OBSERVATORY</span><h3>Het brein achter de keuze</h3></div><div className="lab-toggle"><button className={!matrix ? "active" : ""} onClick={() => setMode("network")}>Netwerk</button><button className={matrix ? "active" : ""} onClick={() => setMode("matrix")}>Gewichten</button><button onClick={() => setRunning(!running)} aria-pressed={running}>{running ? "Ⅱ" : "▶"}</button></div></div>
    {matrix ? <div className="neural-matrices">{[0, 1].map((layer) => <div key={layer}><small>{layer === 0 ? "INPUT → VERBORGEN LAAG" : "VERBORGEN LAAG → STRATEGIE"}</small><div className="weight-matrix" style={{ gridTemplateColumns: "repeat(16, 1fr)" }}>{Array.from({ length: layer === 0 ? 256 : 64 }, (_, i) => {const w = weights?.[(layer === 0 ? 0 : 272) + i] ?? 0;return <span key={i} title={`Gewicht ${i + 1}: ${w.toFixed(5)}`} style={{ background: w < 0 ? `rgba(251,128,151,${.08 + Math.min(Math.abs(w) / 3, .9)})` : `rgba(68,219,224,${.08 + Math.min(Math.abs(w) / 3, .9)})` }} />;})}</div></div>)}</div> : <canvas ref={canvas} aria-label="Interactief neuraal netwerk: zestien inputs, zestien verborgen neuronen, vier strategieën" onPointerDown={(e) => {e.currentTarget.setPointerCapture(e.pointerId);dragging.current = { x: e.clientX, y: e.clientY, moved: false };}} onPointerMove={(e) => {if (!dragging.current) return;const dx = e.clientX - dragging.current.x, dy = e.clientY - dragging.current.y;angle.current.yaw += dx * .004;angle.current.pitch = Math.max(-.45, Math.min(.45, angle.current.pitch + dy * .003));dragging.current = { x: e.clientX, y: e.clientY, moved: dragging.current.moved || Math.abs(dx) + Math.abs(dy) > 3 };}} onPointerUp={(e) => {if (!dragging.current?.moved) {const r = e.currentTarget.getBoundingClientRect();const h = hit.current.find(p => Math.hypot(p.x - (e.clientX - r.left), p.y - (e.clientY - r.top)) < 15);setSelected(h ? { layer: h.dot.layer, index: h.dot.index } : undefined);}dragging.current = undefined;}} />}
    <div className="neural-caption"><span><i className="cyan" /> Positief gewicht <i className="rose" /> Negatief gewicht</span><span>{weights ? `${weights.length} gewichten en biases · tanh → softmax` : "Architectuurpreview · nog geen getraind netwerk"}</span><span>{selected ? selected.layer === 0 ? labels[selected.index] : selected.layer === 2 ? outputs[selected.index] : `Neuron ${selected.index + 1}` : "Sleep om te draaien · klik op een neuron"}</span></div>
    <div className="neural-risk"><span>16 inputs</span><b>→</b><span>16 verborgen neuronen</span><b>→</b><span>4 strategiekeuzes</span><b>→</b><strong>Vaste risicokern</strong></div>
  </div>;
}
