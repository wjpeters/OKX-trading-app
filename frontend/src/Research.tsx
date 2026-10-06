import ForwardComparison from "./ForwardComparison";
import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "./types";
import { post, Spark } from "./studio-ui";
import { usePreferences } from "./store";
type ResearchData = {
  providers: {
    name: string;
    data: {
      data?: {
        value?: string;
        timestamp?: string;
        value_classification?: string;
        title?: string;
        state?: string;
      }[];
      scope?: string;
      ttl_ms?: number;
    };
    received_ms: number;
    error: string | null;
  }[];
  news: {
    provider: string;
    title: string;
    url: string;
    published_ms: number;
    first_seen_ms: number;
    assets: string[];
  }[];
  restricted: { name: string; state: string; reason: string }[];
  jev: {
    model: string;
    provider: "typesafe" | "openrouter";
    configured: boolean;
    enabled: boolean;
    budget_usd: string;
    spent_usd: string;
    reserved_usd: string;
    failures: number;
    blocked_until: number;
  };
  decisions: {
    id: number;
    created_ms: number;
    envelope: {
      state: string;
      answer?: string;
      confidence?: string;
      cost_usd?: string;
      roundtrip_ms?: number;
      error?: string;
      authority: string;
      model_id?: string;
      input?: unknown;
    };
  }[];
};
export default function Research() {
  const { symbol, bar } = usePreferences();
  const qc = useQueryClient();
  const q = useQuery({
    queryKey: ["research"],
    queryFn: () => api<ResearchData>("/api/research"),
    refetchInterval: 15000,
  });
  const [budget, setBudget] = useState("1.00");
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [filter, setFilter] = useState("");
  const d = q.data;
  const fng = d?.providers.find((p) => p.name === "fear_greed");
  return (
    <section className="studio-page">
      <div className="studio-title">
        <div>
          <h2>Nieuws, sentiment & Jev</h2>
          <p>Bronnen met eigen verversing. Jev mag alleen voorstellen doen.</p>
        </div>
        <span className="pill">PROPOSALS ONLY</span>
      </div>
      {q.error && <p className="error">{q.error.message}</p>}
      <div className="research-grid">
        <div className="studio-card">
          <h3>Jev · {d?.jev.model || "typesafe/jev-1.13"}</h3>
          <p>
            {d?.jev.provider === "typesafe"
              ? "Rechtstreeks via TypeSafe"
              : "Via OpenRouter"}{" "}
            · Instellingen bij Systeem
          </p>
          <p>
            {d?.jev.configured
              ? d.jev.enabled
                ? "Beschikbaar voor voorstellen"
                : "Sleutel aanwezig · kostenbudget nog uitgeschakeld"
              : "Sleutel ontbreekt. Voeg de sleutel toe bij Systeem en kies daar je Jev-provider."}
          </p>
          <div className="metric-line">
            <span>Gebruikt / budget (USD)</span>
            <strong>
              {d?.jev.spent_usd ?? "0"} / {d?.jev.budget_usd ?? "0"}
            </strong>
          </div>
          <div className="metric-line">
            <span>Gereserveerde / onbekende kosten (USD)</span>
            <strong>{d?.jev.reserved_usd ?? "0"}</strong>
          </div>
          <div className="studio-toolbar">
            <input
              aria-label="AI-budget USD"
              type="number"
              min="0"
              max="100"
              step=".01"
              value={budget}
              onChange={(e) => setBudget(e.target.value)}
            />
            <label>
              <input
                type="checkbox"
                checked={confirm}
                onChange={(e) => setConfirm(e.target.checked)}
              />{" "}
              Ik bevestig externe AI-kosten
            </label>
            <button
              disabled={!confirm || !d?.jev.configured}
              onClick={async () => {
                try {
                  await post("/api/jev/config", {
                    budget_usd: budget,
                    enabled: true,
                    confirmed: confirm,
                  });
                  qc.invalidateQueries({ queryKey: ["research"] });
                  setMessage("AI-budget bevestigd.");
                } catch (e) {
                  setMessage(String(e));
                }
              }}
            >
              Activeer budget
            </button>
            <button
              disabled={!d?.jev.enabled}
              onClick={async () => {
                await post("/api/jev/config", {
                  budget_usd: d?.jev.budget_usd,
                  enabled: false,
                  confirmed: true,
                });
                qc.invalidateQueries({ queryKey: ["research"] });
              }}
            >
              Stop Jev
            </button>
          </div>
          <button
            className="primary"
            disabled={!d?.jev.enabled || busy}
            onClick={async () => {
              setBusy(true);
              try {
                const r = await post<{ answer: string; state: string }>(
                  "/api/jev/propose",
                  { instrument: symbol, bar },
                );
                setMessage(`${r.answer} · ${r.state}`);
              } catch (e) {
                setMessage(String(e));
              } finally {
                setBusy(false);
                qc.invalidateQueries({ queryKey: ["research"] });
              }
            }}
          >
            {busy ? "Jev beoordeelt…" : `Vraag voorstel · ${symbol} ${bar}`}
          </button>
          <p role="status">{message}</p>
          <p>
            Confidence is geen winstkans. Deadline 2,5 s; één aanvraag tegelijk.
            Onbekende kosten blijven gereserveerd. Drie fouten openen de
            circuit-breaker vijf minuten. Geen automatische modelwissel.
          </p>
        </div>
        <div className="studio-card">
          <h3>Bitcoin Fear & Greed</h3>
          {fng?.data?.data?.length ? (
            <>
              <strong style={{ fontSize: 36 }}>
                {fng.data.data[0].value}{" "}
                <small style={{ fontSize: 14 }}>
                  {fng.data.data[0].value_classification}
                </small>
              </strong>
              <Spark
                values={fng.data.data
                  .slice()
                  .reverse()
                  .map((v) => Number(v.value))}
              />
              <p>
                Bron:{" "}
                <a
                  href="https://alternative.me/crypto/fear-and-greed-index/"
                  target="_blank"
                  rel="noreferrer"
                >
                  Alternative.me
                </a>{" "}
                · publicatie{" "}
                {new Date(
                  Number(fng.data.data[0].timestamp) * 1000,
                ).toLocaleString("nl-NL")}
              </p>
              <p>
                {fng.data.scope}. Ontvangen{" "}
                {new Date(fng.received_ms).toLocaleString("nl-NL")}.
              </p>
            </>
          ) : (
            <p>Index nog niet beschikbaar.</p>
          )}
          {fng?.error && <p className="warning">{fng.error}</p>}
          <h3>Bronstatus</h3>
          {d?.providers.map((p) => (
            <div className="metric-line" key={p.name}>
              <span>{p.name}</span>
              <span className="provider-state">
                {p.error
                  ? "Bronfout"
                  : Date.now() - p.received_ms > (p.data?.ttl_ms || 900000)
                    ? "Verouderd"
                    : "Ontvangen"}
              </span>
            </div>
          ))}
          {d?.restricted.map((r) => (
            <div key={r.name}>
              <div className="metric-line">
                <strong>{r.name}</strong>
                <span className="provider-state">{r.state}</span>
              </div>
              <p>{r.reason}</p>
            </div>
          ))}
        </div>
      </div>
      <div className="research-grid">
        <div className="studio-card">
          <h3>Exchange-nieuws</h3>
          <input
            aria-label="Nieuws zoeken"
            placeholder="Zoek titel of asset"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
          />
          {d?.news
            .filter((n) =>
              (n.title + n.assets.join(" "))
                .toLowerCase()
                .includes(filter.toLowerCase()),
            )
            .map((n) => (
              <article className="list-row" key={n.url}>
                <div>
                  <a
                    className="news-link"
                    href={n.url}
                    target="_blank"
                    rel="noreferrer"
                  >
                    {n.title}
                  </a>
                  <p>
                    {n.provider} ·{" "}
                    {new Date(n.published_ms).toLocaleString("nl-NL")} ·{" "}
                    {n.assets.join(", ") || "Assets niet vastgesteld"}
                  </p>
                  <small>
                    Eerst ontvangen:{" "}
                    {new Date(n.first_seen_ms).toLocaleString("nl-NL")}
                  </small>
                </div>
              </article>
            ))}
          {!d?.news.length && (
            <p>
              Nog geen nieuws ontvangen. Er worden geen voorbeeldberichten
              getoond.
            </p>
          )}
        </div>
        <div className="studio-card">
          <h3>Jev-beslissingen en kosten</h3>
          {d?.decisions.map((x) => (
            <details key={x.id} className="studio-card">
              <summary>
                {x.envelope.answer || x.envelope.state} ·{" "}
                {new Date(x.created_ms).toLocaleTimeString("nl-NL")}
              </summary>
              <p>
                {x.envelope.error ||
                  `Confidence: ${x.envelope.confidence ?? "ontbreekt"} · USD ${x.envelope.cost_usd ?? "onbekend"} · ${x.envelope.roundtrip_ms?.toFixed(0) ?? "?"} ms`}
              </p>
              <p>
                {x.envelope.authority} · {x.envelope.model_id}
              </p>
              <pre style={{ whiteSpace: "pre-wrap", fontSize: 10 }}>
                {JSON.stringify(x.envelope, null, 2)}
              </pre>
            </details>
          ))}
          {!d?.decisions.length && (
            <p>
              Er zijn nog geen Jev-aanvragen gedaan. Na configuratie verschijnen
              hier echte antwoorden, kosten en afwijzingen.
            </p>
          )}
        </div>
      </div>
      <ForwardComparison />
    </section>
  );
}
