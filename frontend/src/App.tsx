import { useEffect, useRef, useState, lazy, Suspense } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  Activity,
  Bell,
  ArrowDownRight,
  ArrowUpRight,
  BarChart3,
  Blocks,
  Check,
  ChevronDown,
  ChevronRight,
  Code2,
  Command,
  Download,
  Expand,
  FlaskConical,
  LockKeyhole,
  Newspaper,
  Search,
  ShieldCheck,
  SlidersHorizontal,
  Sun,
  Moon,
  Wifi,
  WifiOff,
  X,
  PanelLeftClose,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import ChartWorkspace from "./ChartWorkspace";
import Research from "./Research";
import Evaluation from "./Evaluation";
import Operations from "./Operations";
const PineEditor = lazy(() =>
  import("./Studio").then((m) => ({ default: m.PineEditor })),
);
const BotStudio = lazy(() =>
  import("./Studio").then((m) => ({ default: m.BotStudio })),
);
const Alerts = lazy(() =>
  import("./Studio").then((m) => ({ default: m.Alerts })),
);
import "./studio.css";
import PaperWorkspace from "./Paper";
import { api } from "./types";
import type { Candles, Market, Snapshot, Status, Ticker } from "./types";
import { usePreferences } from "./store";
const pages: { name: string; icon: LucideIcon; phase?: number }[] = [
  { name: "Terminal", icon: BarChart3 },
  { name: "Alerts", icon: Bell },
  { name: "Screener", icon: Activity },
  { name: "Pine Editor", icon: Code2 },
  { name: "Paper", icon: ShieldCheck },
  { name: "Bot Studio", icon: Blocks },
  { name: "Backtests", icon: FlaskConical },
  { name: "Nieuws", icon: Newspaper },
  { name: "Systeem", icon: SlidersHorizontal },
];
const empty: Market = {
  instruments: [],
  tickers: {},
  books: {},
  trades: {},
  feed_connected: false,
  feed_error: null,
  last_message_ms: null,
  reconnects: 0,
};
const fmt = (v: string | number | null | undefined, max = 2) =>
  v == null || !Number.isFinite(Number(v))
    ? "—"
    : new Intl.NumberFormat("nl-NL", {
        maximumFractionDigits: max,
        minimumFractionDigits: Math.min(2, max),
      }).format(Number(v));
const change = (t: Ticker | undefined) =>
  t && Number(t.open24h) > 0
    ? (Number(t.last) / Number(t.open24h) - 1) * 100
    : null;
const millis = (x: number | null | undefined) =>
  x == null ? "—" : `${x.toFixed(3)} ms`;
const clock = (t: string | number, zone = "UTC") =>
  new Date(Number(t)).toLocaleTimeString("nl-NL", {
    timeZone: zone,
    hour12: false,
  });
