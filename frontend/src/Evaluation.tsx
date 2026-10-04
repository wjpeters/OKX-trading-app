import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "./types";
import { post, Spark, download } from "./studio-ui";
import { usePreferences } from "./store";
type Part = {
  id: number;
  instrument: string;
  bar: string;
  first_ms: number;
  last_ms: number;
  rows: number;
  gaps: number;
};
type Metric = {
  net_pnl: string;
  fees: string;
  max_drawdown: string;
  fill_count: number;
  turnover: string;
  parameters: { fast: number; slow: number };
};
type Evaluation = {
  id: number;
  report: {
    bars: number;
    instrument: string;
    bar: string;
    sensitivity_training_only: Metric[];
    folds: {
      fold: number;
      training_bars: number;
      test_bars: number;
      selected_without_test_data: string;
      out_of_sample: Metric;
      curve: { equity: string }[];
    }[];
    policy: string;
    uncertainty: string;
    jev_comparison: { reason: string };
  };
};
export default function Evaluation() {
  const { symbol, bar } = usePreferences();
  const qc = useQueryClient();
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [report, setReport] = useState<Evaluation>();
  const [pages, setPages] = useState(2);
  const parts = useQuery({
    queryKey: ["history"],
    queryFn: () => api<Part[]>("/api/history"),
  });
  const saved=useQuery({queryKey:["walk-forward-reports"],queryFn:()=>api<Evaluation[]>("/api/replay/reports?mode=WALK_FORWARD")});
  return (
    <div className="studio-page" style={{ height: "auto" }}>
      <div className="studio-title">
        <div>
          <h2>Evaluatie & data-archief</h2>
          <p>
            Afzonderlijke testvensters, maximaal 10.000 geïmporteerde candles en
            expliciete datadekking.
          </p>
        </div>
        <span>
          {symbol} · {bar}
        </span>
      </div>
      <div className="studio-toolbar">
        <select aria-label="Opgeslagen walk-forwardrapport" value={report?.id || ""} onChange={e=>setReport(saved.data?.find(r=>r.id===Number(e.target.value)))}>
          <option value="">Opgeslagen walk-forwardrapporten</option>
          {saved.data?.map(r=><option key={r.id} value={r.id}>#{r.id} · {r.report.instrument} · {r.report.bar} · {r.report.bars} bars</option>)}
        </select>
        <button
          className="primary"
          disabled={busy || !symbol.endsWith("-EUR")}
          onClick={async () => {
            setBusy(true);
            try {
              setReport(
                await post<Evaluation>("/api/replay/evaluate", {
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
                }),
              );
              qc.invalidateQueries({queryKey:["walk-forward-reports"]});
              setMessage(
                "Walk-forward afgerond. Virtuele aannames: €1.000 kapitaal, €100 per order, stop 1%, target 2%, spread 10 bp, slippage 5 bp.",
              );
            } catch (e) {
              setMessage(String(e));
            } finally {
              setBusy(false);
            }
          }}
        >
          Walk-forward & gevoeligheid
        </button>
        {report && (
          <button
            onClick={() =>
              download(JSON.stringify(report, null, 2), "walk-forward.json")
            }
          >
            Exporteer rapport
          </button>
        )}
        <select
          aria-label="Historiepagina’s"
          value={pages}
          onChange={(e) => setPages(+e.target.value)}
        >
          {[1, 2, 5, 10].map((n) => (
            <option value={n} key={n}>
              {n * 300} candles per import
            </option>
          ))}
        </select>
        <button
          disabled={busy}
          onClick={async () => {
            setBusy(true);
            try {
              const r = await post<{ parts: number[] }>("/api/history", {
                instrument: symbol,
                bar,
                pages,
              });
              setMessage(
                `${r.parts.length} Parquet-bestanden opgeslagen. De volgende import hervat vanaf de oudste ontvangen candle.`,
              );
              qc.invalidateQueries({ queryKey: ["history"] });
            } catch (e) {
              setMessage(String(e));
            } finally {
              setBusy(false);
            }
          }}
        >
          Importeer / hervat historie
        </button>
        <button
          disabled={busy}
          onClick={async () => {
            setBusy(true);
            try {
              await post("/api/history", {
                instrument: symbol,
                bar,
                pages,
                resume: false,
              });
              setMessage("Recente historie vernieuwd.");
              qc.invalidateQueries({ queryKey: ["history"] });
            } catch (e) {
              setMessage(String(e));
            } finally {
              setBusy(false);
            }
          }}
        >
          Ververs recente historie
        </button>
      </div>
      <p role="status">{busy ? "Berekening / import loopt…" : message}</p>
      {report && (
        <>
          <div className="research-grid">
            {report.report.folds.map((f) => (
              <div className="studio-card" key={f.fold}>
                <h3>
                  Testvenster {f.fold} · SMA {f.selected_without_test_data}
                </h3>
                <p>
                  {f.training_bars} trainingsbars → {f.test_bars} nieuwe
                  testbars
                </p>
                <Spark values={f.curve.map((x) => Number(x.equity))} />
                <div className="metric-line">
                  <span>Nettoresultaat</span>
                  <strong>€ {f.out_of_sample.net_pnl}</strong>
                </div>
                <div className="metric-line">
                  <span>Max. drawdown / fees</span>
                  <strong>
                    € {f.out_of_sample.max_drawdown} / € {f.out_of_sample.fees}
                  </strong>
                </div>
                <p>
                  {f.out_of_sample.fill_count} fills · turnover €{" "}
                  {f.out_of_sample.turnover}
                </p>
              </div>
            ))}
          </div>
          <div className="studio-card">
            <h3>Parametergevoeligheid · alleen eerste trainingsvenster</h3>
            <table>
              <thead>
                <tr>
                  <th>SMA</th>
                  <th>Netto €</th>
                  <th>Drawdown €</th>
                  <th>Fees €</th>
                  <th>Fills</th>
                </tr>
              </thead>
              <tbody>
                {report.report.sensitivity_training_only.map((m) => (
                  <tr key={m.parameters.fast + "/" + m.parameters.slow}>
                    <td>
                      {m.parameters.fast}/{m.parameters.slow}
                    </td>
                    <td>{Number(m.net_pnl).toFixed(2)}</td>
                    <td>{Number(m.max_drawdown).toFixed(2)}</td>
                    <td>{Number(m.fees).toFixed(2)}</td>
                    <td>{m.fill_count}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            <p>{report.report.policy}</p>
            <p>{report.report.uncertainty}</p>
            <p>Jev: {report.report.jev_comparison.reason}</p>
          </div>
        </>
      )}
      <div className="studio-card">
        <h3>Geïmporteerde dekking · gesloten OKX-candles</h3>
        <table>
          <thead>
            <tr>
              <th>Instrument</th>
              <th>Resolutie</th>
              <th>Van / tot (UTC)</th>
              <th>Candles</th>
              <th>Gaten</th>
              <th>Bestand</th>
            </tr>
          </thead>
          <tbody>
            {parts.data?.map((p) => (
              <tr key={p.id}>
                <td>{p.instrument}</td>
                <td>{p.bar}</td>
                <td>
                  {new Date(p.first_ms).toISOString()}
                  <br />
                  {new Date(p.last_ms).toISOString()}
                </td>
                <td>{p.rows}</td>
                <td>{p.gaps}</td>
                <td>
                  <a href={`/api/history/export?id=${p.id}`}>Parquet</a>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        {!parts.data?.length && (
          <p>
            Er is nog geen bulkimport gestart. Importeren verandert geen
            paperbalans.
          </p>
        )}
        <p>
          Imports hervatten per instrument/timeframe en dedupliceren per bereik.
          Geen opvulling van ontbrekende data. Parquet met Snappy, exacte
          decimale prijsstrings. Tick- en L2-historie wordt niet uit candles
          gereconstrueerd.
        </p>
      </div>
    </div>
  );
}
