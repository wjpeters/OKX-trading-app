import { useQuery } from "@tanstack/react-query";
import { api } from "./types";
type Ops = {
  database_bytes: number;
  archive_disk_used_percent: number | null;
  storage_alert: boolean;
  unknown_orders: number;
  unprotected_or_dust_positions: number;
  authentication: {
    configured: boolean;
    binding: string;
    session_hours: number;
  };
  gates: { name: string; state: string }[];
};
export default function Operations() {
  const q = useQuery({
    queryKey: ["operations"],
    queryFn: () => api<Ops>("/api/operations"),
    refetchInterval: 10000,
  });
  const d = q.data;
  return (
    <section className="studio-page" style={{ height: "auto" }}>
      <div className="studio-title">
        <div>
          <h2>Opslag, toegang en vrijgave</h2>
          <p>
            Controleer de lokale opslag, toegang en gereedheid voor uitvoering.
          </p>
        </div>
        <span className="pill">LOKAAL</span>
      </div>
      {q.error && <p className="error">{q.error.message}</p>}
      <div className="research-grid">
        <div className="studio-card">
          <h3>Opslag en bescherming</h3>
          <div className="metric-line">
            <span>Database</span>
            <strong>
              {d ? (d.database_bytes / 1048576).toFixed(1) : "?"} MB
            </strong>
          </div>
          <div className="metric-line">
            <span>Archiefschijf gebruikt</span>
            <strong>{d?.archive_disk_used_percent ?? "onbekend"}%</strong>
          </div>
          <div className="metric-line">
            <span>Onbekende orders</span>
            <strong>{d?.unknown_orders ?? "?"}</strong>
          </div>
          <div className="metric-line">
            <span>Bescherming / dust vraagt aandacht</span>
            <strong>{d?.unprotected_or_dust_positions ?? "?"}</strong>
          </div>
          <p>
            {d?.storage_alert
              ? "Opslagstatus vraagt controle."
              : "Opslag onder waarschuwingsdrempel."}{" "}
            Bij 95% bezetting of een onbekende opslagmeting pauzeren nieuwe
            entries. Stops blijven actief.
          </p>
          <p>
            Backups worden lokaal versleuteld met AES-256-GCM. De sleutel blijft
            buiten de database. Gebruik de meegeleverde herstelcontrole voordat
            je een backup nodig hebt.
          </p>
        </div>
        <div className="studio-card">
          <h3>Toegang</h3>
          <p>
            {d?.authentication.configured
              ? "Wachtwoordbeveiliging actief, HttpOnly-sessie verloopt na 8 uur."
              : "Alleen localhost. Stel een wachtwoord in via Beveilig terminal.command in de projectmap."}
          </p>
          <p>
            Geen publieke poort, clouddeployment of Docker-socket in de app.
            Voor toegang vanaf andere apparaten is nog een aparte beveiligde
            inrichting en acceptatie vereist.
          </p>
          {d?.authentication.configured && (
            <button
              onClick={async () => {
                await fetch("/api/auth/logout", {
                  method: "POST",
                  headers: { "X-Terminal-Request": "paper-v2" },
                });
                location.reload();
              }}
            >
              Afmelden
            </button>
          )}
          <h3>Omgevingen</h3>
          <p>
            PAPER_LOCAL gebruikt eigen virtuele opslag. Demo-onboarding draait
            als afzonderlijk proces met eigen credentials. Een paperbot kan niet
            met een toggle live worden.
          </p>
        </div>
      </div>
      <div className="studio-card">
        <h3>Vrijgavevoorwaarden</h3>
        {d?.gates.map((g) => (
          <div className="metric-line" key={g.name}>
            <span>{g.name}</span>
            <span className="provider-state">{g.state}</span>
          </div>
        ))}
      </div>
    </section>
  );
}
