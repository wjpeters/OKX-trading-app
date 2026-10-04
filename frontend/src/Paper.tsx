import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ShieldCheck,
  Play,
  Pause,
  FlaskConical,
  ArrowRight,
  X,
  Download,
} from "lucide-react";
import { api } from "./types";
import type { Market } from "./types";
import "./paper.css";

type Limits = {
  capital: string;
  trade_risk: string;
  daily_loss: string;
  total_loss: string;
  drawdown: string;
  position: string;
  positions: number;
  orders: number;
  leverage: string;
  spread_bps: string;
  slippage_bps: string;
  data_age_ms: number;
  orders_per_minute: number;
  loss_streak: number;
  ai_budget: string;
};
type Ledger = {
  cash: string;
  equity: string;
  baseline: string;
  day_start: string;
  high_water: string;
  daily_loss: string;
  total_loss: string;
  drawdown: string;
  fees: string;
  realized: string;
  loss_streak: number;
  paused: boolean;
  reason: string;
};
type Intent = {
  client_id: string;
  bot_id: string;
  instrument: string;
  side: "BUY" | "SELL";
  kind: "MARKET" | "LIMIT";
  quantity: string;
  limit: string | null;
  stop: string | null;
  target: string | null;
  position_id: string | null;
};
type Order = {
  id: string;
  intent: Intent;
  state: string;
  quantity: string;
  filled: string;
  reserve: string;
  cap: string;
  created_ms: number;
  reason: string;
};
type Position = {
  id: string;
  bot_id: string;
  instrument: string;
  quantity: string;
  cost: string;
  stop: string;
  target: string | null;
  closing_order: string | null;
  protection_state: string;
};
type Bot = {
  id: string;
  name: string;
  instrument: string;
  version: string;
  limits: Limits;
  ledger: Ledger;
  running: boolean;
  last_bar: number;
  last_signal: string;
  fast: number;
  slow: number;
  order_eur: string;
  stop_bps: string;
  target_bps: string;
};
type Fill = {
  id: string;
  order_id: string;
  instrument: string;
  side: string;
  quantity: string;
  price: string;
  fee: string;
  ts: number;
};
type PaperAccount = {
  revision: number;
  limits: Limits;
  ledger: Ledger;
  orders: Record<string, Order>;
  positions: Record<string, Position>;
  bots: Record<string, Bot>;
  fills: Fill[];
  events: {
    seq: number;
    ts: number;
    kind: string;
    subject: string;
    detail: string;
  }[];
  valuation_fresh: boolean;
  heartbeat_ms: number;
  flatten: boolean;
};
type PaperState = {
  account: PaperAccount | null;
  presets: { account: Limits; manual: Limits };
  worker_alive: boolean;
  server_time_ms: number;
};
type Preview = {
  preview_id: number;
  expires_ms: number;
  intent: Intent;
  plan: {
    quantity: string;
    price_bound: string;
    reservation_eur: string;
    fee_bps: number;
    latency_ms: number;
    mode: string;
  };
};
type ReplayReport = {
  mode: string;
  strategy: string;
  instrument: string;
  bar: string;
  bars: number;
  first_ms: number;
  last_ms: number;
  parameters: { capital: string; fast: number; slow: number };
  assumptions: {
    uncertainty: string;
    path: string;
    fee_bps: number;
    spread_bps: string;
    slippage_bps: string;
    participation: string;
  };
  equity: string;
  net_pnl: string;
  fees: string;
  max_drawdown: string;
  max_exposure: string;
  turnover: string;
  open_positions: number;
  fill_count: number;
  risk_state: string;
  curve: { ts: number; equity: string }[];
  fills: Fill[];
  decisions: { ts: number; signal: string; outcome: string }[];
};
type Replay = { id: number; report: ReplayReport };
const money = (s: string | number) =>
  new Intl.NumberFormat("nl-NL", {
    style: "currency",
    currency: "EUR",
    maximumFractionDigits: 2,
  }).format(Number(s));
const number = (s: string | number) =>
  new Intl.NumberFormat("nl-NL", { maximumFractionDigits: 8 }).format(
    Number(s),
  );
const date = (ms: number) =>
  new Date(ms).toLocaleString("nl-NL", { timeZone: "UTC", hour12: false });
const active = (s: string) =>
  [
    "INTENT",
    "PENDING_SUBMIT",
    "ACCEPTED",
    "PARTIAL_FILL",
    "PENDING_CANCEL",
    "UNKNOWN",
  ].includes(s);