function Badge({
  good,
  children,
}: {
  good?: boolean;
  children: React.ReactNode;
}) {
  return (
    <span className={`badge ${good ? "good" : ""}`}>
      <i />
      {children}
    </span>
  );
}
function Delta({ value }: { value: number | null }) {
  return (
    <span
      className={
        value === null ? "muted" : value >= 0 ? "positive" : "negative"
      }
    >
      {value === null ? "—" : `${value >= 0 ? "+" : ""}${value.toFixed(2)}%`}
    </span>
  );
}
function useMarket() {
  const [market, setMarket] = useState<Market>(empty);
  const [connected, setConnected] = useState(false);
  const [received, setReceived] = useState(0);
  useEffect(() => {
    let stopped = false;
    let socket: WebSocket | undefined;
    let timer: ReturnType<typeof setTimeout>;
    let attempt = 0;
    const connect = () => {
      socket = new WebSocket(
        `${location.protocol === "https:" ? "wss" : "ws"}://${location.host}/ws`,
      );
      socket.onopen = () => {
        setConnected(true);
        attempt = 0;
      };
      socket.onmessage = (e) => {
        try {
          const data = JSON.parse(e.data) as Snapshot;
          setMarket(data.market);
          setReceived(Date.now());
        } catch {
          setConnected(false);
        }
      };
      socket.onclose = () => {
        setConnected(false);
        if (!stopped) {
          attempt++;
          timer = setTimeout(
            connect,
            Math.min(15000, 1000 * 2 ** Math.min(attempt, 4)),
          );
        }
      };
      socket.onerror = () => socket?.close();
    };
    connect();
    return () => {
      stopped = true;
      clearTimeout(timer);
      socket?.close();
    };
  }, []);
  return { market, connected, received };
}
export function TerminalApp() {
  const pref = usePreferences();
  const { symbol, bar, set, light, timezone } = pref;
  const [page, setPage] = useState("Terminal");
  const [bottom, setBottom] = useState("Marktgegevens");
  const [search, setSearch] = useState("");
  const [palette, setPalette] = useState(false);
  const [now, setNow] = useState(Date.now());
  const [hideList, setHideList] = useState(false);
  const [dock, setDock] = useState(localStorage.getItem("okx-dock") || "right");
  const dragged = useRef<string | null>(null);
  const chartRef = useRef<HTMLElement>(null);
  const { market, connected, received } = useMarket();
  const status = useQuery({
    queryKey: ["status"],
    queryFn: () => api<Status>("/api/status"),
    refetchInterval: 5000,
  });
  const candles = useQuery({
    queryKey: ["candles", symbol, bar],
    queryFn: () =>
      api<Candles>(
        `/api/candles?instrument=${encodeURIComponent(symbol)}&bar=${encodeURIComponent(bar)}`,
      ),
    refetchInterval: 15000,
  });
  useEffect(() => {
    const i = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(i);
  }, []);
  useEffect(() => {
    document.documentElement.dataset.theme = light ? "light" : "dark";
  }, [light]);
  useEffect(() => {
    if (
      market.instruments.length &&
      !market.instruments.some((i) => i.instId === symbol)
    )
      set({ symbol: market.instruments[0].instId });
  }, [market.instruments, symbol, set]);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setPalette((v) => !v);
      }
      if (e.key === "Escape") setPalette(false);
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, []);
  const ticker = market.tickers[symbol];
  const book = market.books[symbol];
  const instrument = market.instruments.find((i) => i.instId === symbol);
  const pct = change(ticker);
  const digits = Math.min(12, (instrument?.tickSz.split(".")[1] || "").length);
  const streamFresh =
    connected &&
    now - received < 3000 &&
    market.feed_connected &&
    market.last_message_ms !== null &&
    now - market.last_message_ms < 15000;
  const age = ticker
    ? Math.max(0, now + (status.data?.clock_offset_ms || 0) - Number(ticker.ts))
    : null;
  const fresh =
    streamFresh &&
    age !== null &&
    age < 15000 &&
    Number(ticker?.ts) <= now + (status.data?.clock_offset_ms || 0) + 3000;
  const quotes = market.instruments.filter((i) =>
    i.instId.toLowerCase().includes(search.toLowerCase()),
  );
  const buy = book?.bids[0]?.[0],
    sell = book?.asks[0]?.[0];
  const spread = buy && sell ? Number(sell) - Number(buy) : null;
  const data = candles.data;
  const candleAge = data
    ? Math.max(0, (now - data.received_at_ms) / 1000)
    : null;
  function exportCsv() {
    if (!data) return;
    const csv = [
      "timestamp_utc,open,high,low,close,volume,closed",
      ...data.candles.map((c) =>
        [
          new Date(c.ts).toISOString(),
          c.open,
          c.high,
          c.low,
          c.close,
          c.volume,
          c.closed,
        ].join(","),
      ),
    ].join("\n");
    const url = URL.createObjectURL(new Blob([csv], { type: "text/csv" }));
    const a = document.createElement("a");
    a.href = url;
    a.download = `OKX-${symbol}-${bar}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  }
  function choose(id: string) {
    set({ symbol: id });
    setPage("Terminal");
    setPalette(false);
    setSearch("");
  }
  return (
    <div className="application">
      <header className="app-header">
        <div className="brand">
          <div className="okx-mark">
            <b />
            <b />
            <b />
            <b />
            <b />
          </div>
          <strong>
            OKX <span>TERMINAL</span>
          </strong>
          <span className="version">04</span>
        </div>
        <div className="header-center">
          <span className="local-dot" /> Lokaal <span className="slash">/</span>{" "}
          Persoonlijke workspace <ChevronDown size={13} />
        </div>
        <div className="header-actions">
          <Badge>PAPER_LOCAL</Badge>
          <button
            className="icon-button"
            onClick={() => setPalette(true)}
            title="Zoeken (⌘K)"
          >
            <Search size={17} />
          </button>
          <button
            className="icon-button"
            onClick={() => set({ light: !light })}
            title="Wissel thema"
          >
            {light ? <Moon size={17} /> : <Sun size={17} />}
          </button>
          <span className="avatar">WP</span>
        </div>
      </header>
      <aside className="navigation">
        <div className="nav-main">
          {pages.map((p) => (
            <button
              key={p.name}
              className={page === p.name ? "nav-item active" : "nav-item"}
              title={p.phase ? `${p.name} · gepland, fase ${p.phase}` : p.name}
              onClick={() => setPage(p.name)}
            >
              <p.icon size={20} />
              <span>{p.name}</span>
              {p.phase && <small>{p.phase}</small>}
            </button>
          ))}
        </div>
        <div className="nav-bottom">
          <ShieldCheck size={20} />
          <span>Risk first</span>
        </div>
      </aside>
      <main className="workspace">
        <div className="workspace-bar">
          <div>
            <span className="eyebrow">WERKRUIMTE</span>
            <h1>{page}</h1>
          </div>
          <div className="workspace-meta">
            <span>
              <ShieldCheck size={13} /> Virtuele EUR-rekening
            </span>
            <span>{status.data?.active_bots ?? 0} actieve bots</span>
            <button onClick={() => setPalette(true)} className="command">
              <Command size={13} /> K
            </button>
          </div>
        </div>
        {(page === "Terminal" || page === "Screener") && (
          <div className="safety-strip">
            <ShieldCheck size={15} />
            <strong>Paperhandel beschikbaar</strong>
            <span>
              Echte marktdata, virtueel geld. Jev heeft geen orderbevoegdheid.
              De vaste risicokern beslist.
            </span>
            <button onClick={() => setPage("Systeem")}>
              Bekijk controles <ChevronRight size={13} />
            </button>
          </div>
        )}
        {page === "Terminal" ? (
          <div
            className={`terminal-grid ${hideList ? "no-list" : ""} dock-${dock}`}
            onDragOver={(e) => e.preventDefault()}
            onDrop={(e) => {
              if (!dragged.current) return;
              e.preventDefault();
              const rect = e.currentTarget.getBoundingClientRect();
              const side =
                e.clientX < rect.x + rect.width / 2 ? "left" : "right";
              setDock(side);
              localStorage.setItem("okx-dock", side);
              dragged.current = null;
            }}
          >
            {!hideList && (
              <section className="watchlist panel">
                <div className="panel-title">
                  <strong>Watchlist</strong>
                  <span className="count">{market.instruments.length}</span>
                  <button
                    className="icon-button"
                    title="Watchlist inklappen"
                    onClick={() => setHideList(true)}
                  >
                    <PanelLeftClose size={15} />
                  </button>
                </div>
                <label className="search">
                  <Search size={14} />
                  <input
                    aria-label="Zoek instrument"
                    placeholder="Zoek instrument"
                    value={search}
                    onChange={(e) => setSearch(e.target.value)}
                  />
                </label>
                <div className="table-label">
                  <span>Instrument</span>
                  <span>Prijs / 24u</span>
                </div>
                <div className="watch-rows">
                  {quotes.map((i) => (
                    <button
                      className={`watch-row ${i.instId === symbol ? "selected" : ""}`}
                      key={i.instId}
                      onClick={() => choose(i.instId)}
                    >
                      <div
                        className={`coin ${i.baseCcy === "BTC" ? "bitcoin" : ""}`}
                      >
                        {i.baseCcy.slice(0, 1)}
                      </div>
                      <div>
                        <strong>{i.baseCcy}</strong>
                        <small>{i.quoteCcy} · SPOT</small>
                      </div>
                      <div className="watch-price">
                        <strong>
                          {fmt(
                            market.tickers[i.instId]?.last,
                            Math.min(12, (i.tickSz.split(".")[1] || "").length),
                          )}
                        </strong>
                        <Delta value={change(market.tickers[i.instId])} />
                      </div>
                    </button>
                  ))}
                </div>
                {!market.instruments.length && (
                  <div className="empty compact">
                    {market.feed_error || "OKX-instrumenten laden…"}
                  </div>
                )}
                <div className="watch-foot">
                  <span className="eyebrow">MARKTBRON</span>
                  <strong>OKX EEA</strong>
                  <span>
                    Publiek beschikbare spotparen.
                    <br />
                    Accountrechten nog onbevestigd.
                  </span>
                </div>
              </section>
            )}
            <section className="chart-panel panel" ref={chartRef}>
              <div className="instrument-header">
                <div className="instrument-name">
                  {hideList && (
                    <button
                      className="icon-button"
                      onClick={() => setHideList(false)}
                      title="Watchlist tonen"
                    >
                      <BarChart3 size={17} />
                    </button>
                  )}
                  <div className="coin large bitcoin">
                    {symbol.split("-")[0].slice(0, 1)}
                  </div>
                  <div>
                    <h2>{symbol.replace("-", " / ")}</h2>
                    <small>
                      OKX <span>·</span> SPOT <span>·</span>{" "}
                      {fresh ? "ACTUEEL" : "WACHT OP VERSE DATA"}
                    </small>
                  </div>
                  <div className="main-price">
                    <strong>{fmt(ticker?.last, digits)}</strong>
                    <Delta value={pct} />
                  </div>
                </div>
                <div className="ohlc-stats">
                  <div>
                    <small>24u hoog</small>
                    <span>{fmt(ticker?.high24h, digits)}</span>
                  </div>
                  <div>
                    <small>24u laag</small>
                    <span>{fmt(ticker?.low24h, digits)}</span>
                  </div>
                  <div>
                    <small>Volume · {instrument?.baseCcy}</small>
                    <span>{fmt(ticker?.vol24h, 0)}</span>
                  </div>
                </div>
              </div>
              <div className="chart-toolbar">
                <div className="timeframes">
                  {["1m", "5m", "15m", "1H", "4H", "1Dutc"].map((t) => (
                    <button
                      className={bar === t ? "selected" : ""}
                      onClick={() => set({ bar: t })}
                      key={t}
                    >
                      {t === "1Dutc" ? "1D" : t}
                    </button>
                  ))}
                </div>
                <span className="separator" />
                <select
                  aria-label="Grafiektype"
                  value={pref.chartType}
                  onChange={(e) => set({ chartType: e.target.value })}
                >
                  {["Candles", "OHLC", "Lijn", "Heikin Ashi"].map((t) => (
                    <option key={t}>{t}</option>
                  ))}
                </select>
                <select
                  aria-label="Overlayindicator"
                  value={pref.overlay}
                  onChange={(e) => set({ overlay: e.target.value })}
                >
                  <option value="none">Geen overlay</option>
                  <option value="ema">EMA</option>
                  <option value="sma">SMA</option>
                  <option value="bb">Bollinger Bands</option>
                  <option value="vwap">VWAP · UTC*</option>
                </select>
                <select
                  aria-label="Indicatorpaneel"
                  value={pref.oscillator}
                  onChange={(e) => set({ oscillator: e.target.value })}
                >
                  <option value="none">Geen oscillator</option>
                  <option value="rsi">RSI</option>
                  <option value="macd">MACD</option>
                  <option value="atr">ATR</option>
                  <option value="stochastic">Stochastic</option>
                  <option value="adx">ADX</option>
                  <option value="obv">OBV*</option>
                </select>
                <div className="toolbar-end">
                  <button
                    className="icon-button"
                    title="Export geladen candles als CSV"
                    onClick={exportCsv}
                    disabled={!data}
                  >
                    <Download size={15} />
                  </button>
                  <button
                    className="icon-button"
                    title="Grafiek fullscreen"
                    onClick={() => {
                      if (document.fullscreenElement)
                        void document.exitFullscreen();
                      else void chartRef.current?.requestFullscreen();
                    }}
                  >
                    <Expand size={15} />
                  </button>
                </div>
              </div>
              {(candles.isError ||
                data?.gaps ||
                pref.chartType === "Heikin Ashi") && (
                <div className="chart-notice">
                  {candles.isError
                    ? `Data ophalen mislukt: ${candles.error.message}`
                    : data?.gaps
                      ? `${data.gaps} datagat(en). Indicatoren herstarten na elk gat.`
                      : "Heikin Ashi toont synthetische prijzen. Niet bruikbaar als uitvoeringsprijs."}
                </div>
              )}
              <div className="chart-body">
                {data ? (
                  <ChartWorkspace
                    data={data}
                    tickSize={instrument?.tickSz || "0.01"}
                  />
                ) : (
                  <div className="chart-empty">
                    <BarChart3 size={34} />
                    <strong>
                      {candles.isError
                        ? "Marktdata niet beschikbaar"
                        : "Koershistorie laden"}
                    </strong>
                    <span>
                      {candles.isError
                        ? "Controleer de verbinding bij Systeem."
                        : "Rechtstreeks van OKX. Geen voorbeelddata."}
                    </span>
                  </div>
                )}
              </div>
              <div className="chart-footer">
                <span>
                  {data
                    ? `${data.candles.length} candles · ${new Date(data.candles[0].ts).toLocaleDateString("nl-NL")} → nu`
                    : "Geen historie geladen"}
                </span>
                <span title="VWAP en OBV gebruiken uitsluitend geladen historie">
                  * Basis: geladen historie
                </span>
                <span
                  className={
                    candles.isError ||
                    !data?.fresh ||
                    (candleAge !== null && candleAge > 30)
                      ? "warning"
                      : ""
                  }
                >
                  {data ? `REST · ${Math.round(candleAge || 0)}s geleden` : ""}
                </span>
                <select
                  aria-label="Tijdzone"
                  value={timezone || "UTC"}
                  onChange={(e) => set({ timezone: e.target.value })}
                >
                  {[
                    "UTC",
                    "Europe/Amsterdam",
                    "America/New_York",
                    "Asia/Tokyo",
                  ].map((z) => (
                    <option key={z}>{z}</option>
                  ))}
                </select>
                <a
                  href="https://www.tradingview.com/"
                  target="_blank"
                  rel="noreferrer"
                >
                  TradingView Lightweight Charts™ · © 2025 TradingView, Inc.
                </a>
              </div>
            </section>
            <aside
              className="right-panels"
              draggable
              onDragStart={() => {
                dragged.current = "orders";
              }}
              title="Sleep dit paneel naar links of rechts"
            >
              <section className="orderbook panel">
                <div className="panel-title">
                  <strong>Orderboek</strong>
                  <Badge good={streamFresh && !!book}>
                    {streamFresh && book ? "LIVE BOEK" : "OFFLINE"}
                  </Badge>
                </div>
                <div className="table-label">
                  <span>Prijs · {instrument?.quoteCcy || "—"}</span>
                  <span>Aantal · {instrument?.baseCcy || "—"}</span>
                </div>
                <div className="book-levels">
                  {book?.asks
                    .slice()
                    .reverse()
                    .map((r, i) => (
                      <div
                        className="book-row ask"
                        key={i}
                        style={
                          {
                            "--depth": `${Math.min(100, (Number(r[1]) / Math.max(...book.asks.map((x) => Number(x[1])), 0.000001)) * 100)}%`,
                          } as React.CSSProperties
                        }
                      >
                        <span>{fmt(r[0], digits)}</span>
                        <span>{fmt(r[1], 6)}</span>
                      </div>
                    ))}
                </div>
                <div className="spread">
                  <strong
                    className={
                      pct !== null && pct >= 0 ? "positive" : "negative"
                    }
                  >
                    {fmt(ticker?.last, digits)}{" "}
                    {pct !== null &&
                      (pct >= 0 ? (
                        <ArrowUpRight size={15} />
                      ) : (
                        <ArrowDownRight size={15} />
                      ))}
                  </strong>
                  <small>Spread {fmt(spread, 4)}</small>
                </div>
                <div className="book-levels">
                  {book?.bids.map((r, i) => (
                    <div
                      className="book-row bid"
                      key={i}
                      style={
                        {
                          "--depth": `${Math.min(100, (Number(r[1]) / Math.max(...book.bids.map((x) => Number(x[1])), 0.000001)) * 100)}%`,
                        } as React.CSSProperties
                      }
                    >
                      <span>{fmt(r[0], digits)}</span>
                      <span>{fmt(r[1], 6)}</span>
                    </div>
                  ))}
                </div>
                {!book && (
                  <div className="empty compact">Wachten op OKX-snapshot</div>
                )}
                <div className="book-foot">
                  Top 5 · gecontroleerde boekupdates{" "}
                  <span>
                    {book
                      ? `${Math.max(0, Math.round((now - book.received_at_ms) / 1000))}s oud`
                      : "—"}
                  </span>
                </div>
              </section>
              <section className="execution panel">
                <div className="panel-title">
                  <strong>Orderuitvoering</strong>
                  <LockKeyhole size={14} />
                </div>
                <div className="execution-content">
                  <div className="locked-icon">
                    <ShieldCheck size={25} />
                  </div>
                  <h3>De risicokern beslist.</h3>
                  <p>
                    De paperadapter voert uitsluitend door de vaste risicokern
                    goedgekeurde orders uit. Controleer budget, prijs en stop in
                    de preview.
                  </p>
                  <div className="gate-row">
                    <Check size={13} />
                    <span>Geen live-orderpad in deze build</span>
                  </div>
                  <div className="gate-row muted">
                    <LockKeyhole size={13} />
                    <span>Virtuele budgetten eerst bewust bevestigen</span>
                  </div>
                  <button
                    onClick={() => setPage("Paper")}
                    className="paper-primary"
                  >
                    <ShieldCheck size={14} /> Open paperhandel
                  </button>
                  <small>
                    Verlieslimieten zijn actiedrempels, geen verliesgarantie.
                  </small>
                </div>
              </section>
            </aside>
            <section className="bottom-panel panel">
              <div className="bottom-tabs">
                {["Marktgegevens", "Transactiestroom", "Risicocontroles"].map(
                  (b) => (
                    <button
                      className={bottom === b ? "active" : ""}
                      key={b}
                      onClick={() => setBottom(b)}
                    >
                      {b}
                    </button>
                  ),
                )}
                <span className="bottom-caption">
                  {symbol} <span className="muted">/ Publieke data</span>
                </span>
              </div>
              {bottom === "Marktgegevens" ? (
                <div className="market-details">
                  <div>
                    <small>Tick size</small>
                    <strong>{instrument?.tickSz || "—"}</strong>
                    <span>Prijsstap</span>
                  </div>
                  <div>
                    <small>Lot size</small>
                    <strong>{instrument?.lotSz || "—"}</strong>
                    <span>Aantalstap</span>
                  </div>
                  <div>
                    <small>Minimumorder</small>
                    <strong>
                      {instrument?.minSz || "—"} <em>{instrument?.baseCcy}</em>
                    </strong>
                    <span>Publieke instrumentmetadata</span>
                  </div>
                  <div>
                    <small>Candlekwaliteit</small>
                    <strong>
                      {!data
                        ? "Onbekend"
                        : data.gaps
                          ? "Datagat"
                          : data.fresh && !candles.isError
                            ? "Actueel"
                            : "Controle nodig"}
                    </strong>
                    <span>
                      {data?.candles.at(-1)?.closed
                        ? "Laatste candle gesloten"
                        : "Laatste candle kan nog wijzigen"}
                    </span>
                  </div>
                  <div>
                    <small>Berekening</small>
                    <strong>{millis(data?.calculation_ms)}</strong>
                    <span>10 native indicatoren</span>
                  </div>
                </div>
              ) : bottom === "Transactiestroom" ? (
                <div className="trade-scroll">
                  <table>
                    <thead>
                      <tr>
                        <th>Tijd · {timezone}</th>
                        <th>Prijs</th>
                        <th>Aantal</th>
                        <th>Agressor</th>
                        <th>Trade-ID</th>
                      </tr>
                    </thead>
                    <tbody>
                      {(market.trades[symbol] || []).slice(0, 40).map((t) => (
                        <tr key={t.tradeId}>
                          <td>{clock(t.ts, timezone)}</td>
                          <td>{fmt(t.px, 4)}</td>
                          <td>{fmt(t.sz, 6)}</td>
                          <td
                            className={
                              t.side === "buy" ? "positive" : "negative"
                            }
                          >
                            {t.side === "buy" ? "Koop" : "Verkoop"}
                          </td>
                          <td className="muted">{t.tradeId}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                  {!market.trades[symbol]?.length && (
                    <div className="empty compact">
                      Nog geen transacties ontvangen. Dit zijn markttransacties,
                      geen eigen fills.
                    </div>
                  )}
                </div>
              ) : (
                <div className="risk-summary">
                  <ShieldCheck size={29} />
                  <div>
                    <strong>{status.data?.risk_state || "ONBEKEND"}</strong>
                    <p>
                      De paperkern bewaakt budgetten, verse waardering,
                      verlieslimieten en bescherming. Jev en scripts krijgen
                      geen rechtstreekse orderroute.
                    </p>
                  </div>
                  <button onClick={() => setPage("Systeem")}>
                    Alle controles <ChevronRight size={14} />
                  </button>
                </div>
              )}
            </section>
          </div>
        ) : page === "Screener" ? (
          <section className="screener panel">
            <div className="panel-title">
              <strong>Spotmarkten</strong>
              <span className="muted">
                {market.instruments.length} geverifieerde publieke instrumenten
              </span>
            </div>
            <table>
              <thead>
                <tr>
                  <th>Instrument</th>
                  <th>Laatste prijs</th>
                  <th>24u verandering</th>
                  <th>Hoog</th>
                  <th>Laag</th>
                  <th>Volume in basisvaluta</th>
                </tr>
              </thead>
              <tbody>
                {market.instruments.map((i) => {
                  const t = market.tickers[i.instId];
                  return (
                    <tr
                      key={i.instId}
                      onClick={() => choose(i.instId)}
                      tabIndex={0}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") choose(i.instId);
                      }}
                    >
                      <td>
                        <strong>{i.instId}</strong>
                      </td>
                      <td>{fmt(t?.last, 4)}</td>
                      <td>
                        <Delta value={change(t)} />
                      </td>
                      <td>{fmt(t?.high24h, 4)}</td>
                      <td>{fmt(t?.low24h, 4)}</td>
                      <td>
                        {fmt(t?.vol24h, 2)} {i.baseCcy}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
            <div className="panel-note">
              Publieke beschikbaarheid betekent niet dat jouw account dit
              instrument mag verhandelen. Klik op een instrument om de chart te
              openen.
            </div>
          </section>
        ) : page === "Nieuws" || page === "Nieuws / Sentiment" ? (
          <Research />
        ) : page === "Pine Editor" ? (
          <Suspense fallback={<p>Editor laden…</p>}>
            <PineEditor />
          </Suspense>
        ) : page === "Alerts" ? (
          <Suspense fallback={<p>Alerts laden…</p>}>
            <Alerts />
          </Suspense>
        ) : page === "Paper" ? (
          <PaperWorkspace market={market} />
        ) : page === "Bot Studio" ? (
          <Suspense fallback={<p>Bot Studio laden…</p>}>
            <BotStudio market={market} />
          </Suspense>
        ) : page === "Backtests" ? (
          <div style={{ overflow: "auto", height: "100%" }}>
            <PaperWorkspace market={market} view="replay" />
            <Evaluation />
          </div>
        ) : page === "Systeem" ? (
          <div style={{ overflow: "auto", height: "100%" }}>
            <System
              status={status.data}
              error={status.isError}
              streamFresh={streamFresh}
            />
            <Operations />
          </div>
        ) : (
          <p>Werkruimte niet gevonden.</p>
        )}
      </main>
      <footer className="statusbar">
        <span className={streamFresh ? "positive" : "warning"}>
          {streamFresh ? <Wifi size={12} /> : <WifiOff size={12} />}{" "}
          {streamFresh ? "OKX verbonden" : "Marktverbinding onderbroken"}
        </span>
        <span
          className={status.data?.database && !status.isError ? "" : "warning"}
        >
          Database{" "}
          {status.isError
            ? "onbekend"
            : status.data?.database
              ? "verbonden"
              : "niet beschikbaar"}
        </span>
        <span className={fresh ? "" : "warning"}>
          Ticker {age === null ? "onbekend" : `${Math.round(age / 1000)}s oud`}
        </span>
        <span className="push-right">Jev: voorstellen</span>
        <span>Risico: {status.data?.risk_state || "ONBEKEND"}</span>
        <span>
          {clock(now, timezone)} {timezone}
        </span>
      </footer>
      {palette && (
        <div className="modal-backdrop" onClick={() => setPalette(false)}>
          <div
            className="palette"
            role="dialog"
            aria-modal="true"
            aria-label="Command palette"
            onClick={(e) => e.stopPropagation()}
          >
            <label className="palette-input">
              <Search size={18} />
              <input
                autoFocus
                placeholder="Zoek een markt of werkruimte…"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
              <button className="icon-button" onClick={() => setPalette(false)}>
                <X size={18} />
              </button>
            </label>
            <div className="palette-results">
              {pages
                .filter((p) =>
                  p.name.toLowerCase().includes(search.toLowerCase()),
                )
                .map((p) => (
                  <button
                    key={p.name}
                    onClick={() => {
                      setPage(p.name);
                      setPalette(false);
                      setSearch("");
                    }}
                  >
                    <p.icon size={17} />
                    {p.name}
                    <small>
                      {p.phase ? `Gepland · fase ${p.phase}` : "Werkruimte"}
                    </small>
                  </button>
                ))}
              {quotes.map((i) => (
                <button key={i.instId} onClick={() => choose(i.instId)}>
                  <BarChart3 size={17} />
                  {i.instId}
                  <small>OKX · SPOT</small>
                </button>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
function System({
  status: s,
  error,
  streamFresh,
}: {
  status: Status | undefined;
  error: boolean;
  streamFresh: boolean;
}) {
  return (
    <div className="system">
      <div className="system-heading">
        <span className="eyebrow">LOKALE CONTROLEKAMER</span>
        <h2>Inzicht in elke verbinding.</h2>
        <p>
          Gemeten status van deze lokale installatie. Echte handel is niet
          vrijgegeven.
        </p>
      </div>
      <div className="system-grid">
        {[
          [
            "Marktdata",
            streamFresh ? "Verbonden" : "Onderbroken",
            "OKX EEA · publieke WebSocket",
          ],
          [
            "Database",
            error ? "Onbekend" : s?.database ? "Verbonden" : "Niet beschikbaar",
            "PostgreSQL · lokale Docker-opslag",
          ],
          [
            "Account",
            "Niet geverifieerd",
            "Regio, rechten en fees nog te controleren",
          ],
          [
            "Uitvoering",
            "PAPER_LOCAL",
            "Lokale simulatie; geen exchange-orderadapter",
          ],
        ].map(([title, value, detail]) => (
          <section className="system-card panel" key={title}>
            <span>{title}</span>
            <strong className={value === "Verbonden" ? "positive" : "warning"}>
              {value}
            </strong>
            <small>{detail}</small>
          </section>
        ))}
      </div>
      <section className="panel latency">
        <div className="panel-title">
          <strong>Gemeten lokale verwerking</strong>
          <Badge good={!!s?.processing_ms.samples}>NATIVE RUST</Badge>
        </div>
        <div className="market-details">
          <div>
            <small>p50</small>
            <strong>{millis(s?.processing_ms.p50)}</strong>
          </div>
          <div>
            <small>p95</small>
            <strong>{millis(s?.processing_ms.p95)}</strong>
          </div>
          <div>
            <small>p99</small>
            <strong>{millis(s?.processing_ms.p99)}</strong>
          </div>
          <div>
            <small>Samples</small>
            <strong>{s?.processing_ms.samples || 0}</strong>
          </div>
        </div>
        <p className="panel-note">
          WebSocket-validatie en lokale marktupdate, laatste maximaal 2.048
          berichten. Dit is geen orderlatency of strategiebenchmark. OKX
          tijdmeting bij start: RTT {s?.clock_rtt_ms ?? "—"} ms; geschatte
          klokafwijking {s?.clock_offset_ms ?? "—"} ms.
        </p>
      </section>
      <section className="panel system-gates">
        <div className="panel-title">
          <strong>Acceptatie voor orderuitvoering</strong>
          <span className="warning">0 live-adapters actief</span>
        </div>
        {[
          "Accountregio, instrumentrechten en actuele fees vaststellen",
          "Verplichte account- en botlimieten expliciet invullen",
          "Paper risk/OMS getest; private reconcile moet nog met jouw Demo-account worden geaccepteerd",
          "Paper en OKX Demo isoleren en foutinjectie doorlopen",
          "Beveiligingsreview en bewuste vrijgave voor LIVE",
        ].map((x, i) => (
          <div className="system-gate" key={x}>
            <span>{String(i + 1).padStart(2, "0")}</span>
            <p>{x}</p>
            <Badge>NOG NIET VRIJGEGEVEN</Badge>
          </div>
        ))}
      </section>
      {s?.feed_error && (
        <p className="warning">Verbindingsmelding: {s.feed_error}</p>
      )}
    </div>
  );
}

export default function App() {
  const auth = useQuery({
    queryKey: ["auth-status"],
    queryFn: () =>
      api<{ configured: boolean; authenticated: boolean }>("/api/auth/status"),
    refetchInterval: 30000,
    retry: false,
  });
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  if (auth.isPending)
    return <div className="studio-card">Lokale sessie controleren…</div>;
  if (auth.error)
    return (
      <div className="studio-card">
        <p>Verbinding met de lokale terminal ontbreekt.</p>
        <button onClick={() => auth.refetch()}>Opnieuw proberen</button>
      </div>
    );
  if (auth.data?.authenticated)
    return (
      <>
        <TerminalApp />
        {auth.data.configured && (
          <button
            className="session-logout"
            onClick={async () => {
              await fetch("/api/auth/logout", {
                method: "POST",
                headers: { "X-Terminal-Request": "paper-v2" },
              });
              await auth.refetch();
            }}
          >
            Afmelden
          </button>
        )}
      </>
    );
  return (
    <main className="auth-gate">
      <form
        className="studio-card"
        onSubmit={async (e) => {
          e.preventDefault();
          setBusy(true);
          setError("");
          try {
            const r = await fetch("/api/auth/login", {
              method: "POST",
              headers: {
                "Content-Type": "application/json",
                "X-Terminal-Request": "paper-v2",
              },
              body: JSON.stringify({ password }),
            });
            if (!r.ok) throw Error("Aanmelden mislukt");
            setPassword("");
            await auth.refetch();
          } catch (e) {
            setError(String(e));
          } finally {
            setBusy(false);
          }
        }}
      >
        <h1>OKX Terminal</h1>
        <p>Beveiligde lokale sessie</p>
        <label>
          Wachtwoord
          <input
            type="password"
            autoComplete="current-password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            maxLength={256}
            required
          />
        </label>
        <button className="primary" disabled={busy}>
          {busy ? "Controleren…" : "Aanmelden"}
        </button>
        <p role="alert">{error}</p>
      </form>
    </main>
  );
}
