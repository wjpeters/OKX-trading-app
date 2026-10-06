import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "./types";
import type { Bot, PaperState, PaperAccount } from "./Paper";

export const money = (value: string | number) =>
  new Intl.NumberFormat("nl-NL", { style: "currency", currency: "EUR" }).format(
    Number(value),
  );
export const time = (value: number) =>
  value ? new Date(value).toLocaleString("nl-NL") : "Nog geen candle verwerkt";
export function decision(value: string) {
  const labels: Record<string, string> = {
    HOLD: "Wachten · geen koop- of verkoopsignaal",
    WARMUP: "Wachten · meer koershistorie nodig",
    STALE_CLOSE: "Overgeslagen · candle kwam te laat binnen",
    CANDLE_GAP_OR_STALE: "Geblokkeerd · koersdata is onvolledig of verouderd",
    BUY: "Koopsignaal",
    SELL: "Verkoopsignaal",
  };
  return (
    labels[value] ||
    value
      .replace(/^BUY → /, "Kooporder klaargezet · ")
      .replace(/^SELL → /, "Verkooporder klaargezet · ")
      .replace(/^BUY · /, "Koop · ")
      .replace(/^SELL · /, "Verkoop · ") ||
    "Wachten op eerste beslissing"
  );
}
const orderState: Record<string, string> = {
  INTENT: "Voorstel",
  PENDING_SUBMIT: "Klaargezet",
  ACCEPTED: "Geaccepteerd",
  PARTIAL_FILL: "Deels uitgevoerd",
  FILLED: "Volledig uitgevoerd",
  PENDING_CANCEL: "Wordt geannuleerd",
  CANCELED: "Geannuleerd",
  REJECTED: "Geweigerd",
  UNKNOWN: "Status onbekend",
  ARMED: "Stop actief",
  EXIT_PENDING: "Sluitorder actief",
  CLOSED: "Gesloten",
  DUST: "Restpositie onder orderminimum",
};
function eventDetail(e: PaperAccount["events"][number]) {
  const plain: Record<string, string> = {
    INTENT: "Risicocontrole geslaagd. Benodigd virtueel saldo gereserveerd.",
    PENDING_SUBMIT: "De order wacht op virtuele uitvoering.",
    ACCEPTED: "De order is geaccepteerd, maar nog niet uitgevoerd.",
    BOT_CREATED: "Instellingen vastgelegd. De bot staat klaar om te starten.",
    BOT_STARTED: "De bot volgt live koersen. Positiebescherming is actief.",
    BOT_PAUSED:
      "Nieuwe strategieorders gestopt. Bescherming van open posities blijft actief.",
  };
  return (
    plain[e.kind] ||
    decision(
      e.detail
        .replace(/Candle (\d+)/, (_, ts) => "Candle " + time(Number(ts)))
        .replace("Signaal BUY", "Koopsignaal")
        .replace("Signaal SELL", "Verkoopsignaal")
        .replace("Signaal HOLD", "Wachtsignaal")
        .replace("BUY →", "Kooporder →")
        .replace("SELL →", "Verkooporder →")
        .replace("; fee ", "; kosten ")
        .replace("PARTIAL_FILL", "deels uitgevoerd")
        .replace("FILLED", "volledig uitgevoerd")
        .replace("; HOLD", "; geen order"),
    )
  );
}
const eventNames: Record<string, string> = {
  BOT_DECISION: "Beslissing",
  BOT_CANDLE_STOP: "Candle-daling · noodregel uitgevoerd",
  BOT_DATA_BLOCK: "Data of strategie geblokkeerd",
  BOT_STARTED: "Bot gestart",
  BOT_PAUSED: "Bot gepauzeerd",
  BOT_CONTROL: "Botbediening",
  BOT_CREATED: "Bot klaargezet",
  DEPLOY_GRAPH: "Ontwerp toegepast",
  INTENT: "Order goedgekeurd",
  PENDING_SUBMIT: "Order klaargezet",
  ACCEPTED: "Order geaccepteerd",
  FILL: "Virtuele uitvoering",
  REJECTED: "Order geweigerd",
  CANCELED: "Order geannuleerd",
  PENDING_CANCEL: "Annulering aangevraagd",
  RISK_STOP: "Risicolimiet bereikt",
  ACCOUNT_CREATED: "Rekening aangemaakt",
  RECOVERED: "Rekening hersteld",
  PROTECTION: "Positiebescherming",
};
export default function BotActivity({
  bot,
  state,
}: {
  bot: Bot;
  state: PaperState;
}) {
  const [view, setView] = useState("Activiteit");
  const activity = useQuery({
    queryKey: ["bot-activity", bot.id],
    queryFn: () =>
      api<PaperAccount["events"]>(
        "/api/paper/bots/activity?bot_id=" + encodeURIComponent(bot.id),
      ),
    refetchInterval: 3000,
  });
  const account = state.account!;
  const orders = Object.values(account.orders)
    .filter((o) => o.intent.bot_id === bot.id)
    .sort((a, b) => b.created_ms - a.created_ms);
  const ids = new Set(orders.map((o) => o.id));
  const fills = account.fills
    .filter((f) => ids.has(f.order_id))
    .slice()
    .reverse();
  const positions = Object.values(account.positions).filter(
    (p) => p.bot_id === bot.id && Number(p.quantity) > 0,
  );
  const blocked = bot.ledger.paused || account.ledger.paused;
  return (
    <section className="bot-activity" aria-label={"Uitvoering van " + bot.name}>
      <div className="bot-runtime-heading">
        <div>
          <h3>{bot.name} · uitvoering</h3>
          <p>
            Paperhandel op live OKX-data · virtueel geld · {bot.instrument} ·{" "}
            {bot.version.startsWith("graph:")
              ? "versie " + bot.version.split(":").at(-1)
              : bot.version}
          </p>
        </div>
        <span className="pill">
          {!state.worker_alive
            ? "Uitvoering niet bereikbaar"
            : blocked
              ? "Risicostop"
              : bot.running
                ? "Actief op live data"
                : "Gepauzeerd"}
        </span>
      </div>
      {account.bot_exits?.includes(bot.id) && (
        <p className="warning" role="status">
          Candle-stop: afbouw is aangevraagd. De uitvoerder blijft proberen met
          verse marktdata totdat de positie gesloten is. Nieuwe aankopen zijn
          geblokkeerd.
        </p>
      )}
      <div className="bot-stats">
        <div>
          <small>Laatste beslissing</small>
          <strong>{decision(bot.last_signal)}</strong>
          <span>{time(bot.last_bar)} · candle-start</span>
        </div>
        <div>
          <small>Botvermogen</small>
          <strong>{money(bot.ledger.equity)}</strong>
          <span>
            Resultaat{" "}
            {money(Number(bot.ledger.equity) - Number(bot.limits.capital))}
            {!account.valuation_fresh ? " · waardering verouderd" : ""}
          </span>
        </div>
        <div>
          <small>Uitgevoerde transacties</small>
          <strong>{fills.length}</strong>
          <span>
            {positions.length} open posities · kosten {money(bot.ledger.fees)}
          </span>
        </div>
      </div>
      {(blocked || !state.worker_alive) && (
        <p className="warning" role="status">
          {!state.worker_alive
            ? "De uitvoerder is niet bereikbaar. Onderstaande gegevens zijn de laatst ontvangen stand."
            : account.ledger.reason || bot.ledger.reason}
        </p>
      )}
      <p className="bot-runtime-note">
        {bot.running
          ? "Beoordeelt iedere nieuw gesloten 1-minuutcandle. Geen order bij iedere koerswijziging."
          : "Nieuwe strategieorders staan uit. Open posities en hun bescherming blijven bestaan."}{" "}
        Bijgewerkt: {time(state.server_time_ms)}.
      </p>
      <div className="studio-tabs" role="group" aria-label="Uitvoeringsdetails">
        {["Activiteit", "Orders", "Uitvoeringen", "Posities"].map((v) => (
          <button
            key={v}
            aria-pressed={view === v}
            className={view === v ? "active" : ""}
            onClick={() => setView(v)}
          >
            {v}
          </button>
        ))}
      </div>
      {view === "Activiteit" && (
        <>
          <p className="muted">
            Laatste 150 gebeurtenissen van deze bot en gedeelde
            rekeningcontroles. Nieuwste bovenaan.
          </p>
          {activity.isError ? (
            <p role="alert">
              Activiteit kon niet worden vernieuwd: {String(activity.error)}
            </p>
          ) : activity.isPending ? (
            <p>Activiteit laden…</p>
          ) : (
            <ol className="bot-timeline">
              {activity.data?.map((e) => (
                <li key={e.seq}>
                  <time>{time(e.ts)}</time>
                  <div>
                    <strong>
                      {eventNames[e.kind] || e.kind}
                      {["account", "paper"].includes(e.subject)
                        ? " · rekening"
                        : ""}
                    </strong>
                    <p>{eventDetail(e)}</p>
                  </div>
                </li>
              ))}
              {!activity.data?.length && (
                <li>Nog geen activiteit voor deze bot.</li>
              )}
            </ol>
          )}
        </>
      )}
      {view === "Orders" && (
        <div className="bot-table">
          <table>
            <thead>
              <tr>
                <th>Tijd</th>
                <th>Actie</th>
                <th>Aantal</th>
                <th>Uitgevoerd</th>
                <th>Status / reden</th>
              </tr>
            </thead>
            <tbody>
              {orders.slice(0, 100).map((o) => (
                <tr key={o.id}>
                  <td>{time(o.created_ms)}</td>
                  <td>{o.intent.side === "BUY" ? "Koop" : "Verkoop"}</td>
                  <td>{o.quantity}</td>
                  <td>{o.filled}</td>
                  <td>
                    {orderState[o.state] || o.state}
                    {o.reason && " · " + o.reason}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {!orders.length && (
            <p>
              Nog geen orders. Bekijk Activiteit om te zien waarom de bot wacht.
            </p>
          )}
        </div>
      )}
      {view === "Uitvoeringen" && (
        <div className="bot-table">
          <table>
            <thead>
              <tr>
                <th>Tijd</th>
                <th>Actie</th>
                <th>Aantal</th>
                <th>Prijs</th>
                <th>Kosten</th>
              </tr>
            </thead>
            <tbody>
              {fills.slice(0, 100).map((f) => (
                <tr key={f.id}>
                  <td>{time(f.ts)}</td>
                  <td>{f.side === "BUY" ? "Koop" : "Verkoop"}</td>
                  <td>{f.quantity}</td>
                  <td>{money(f.price)}</td>
                  <td>{money(f.fee)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {!fills.length && (
            <p>
              Nog geen virtuele uitvoeringen. Een signaal is pas een transactie
              nadat een order is goedgekeurd en gevuld.
            </p>
          )}
        </div>
      )}
      {view === "Posities" && (
        <div className="bot-table">
          <table>
            <thead>
              <tr>
                <th>Markt</th>
                <th>Aantal</th>
                <th>Kostprijs totaal</th>
                <th>Stop-loss</th>
                <th>Take-profit</th>
                <th>Bescherming</th>
              </tr>
            </thead>
            <tbody>
              {positions.map((p) => (
                <tr key={p.id}>
                  <td>{p.instrument}</td>
                  <td>{p.quantity}</td>
                  <td>{money(p.cost)}</td>
                  <td>{money(p.stop)}</td>
                  <td>{p.target ? money(p.target) : "Geen"}</td>
                  <td>
                    {orderState[p.protection_state] || p.protection_state}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {!positions.length && <p>Deze bot heeft geen open posities.</p>}
        </div>
      )}
    </section>
  );
}