const preset = (capital: number): Limits => ({
  capital: String(capital),
  trade_risk: String(capital * 0.01),
  daily_loss: String(capital * 0.03),
  total_loss: String(capital * 0.1),
  drawdown: String(capital * 0.08),
  position: String(capital * 0.25),
  positions: 4,
  orders: 8,
  leverage: "1",
  spread_bps: "30",
  slippage_bps: "20",
  data_age_ms: 5000,
  orders_per_minute: 6,
  loss_streak: 3,
  ai_budget: "0",
});
async function post<T>(path: string, body: unknown): Promise<T> {
  const r = await fetch(path, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "x-terminal-request": "paper-v2",
    },
    body: JSON.stringify(body),
  });
  let result;
  try {
    result = await r.json();
  } catch {
    throw new Error(`Opdracht geweigerd (${r.status})`);
  }
  if (!r.ok)
    throw new Error(result.error || `Opdracht geweigerd (${r.status})`);
  return result as T;
}
const cashFields: [keyof Limits, string][] = [
  ["capital", "Toegewezen kapitaal"],
  ["trade_risk", "Risico per transactie"],
  ["daily_loss", "Max. dagverlies"],
  ["total_loss", "Max. cumulatief verlies"],
  ["drawdown", "Max. drawdown"],
  ["position", "Max. exposure per munt"],
];
const otherFields: [keyof Limits, string, string][] = [
  ["positions", "Open posities", "aantal"],
  ["orders", "Open orders", "aantal"],
  ["spread_bps", "Max. spread", "bp"],
  ["slippage_bps", "Max. slippage", "bp"],
  ["data_age_ms", "Max. data-ouderdom", "ms"],
  ["orders_per_minute", "Instaporders per minuut", "aantal"],
  ["loss_streak", "Verliesreeks", "aantal"],
];
function LimitsForm({
  value,
  onChange,
  title,
}: {
  value: Limits;
  onChange: (v: Limits) => void;
  title: string;
}) {
  const set = (k: keyof Limits, v: string) =>
    onChange({ ...value, [k]: typeof value[k] === "number" ? Number(v) : v });
  return (
    <fieldset className="limits-form">
      <legend>{title}</legend>
      <div className="limits-grid">
        {cashFields.map(([k, label]) => (
          <label key={k}>
            {label}
            <span className="input-unit">
              <input
                required
                inputMode="decimal"
                value={value[k]}
                onChange={(e) => set(k, e.target.value)}
              />
              <span>EUR</span>
            </span>
            <small>
              {k === "daily_loss"
                ? `${((Number(value[k]) / Number(value.capital)) * 100).toFixed(2)}% bij start · grondslag: equity bij UTC-dagstart`
                : k === "drawdown"
                  ? `${((Number(value[k]) / Number(value.capital)) * 100).toFixed(2)}% bij start · grondslag: hoogste equity`
                  : `${((Number(value[k]) / Number(value.capital)) * 100).toFixed(2)}% van toegewezen kapitaal`}
            </small>
          </label>
        ))}
        {otherFields.map(([k, label, unit]) => (
          <label key={k}>
            {label}
            <span className="input-unit">
              <input
                required
                inputMode="decimal"
                value={value[k]}
                onChange={(e) => set(k, e.target.value)}
              />
              <span>{unit}</span>
            </span>
          </label>
        ))}
      </div>
      <label className="paper-field">
        AI-kostenbudget (USD, afzonderlijk van EUR-handelskapitaal)
        <input
          type="number"
          min="0"
          max="100"
          step="0.01"
          value={value.ai_budget}
          onChange={(e) => onChange({ ...value, ai_budget: e.target.value })}
        />
      </label>
      <p className="paper-note">
        Spot 1× · AI-budget in USD · Geen funding · Geen tussentijdse
        stortingen/opnames. Verliespercentages worden bij de actuele grondslag
        getoond. Limieten staan na aanmaak vast.
      </p>
    </fieldset>
  );
}
function Metric({
  label,
  value,
  detail,
}: {
  label: string;
  value: string;
  detail?: string;
}) {
  return (
    <div className="paper-metric">
      <small>{label}</small>
      <strong>{value}</strong>
      {detail && <span>{detail}</span>}
    </div>
  );
}
function Empty({ children }: { children: React.ReactNode }) {
  return <div className="paper-empty">{children}</div>;
}
export default function PaperWorkspace({
  market,
  view = "paper",
}: {
  market: Market;
  view?: "paper" | "bots" | "replay";
}) {
  const cache = useQueryClient();
  const state = useQuery({
    queryKey: ["paper"],
    queryFn: () => api<PaperState>("/api/paper"),
    refetchInterval: 1000,
    retry: 1,
  });
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const [tab, setTab] = useState("Posities");
  const [confirmation, setConfirmation] = useState<{
    action: string;
    label: string;
  } | null>(null);
  const [exit, setExit] = useState<Intent | null>(null);
  const [preview, setPreview] = useState<Preview | null>(null);
  const account = state.data?.account;
  const symbols = market.instruments.filter((i) => i.quoteCcy === "EUR");
  const perform = async (fn: () => Promise<unknown>, success: string) => {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await fn();
      setNotice(success);
      await cache.invalidateQueries({ queryKey: ["paper"] });
      await cache.invalidateQueries({ queryKey: ["status"] });
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  };
  const requestPreview = (i: Intent) =>
    perform(async () => {
      const p = await post<Preview>("/api/paper/preview", i);
      setPreview(p);
    }, "Risicocontrole geslaagd. Controleer de preview.");
  if (view === "replay")
    return <ReplayWorkspace symbols={symbols.map((i) => i.instId)} />;
  return (
    <div className="paper-workspace">
      <div className="paper-heading">
        <div>
          <span className="eyebrow">PAPER_LOCAL · FASE 2</span>
          <h2>
            {view === "bots"
              ? "Eenvoudige serverbots"
              : "Virtuele handelsrekening"}
          </h2>
          <p>
            Echte OKX-marktdata. Virtueel geld. Alleen de vaste risicokern geeft
            orders vrij.
          </p>
        </div>
        <span
          className={`paper-status ${state.data?.worker_alive ? "positive" : "warning"}`}
        >
          {state.data?.worker_alive
            ? "● Paperworker actief"
            : "● Paperworker niet beschikbaar"}
        </span>
      </div>
      {state.isError && (
        <div role="alert" className="paper-alert">
          Paperstate niet beschikbaar: {state.error.message}. Bediening
          geblokkeerd.
        </div>
      )}
      {error && (
        <div role="alert" className="paper-alert">
          <strong>Opdracht geweigerd</strong> {error}
          <button aria-label="Melding sluiten" onClick={() => setError("")}>
            <X size={14} />
          </button>
        </div>
      )}
      {notice && (
        <div role="status" className="paper-notice">
          {notice}
        </div>
      )}
      {!state.data ? (
        <Empty>Paperomgeving laden…</Empty>
      ) : !account ? (
        <Setup
          presets={state.data.presets}
          busy={busy || state.isError}
          onSetup={(a, m) =>
            perform(
              () =>
                post("/api/paper/setup", {
                  account: a,
                  manual: m,
                  confirmed: true,
                }),
              "Paperrekening aangemaakt. Limieten staan vast.",
            )
          }
        />
      ) : (
        <>
          <div className="paper-metrics">
            <Metric
              label="Equity · EUR"
              value={money(account.ledger.equity)}
              detail={
                account.valuation_fresh
                  ? "Biedwaardering minus exitfees"
                  : "Waardering verouderd; instappen geblokkeerd"
              }
            />
            <Metric
              label="Vrij virtueel geld"
              value={money(
                Number(account.ledger.cash) -
                  Object.values(account.orders).reduce(
                    (a, o) => a + Number(o.reserve),
                    0,
                  ),
              )}
              detail={`Cash ${money(account.ledger.cash)}`}
            />
            <Metric
              label="Dagverlies · UTC"
              value={money(account.ledger.daily_loss)}
              detail={`${((Number(account.ledger.daily_loss) / Number(account.ledger.day_start)) * 100).toFixed(2)}% van ${money(account.ledger.day_start)} · limiet ${money(account.limits.daily_loss)}`}
            />
            <Metric
              label="Drawdown"
              value={money(account.ledger.drawdown)}
              detail={`${((Number(account.ledger.drawdown) / Number(account.ledger.high_water)) * 100).toFixed(2)}% van ${money(account.ledger.high_water)} · limiet ${money(account.limits.drawdown)}`}
            />
            <Metric
              label="Fees / gerealiseerd"
              value={money(account.ledger.fees)}
              detail={`Netto gerealiseerd ${money(account.ledger.realized)}`}
            />
          </div>
          <div className="paper-controls">
            <span className={account.ledger.paused ? "warning" : "positive"}>
              <ShieldCheck size={15} />
              {account.ledger.paused
                ? `PAUSE_ENTRIES · ${account.ledger.reason}`
                : "Risicokern actief"}
              {account.flatten ? " · noodafbouw loopt" : ""}
            </span>
            <div>
              <button
                disabled={busy || state.isError}
                onClick={() =>
                  setConfirmation({
                    action: account.ledger.paused
                      ? "RESUME_ENTRIES"
                      : "PAUSE_ENTRIES",
                    label: account.ledger.paused
                      ? "Instappen hervatten"
                      : "Nieuwe instappen pauzeren",
                  })
                }
              >
                {account.ledger.paused ? (
                  <Play size={13} />
                ) : (
                  <Pause size={13} />
                )}{" "}
                {account.ledger.paused ? "Hervatten" : "Pauze"}
              </button>
              <button
                disabled={busy || state.isError}
                onClick={() =>
                  setConfirmation({
                    action: "CANCEL_ENTRY_ORDERS",
                    label: "Open instaporders annuleren",
                  })
                }
              >
                Annuleer instappen
              </button>
              <button
                className="paper-danger"
                disabled={busy || state.isError}
                onClick={() =>
                  setConfirmation({
                    action: "EMERGENCY_FLATTEN",
                    label: "Alle paperposities begrensd afbouwen",
                  })
                }
              >
                Noodafbouw
              </button>
            </div>
          </div>
          {view === "bots" ? (
            <Bots
              account={account}
              symbols={symbols.map((i) => i.instId)}
              busy={busy || state.isError}
              perform={perform}
            />
          ) : (
            <div className="paper-main-grid">
              <section className="panel paper-ledger">
                <div className="bottom-tabs">
                  {[
                    "Posities",
                    "Orders",
                    "Fills",
                    "Risicolimieten",
                    "Logboek",
                  ].map((t) => (
                    <button
                      key={t}
                      className={tab === t ? "active" : ""}
                      onClick={() => setTab(t)}
                    >
                      {t}
                      {t === "Posities"
                        ? ` (${Object.values(account.positions).filter((p) => Number(p.quantity) > 0).length})`
                        : ""}
                    </button>
                  ))}
                </div>
                {tab === "Posities" ? (
                  <>
                    <div className="paper-table-scroll">
                      <table className="paper-table">
                        <thead>
                          <tr>
                            <th>Markt / eigenaar</th>
                            <th>Hoeveelheid</th>
                            <th>Kostprijs</th>
                            <th>Stop / target</th>
                            <th>Bescherming</th>
                            <th />
                          </tr>
                        </thead>
                        <tbody>
                          {Object.values(account.positions)
                            .filter((p) => Number(p.quantity) > 0)
                            .map((p) => (
                              <tr key={p.id}>
                                <td>
                                  <strong>{p.instrument}</strong>
                                  <small>{account.bots[p.bot_id]?.name}</small>
                                </td>
                                <td>{number(p.quantity)}</td>
                                <td>{money(p.cost)}</td>
                                <td>
                                  {number(p.stop)}
                                  <small>
                                    {p.target
                                      ? number(p.target)
                                      : "Geen target"}
                                  </small>
                                </td>
                                <td>
                                  <span className="positive">
                                    {p.closing_order
                                      ? "Afbouworder actief"
                                      : p.protection_state === "DUST"
                                        ? "Onder orderminimum"
                                        : "Stop bewaakt op server"}
                                  </span>
                                </td>
                                <td>
                                  <button
                                    disabled={
                                      busy || state.isError || !!p.closing_order
                                    }
                                    onClick={() => {
                                      const i: Intent = {
                                        client_id: crypto.randomUUID(),
                                        bot_id: p.bot_id,
                                        instrument: p.instrument,
                                        side: "SELL",
                                        kind: "MARKET",
                                        quantity: p.quantity,
                                        limit: null,
                                        stop: null,
                                        target: null,
                                        position_id: p.id,
                                      };
                                      setExit(i);
                                      void requestPreview(i);
                                    }}
                                  >
                                    Afbouwen <ArrowRight size={12} />
                                  </button>
                                </td>
                              </tr>
                            ))}
                        </tbody>
                      </table>
                    </div>
                    {!Object.values(account.positions).some(
                      (p) => Number(p.quantity) > 0,
                    ) && (
                      <Empty>
                        Geen open paperposities. Een goedgekeurde order
                        verschijnt hier pas na een fill.
                      </Empty>
                    )}
                  </>
                ) : tab === "Orders" ? (
                  <>
                    <div className="paper-table-scroll">
                      <table className="paper-table">
                        <thead>
                          <tr>
                            <th>Tijd / markt</th>
                            <th>Opdracht</th>
                            <th>Status</th>
                            <th>Gevuld / totaal</th>
                            <th>Reserve</th>
                            <th />
                          </tr>
                        </thead>
                        <tbody>
                          {Object.values(account.orders)
                            .sort((a, b) => b.created_ms - a.created_ms)
                            .slice(0, 100)
                            .map((o) => (
                              <tr key={o.id}>
                                <td>
                                  {o.intent.instrument}
                                  <small>{date(o.created_ms)} UTC</small>
                                </td>
                                <td>
                                  {o.intent.side} · {o.intent.kind}
                                  <small>{o.intent.bot_id}</small>
                                </td>
                                <td>
                                  {o.state}
                                  <small title={o.reason}>{o.reason}</small>
                                </td>
                                <td>
                                  {number(o.filled)} / {number(o.quantity)}
                                </td>
                                <td>{money(o.reserve)}</td>
                                <td>
                                  {active(o.state) && (
                                    <button
                                      disabled={
                                        busy ||
                                        state.isError ||
                                        o.reason.startsWith("PROTECT")
                                      }
                                      onClick={() =>
                                        void perform(
                                          () =>
                                            post("/api/paper/cancel", {
                                              order_id: o.id,
                                            }),
                                          "Annulering aangevraagd; reservering blijft staan tot bevestiging.",
                                        )
                                      }
                                    >
                                      Annuleer
                                    </button>
                                  )}
                                </td>
                              </tr>
                            ))}
                        </tbody>
                      </table>
                    </div>
                    {!Object.keys(account.orders).length && (
                      <Empty>
                        Nog geen paperorders. Een acknowledgement geldt niet als
                        fill.
                      </Empty>
                    )}
                  </>
                ) : tab === "Fills" ? (
                  <FillTable fills={account.fills} />
                ) : tab === "Risicolimieten" ? (
                  <RiskSummary account={account} />
                ) : (
                  <div className="paper-events">
                    {[...account.events].reverse().map((e) => (
                      <div key={e.seq}>
                        <time>{date(e.ts)} UTC</time>
                        <strong>{e.kind}</strong>
                        <span>
                          {e.subject} · {e.detail}
                        </span>
                      </div>
                    ))}
                  </div>
                )}
              </section>
              <Ticket
                symbols={symbols.map((i) => i.instId)}
                market={market}
                busy={busy || state.isError || !state.data.worker_alive}
                paused={account.ledger.paused}
                onPreview={requestPreview}
              />
            </div>
          )}
          <p className="paper-footnote">
            Verlieslimieten zijn actiedrempels. Een prijsplafond kan afbouw
            uitstellen. Stops bestaan in deze lokale paperworker en gebruiken
            verse marktdata. Fees: 10 bp per fill, deelname maximaal 10% van het
            beste boekniveau, 500 ms modelvertraging. Dit zijn
            simulatieaannames, geen OKX-accounttarieven.
          </p>
        </>
      )}
      {confirmation && (
        <div className="modal-backdrop">
          <div
            className="paper-modal"
            role="dialog"
            aria-modal="true"
            aria-label={confirmation.label}
          >
            <span className="eyebrow">ALLEEN PAPER_LOCAL</span>
            <h2>{confirmation.label}</h2>
            <p>
              {confirmation.action === "PAUSE_ENTRIES"
                ? "Nieuwe instappen worden geblokkeerd. Bestaande orders blijven gereserveerd; stops blijven actief."
                : confirmation.action === "CANCEL_ENTRY_ORDERS"
                  ? "Open kooporders worden geannuleerd. Posities en beschermende afbouw blijven bestaan."
                  : confirmation.action === "EMERGENCY_FLATTEN"
                    ? "Alle instaporders worden geannuleerd. De risicokern probeert alle paperposities te verkopen binnen de vaste prijs- en datalimieten. Slechte liquiditeit kan afbouw vertragen."
                    : "De risicokern controleert verse waardering en verlieslimieten opnieuw. Baselines en limieten blijven ongewijzigd."}
            </p>
            <div className="paper-form-actions">
              <button onClick={() => setConfirmation(null)}>Terug</button>
              <button
                className="paper-primary"
                disabled={busy}
                onClick={() => {
                  const c = confirmation;
                  setConfirmation(null);
                  void perform(
                    () =>
                      post("/api/paper/control", {
                        action: c.action,
                        confirmed: true,
                      }),
                    "Controle uitgevoerd.",
                  );
                }}
              >
                Bevestig
              </button>
            </div>
          </div>
        </div>
      )}
      {preview && (
        <div className="modal-backdrop">
          <div
            className="paper-modal"
            role="dialog"
            aria-modal="true"
            aria-label="Paperorder controleren"
          >
            <span className="eyebrow">
              RISICOCONTROLE GESLAAGD · VIRTUEEL GELD
            </span>
            <h2>
              {preview.intent.side === "BUY" ? "Koop" : "Verkoop"}{" "}
              {preview.intent.instrument}
            </h2>
            <dl className="preview-details">
              <dt>Hoeveelheid</dt>
              <dd>{number(preview.plan.quantity)}</dd>
              <dt>Prijsgrens</dt>
              <dd>{money(preview.plan.price_bound)}</dd>
              <dt>Reservering incl. fee</dt>
              <dd>{money(preview.plan.reservation_eur)}</dd>
              <dt>Stop / target</dt>
              <dd>
                {preview.intent.stop || "Afbouw"} /{" "}
                {preview.intent.target || "—"}
              </dd>
              <dt>Eigenaar</dt>
              <dd>{preview.intent.bot_id}</dd>
              <dt>Geldig tot</dt>
              <dd>{date(preview.expires_ms)} UTC</dd>
            </dl>
            <p>
              Een gewijzigde rekening of slechtere prijs vereist een nieuwe
              preview. Een bevestiging is nog geen fill.
            </p>
            <div className="paper-form-actions">
              <button
                onClick={() => {
                  setPreview(null);
                  setExit(null);
                }}
              >
                Terug
              </button>
              <button
                className="paper-primary"
                disabled={busy || Date.now() >= preview.expires_ms}
                onClick={() => {
                  const id = preview.preview_id;
                  setPreview(null);
                  setExit(null);
                  setTab("Orders");
                  void perform(
                    () =>
                      post("/api/paper/submit", {
                        preview_id: id,
                        confirmed: true,
                      }),
                    "Order duurzaam vastgelegd. De simulator beoordeelt de uitvoering opnieuw.",
                  );
                }}
              >
                Bevestig paperorder
              </button>
            </div>
          </div>
        </div>
      )}
      {exit && !preview && error && (
        <button
          onClick={() =>
            void requestPreview({ ...exit, client_id: crypto.randomUUID() })
          }
        >
          Nieuwe afbouwpreview
        </button>
      )}
    </div>
  );
}
function Setup({
  presets,
  busy,
  onSetup,
}: {
  presets: { account: Limits; manual: Limits };
  busy: boolean;
  onSetup: (a: Limits, m: Limits) => void;
}) {
  const [a, setA] = useState(presets.account);
  const [m, setM] = useState(presets.manual);
  const [agree, setAgree] = useState(false);
  return (
    <form
      className="paper-setup"
      onSubmit={(e) => {
        e.preventDefault();
        onSetup(a, m);
      }}
    >
      <div className="paper-setup-intro">
        <ShieldCheck size={24} />
        <div>
          <h3>Stel je paperrisico vast</h3>
          <p>
            Start met virtueel kapitaal. Onderstaande waarden zijn bewerkbare
            voorstellen. De rekening wordt pas na jouw bevestiging aangemaakt.
            Automatische bots krijgen later een eigen deelbudget.
          </p>
        </div>
      </div>
      <LimitsForm title="Rekeninglimieten" value={a} onChange={setA} />
      <LimitsForm title="Handmatig deelbudget" value={m} onChange={setM} />
      <label className="paper-checkbox">
        <input
          type="checkbox"
          checked={agree}
          onChange={(e) => setAgree(e.target.checked)}
        />
        Ik bevestig deze virtuele budgetten, vaste limieten, UTC-daggrens en het
        kostenmodel van 10 bp per fill.
      </label>
      <button className="paper-primary" disabled={!agree || busy} type="submit">
        Paperrekening aanmaken
      </button>
    </form>
  );
}
function Ticket({
  symbols,
  market,
  busy,
  paused,
  onPreview,
}: {
  symbols: string[];
  market: Market;
  busy: boolean;
  paused: boolean;
  onPreview: (i: Intent) => Promise<void>;
}) {
  const [symbol, setSymbol] = useState(symbols[0] || "BTC-EUR");
  const [kind, setKind] = useState<"MARKET" | "LIMIT">("MARKET");
  const [qty, setQty] = useState("");
  const [limit, setLimit] = useState("");
  const [stop, setStop] = useState("");
  const [target, setTarget] = useState("");
  const meta = market.instruments.find((i) => i.instId === symbol);
  const book = market.books[symbol];
  return (
    <form
      className="paper-ticket panel"
      onSubmit={(e) => {
        e.preventDefault();
        void onPreview({
          client_id: crypto.randomUUID(),
          bot_id: "manual",
          instrument: symbol,
          side: "BUY",
          kind,
          quantity: qty,
          limit: kind === "LIMIT" ? limit : null,
          stop,
          target: target || null,
          position_id: null,
        });
      }}
    >
      <div className="panel-title">
        <strong>Paper orderticket</strong>
        <ShieldCheck size={15} />
      </div>
      <div className="paper-ticket-body">
        <span className="paper-buy-label">KOOP · SPOT · HANDMATIG</span>
        <label>
          Markt
          <select value={symbol} onChange={(e) => setSymbol(e.target.value)}>
            {symbols.map((s) => (
              <option key={s}>{s}</option>
            ))}
          </select>
        </label>
        <div className="paper-quote">
          <span>
            Bied <b>{book ? number(book.bids[0]?.[0] || 0) : "—"}</b>
          </span>
          <span>
            Laat <b>{book ? number(book.asks[0]?.[0] || 0) : "—"}</b>
          </span>
        </div>
        <label>
          Ordertype
          <select
            value={kind}
            onChange={(e) => setKind(e.target.value as "MARKET" | "LIMIT")}
          >
            <option value="MARKET">Market met prijsplafond</option>
            <option value="LIMIT">Limit · aanraking ≠ fill</option>
          </select>
        </label>
        <label>
          Hoeveelheid {meta?.baseCcy}
          <input
            required
            value={qty}
            inputMode="decimal"
            onChange={(e) => setQty(e.target.value)}
            placeholder={`Min. ${meta?.minSz || "—"}`}
          />
          <small>Stap {meta?.lotSz || "—"}</small>
        </label>
        {kind === "LIMIT" && (
          <label>
            Limietprijs · EUR
            <input
              required
              value={limit}
              inputMode="decimal"
              onChange={(e) => setLimit(e.target.value)}
            />
          </label>
        )}
        <label>
          Stop-loss · EUR
          <input
            required
            value={stop}
            inputMode="decimal"
            onChange={(e) => setStop(e.target.value)}
            placeholder="Verplicht, onder de instap"
          />
        </label>
        <label>
          Take-profit · EUR
          <input
            value={target}
            inputMode="decimal"
            onChange={(e) => setTarget(e.target.value)}
            placeholder="Optioneel"
          />
        </label>
        <small>
          Prijsstap {meta?.tickSz || "—"}. De risicokern controleert rekening én
          handmatig deelbudget.
        </small>
        <button
          className="paper-primary"
          disabled={busy || paused || !book}
          type="submit"
        >
          Controleer paperorder <ArrowRight size={14} />
        </button>
        {paused && <span className="warning">Nieuwe instappen gepauzeerd</span>}
      </div>
    </form>
  );
}
function RiskSummary({ account }: { account: PaperAccount }) {
  return (
    <div className="risk-summaries">
      {[
        {
          id: "account",
          name: "Rekening",
          limits: account.limits,
          ledger: account.ledger,
        },
        ...Object.values(account.bots),
      ].map((b) => (
        <section key={b.id}>
          <h3>{b.name}</h3>
          <dl>
            {cashFields.map(([k, label]) => (
              <div key={k}>
                <dt>{label}</dt>
                <dd>
                  {money(b.limits[k])}{" "}
                  <small>
                    {(
                      (Number(b.limits[k]) /
                        Number(
                          k === "daily_loss"
                            ? b.ledger.day_start
                            : k === "drawdown"
                              ? b.ledger.high_water
                              : b.limits.capital,
                        )) *
                      100
                    ).toFixed(2)}
                    % van{" "}
                    {k === "daily_loss"
                      ? "dagstart"
                      : k === "drawdown"
                        ? "high-water-mark"
                        : "budget"}
                  </small>
                </dd>
              </div>
            ))}
            {otherFields.map(([k, label, unit]) => (
              <div key={k}>
                <dt>{label}</dt>
                <dd>
                  {b.limits[k]} {unit}
                </dd>
              </div>
            ))}
            <div>
              <dt>Cumulatief verlies / verliesreeks</dt>
              <dd>
                {money(b.ledger.total_loss)} / {b.ledger.loss_streak}
              </dd>
            </div>
            <div>
              <dt>Leverage / AI-budget</dt>
              <dd>1× / €0</dd>
            </div>
          </dl>
        </section>
      ))}
    </div>
  );
}
function FillTable({ fills }: { fills: Fill[] }) {
  return (
    <>
      <div className="paper-table-scroll">
        <table className="paper-table">
          <thead>
            <tr>
              <th>Tijd · UTC</th>
              <th>Markt</th>
              <th>Zijde</th>
              <th>Hoeveelheid</th>
              <th>Prijs · EUR</th>
              <th>Fee · EUR</th>
            </tr>
          </thead>
          <tbody>
            {[...fills]
              .reverse()
              .slice(0, 100)
              .map((f) => (
                <tr key={f.id}>
                  <td>{date(f.ts)}</td>
                  <td>{f.instrument}</td>
                  <td className={f.side === "BUY" ? "positive" : "warning"}>
                    {f.side}
                  </td>
                  <td>{number(f.quantity)}</td>
                  <td>{number(f.price)}</td>
                  <td>{money(f.fee)}</td>
                </tr>
              ))}
          </tbody>
        </table>
      </div>
      {!fills.length && (
        <Empty>
          Nog geen fills. De simulator verzint geen uitgevoerde transacties.
        </Empty>
      )}
    </>
  );
}
function Bots({
  account,
  symbols,
  busy,
  perform,
}: {
  account: PaperAccount;
  symbols: string[];
  busy: boolean;
  perform: (fn: () => Promise<unknown>, s: string) => Promise<void>;
}) {
  const [create, setCreate] = useState(false);
  const [name, setName] = useState("SMA trend");
  const [symbol, setSymbol] = useState(symbols[0] || "BTC-EUR");
  const [fast, setFast] = useState(20);
  const [slow, setSlow] = useState(50);
  const [order, setOrder] = useState("100");
  const [stop, setStop] = useState("100");
  const [target, setTarget] = useState("200");
  const [limits, setLimits] = useState(preset(1000));
  return (
    <>
      <div className="paper-bot-toolbar">
        <p>
          SMA-kruising op gesloten 1-minuutcandles. Eén positie per bot. Draait
          ook met de browser gesloten.
        </p>
        <button className="paper-primary" onClick={() => setCreate(!create)}>
          {create ? "Sluit formulier" : "Nieuwe paperbot"}
        </button>
      </div>
      {create && (
        <form
          className="paper-setup"
          onSubmit={(e) => {
            e.preventDefault();
            void perform(
              () =>
                post("/api/paper/bots", {
                  id: `bot-${crypto.randomUUID().slice(0, 8)}`,
                  name,
                  instrument: symbol,
                  limits,
                  fast,
                  slow,
                  order_eur: order,
                  stop_bps: stop,
                  target_bps: target,
                }),
              "Bot aangemaakt, gepauzeerd. Start de bot bewust om signalen te laten uitvoeren.",
            );
          }}
        >
          <div className="bot-config-grid">
            <label>
              Naam
              <input
                required
                value={name}
                maxLength={60}
                onChange={(e) => setName(e.target.value)}
              />
            </label>
            <label>
              Markt
              <select
                value={symbol}
                onChange={(e) => setSymbol(e.target.value)}
              >
                {symbols.map((s) => (
                  <option key={s}>{s}</option>
                ))}
              </select>
            </label>
            <label>
              Snelle SMA
              <input
                type="number"
                min={2}
                max={99}
                value={fast}
                onChange={(e) => setFast(Number(e.target.value))}
              />
            </label>
            <label>
              Langzame SMA
              <input
                type="number"
                min={3}
                max={100}
                value={slow}
                onChange={(e) => setSlow(Number(e.target.value))}
              />
            </label>
            <label>
              Orderbudget · EUR
              <input value={order} onChange={(e) => setOrder(e.target.value)} />
            </label>
            <label>
              Stopafstand · bp
              <input value={stop} onChange={(e) => setStop(e.target.value)} />
            </label>
            <label>
              Targetafstand · bp
              <input
                value={target}
                onChange={(e) => setTarget(e.target.value)}
              />
            </label>
          </div>
          <LimitsForm
            title="Vaste botlimieten"
            value={limits}
            onChange={setLimits}
          />
          <button disabled={busy} className="paper-primary">
            Bevestig configuratie en maak bot
          </button>
        </form>
      )}
      <div className="paper-bot-list">
        {Object.values(account.bots)
          .filter((b) => b.id !== "manual")
          .map((b) => (
            <section className="panel paper-bot" key={b.id}>
              <div className="paper-bot-title">
                <div>
                  <span className="eyebrow">{b.version} · PAPER_LOCAL</span>
                  <h3>{b.name}</h3>
                  <span>
                    {b.instrument} · SMA {b.fast}/{b.slow} · 1m
                  </span>
                </div>
                <button
                  disabled={busy}
                  onClick={() =>
                    void perform(
                      () =>
                        post("/api/paper/bots/control", {
                          bot_id: b.id,
                          running: !b.running,
                        }),
                      b.running
                        ? "Bot gepauzeerd. Stops blijven actief."
                        : "Bot gestart binnen de vaste limieten.",
                    )
                  }
                >
                  {b.running ? <Pause size={14} /> : <Play size={14} />}{" "}
                  {b.running ? "Pauzeer" : "Start bot"}
                </button>
              </div>
              <div className="paper-metrics">
                <Metric
                  label="Equity"
                  value={money(b.ledger.equity)}
                  detail={`Budget ${money(b.limits.capital)}`}
                />
                <Metric
                  label="Drawdown"
                  value={money(b.ledger.drawdown)}
                  detail={`Limiet ${money(b.limits.drawdown)}`}
                />
                <Metric
                  label="Order / risico"
                  value={money(b.order_eur)}
                  detail={`Risicolimiet ${money(b.limits.trade_risk)}`}
                />
              </div>
              <div className="paper-bot-decision">
                <span
                  className={
                    b.ledger.paused
                      ? "warning"
                      : b.running
                        ? "positive"
                        : "muted"
                  }
                >
                  {b.ledger.paused
                    ? b.ledger.reason
                    : b.running
                      ? "Actief"
                      : "Gepauzeerd"}
                </span>
                <p>{b.last_signal}</p>
                <small>
                  Laatste gesloten candle:{" "}
                  {b.last_bar
                    ? `${date(b.last_bar)} UTC`
                    : "nog niet beoordeeld"}{" "}
                  · Stop {Number(b.stop_bps) / 100}% · Target{" "}
                  {Number(b.target_bps) / 100}% · Jev alleen met bevestigd
                  AI-budget
                </small>
              </div>
            </section>
          ))}
      </div>
      {Object.keys(account.bots).length === 1 && (
        <Empty>
          Nog geen automatische bot. Begin met een begrensd deelbudget en
          controleer de resultaten via paperhandel en replay.
        </Empty>
      )}
      <p className="paper-note">
        Koppel hierboven een opgeslagen grafiekversie aan een gepauzeerde bot
        zonder exposure. Parameters en limieten van deze deployments wijzigen
        niet tijdens uitvoering.
      </p>
    </>
  );
}
function ReplayWorkspace({ symbols }: { symbols: string[] }) {
  const [symbol, setSymbol] = useState(symbols[0] || "BTC-EUR");
  const [bar, setBar] = useState("15m");
  const [fast, setFast] = useState(20);
  const [slow, setSlow] = useState(50);
  const [capital, setCapital] = useState("1000");
  const [order, setOrder] = useState("100");
  const [stop, setStop] = useState("100");
  const [target, setTarget] = useState("200");
  const [spread, setSpread] = useState("10");
  const [slippage, setSlippage] = useState("5");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [report, setReport] = useState<Replay | null>(null);
  const saved = useQuery({
    queryKey: ["replay-reports"],
    queryFn: () => api<Replay[]>("/api/replay/reports"),
  });
  const run = async () => {
    setBusy(true);
    setError("");
    try {
      setReport(
        await post<Replay>("/api/replay", {
          instrument: symbol,
          bar,
          fast,
          slow,
          capital,
          order_eur: order,
          stop_bps: stop,
          target_bps: target,
          spread_bps: spread,
          slippage_bps: slippage,
        }),
      );
      await saved.refetch();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  };
  const r = report?.report;
  const exportReport = () => {
    if (!report) return;
    const a = document.createElement("a");
    const url = URL.createObjectURL(
      new Blob([JSON.stringify(report, null, 2)], { type: "application/json" }),
    );
    a.href = url;
    a.download = `okx-replay-${report.id}.json`;
    a.click();
    URL.revokeObjectURL(url);
  };
  return (
    <div className="paper-workspace">
      <div className="paper-heading">
        <div>
          <span className="eyebrow">REPLAY · AFZONDERLIJKE SIMULATIE</span>
          <h2>Test dezelfde strategie en risicokern</h2>
          <p>
            Recente echte OKX-candles, uitsluitend gesloten bars. Geen gevolgen
            voor je paperrekening.
          </p>
        </div>
        <FlaskConical size={24} />
      </div>
      <form
        className="panel replay-form"
        onSubmit={(e) => {
          e.preventDefault();
          void run();
        }}
      >
        <div className="bot-config-grid">
          <label>
            Markt
            <select value={symbol} onChange={(e) => setSymbol(e.target.value)}>
              {symbols.map((s) => (
                <option key={s}>{s}</option>
              ))}
            </select>
          </label>
          <label>
            Timeframe
            <select value={bar} onChange={(e) => setBar(e.target.value)}>
              {["1m", "5m", "15m", "1H", "4H", "1Dutc"].map((s) => (
                <option key={s}>{s}</option>
              ))}
            </select>
          </label>
          <label>
            Snelle SMA
            <input
              type="number"
              min={2}
              max={99}
              value={fast}
              onChange={(e) => setFast(Number(e.target.value))}
            />
          </label>
          <label>
            Langzame SMA
            <input
              type="number"
              min={3}
              max={100}
              value={slow}
              onChange={(e) => setSlow(Number(e.target.value))}
            />
          </label>
          <label>
            Virtueel kapitaal · EUR
            <input
              value={capital}
              onChange={(e) => setCapital(e.target.value)}
            />
          </label>
          <label>
            Orderbudget · EUR
            <input value={order} onChange={(e) => setOrder(e.target.value)} />
          </label>
          <label>
            Stopafstand · bp
            <input value={stop} onChange={(e) => setStop(e.target.value)} />
          </label>
          <label>
            Targetafstand · bp
            <input value={target} onChange={(e) => setTarget(e.target.value)} />
          </label>
          <label>
            Spread · bp
            <input value={spread} onChange={(e) => setSpread(e.target.value)} />
          </label>
          <label>
            Extra slippage · bp
            <input
              value={slippage}
              onChange={(e) => setSlippage(e.target.value)}
            />
          </label>
        </div>
        <div className="paper-form-actions">
          <small>
            Fees 10 bp · maximaal 300 candles · geen fills op dezelfde
            signaalcandle · vaste risicopreset
          </small>
          <button className="paper-primary" disabled={busy || !symbols.length}>
            {busy ? "Replay uitvoeren…" : "Voer replay uit"} <Play size={14} />
          </button>
        </div>
      </form>
      {error && (
        <div role="alert" className="paper-alert">
          Replay geblokkeerd: {error}
        </div>
      )}
      <div className="replay-history">
        <label>
          Opgeslagen rapport
          <select
            value={report?.id || ""}
            onChange={(e) =>
              setReport(
                saved.data?.find((x) => x.id === Number(e.target.value)) ||
                  null,
              )
            }
          >
            <option value="">Kies rapport</option>
            {saved.data?.map((x) => (
              <option value={x.id} key={x.id}>
                #{x.id} · {x.report.instrument} · {x.report.bar} ·{" "}
                {x.report.bars} bars
              </option>
            ))}
          </select>
        </label>
        {r && (
          <button onClick={exportReport}>
            <Download size={14} />
            Exporteer rapport
          </button>
        )}
      </div>
      {r ? (
        <>
          <div className="paper-metrics">
            <Metric
              label="Nettoresultaat"
              value={money(r.net_pnl)}
              detail={`${((Number(r.net_pnl) / Number(r.parameters.capital)) * 100).toFixed(2)}% van startkapitaal`}
            />
            <Metric label="Max. drawdown" value={money(r.max_drawdown)} />
            <Metric
              label="Fees"
              value={money(r.fees)}
              detail={`${r.fill_count} fills`}
            />
            <Metric label="Turnover" value={money(r.turnover)} />
            <Metric
              label="Max. exposure"
              value={money(r.max_exposure)}
              detail={`${r.open_positions} resterende posities`}
            />
          </div>
          <section className="panel replay-result">
            <div className="panel-title">
              <strong>
                Equity · {r.instrument} · SMA {r.parameters.fast}/
                {r.parameters.slow}
              </strong>
              <span>{r.bars} gesloten candles</span>
            </div>
            <EquityCurve points={r.curve} />
            <div className="paper-note">
              {date(r.first_ms)} tot {date(r.last_ms)} UTC ·{" "}
              {r.assumptions.path} · Risicostatus{" "}
              {r.risk_state || "binnen limieten"}
              <br />
              {r.assumptions.uncertainty} Open posities zijn tegen de laatste
              biedprijs minus exitfees gewaardeerd.
            </div>
          </section>
          <details className="panel replay-details">
            <summary>Fills en beslissingen</summary>
            <FillTable fills={r.fills} />
            <div className="paper-events">
              {r.decisions.map((v, i) => (
                <div key={i}>
                  <time>{date(v.ts)} UTC</time>
                  <strong>{v.signal}</strong>
                  <span>{v.outcome}</span>
                </div>
              ))}
            </div>
          </details>
        </>
      ) : (
        <Empty>
          Voer een replay uit of open een opgeslagen rapport. Resultaten
          verschijnen alleen na verwerking van echte historische candles.
        </Empty>
      )}
      <p className="paper-footnote">
        Candle-replay schat spread, slippage en liquiditeit. Walk-forward,
        parametergevoeligheid en vergelijking met Jev volgen in fase 4. Een
        historische simulatie bewijst geen winstgevendheid.
      </p>
    </div>
  );
}
function EquityCurve({ points }: { points: { ts: number; equity: string }[] }) {
  const values = points.map((p) => Number(p.equity));
  const low = Math.min(...values);
  const high = Math.max(...values);
  const range = Math.max(high - low, 1);
  const line = values
    .map(
      (v, i) =>
        `${(i / (values.length - 1)) * 1000},${150 - ((v - low) / range) * 130}`,
    )
    .join(" ");
  return (
    <div className="equity-curve">
      <div>
        <small>{money(high)}</small>
        <small>{money(low)}</small>
      </div>
      <svg
        viewBox="0 0 1000 170"
        role="img"
        aria-label="Equitycurve van de replay"
        preserveAspectRatio="none"
      >
        <line x1="0" x2="1000" y1="150" y2="150" stroke="var(--border)" />
        <polyline
          points={line}
          fill="none"
          stroke="var(--green, #5bbd9f)"
          strokeWidth="2"
          vectorEffect="non-scaling-stroke"
        />
      </svg>
    </div>
  );
}
