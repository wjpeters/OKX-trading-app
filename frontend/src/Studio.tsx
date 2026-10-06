import { post, Spark, download, previewPine } from "./studio-ui";
import type { Doc } from "./studio-ui";
import { useEffect, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import Editor, { loader } from "@monaco-editor/react";
import * as monaco from "monaco-editor/editor/editor.api";
import EditorWorker from "monaco-editor/editor/editor.worker.js?worker";
import { api } from "./types";
import type { Market } from "./types";
import { usePreferences } from "./store";
import { navigate, readDraft } from "./bot-model";
import "./studio.css";
(self as unknown as { MonacoEnvironment: unknown }).MonacoEnvironment = {
  getWorker: () => new EditorWorker(),
};
loader.config({ monaco });
monaco.languages.register({ id: "pine" });
monaco.languages.setMonarchTokensProvider("pine", {
  tokenizer: {
    root: [
      [/\/\/.*$/, "comment"],
      [/"[^"\n]*"/, "string"],
      [/\b(if|else|for|while|var|varip|true|false|na|and|or|not)\b/, "keyword"],
      [/\b(ta|math|input|plot|indicator|strategy|request|barstate)\b/, "type"],
      [/\d+(\.\d+)?/, "number"],
    ],
  },
});
monaco.languages.registerCompletionItemProvider("pine", {
  provideCompletionItems: (m, p) => ({
    suggestions: [
      "ta.sma(close, 20)",
      "ta.ema(close, 20)",
      "ta.rsi(close, 14)",
      "ta.crossover(close, average)",
      'plot(close, "Close")',
      'input.int(20, "Periode")',
      "barstate.isconfirmed",
    ].map((label) => ({
      label,
      kind: monaco.languages.CompletionItemKind.Function,
      insertText: label,
      range: new monaco.Range(
        p.lineNumber,
        m.getWordUntilPosition(p).startColumn,
        p.lineNumber,
        p.column,
      ),
    })),
  }),
});
const pineExample =
  '//@version=6\nindicator("Trend met risicovoorstel", overlay=true)\nfastPeriod = input.int(9, "Snel", minval=2, maxval=100)\nslowPeriod = input.int(21, "Langzaam", minval=3, maxval=200)\nfast = ta.sma(close, fastPeriod)\nslow = ta.sma(close, slowPeriod)\nplot(fast, "SMA snel")\nplot(slow, "SMA langzaam")\nplot(ta.crossover(fast, slow) ? 1 : 0, "entry")\nplot(ta.crossunder(fast, slow) ? 1 : 0, "exit")\n';
type PineInput = {
  id: string;
  name: string;
  title?: string;
  type: string;
  defval: string | number | boolean;
  minval?: number;
  maxval?: number;
  step?: number;
  options?: (string | number)[];
};
type PineResult = {
  inputs?: PineInput[];
  ok: boolean;
  runtime: string;
  duration_ms: number;
  bars: number;
  plots: { title: string; data: { time: number; value: number | null }[] }[];
  intents: { bar_ms: number; action: string }[];
  diagnostics: { line: number; message: string }[];
};
export function PineEditor() {
  const { symbol, bar, light } = usePreferences();
  const qc = useQueryClient();
  const [draft] = useState(() =>
    readDraft("okx-pine-draft-v2", {
      name: "Mijn trend",
      source: pineExample,
      inputs: {} as Record<string, string | number | boolean>,
      version: 0,
      display: "overlay",
      scriptKind: "strategy",
    }),
  );
  const [display, setDisplay] = useState(draft.display);
  const [scriptKind, setScriptKind] = useState(draft.scriptKind);
  const [name, setName] = useState(draft.name);
  const [source, setSource] = useState(draft.source);
  const [inputs, setInputs] = useState<
    Record<string, string | number | boolean>
  >(draft.inputs);
  const [version, setVersion] = useState(draft.version);
  useEffect(() => {
    localStorage.setItem(
      "okx-pine-draft-v2",
      JSON.stringify({ name, source, inputs, version, display, scriptKind }),
    );
  }, [name, source, inputs, version, display, scriptKind]);
  const [computedSource, setComputedSource] = useState("");
  const [inputMeta, setInputMeta] = useState<PineInput[]>([]);
  const [result, setResult] = useState<PineResult>();
  const [backtest, setBacktest] = useState<{
    report: {
      net_pnl: string;
      fees: string;
      max_drawdown: string;
      bars: number;
      fill_count: number;
      curve: { equity: string }[];
    };
  }>();
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const model = useRef<monaco.editor.ITextModel | null>(null);
  const versions = useQuery({
    queryKey: ["documents", "pine"],
    queryFn: () => api<Doc[]>("/api/studio/documents?kind=pine"),
  });
  async function run() {
    setBusy(true);
    setError("");
    try {
      const r = await previewPine<PineResult>({
        source,
        inputs,
        instrument: symbol,
        bar,
      });
      setResult(r);
      setComputedSource(JSON.stringify({ source, inputs }));
      if (r.inputs?.length) setInputMeta(r.inputs);
      if (model.current)
        monaco.editor.setModelMarkers(
          model.current,
          "pine",
          (r.diagnostics || []).map((d) => ({
            startLineNumber: d.line,
            endLineNumber: d.line,
            startColumn: 1,
            endColumn: 150,
            message: d.message,
            severity: monaco.MarkerSeverity.Error,
          })),
        );
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function save(destination?: "Terminal" | "Bot Studio") {
    setBusy(true);
    setError("");
    try {
      let checked = result;
      if (destination) {
        checked = await previewPine<PineResult>({
          source,
          inputs,
          instrument: symbol,
          bar,
        });
        setResult(checked);
        setComputedSource(JSON.stringify({ source, inputs }));
        if (checked.inputs) setInputMeta(checked.inputs);
        if (!checked.ok)
          throw Error(checked.diagnostics.map((d) => d.message).join(" · "));
      }
      const payload = {
        source,
        inputs,
        profile: "closed-bar-1",
        display,
        scriptKind,
        plots: checked?.plots?.map((p) => p.title) || [],
      };
      const r = await post<{ version: number }>("/api/studio/documents", {
        kind: "pine",
        name,
        base_version: version,
        payload,
      });
      setVersion(r.version);
      await qc.invalidateQueries({ queryKey: ["documents", "pine"] });
      setError(`Script opgeslagen · v${r.version}`);
      const doc = { name, version: r.version, payload, created_ms: Date.now() };
      if (destination === "Terminal") {
        const pref = usePreferences.getState();
        pref.set({
          pineScripts: [
            ...pref.pineScripts.filter((p) => p.name !== name),
            { name, version: r.version },
          ],
        });
        navigate("Terminal");
      }
      if (destination === "Bot Studio") {
        sessionStorage.setItem("okx-pine-node", JSON.stringify(doc));
        navigate("Bot Studio");
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="studio-page">
      <div className="studio-title">
        <div>
          <h2>Pine Editor</h2>
          <p>
            Schrijf je eigen indicator of strategie. Gebruik hem op de chart of
            in je bot.
          </p>
        </div>
        <span className="pill">Concept automatisch bewaard</span>
      </div>
      <div className="studio-toolbar">
        <input
          aria-label="Scriptnaam"
          value={name}
          onChange={(e) => {
            setName(e.target.value);
            setVersion(0);
          }}
        />
        <select
          aria-label="Scriptversie"
          value=""
          onChange={(e) => {
            const d =
              e.target.value === ""
                ? undefined
                : versions.data?.[+e.target.value];
            if (d) {
              setSource(String(d.payload.source));
              setInputs(
                (d.payload.inputs || {}) as Record<
                  string,
                  string | number | boolean
                >,
              );
              setName(d.name);
              setVersion(d.version);
              setDisplay(String(d.payload.display || "overlay"));
              setScriptKind(String(d.payload.scriptKind || "strategy"));
              setResult(undefined);
              setInputMeta([]);
            }
          }}
        >
          <option value="">Opgeslagen versies</option>
          {versions.data?.map((d, i) => (
            <option key={i} value={i}>
              {d.name} · v{d.version}
            </option>
          ))}
        </select>
        <button disabled={busy} onClick={() => save()}>
          Script opslaan
        </button>
        <button disabled={busy} onClick={() => save("Terminal")}>
          Opslaan en op chart
        </button>
        <button disabled={busy} onClick={() => save("Bot Studio")}>
          Opslaan en als botnode
        </button>
        <label className="button">
          Importeer .pine
          <input
            hidden
            type="file"
            accept=".pine,.txt"
            onChange={async (e) => {
              const f = e.target.files?.[0];
              if (f && f.size <= 12000) {
                setSource(await f.text());
                setInputs({});
                setName(f.name.replace(".pine", ""));
                setVersion(0);
              }
            }}
          />
        </label>
        <button onClick={() => download(source, name + ".pine", "text/plain")}>
          Exporteer
        </button>
        <button className="primary" disabled={busy} onClick={run}>
          {busy ? "Uitvoeren…" : `Preview ${symbol} · ${bar}`}
        </button>
      </div>
      <div className="studio-toolbar">
        <label>
          Type
          <select
            value={scriptKind}
            onChange={(e) => setScriptKind(e.target.value)}
          >
            <option value="indicator">Indicator</option>
            <option value="strategy">Strategie (entry / exit)</option>
          </select>
        </label>
        <label>
          Weergave
          <select value={display} onChange={(e) => setDisplay(e.target.value)}>
            <option value="overlay">Op de koersgrafiek</option>
            <option value="pane">Eigen indicatorpaneel</option>
          </select>
        </label>
        <label>
          Interval
          <select
            value={bar}
            onChange={(e) =>
              usePreferences.getState().set({ bar: e.target.value })
            }
          >
            {[
              "1m",
              "3m",
              "5m",
              "15m",
              "30m",
              "1H",
              "2H",
              "4H",
              "6H",
              "12H",
              "1D",
              "1W",
            ].map((b) => (
              <option key={b}>{b}</option>
            ))}
          </select>
        </label>
        <button
          onClick={() => {
            setSource(
              scriptKind === "strategy"
                ? pineExample
                : '//@version=6\nindicator("Mijn indicator", overlay=true)\nperiode = input.int(20, "Periode", minval=2, maxval=100)\nplot(ta.sma(close, periode), "Mijn indicator")\n',
            );
            setInputs({});
            setInputMeta([]);
            setName("Nieuw script " + new Date().toLocaleTimeString("nl-NL"));
            setVersion(0);
            setResult(undefined);
          }}
        >
          Nieuw voorbeeld
        </button>
      </div>
      {scriptKind === "strategy" && (
        <p className="panel-note">
          Schrijf koop- en verkoopsignalen met plot(..., "entry") en plot(...,
          "exit"). Native strategy.* wordt nog niet ondersteund. Configureer
          budget, stop en target in Bot Studio; bots gebruiken 1-minuutcandles.
        </p>
      )}
      <details className="studio-card">
        <summary>Snelle scripttest met voorbeeldbedragen</summary>
        <div className="studio-toolbar">
          <button
            disabled={busy || !symbol.endsWith("-EUR")}
            onClick={async () => {
              setBusy(true);
              try {
                setBacktest(
                  await post("/api/replay/pine", {
                    source,
                    inputs,
                    config: {
                      instrument: symbol,
                      bar,
                      fast: 9,
                      slow: 21,
                      capital: "1000",
                      order_eur: "100",
                      stop_bps: "100",
                      target_bps: "200",
                      spread_bps: "10",
                      slippage_bps: "5",
                    },
                  }),
                );
                setError("Candle-backtest afgerond.");
              } catch (e) {
                setError(String(e));
              } finally {
                setBusy(false);
              }
            }}
          >
            Backtest dit script
          </button>
          <span>
            Virtueel: €1.000 start · €100/order · stop 1% · target 2% · spread
            10 bp · slippage 5 bp · fee 10 bp
          </span>
        </div>
      </details>
      {backtest && (
        <div className="studio-card">
          <strong>
            Pine-replay · {backtest.report.bars} gesloten candles ·{" "}
            {backtest.report.fill_count} fills
          </strong>
          <p>
            Netto € {Number(backtest.report.net_pnl).toFixed(2)} · fees €{" "}
            {Number(backtest.report.fees).toFixed(2)} · max. drawdown €{" "}
            {Number(backtest.report.max_drawdown).toFixed(2)}
          </p>
          <Spark values={backtest.report.curve.map((p) => Number(p.equity))} />
          <button
            onClick={() =>
              download(JSON.stringify(backtest, null, 2), "pine-replay.json")
            }
          >
            Exporteer rapport
          </button>
          <p>
            OHLC-simulatie; dezelfde Pine-runtime en risicokern als paper.
            Uitvoering pas na sluiten van de signaalcandle. Geen winstbewijs.
          </p>
        </div>
      )}
      <div className="pine-grid">
        <div className="editor-shell">
          <Editor
            height="480px"
            language="pine"
            theme={light ? "light" : "vs-dark"}
            value={source}
            onChange={(v) => setSource(v || "")}
            onMount={(e) => {
              model.current = e.getModel();
            }}
            options={{
              minimap: { enabled: false },
              fontSize: 13,
              automaticLayout: true,
              scrollBeyondLastLine: false,
              tabSize: 4,
            }}
          />
        </div>
        <aside className="studio-card">
          <h3>Resultaat</h3>
          {!!inputMeta.length && (
            <div className="pine-inputs">
              <h4>Scriptparameters</h4>
              {inputMeta.map((m) => (
                <label key={m.id}>
                  {m.title || m.name}
                  {m.type === "bool" ? (
                    <input
                      type="checkbox"
                      checked={Boolean(inputs[m.name] ?? m.defval)}
                      onChange={(e) =>
                        setInputs({ ...inputs, [m.name]: e.target.checked })
                      }
                    />
                  ) : m.options ? (
                    <select
                      value={String(inputs[m.name] ?? m.defval)}
                      onChange={(e) =>
                        setInputs({
                          ...inputs,
                          [m.name]: ["int", "float", "price", "time"].includes(
                            m.type,
                          )
                            ? Number(e.target.value)
                            : e.target.value,
                        })
                      }
                    >
                      {m.options.map((v) => (
                        <option key={String(v)}>{v}</option>
                      ))}
                    </select>
                  ) : (
                    <input
                      disabled={m.type === "source"}
                      type={
                        ["int", "float", "price", "time"].includes(m.type)
                          ? "number"
                          : "text"
                      }
                      value={String(inputs[m.name] ?? m.defval)}
                      min={m.minval}
                      max={m.maxval}
                      step={m.type === "int" ? 1 : m.step || "any"}
                      onChange={(e) =>
                        setInputs({
                          ...inputs,
                          [m.name]: ["int", "float", "price", "time"].includes(
                            m.type,
                          )
                            ? Number(e.target.value)
                            : e.target.value,
                        })
                      }
                    />
                  )}
                </label>
              ))}
              <p>
                Preview opnieuw om wijzigingen te berekenen. Opslaan legt inputs
                vast in de scriptversie.
              </p>
            </div>
          )}

          {error && <p role="status">{error}</p>}
          {!result && (
            <p className="muted">
              Start de preview. Alleen gesloten, aaneengesloten OKX-candles
              worden berekend.
            </p>
          )}
          {result && (
            <>
              {computedSource !== JSON.stringify({ source, inputs }) && (
                <p className="warning">
                  Code of inputs gewijzigd. Voer opnieuw uit om het resultaat te
                  vernieuwen.
                </p>
              )}
              <p>
                {result.ok ? "Berekening geslaagd" : "Uitvoering geblokkeerd"} ·{" "}
                {result.duration_ms?.toFixed(1)} ms · {result.bars || 0} candles
              </p>
              {result.diagnostics?.map((d, i) => (
                <p className="error" key={i}>
                  Regel {d.line}: {d.message}
                </p>
              ))}
              {result.plots
                ?.filter((p) => !p.title.startsWith("__"))
                .map((p) => (
                  <div className="plot-result" key={p.title}>
                    <strong>{p.title}</strong>
                    <Spark values={p.data.map((d) => d.value)} />
                    <span>
                      {p.data.at(-1)?.value?.toLocaleString("nl-NL", {
                        maximumFractionDigits: 6,
                      }) ?? "warm-up"}
                    </span>
                  </div>
                ))}
              <p>
                {result.intents?.length || 0} historische voorstellen. Preview
                plaatst geen orders.
              </p>
            </>
          )}
        </aside>
      </div>
      <details className="studio-card">
        <summary>Compatibiliteit en uitvoering</summary>
        <p>
          Profiel 1 ondersteunt gesloten bars, series, history indexing, inputs,
          var, control flow, TA en numerieke plots. Open bars, varip, externe
          request-data en strategy.* zijn expliciet geblokkeerd totdat hun
          uitvoeringssemantiek is gevalideerd. Geen volledige
          TradingView-compatibiliteit. Scripts mogen entry/exit-plots met waarde
          1 als signaal aanbieden.
        </p>
        <p>
          Container zonder netwerk, database of sleutels. Maximaal 12 KB bron, 3
          seconden, 128 MB scriptheap en 1 MB uitvoer. Onbekende functies geven
          een fout; een gedeeltelijk resultaat wordt niet als voorstel gebruikt.
        </p>
      </details>
    </section>
  );
}
type Rule = {
  id: number;
  config: {
    name: string;
    instrument: string;
    field: string;
    value: number;
    trigger: string;
  };
  enabled: boolean;
  error: string | null;
};
export function Alerts() {
  const { symbol, bar } = usePreferences();
  const qc = useQueryClient();
  const [field, setField] = useState("price");
  const [botId, setBotId] = useState("");
  const [value, setValue] = useState("");
  const [comparison, setComparison] = useState("above");
  const [trigger, setTrigger] = useState("close");
  const [name, setName] = useState("Koersalarm");
  const [error, setError] = useState("");
  const q = useQuery({
    queryKey: ["alerts"],
    queryFn: () =>
      api<{
        rules: Rule[];
        events: {
          id: number;
          ts: number;
          read: boolean;
          detail: {
            name: string;
            value: number;
            instrument: string;
            delivery: string;
          };
        }[];
      }>("/api/studio/alerts"),
    refetchInterval: 3000,
  });
  return (
    <section className="studio-page">
      <div className="studio-title">
        <div>
          <h2>Serveralerts</h2>
          <p>
            Blijven actief met gesloten browser. Aflevering in deze lokale
            inbox.
          </p>
        </div>
        <button
          onClick={async () => {
            await post("/api/studio/deliveries/read");
            qc.invalidateQueries({ queryKey: ["alerts"] });
          }}
        >
          Markeer gelezen
        </button>
      </div>
      <form
        className="studio-toolbar"
        onSubmit={async (e) => {
          e.preventDefault();
          try {
            await post("/api/studio/alerts", {
              name,
              instrument: symbol,
              bar,
              field,
              value: Number(value),
              comparison,
              trigger,
              cooldown_secs: 60,
              bot_id: botId || null,
            });
            qc.invalidateQueries({ queryKey: ["alerts"] });
            setError("Alert opgeslagen.");
          } catch (e) {
            setError(String(e));
          }
        }}
      >
        <input
          aria-label="Alertnaam"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <span>
          {symbol} · {bar}
        </span>
        <select
          aria-label="Alertveld"
          value={field}
          onChange={(e) => setField(e.target.value)}
        >
          {[
            "price",
            "sma",
            "ema",
            "rsi",
            "macd",
            "atr",
            "bb_upper",
            "bb_lower",
            "vwap",
            "stochastic",
            "adx",
            "obv",
            "risk_paused",
            "bot_running",
            "bot_drawdown",
            "bot_total_loss",
          ].map((v) => (
            <option key={v}>{v}</option>
          ))}
        </select>
        {field.startsWith("bot_") && (
          <input
            aria-label="Alertbot-id"
            placeholder="Bot-id"
            required
            value={botId}
            onChange={(e) => setBotId(e.target.value)}
          />
        )}
        <select
          aria-label="Vergelijking"
          value={comparison}
          onChange={(e) => setComparison(e.target.value)}
        >
          <option value="above">Boven</option>
          <option value="below">Onder</option>
        </select>
        <input
          required
          type="number"
          step="any"
          aria-label="Alertdrempel"
          value={value}
          onChange={(e) => setValue(e.target.value)}
        />
        <select
          aria-label="Triggermoment"
          value={trigger}
          onChange={(e) => setTrigger(e.target.value)}
        >
          <option value="close">Candle-close</option>
          <option value="intrabar">Intrabar (prijs/risico)</option>
        </select>
        <button className="primary">Bewaar alert</button>
      </form>
      <p role="status">{error || q.error?.message}</p>
      <div className="research-grid">
        <div className="studio-card">
          <h3>Regels · cooldown 60 s</h3>
          {q.data?.rules.map((r) => (
            <div className="list-row" key={r.id}>
              <div>
                <strong>{r.config.name}</strong>
                <p>
                  {r.config.instrument} · {r.config.field} · {r.config.value} ·{" "}
                  {r.config.trigger}
                </p>
                {r.error && <p className="warning">{r.error}</p>}
              </div>
              <button
                onClick={async () => {
                  await post("/api/studio/alerts/control", {
                    id: r.id,
                    enabled: !r.enabled,
                  });
                  qc.invalidateQueries({ queryKey: ["alerts"] });
                }}
              >
                {r.enabled ? "Pauzeer" : "Activeer"}
              </button>
            </div>
          ))}
          {!q.data?.rules.length && <p>Nog geen alerts ingesteld.</p>}
        </div>
        <div className="studio-card">
          <h3>Inbox</h3>
          {q.data?.events.map((e) => (
            <div className="list-row" key={e.id}>
              <div>
                <strong>
                  {e.read ? "" : "● "}
                  {e.detail.name}
                </strong>
                <p>
                  {e.detail.instrument} · {e.detail.value} ·{" "}
                  {new Date(e.ts).toLocaleString("nl-NL")}
                </p>
                <small>Afgeleverd in lokale inbox</small>
              </div>
            </div>
          ))}
          {!q.data?.events.length && <p>Nog geen getriggerde alerts.</p>}
        </div>
      </div>
    </section>
  );
}
