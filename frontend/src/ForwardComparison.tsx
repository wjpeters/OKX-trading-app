import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "./types";
import { post } from "./studio-ui";
export default function ForwardComparison() {
  const [withJev, setWith] = useState("");
  const [without, setWithout] = useState("");
  const [result, setResult] = useState<{
    metrics: {
      bot: string;
      name: string;
      net_pnl_eur_before_ai: string;
      fees_eur: string;
      drawdown_eur: string;
      ai_spent_usd: string;
      ai_reserved_usd: string;
      ai_decisions: number;
      fills: number;
    }[];
    limitations: string;
  }>();
  const [message, setMessage] = useState("");
  const q = useQuery({
    queryKey: ["comparison-bots"],
    queryFn: () =>
      api<{
        account: { bots: Record<string, { id: string; name: string }> } | null;
      }>("/api/paper"),
    refetchInterval: 5000,
  });
  const bots = Object.values(q.data?.account?.bots || {}).filter(
    (b) => b.id !== "manual",
  );
  return (
    <div className="studio-card">
      <h3>Forwardvergelijking · met en zonder Jev</h3>
      <p>
        Gebruik twee nieuwe bots met gelijke financiële limieten en dezelfde
        lineaire strategie, waarvan één met één Jev-filter. Koppel beide binnen
        dezelfde minuut voordat ze handelen.
      </p>
      <div className="studio-toolbar">
        <select
          aria-label="Bot met Jev"
          value={withJev}
          onChange={(e) => setWith(e.target.value)}
        >
          <option value="">Met Jev</option>
          {bots.map((b) => (
            <option key={b.id} value={b.id}>
              {b.name}
            </option>
          ))}
        </select>
        <select
          aria-label="Bot zonder Jev"
          value={without}
          onChange={(e) => setWithout(e.target.value)}
        >
          <option value="">Zonder Jev</option>
          {bots.map((b) => (
            <option key={b.id} value={b.id}>
              {b.name}
            </option>
          ))}
        </select>
        <button
          disabled={!withJev || !without || withJev === without}
          onClick={async () => {
            try {
              setResult(
                await post("/api/research/compare", {
                  with_jev: withJev,
                  without_jev: without,
                }),
              );
              setMessage("Gelijke strategie en limieten gecontroleerd.");
            } catch (e) {
              setResult(undefined);
              setMessage(String(e));
            }
          }}
        >
          Vergelijk vastgelegde resultaten
        </button>
      </div>
      <p role="status">{message}</p>
      {result && (
        <>
          <table>
            <thead>
              <tr>
                <th>Bot</th>
                <th>Netto EUR vóór AI</th>
                <th>Fees / drawdown EUR</th>
                <th>AI USD / reserve</th>
                <th>Fills / beslissingen</th>
              </tr>
            </thead>
            <tbody>
              {result.metrics.map((m) => (
                <tr key={m.bot}>
                  <td>{m.name}</td>
                  <td>{Number(m.net_pnl_eur_before_ai).toFixed(2)}</td>
                  <td>
                    {Number(m.fees_eur).toFixed(2)} /{" "}
                    {Number(m.drawdown_eur).toFixed(2)}
                  </td>
                  <td>
                    {m.ai_spent_usd} / {m.ai_reserved_usd}
                  </td>
                  <td>
                    {m.fills} / {m.ai_decisions}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <p>{result.limitations}</p>
        </>
      )}
    </div>
  );
}
