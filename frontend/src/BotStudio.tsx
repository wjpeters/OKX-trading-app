import { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ReactFlow,
  Background,
  Controls,
  Handle,
  Position,
  addEdge,
  applyNodeChanges,
  applyEdgeChanges,
} from "@xyflow/react";
import type { Node, Edge, NodeProps } from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { api } from "./types";
import type { Market } from "./types";
import { usePreferences } from "./store";
import { post, Spark, download } from "./studio-ui";
import type { Doc } from "./studio-ui";
import PaperWorkspace, { LimitsForm, PaperSetup } from "./Paper";
import BotOverview from "./BotOverview";
import BotActivity, { money } from "./BotActivity";
import type { PaperState } from "./Paper";
import {
  template,
  settings,
  names,
  defaults,
  readDraft,
  stableJSON,
} from "./bot-model";
import type { Graph, Params, BotSettings } from "./bot-model";
import "./studio.css";
import TrainingLab from "./TrainingLab";
function StudioNode({ data, selected }: NodeProps) {
  const kind = String(data.kind);
  const p = data.params as Params;
  return (
    <div className={`studio-node ${kind} ${selected ? "selected" : ""}`}>
      {kind !== "market" && <Handle type="target" position={Position.Left} />}
      <small>
        {kind === "pine"
          ? "PINE"
          : kind === "risk_gate"
            ? "BESCHERMING"
            : kind === "if_else"
              ? "BESLISSING"
              : "BOTSTAP"}
      </small>
      <strong>{names[kind]}</strong>
      <span>
        {kind === "pine"
          ? p.name
            ? `${p.name} · v${p.version}`
            : "Kies je script"
          : kind === "candle_stop"
            ? `Daling ≥ ${p.drop_pct ?? 1}% · 1 minuut`
            : ["sma_cross", "ema_cross", "macd_cross"].includes(kind)
              ? `${p.fast} / ${p.slow}`
              : kind === "if_else" || kind === "condition"
                ? `${p.field} ${p.comparison === "below" ? "<" : ">"} ${p.value}`
                : kind === "action"
                  ? { buy: "Koop", sell: "Verkoop", hold: "Wacht" }[
                      String(p.action)
                    ]
                  : "Klik om in te stellen"}
      </span>
      {kind === "if_else" ? (
        <>
          <Handle
            type="source"
            id="true"
            position={Position.Right}
            style={{ top: "38%" }}
          />
          <em className="branch-yes">Ja</em>
          <Handle
            type="source"
            id="false"
            position={Position.Right}
            style={{ top: "78%" }}
          />
          <em className="branch-no">Nee</em>
        </>
      ) : (
        kind !== "risk_gate" && (
          <Handle type="source" position={Position.Right} />
        )
      )}
    </div>
  );
}
const nodeTypes = { studio: StudioNode };
function toNodes(g: Graph): Node[] {
  return g.nodes.map((n) => ({
    id: n.id,
    position: n.position,
    type: "studio",
    data: { kind: n.kind, params: n.params },
    deletable: !["market", "intent", "risk_gate"].includes(n.kind),
  }));
}
function toEdges(g: Graph): Edge[] {
  return g.edges.map((e, i) => ({
    ...e,
    id: "e" + i,
    label:
      e.sourceHandle === "true"
        ? "Ja"
        : e.sourceHandle === "false"
          ? "Nee"
          : undefined,
  }));
}
type Draft = { name: string; version: number; graph: Graph; saved: string };
type Report = {
  id: number;
  report: {
    strategy: string;
    bars: number;
    fill_count: number;
    net_pnl: string;
    fees: string;
    max_drawdown: string;
    first_ms: number;
    last_ms: number;
    curve: { equity: string }[];
    warmup: string;
    decisions: { signal: string; outcome: string; ts: number }[];
  };
};
export default function BotStudio({ market }: { market: Market }) {
  const pref = usePreferences();
  const qc = useQueryClient();
  const [initial] = useState(() =>
    readDraft<Draft>("okx-bot-draft-v2", {
      name: "Mijn bot",
      version: 0,
      graph: { ...template(), bot: settings(pref.symbol) },
      saved: "",
    }),
  );
  const [nodes, setNodes] = useState<Node[]>(toNodes(initial.graph));
  const [edges, setEdges] = useState<Edge[]>(toEdges(initial.graph));
  const [name, setName] = useState(initial.name);
  const [version, setVersion] = useState(initial.version);
  const [config, setConfig] = useState<BotSettings>(
    initial.graph.bot || settings(pref.symbol),
  );
  const [saved, setSaved] = useState(initial.saved);
  const [canvasKey, setCanvasKey] = useState(0);
  const [selected, setSelected] = useState<string>();
  const [tab, setTab] = useState("Bouwen");
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const [report, setReport] = useState<Report>();
  const [spread, setSpread] = useState("10");
  const [slippage, setSlippage] = useState("5");
  const paper = useQuery({
    queryKey: ["paper"],
    queryFn: () => api<PaperState>("/api/paper"),
    refetchInterval: 3000,
  });
  const runtime = paper.data?.account?.bots[config.config.id];
  const deployedVersion = runtime?.version.split(":").at(-1);
  const [searchNodes, setSearchNodes] = useState("");
  const docs = useQuery({
    queryKey: ["documents", "graph"],
    queryFn: () => api<Doc[]>("/api/studio/documents?kind=graph"),
  });
  const systemBots = useQuery({
    queryKey: ["system-bots"],
    queryFn: () =>
      api<{ id: string; name: string; description: string; graph: Graph }[]>(
        "/api/studio/system-bots",
      ),
  });
  const pine = useQuery({
    queryKey: ["documents", "pine"],
    queryFn: () => api<Doc[]>("/api/studio/documents?kind=pine"),
  });
  const reports = useQuery({
    queryKey: ["graph-reports"],
    queryFn: () => api<Report[]>("/api/replay/reports?mode=GRAPH_REPLAY"),
    enabled: tab === "Backtest",
  });
  const canonical = (): Graph => ({
    schema: 1,
    nodes: nodes.map((n) => ({
      id: n.id,
      kind: String(n.data.kind),
      params: n.data.params as Params,
      position: n.position,
    })),
    edges: edges.map((e) => ({
      source: e.source,
      target: e.target,
      ...(e.sourceHandle ? { sourceHandle: e.sourceHandle } : {}),
    })),
    bot: { ...config, config: { ...config.config, name } },
  });
  const fingerprint = stableJSON({ name, graph: canonical() });
  const dirty = saved !== fingerprint;
  const runtimeCurrent =
    !!runtime && !dirty && runtime.version === `graph:${name}:${version}`;
  useEffect(() => {
    localStorage.setItem(
      "okx-bot-draft-v2",
      JSON.stringify({ name, version, graph: canonical(), saved }),
    );
  }, [fingerprint, version, saved]);
  function load(d: Doc) {
    setCanvasKey((k) => k + 1);
    const g = d.payload as unknown as Graph;
    setNodes(toNodes(g));
    setEdges(toEdges(g));
    setName(d.name);
    setVersion(d.version);
    const b = g.bot || settings(pref.symbol);
    setConfig(b);
    setSaved(
      stableJSON({
        name: d.name,
        graph: { ...g, bot: { ...b, config: { ...b.config, name: d.name } } },
      }),
    );
    setSelected(undefined);
    setReport(undefined);
    setTab("Bouwen");
    setMessage(`${d.name} v${d.version} geladen.`);
  }
  useEffect(() => {
    const raw = sessionStorage.getItem("okx-open-bot");
    if (raw) {
      sessionStorage.removeItem("okx-open-bot");
      try {
        load(JSON.parse(raw));
      } catch {
        setMessage("Bot kon niet worden geopend.");
      }
    }
    const fromPine = sessionStorage.getItem("okx-pine-node");
    if (fromPine) {
      sessionStorage.removeItem("okx-pine-node");
      try {
        const d = JSON.parse(fromPine) as Doc;
        addNode("pine", { name: d.name, version: d.version });
        setMessage("Pine-node toegevoegd. Verbind deze met je bot.");
      } catch {
        setMessage("Script kon niet worden geopend.");
      }
    }
  }, []);
  async function perform(fn: () => Promise<void>) {
    setBusy(true);
    setMessage("");
    try {
      await fn();
    } catch (e) {
      setMessage(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function save() {
    const g = canonical();
    await post("/api/studio/graph/validate", g);
    const r = await post<{ version: number }>("/api/studio/documents", {
      kind: "graph",
      name,
      base_version: version,
      payload: g,
    });
    setVersion(r.version);
    setSaved(fingerprint);
    await qc.invalidateQueries({ queryKey: ["documents", "graph"] });
    setMessage(`${name} · versie ${r.version} opgeslagen.`);
    return r.version;
  }
  function addNode(kind: string, params: Params = { ...defaults[kind] }) {
    setCanvasKey((k) => k + 1);
    const id = crypto.randomUUID().slice(0, 8);
    setNodes((ns) => [
      ...ns,
      {
        id,
        type: "studio",
        position: {
          x: 260 + (ns.length % 3) * 210,
          y: 260 + Math.floor(ns.length / 3) * 40,
        },
        data: { kind, params },
      },
    ]);
    setSelected(id);
    setTab("Bouwen");
  }
  function newBot(type: string) {
    setTab("Bouwen");
    setCanvasKey((k) => k + 1);
    const g = template(type);
    setNodes(toNodes(g));
    setEdges(toEdges(g));
    setConfig(settings(pref.symbol));
    setName(
      type === "branch"
        ? "Mijn if-else bot"
        : "Mijn bot " + new Date().toLocaleTimeString("nl-NL"),
    );
    setVersion(0);
    setSaved("");
    setSelected(undefined);
    setReport(undefined);
  }
  const node = nodes.find((n) => n.id === selected);
  const params = (node?.data.params || {}) as Params;
  const update = (key: string, value: string | number | boolean) =>
    setNodes((ns) =>
      ns.map((n) =>
        n.id === selected
          ? { ...n, data: { ...n.data, params: { ...params, [key]: value } } }
          : n,
      ),
    );
  const kind = String(node?.data.kind || "");
  const number = (key: string, label: string, min: number, max: number) => (
    <label key={key}>
      {label}
      <input
        type="number"
        min={min}
        max={max}
        step="any"
        value={String(params[key] ?? defaults[kind]?.[key] ?? 0)}
        onChange={(e) => update(key, Number(e.target.value))}
      />
    </label>
  );
  const select = (key: string, label: string, options: [string, string][]) => (
    <label>
      {label}
      <select
        value={String(params[key] ?? defaults[kind]?.[key] ?? options[0][0])}
        onChange={(e) => update(key, e.target.value)}
      >
        {options.map(([v, l]) => (
          <option key={v} value={v}>
            {l}
          </option>
        ))}
      </select>
    </label>
  );
  const botField = (
    key: "order_eur" | "stop_bps" | "target_bps",
    label: string,
    scale = 1,
  ) => (
    <label>
      {label}
      <input
        type="number"
        min={key === "order_eur" ? 5 : 0.1}
        step="any"
        value={Number(config.config[key]) / scale}
        onChange={(e) =>
          setConfig({
            ...config,
            config: {
              ...config.config,
              [key]: String(Number(e.target.value) * scale),
            },
          })
        }
      />
    </label>
  );
  return (
    <section className={`studio-page bot-studio${tab === "Training Lab" ? " is-training" : ""}`}>
      <div className="studio-title">
        <div>
          <h2>Bot Studio</h2>
          <p>
            Bouw met nodes, test je strategie en volg je bot op live koersen.
          </p>
        </div>
        <span className="pill">
          {dirty ? "Concept automatisch bewaard" : `Opgeslagen · v${version}`}
        </span>
      </div>
      <div className="studio-toolbar">
        <input
          aria-label="Botnaam"
          value={name}
          maxLength={60}
          onChange={(e) => {
            setName(e.target.value);
            setVersion(0);
          }}
        />
        <button
          className="primary"
          disabled={busy || !name.trim()}
          onClick={() =>
            perform(async () => {
              await save();
            })
          }
        >
          {busy ? "Bezig…" : "Bot opslaan"}
        </button>
        <select
          aria-label="Bot laden"
          value=""
          onChange={(e) => {
            if (e.target.value !== "") {
              const d = docs.data?.[+e.target.value];
              if (d) load(d);
            }
          }}
        >
          <option value="">Opgeslagen bots</option>
          {docs.data?.map((d, i) => (
            <option key={i} value={i}>
              {d.name} · v{d.version}
            </option>
          ))}
        </select>
        <button
          disabled={busy}
          onClick={() => {
            setName(name + " kopie");
            setVersion(0);
            setConfig({
              ...config,
              config: { ...config.config, id: settings().config.id },
            });
            setSaved("");
            setMessage("Kopie klaar. Kies een naam en sla op.");
          }}
        >
          Kopie maken
        </button>
        <select
          aria-label="Nieuwe bot"
          value=""
          onChange={(e) => {
            if (e.target.value) newBot(e.target.value);
          }}
        >
          <option value="">Nieuwe bot…</option>
          <option value="trend">SMA-trend</option>
          <option value="range">RSI-zone</option>
          <option value="branch">If / else voorbeeld</option>
        </select>
      </div>
      <div className="studio-toolbar">
        <button className="primary" onClick={() => setTab("Systeembots")}>
          Kies uit 5 systeembots
        </button>
        <span className="muted">
          {config.ai_enabled
            ? "AI alleen via een expliciete Jev-node"
            : "Zonder AI · jouw signaal bepaalt de actie"}
          . De risicocontrole volgt uitsluitend vaste regels.
        </span>
      </div>
      <div
        className="studio-tabs bot-steps"
        role="group"
        aria-label="Botwerkruimte"
      >
        {[
          "Bouwen",
          "Instellingen",
          "Backtest",
          "Start & volg",
          "Mijn bots",
          "Systeembots",
          "Training Lab",
        ].map((t, i) => (
          <button
            key={t}
            className={tab === t ? "active" : ""}
            aria-pressed={tab === t}
            onClick={() => setTab(t)}
          >
            {i < 4 && <span>{i + 1}</span>}
            {t}
          </button>
        ))}
      </div>
      <div className="bot-context-bar">
        <span>
          <strong>{name}</strong> · {config.config.instrument} ·{" "}
          {config.mode === "paper"
            ? "Live data / virtueel geld"
            : "Echt geld / nog niet beschikbaar"}
        </span>
        <span>
          {runtime
            ? `Uitvoering v${deployedVersion} · ${runtime.running ? "actief" : "gepauzeerd"}${!runtimeCurrent ? " · ontwerp wijkt af" : ""}`
            : "Ontwerp · nog niet geactiveerd"}
        </span>
        <button onClick={() => setTab("Start & volg")}>Start & volg →</button>
      </div>
      <p className="studio-feedback" role="status">
        {message ||
          {
            Bouwen:
              "Klik op een node voor instellingen. Sleep tussen de ronde aansluitpunten om stappen te verbinden.",
            Instellingen:
              "Kies de markt, het virtuele botbudget en de risicolimieten.",
            Backtest: "Test je opgeslagen ontwerp op historische koersen.",
            "Start & volg":
              "Controleer je instellingen en start met virtueel geld op actuele OKX-koersen.",
            "Mijn bots":
              "Open een ontwerp of volg de daadwerkelijke uitvoering.",
          }[tab]}
      </p>
      {tab === "Bouwen" && (
        <>
          <div className="studio-toolbar">
            <label className="node-search">
              Node zoeken
              <input
                aria-label="Node zoeken"
                placeholder="Bijvoorbeeld Pine, if / else of RSI"
                value={searchNodes}
                onChange={(e) => setSearchNodes(e.target.value)}
              />
            </label>
            <button onClick={() => setTab("Instellingen")}>
              Verder: instellingen →
            </button>
          </div>
          <div className="node-palette">
            {[
              [
                "Signalen",
                [
                  "sma_cross",
                  "ema_cross",
                  "rsi_range",
                  "macd_cross",
                  "bollinger",
                  "pine",
                ],
              ],
              [
                "Logica",
                ["if_else", "condition", "and", "or", "not", "action"],
              ],
              ["Bescherming", ["candle_stop"]],
              [
                "Filters",
                [
                  "time_window",
                  "cooldown",
                  "news",
                  ...(config.ai_enabled ? ["jev"] : []),
                ],
              ],
            ].map(([label, kinds]) => (
              <div key={String(label)}>
                <small>{String(label)}</small>
                {(kinds as string[])
                  .filter((k) =>
                    (names[k] + " " + k)
                      .toLowerCase()
                      .includes(searchNodes.toLowerCase()),
                  )
                  .map((k) => (
                    <button key={k} onClick={() => addNode(k)} title={names[k]}>
                      + {names[k]}
                    </button>
                  ))}
              </div>
            ))}
          </div>
          <div className="graph-grid">
            <div className="graph-canvas">
              <ReactFlow
                key={canvasKey}
                nodes={nodes}
                edges={edges}
                nodeTypes={nodeTypes}
                colorMode={pref.light ? "light" : "dark"}
                fitView
                minZoom={0.15}
                onNodesChange={(c) => setNodes((ns) => applyNodeChanges(c, ns))}
                onEdgesChange={(c) => setEdges((es) => applyEdgeChanges(c, es))}
                onConnect={(c) =>
                  setEdges((es) =>
                    addEdge(
                      {
                        ...c,
                        label:
                          c.sourceHandle === "true"
                            ? "Ja"
                            : c.sourceHandle === "false"
                              ? "Nee"
                              : undefined,
                      },
                      es,
                    ),
                  )
                }
                onNodeClick={(_, n) => setSelected(n.id)}
              >
                <Background />
                <Controls />
              </ReactFlow>
            </div>
            <aside className="studio-card node-inspector">
              <h3>{node ? names[kind] : "Node-instellingen"}</h3>
              {node && (
                <p className="node-help">
                  {
                    (
                      {
                        market:
                          "Levert gesloten candles en marktgegevens aan de volgende stap.",
                        sma_cross:
                          "Geeft een koop- of verkoopsignaal als de snelle SMA de langzame SMA kruist.",
                        rsi_range:
                          "Koopsignaal onder de lage RSI-grens; verkoopsignaal boven de hoge grens.",
                        pine: "Gebruikt een vast opgeslagen script en versie uit de Pine Editor. Kies entry/exit-signalen of vergelijk een numerieke plot.",
                        if_else:
                          "Vergelijkt een marktwaarde. Alleen de Ja- of Nee-route gaat verder. Verbind beide uitgangen.",
                        condition:
                          "Laat een koopsignaal alleen door als deze vergelijking klopt. Verkoopsignalen blijven doorgaan.",
                        and: "Koopsignaal als alle actieve ingangen koop aangeven. Een verkoopsignaal heeft altijd voorrang.",
                        or: "Combineert actieve routes. Een verkoopsignaal heeft voorrang op een koopsignaal.",
                        not: "Keert een koopvoorwaarde om: wachten wordt koop en koop wordt wachten. Verkoop en inactieve routes blijven behouden.",
                        action:
                          "Maakt van de actieve route een koop-, verkoop- of wachtsignaal.",
                        time_window:
                          "Laat koopsignalen alleen binnen het ingestelde UTC-tijdvenster door. Verkopen blijft mogelijk.",
                        cooldown:
                          "Beperkt nieuwe kooporders na een eerdere order. Verkopen blijft mogelijk.",
                        intent:
                          "Maakt een ordervoorstel van het signaal, met het orderbedrag en de stop uit Instellingen.",
                        risk_gate:
                          "Verplichte laatste stap. De risicokern toetst budgetten, posities en verse marktdata vóór uitvoering.",
                        news: "Gebruikt recente nieuwscontext. Deze node is niet beschikbaar in historische backtests.",
                        jev: "Vraagt Jev om advies binnen het ingestelde AI-budget. Jev kan zelf geen orders uitvoeren; historische backtests ondersteunen deze node niet.",
                        ema_cross:
                          "Koop of verkoop bij een kruising van de twee exponentiële gemiddelden.",
                        macd_cross:
                          "Koop of verkoop bij een kruising van MACD en de signaallijn.",
                        bollinger:
                          "Koop onder de onderste band; verkoop bij het gemiddelde.",
                        candle_stop:
                          "Vaste noodregel voor een scherpe daling binnen een gesloten minuutcandle. Werkt zonder AI en vóór de gewone signaalroute.",
                        sizing:
                          "Past de ordergrootte binnen de ingestelde botlimieten aan.",
                      } as Record<string, string>
                    )[kind]
                  }
                </p>
              )}
              {!node && (
                <p>
                  Selecteer een blok in de grafiek. Gebruik + hierboven om een
                  stap toe te voegen. Lees de route van links naar rechts: markt
                  → signaal → ordervoorstel → risicocontrole.
                </p>
              )}
              {["sma_cross", "ema_cross", "macd_cross"].includes(kind) && (
                <>
                  {number("fast", "Snelle periode", 2, 99)}
                  {number("slow", "Langzame periode", 3, 100)}
                </>
              )}
              {kind === "candle_stop" && (
                <>
                  {number(
                    "drop_pct",
                    "Stop bij daling vanaf candle-opening (%)",
                    0.01,
                    50,
                  )}
                  {select("action", "Actie bij daling", [
                    ["close_pause", "Positie sluiten en bot pauzeren"],
                    ["close", "Alleen positie sluiten"],
                    ["pause", "Alleen pauzeren; positie behouden"],
                  ])}
                  <p>
                    Controleert altijd elke gesloten 1-minuutcandle, vóór het
                    handelssignaal en eventuele AI. Daling = (opening − slot) /
                    opening. Deze bescherming geldt voor de hele bot, ook in een
                    niet gekozen tak. Geen controle tijdens de nog lopende
                    candle; de gewone stop-loss blijft wel actief.
                  </p>
                </>
              )}
              {kind === "bollinger" && (
                <>
                  {number("period", "Periode (candles)", 2, 100)}
                  {number("deviations", "Standaardafwijkingen", 0.1, 5)}
                  <p>
                    Koop onder de onderste band. Sluit de positie bij terugkeer
                    naar het gemiddelde.
                  </p>
                </>
              )}
              {kind === "macd_cross" && (
                <p>De signaallijn gebruikt een EMA van 9 candles.</p>
              )}
              {kind === "rsi_range" && (
                <>
                  {number("low", "Koop onder RSI", 1, 49)}
                  {number("high", "Verkoop boven RSI", 51, 99)}
                </>
              )}
              {["condition", "if_else"].includes(kind) && (
                <>
                  {select(
                    "field",
                    "Indicator",
                    ["rsi", "adx", "atr", "sma", "ema"].map((k) => [
                      k,
                      k.toUpperCase(),
                    ]),
                  )}
                  {select("comparison", "Vergelijking", [
                    ["above", "Groter dan"],
                    ["below", "Kleiner dan"],
                  ])}
                  {number("value", "Drempel", -10000000, 10000000)}
                  {kind === "if_else" && (
                    <p>
                      Ja volgt de bovenste uitgang; Nee de onderste. Voeg de
                      takken weer samen met ‘Eén voorwaarde’.
                    </p>
                  )}
                </>
              )}
              {kind === "action" &&
                select("action", "Bij een signaal", [
                  ["buy", "Koop voorstellen"],
                  ["sell", "Positie sluiten"],
                  ["hold", "Wachten"],
                ])}
              {kind === "time_window" && (
                <>
                  {number("start_hour", "Vanaf (UTC-uur)", 0, 23)}
                  {number("end_hour", "Tot en met (UTC-uur)", 0, 23)}
                </>
              )}
              {kind === "cooldown" &&
                number("bars", "Wacht candles na laatste kooporder", 1, 1440)}
              {kind === "news" &&
                number("quiet_minutes", "Minuten zonder nieuws", 5, 1440)}
              {kind === "pine" && (
                <>
                  <label>
                    Opgeslagen script
                    <select
                      aria-label="Pine-script voor node"
                      value={
                        params.name ? `${params.name}::${params.version}` : ""
                      }
                      onChange={(e) => {
                        const d = pine.data?.find(
                          (d) => `${d.name}::${d.version}` === e.target.value,
                        );
                        if (d)
                          setNodes((ns) =>
                            ns.map((n) =>
                              n.id === selected
                                ? {
                                    ...n,
                                    data: {
                                      ...n.data,
                                      params: {
                                        name: d.name,
                                        version: d.version,
                                      },
                                    },
                                  }
                                : n,
                            ),
                          );
                      }}
                    >
                      <option value="">Kies een scriptversie</option>
                      {pine.data?.map((d) => (
                        <option
                          key={`${d.name}:${d.version}`}
                          value={`${d.name}::${d.version}`}
                        >
                          {d.name} · v{d.version}
                        </option>
                      ))}
                    </select>
                  </label>
                  {!pine.data?.length && (
                    <p>Bewaar eerst een script in de Pine Editor.</p>
                  )}
                  <label>
                    Signaalbron
                    <select
                      value={params.plot ? "plot" : "signals"}
                      onChange={(e) =>
                        update(
                          "plot",
                          e.target.value === "plot" ? "Mijn indicator" : "",
                        )
                      }
                    >
                      <option value="signals">
                        Entry / exit uit mijn strategie
                      </option>
                      <option value="plot">Indicatorplot vergelijken</option>
                    </select>
                  </label>
                  {!!params.plot && (
                    <>
                      <label>
                        Plotnaam
                        <input
                          value={String(params.plot)}
                          onChange={(e) => update("plot", e.target.value)}
                        />
                      </label>
                      {select("comparison", "Vergelijking", [
                        ["above", "Groter dan"],
                        ["below", "Kleiner dan"],
                      ])}
                      {number("value", "Drempel", -10000000, 10000000)}
                    </>
                  )}
                  <p>
                    Deze exacte versie en de opgeslagen scriptinputs worden
                    gebruikt. Botinterval: 1 minuut.
                  </p>
                </>
              )}
              {kind === "sizing" && (
                <>
                  {botField("order_eur", "Orderbedrag (€)")}
                  <p>Budget en bescherming staan bij Instellingen.</p>
                </>
              )}
              {kind === "market" && (
                <p>
                  {config.config.instrument} · gesloten 1-minuutcandles. De
                  markt kies je bij Instellingen.
                </p>
              )}
              {kind === "risk_gate" && (
                <p>
                  Elke order wordt getoetst aan je rekening- en botlimieten.
                  Deze stap blijft verplicht.
                </p>
              )}
              {kind === "jev" && (
                <p>
                  Jev geeft alleen voorstellen. Vereist een gekoppelde provider
                  en bevestigd AI-budget. Geen historische backtest beschikbaar.
                </p>
              )}
              {["and", "or", "not", "intent"].includes(kind) && (
                <p>
                  {kind === "and"
                    ? "Alle verbonden actieve voorwaarden moeten een signaal geven."
                    : kind === "or"
                      ? "Eén actieve tak is genoeg. Verkoopsignalen gaan voor."
                      : kind === "not"
                        ? "Keert een actieve voorwaarde om. Een niet gekozen if/else-tak blijft inactief."
                        : "Geeft het voorstel door aan de verplichte risicocontrole."}
                </p>
              )}
              {node && !["market", "risk_gate", "intent"].includes(kind) && (
                <button
                  onClick={() => {
                    setNodes((ns) => ns.filter((n) => n.id !== selected));
                    setEdges((es) =>
                      es.filter(
                        (e) => e.source !== selected && e.target !== selected,
                      ),
                    );
                    setSelected(undefined);
                  }}
                >
                  Node verwijderen
                </button>
              )}
            </aside>
          </div>
          <div className="studio-toolbar">
            <button
              disabled={busy}
              onClick={() =>
                perform(async () => {
                  await post("/api/studio/graph/validate", canonical());
                  setMessage("Alle routes en node-instellingen zijn geldig.");
                })
              }
            >
              Controleer verbindingen
            </button>
            <button
              onClick={() =>
                download(JSON.stringify(canonical(), null, 2), name + ".json")
              }
            >
              Exporteer bot
            </button>
            <label className="button">
              Importeer bot
              <input
                type="file"
                hidden
                accept=".json"
                onChange={(e) => {
                  const f = e.target.files?.[0];
                  if (f)
                    void perform(async () => {
                      if (f.size > 14000) throw Error("Bestand te groot");
                      const g = JSON.parse(await f.text()) as Graph;
                      await post("/api/studio/graph/validate", g);
                      load({
                        name: f.name.replace(/\.json$/, ""),
                        version: 0,
                        payload: g as unknown as Record<string, unknown>,
                        created_ms: 0,
                      });
                      setConfig({
                        ...(g.bot || settings(pref.symbol)),
                        config: {
                          ...(g.bot || settings(pref.symbol)).config,
                          id: settings().config.id,
                        },
                      });
                      setSaved("");
                    });
                }}
              />
            </label>
          </div>
        </>
      )}
      {tab === "Instellingen" && (
        <>
          <div className="studio-card">
            <h3>Botconfiguratie</h3>
            <div className="bot-ai-choice">
              <label>
                <input
                  type="checkbox"
                  checked={!!config.ai_enabled}
                  onChange={(e) =>
                    setConfig({ ...config, ai_enabled: e.target.checked })
                  }
                />{" "}
                AI-beoordeling via een Jev-node toestaan (optioneel)
              </label>
              <p>
                {config.ai_enabled
                  ? "Alleen een aangesloten Jev-node kan AI aanroepen. Daarvoor zijn apart bevestigde AI-budgetten nodig."
                  : "AI staat uit. Indicatoren en Pine-signalen worden direct volgens je nodes verwerkt. De risicocontrole is vaste software, geen AI."}
              </p>
              {!config.ai_enabled &&
                nodes.some((n) => n.data.kind === "jev") && (
                  <p className="warning">
                    Verwijder de Jev-node en verbind de route opnieuw voordat je
                    deze bot opslaat.
                  </p>
                )}
            </div>
            <div className="bot-config-grid">
              <label>
                Handelsmodus
                <select
                  aria-label="Handelsmodus"
                  value={config.mode}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      mode: e.target.value as "paper" | "live",
                    })
                  }
                >
                  <option value="paper">
                    Paper · live koersen, virtueel geld
                  </option>
                  <option value="live">
                    Echt geld · alleen ontwerp, starten niet beschikbaar
                  </option>
                </select>
              </label>
              <label>
                Markt
                <select
                  value={config.config.instrument}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      config: { ...config.config, instrument: e.target.value },
                    })
                  }
                >
                  {Array.from(
                    new Set([
                      config.config.instrument,
                      ...market.instruments
                        .filter((i) => i.quoteCcy === "EUR")
                        .map((i) => i.instId),
                    ]),
                  ).map((s) => (
                    <option key={s}>{s}</option>
                  ))}
                </select>
              </label>
              <label>
                Interval
                <input disabled value="1 minuut · gesloten candles" />
              </label>
              {botField("order_eur", "Bedrag per order (€)")}
              {botField("stop_bps", "Stop-loss (%)", 100)}
              {botField("target_bps", "Take-profit (%)", 100)}
            </div>
            {config.mode === "live" && (
              <p className="warning">
                Je kunt deze livebot opslaan en backtesten. Live starten is nog
                niet beschikbaar.
              </p>
            )}
            <div className="bot-config-grid">
              {(
                [
                  ["capital", "Toegewezen budget (€)"],
                  ["daily_loss", "Maximaal dagverlies (€)"],
                  ["total_loss", "Maximaal totaalverlies (€)"],
                  ["position", "Maximaal per positie (€)"],
                ] as const
              ).map(([key, label]) => (
                <label key={key}>
                  {label}
                  <input
                    type="number"
                    min="0"
                    step="any"
                    value={config.config.limits[key]}
                    onChange={(e) =>
                      setConfig({
                        ...config,
                        config: {
                          ...config.config,
                          limits: {
                            ...config.config.limits,
                            [key]: e.target.value,
                          },
                        },
                      })
                    }
                  />
                </label>
              ))}
            </div>
            <details className="studio-card">
              <summary>
                Alle risicolimieten en geavanceerde instellingen
              </summary>
              <LimitsForm
                title="Budget en risicolimieten"
                value={config.config.limits}
                onChange={(limits) =>
                  setConfig({ ...config, config: { ...config.config, limits } })
                }
              />
            </details>
            <div className="studio-toolbar">
              <button onClick={() => setTab("Backtest")}>
                Verder: backtesten →
              </button>
              <button
                className="primary"
                onClick={() => setTab("Start & volg")}
              >
                Naar starten op live data →
              </button>
            </div>
          </div>
        </>
      )}
      {tab === "Start & volg" && (
        <>
          <div className="studio-card bot-launch">
            <div className="studio-title">
              <div>
                <h2>Paperhandel op live data</h2>
                <p>
                  Je bot gebruikt actuele OKX-koersen. Alle orders, posities en
                  resultaten zijn virtueel.
                </p>
              </div>
              <span className="pill">Geen echt geld</span>
            </div>
            <div className="bot-stats">
              <div>
                <small>1 · Rekening</small>
                <strong>
                  {paper.data?.account
                    ? "Virtuele rekening klaar"
                    : "Nog aanmaken"}
                </strong>
                <span>
                  {paper.data?.account
                    ? money(paper.data.account.limits.capital) +
                      " startkapitaal"
                    : "Eenmalig je virtuele kapitaal kiezen"}
                </span>
              </div>
              <div>
                <small>2 · Deze bot</small>
                <strong>{money(config.config.limits.capital)} budget</strong>
                <span>
                  {config.config.instrument} · {money(config.config.order_eur)}{" "}
                  per order · stop {Number(config.config.stop_bps) / 100}%
                </span>
              </div>
              <div>
                <small>3 · Uitvoering</small>
                <strong>
                  {runtime?.running ? "Actief" : "Klaarzetten en starten"}
                </strong>
                <span>Elke gesloten 1-minuutcandle</span>
              </div>
            </div>
            {paper.isError && (
              <p role="alert">
                Rekening niet bereikbaar: {String(paper.error)}
              </p>
            )}
            {paper.isPending && <p>Rekening laden…</p>}
            {paper.data && !paper.data.account && (
              <PaperSetup
                presets={paper.data.presets}
                busy={busy || paper.isError}
                onSetup={(account, manual) =>
                  perform(async () => {
                    await post("/api/paper/setup", {
                      account,
                      manual,
                      confirmed: true,
                    });
                    await qc.invalidateQueries({ queryKey: ["paper"] });
                    setMessage(
                      "Virtuele rekening aangemaakt. Controleer je bot en klik op starten.",
                    );
                  })
                }
              />
            )}
            {paper.data?.account && (
              <>
                <p>
                  Starten bewaart dit ontwerp, past de versie toe en activeert
                  de bot. De uitvoerder blijft werken zolang deze lokale
                  appserver en Docker draaien, ook als je het browsertabblad
                  sluit.
                </p>
                {runtime && !runtimeCurrent && (
                  <p className="warning">
                    De draaiende/toegepaste versie is v{deployedVersion}. Je
                    wijzigingen zijn nog niet toegepast. Pauzeer eerst;
                    vervangen kan alleen zonder open orders of posities en met
                    hetzelfde botbudget.
                  </p>
                )}
                {config.mode === "live" && (
                  <p className="warning">
                    Echt geld is nog niet beschikbaar. Kies Paper bij
                    Instellingen om met live data en virtueel geld te handelen.
                  </p>
                )}
                {paper.data.account.ledger.paused && (
                  <p className="warning">
                    Rekening gepauzeerd: {paper.data.account.ledger.reason}.
                    Bekijk rekeningbeheer hieronder.
                  </p>
                )}
                {!paper.data.worker_alive && (
                  <p className="warning">
                    De uitvoerder is niet bereikbaar. Starten is tijdelijk
                    uitgeschakeld.
                  </p>
                )}
                <div className="studio-toolbar">
                  <button
                    className="primary"
                    disabled={
                      busy ||
                      paper.isError ||
                      (!runtime?.running &&
                        (config.mode !== "paper" ||
                          !paper.data.worker_alive ||
                          paper.data.account.ledger.paused))
                    }
                    onClick={() =>
                      perform(async () => {
                        if (runtime?.running) {
                          await post("/api/paper/bots/control", {
                            bot_id: runtime.id,
                            running: false,
                          });
                          setMessage(
                            "Bot gepauzeerd. Bescherming van open posities blijft actief.",
                          );
                        } else {
                          if (!runtimeCurrent) {
                            const v = dirty ? await save() : version;
                            await post("/api/studio/install", {
                              name,
                              version: v,
                            });
                          }
                          await post("/api/paper/bots/control", {
                            bot_id: config.config.id,
                            running: true,
                          });
                          setMessage(
                            "Bot gestart op live data. Volg hieronder de beslissingen en virtuele transacties.",
                          );
                        }
                      }).finally(() => {
                        void qc.invalidateQueries({ queryKey: ["paper"] });
                        void qc.invalidateQueries({
                          queryKey: ["bot-activity"],
                        });
                      })
                    }
                  >
                    {busy
                      ? "Bezig…"
                      : runtime?.running
                        ? "Pauzeer bot"
                        : runtimeCurrent
                          ? "Start op live data"
                          : "Opslaan & starten met virtueel geld"}
                  </button>
                  <button onClick={() => setTab("Instellingen")}>
                    Instellingen aanpassen
                  </button>
                </div>
              </>
            )}
          </div>
          {runtime && paper.data && (
            <BotActivity key={runtime.id} bot={runtime} state={paper.data} />
          )}
          {paper.data?.account && (
            <details className="studio-card">
              <summary>
                Rekeningbeheer · alle posities, orders en noodstop
              </summary>
              <PaperWorkspace market={market} />
            </details>
          )}
        </>
      )}
      {tab === "Backtest" && (
        <>
          <div className="studio-card">
            <h3>Test {name}</h3>
            <p>
              De opgeslagen nodegrafiek, Pine-versies en botlimieten worden
              uitgevoerd op beschikbare historische 1-minuutcandles. Geen echte
              orders.
            </p>
            <div className="studio-toolbar">
              <label>
                Spread (basispunten)
                <input
                  type="number"
                  min={1}
                  max={30}
                  value={spread}
                  onChange={(e) => setSpread(e.target.value)}
                />
              </label>
              <label>
                Slippage (basispunten)
                <input
                  type="number"
                  min={0}
                  max={20}
                  value={slippage}
                  onChange={(e) => setSlippage(e.target.value)}
                />
              </label>
              <button
                className="primary"
                disabled={busy}
                onClick={() =>
                  perform(async () => {
                    const v = dirty ? await save() : version;
                    const r = await post<Report>("/api/replay/graph", {
                      name,
                      version: v,
                      spread_bps: spread,
                      slippage_bps: slippage,
                    });
                    setReport(r);
                    await qc.invalidateQueries({ queryKey: ["graph-reports"] });
                    setMessage("Backtest afgerond en rapport opgeslagen.");
                  })
                }
              >
                {busy ? "Backtest uitvoeren…" : "Opslaan en backtesten"}
              </button>
              <select
                aria-label="Eerder backtestrapport"
                value=""
                onChange={(e) => {
                  if (e.target.value !== "")
                    setReport(reports.data?.[+e.target.value]);
                }}
              >
                <option value="">Eerder rapport openen</option>
                {reports.data?.map((r, i) => (
                  <option key={r.id} value={i}>
                    #{r.id} · {r.report.strategy}
                  </option>
                ))}
              </select>
            </div>
            <p>
              Fee 10 basispunten per fill. Maximaal 5.000 candles; 100 candles
              warm-up. Nieuws/Jev vereisen historische beslissingen en zijn hier
              niet beschikbaar. Pine initialiseert op de geladen historie;
              scripts met langdurig geheugen kunnen afwijken van doorlopende
              paperbots.
            </p>
          </div>
          {report && (
            <div className="studio-card">
              <h3>
                {report.report.strategy} · rapport #{report.id}
              </h3>
              <p>
                {new Date(report.report.first_ms).toLocaleString("nl-NL")} t/m{" "}
                {new Date(report.report.last_ms).toLocaleString("nl-NL")} ·{" "}
                {report.report.bars} candles
              </p>
              <div className="backtest-metrics">
                {[
                  ["Nettoresultaat", report.report.net_pnl],
                  ["Max. drawdown", report.report.max_drawdown],
                  ["Kosten", report.report.fees],
                ].map(([l, v]) => (
                  <div key={l}>
                    <small>{l}</small>
                    <strong>€ {Number(v).toFixed(2)}</strong>
                  </div>
                ))}
                <div>
                  <small>Fills</small>
                  <strong>{report.report.fill_count}</strong>
                </div>
              </div>
              <Spark
                values={report.report.curve.map((p) => Number(p.equity))}
              />
              <button
                onClick={() =>
                  download(
                    JSON.stringify(report, null, 2),
                    `bot-backtest-${report.id}.json`,
                  )
                }
              >
                Download volledig rapport
              </button>
              <details>
                <summary>
                  Beslissingen ({report.report.decisions.length})
                </summary>
                {report.report.decisions.slice(-40).map((d, i) => (
                  <p key={i}>
                    {new Date(d.ts).toLocaleString("nl-NL")} · {d.signal} ·{" "}
                    {d.outcome}
                  </p>
                ))}
              </details>
            </div>
          )}
        </>
      )}
      {tab === "Systeembots" && (
        <section className="system-bot-library">
          <div className="studio-title">
            <div>
              <h2>5 systeembots · zonder AI</h2>
              <p>
                Kant-en-klare voorbeelden met gewone indicatorsignalen. Open een
                eigen kopie, pas je instellingen aan en backtest of start met
                virtueel geld.
              </p>
            </div>
          </div>
          <p className="muted">
            Alle voorbeelden gebruiken gesloten 1-minuutcandles en een
            instelbare candle-stop van 1%. Ze starten nooit vanzelf.
          </p>
          {systemBots.isError && (
            <p role="alert">
              Voorbeelden konden niet worden geladen: {String(systemBots.error)}
            </p>
          )}
          {systemBots.isPending && <p>Voorbeelden laden…</p>}
          <div className="system-bot-cards">
            {systemBots.data?.map((example) => (
              <article className="studio-card" key={example.id}>
                <span className="pill">Systeemvoorbeeld · geen AI</span>
                <h3>{example.name}</h3>
                <p>{example.description}</p>
                <p>
                  Bij ≥ 1% candle-daling: afbouw aanvragen en pauzeren.
                  Aanpasbaar in de beschermingsnode.
                </p>
                <button
                  onClick={() => {
                    const bot = settings(pref.symbol);
                    load({
                      name: example.name + " · " + bot.config.id.slice(-4),
                      version: 0,
                      payload: { ...example.graph, bot } as unknown as Record<
                        string,
                        unknown
                      >,
                      created_ms: 0,
                    });
                    setSaved("");
                    setMessage(
                      "Eigen kopie geopend. Controleer de nodes en je instellingen. Opslaan of backtesten start geen bot.",
                    );
                  }}
                >
                  Gebruik {example.name}
                </button>
              </article>
            ))}
          </div>
        </section>
      )}
      {tab === "Mijn bots" && <BotOverview onOpen={load} />}
      {tab === "Training Lab" && <TrainingLab />}
      <details className="studio-card bot-help">
        <summary>Hoe werken Pine, bots en uitvoering samen?</summary>
        <p>
          <strong>Pine Editor:</strong> schrijf en bewaar een indicator of
          signaalscript. Numerieke plots kunnen op de terminalchart;
          entry/exit-plots of een plotvergelijking kunnen een botnode aansturen.
        </p>
        <p>
          <strong>Bot Studio:</strong> verbind signalen, voorwaarden en acties.
          Opslaan maakt een versie; een gewijzigde versie gaat pas draaien
          wanneer je die toepast via Start & volg.
        </p>
        <p>
          <strong>Uitvoering:</strong> de server beoordeelt nieuwe gesloten
          1-minuutcandles, controleert het ordervoorstel op risico en simuleert
          de order met live OKX-data. Volg de beslissingen en transacties hier
          of onder Bots in de terminal.
        </p>
        <p>
          Native TradingView strategy.*-orders worden nog niet ondersteund. Echt
          geld starten is nog niet beschikbaar.
        </p>
      </details>
    </section>
  );
}
