import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { KeyRound, ShieldCheck, Cpu, PlugZap } from "lucide-react";
import { api } from "./types";
import { post } from "./studio-ui";
import "./system.css";

type Provider = "openrouter" | "typesafe";
type Target = "okx_demo" | "okx_live" | "typesafe_key" | "openrouter_key";
type Settings = {
  provider: Provider;
  model: string;
  storage_ready: boolean;
  credentials: Record<Target, { configured: boolean }>;
  enabled: boolean;
  budget_usd: string;
  spent_usd: string;
  reserved_usd: string;
};
const providerName = (p: Provider) =>
  p === "typesafe" ? "Jev AI direct" : "Jev AI via OpenRouter";

function CredentialForm({
  target,
  title,
  configured,
  ready,
  changed,
}: {
  target: Target;
  title: string;
  configured: boolean;
  ready: boolean;
  changed: () => Promise<void>;
}) {
  const [apiKey, setApiKey] = useState("");
  const [secret, setSecret] = useState("");
  const [passphrase, setPassphrase] = useState("");
  const [busy, setBusy] = useState(false);
  const [remove, setRemove] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const okx = target.startsWith("okx_");
  async function save(deleting = false) {
    setBusy(true);
    setError("");
    setMessage("");
    try {
      await post(
        "/api/settings/credentials",
        deleting
          ? { target, remove: true }
          : {
              target,
              api_key: apiKey,
              ...(okx ? { secret, passphrase } : {}),
            },
      );
      setApiKey("");
      setSecret("");
      setPassphrase("");
      setRemove(false);
      setMessage(
        deleting
          ? "Sleutel verwijderd."
          : "Versleuteld opgeslagen. De invoervelden zijn gewist.",
      );
      await changed();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Opslaan mislukt.");
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="credential-form">
      <div className="connection-status">
        <span className={`connection-dot ${configured ? "stored" : ""}`} />
        {configured
          ? "Sleutel opgeslagen · verbinding nog niet geverifieerd"
          : "Nog geen sleutel opgeslagen"}
      </div>
      <fieldset disabled={busy || !ready}>
        <label>
          {title} API-key
          <input
            type="password"
            autoComplete="new-password"
            spellCheck={false}
            maxLength={2048}
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder={
              configured ? "Nieuwe key om te vervangen" : "Plak je API-key"
            }
          />
        </label>
        {okx && (
          <>
            <label>
              API secret
              <input
                type="password"
                autoComplete="new-password"
                spellCheck={false}
                maxLength={2048}
                value={secret}
                onChange={(e) => setSecret(e.target.value)}
                placeholder="Bijbehorende secret"
              />
            </label>
            <label>
              Passphrase
              <input
                type="password"
                autoComplete="new-password"
                spellCheck={false}
                maxLength={2048}
                value={passphrase}
                onChange={(e) => setPassphrase(e.target.value)}
                placeholder="Passphrase van deze key"
              />
            </label>
          </>
        )}
        <div className="system-actions">
          <button
            className="primary"
            disabled={
              !apiKey.trim() || (okx && (!secret.trim() || !passphrase.trim()))
            }
            onClick={() => void save()}
          >
            {busy
              ? "Bezig…"
              : configured
                ? "Sleutel vervangen"
                : "Sleutel opslaan"}
          </button>
          {configured && (
            <button onClick={() => setRemove(!remove)}>Verwijderen</button>
          )}
        </div>
        {remove && (
          <div className="remove-confirm">
            <p>De opgeslagen sleutel voor {title} verwijderen?</p>
            <button onClick={() => void save(true)}>
              Ja, verwijder sleutel
            </button>{" "}
            <button onClick={() => setRemove(false)}>Annuleren</button>
          </div>
        )}
      </fieldset>
      {message && (
        <p className="system-success" role="status">
          {message}
        </p>
      )}
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}

export default function SystemSettings() {
  const qc = useQueryClient();
  const q = useQuery({
    queryKey: ["system-settings"],
    queryFn: () => api<Settings>("/api/settings"),
    refetchInterval: 15000,
  });
  const d = q.data;
  const [environment, setEnvironment] = useState<"okx_demo" | "okx_live">(
    "okx_demo",
  );
  const [selection, setSelection] = useState<Provider | null>(null);
  const [budget, setBudget] = useState<string | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  async function refresh() {
    await Promise.all(
      ["system-settings", "research", "operations", "status"].map((key) =>
        qc.invalidateQueries({ queryKey: [key] }),
      ),
    );
  }
  async function mutate(path: string, body: unknown, success: string) {
    setBusy(true);
    setMessage("");
    setError("");
    try {
      await post(path, body);
      setMessage(success);
      setConfirmed(false);
      setSelection(null);
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Opslaan mislukt.");
    } finally {
      setBusy(false);
    }
  }
  const chosen = selection ?? d?.provider ?? "openrouter";
  const keyReady =
    d?.credentials[
      d.provider === "typesafe" ? "typesafe_key" : "openrouter_key"
    ].configured;
  return (
    <div className="system-settings">
      {q.isPending && <p role="status">Verbindingsinstellingen laden…</p>}
      {q.error && (
        <p className="error" role="alert">
          {q.error.message}
        </p>
      )}
      {d && !d.storage_ready && (
        <p className="warning">
          Sleutelbeheer is nog niet beschikbaar. Start de app opnieuw via Start
          OKX Terminal.command.
        </p>
      )}
      <div className="system-summary">
        <div>
          <PlugZap size={18} />
          <span>
            Handelsomgeving<strong>Lokale paperhandel</strong>
          </span>
        </div>
        <div>
          <Cpu size={18} />
          <span>
            Gekozen AI<strong>{d ? providerName(d.provider) : "Laden…"}</strong>
          </span>
        </div>
        <div>
          <ShieldCheck size={18} />
          <span>
            Jev-status
            <strong>
              {d
                ? d.enabled
                  ? "Voorstellen ingeschakeld"
                  : "Uitgeschakeld"
                : "Laden…"}
            </strong>
          </span>
        </div>
      </div>
      <div className="system-connections">
        <section className="studio-card connection-card">
          <div className="system-card-heading">
            <KeyRound size={19} />
            <h3>OKX-aansluiting</h3>
          </div>
          <p>
            Bewaar de drie gegevens van dezelfde OKX API-key. Demo en Live
            hebben ieder hun eigen sleutelset.
          </p>
          <div
            className="system-segment"
            role="group"
            aria-label="OKX-sleutelomgeving"
          >
            <button
              aria-pressed={environment === "okx_demo"}
              onClick={() => setEnvironment("okx_demo")}
            >
              Demo
            </button>
            <button
              aria-pressed={environment === "okx_live"}
              onClick={() => setEnvironment("okx_live")}
            >
              Live
            </button>
          </div>
          <CredentialForm
            key={environment}
            target={environment}
            title={environment === "okx_demo" ? "OKX Demo" : "OKX Live"}
            configured={d?.credentials[environment].configured ?? false}
            ready={!!d?.storage_ready}
            changed={refresh}
          />
          <p className="system-footnote">
            {environment === "okx_demo"
              ? "De Demo-aansluiting haalt virtuele saldi bij OKX op. Bevestig je account en handelslimieten in de Terminal voor orders."
              : "De Live-aansluiting haalt je echte saldi op. Bevestig daarna in de Terminal je account, markten en handelslimieten en kies Live vrijgeven."}
          </p>
        </section>
        <section className="studio-card connection-card">
          <div className="system-card-heading">
            <Cpu size={19} />
            <h3>Jev AI direct</h3>
          </div>
          <p>Rechtstreeks via TypeSafe, met je eigen TypeSafe API-key.</p>
          <div className="system-model">Jev 1.13 · jev-1.13.0</div>
          <CredentialForm
            target="typesafe_key"
            title="TypeSafe"
            configured={d?.credentials.typesafe_key.configured ?? false}
            ready={!!d?.storage_ready}
            changed={refresh}
          />
          <p className="system-footnote">
            Kosten worden berekend uit het ontvangen tokengebruik en het
            vastgelegde openbare tarief. De providerfactuur blijft leidend.
          </p>
          <a
            href="https://console.typesafe.ai"
            target="_blank"
            rel="noreferrer"
          >
            TypeSafe-console openen ↗
          </a>
        </section>
        <section className="studio-card connection-card">
          <div className="system-card-heading">
            <Cpu size={19} />
            <h3>Jev AI via OpenRouter</h3>
          </div>
          <p>
            Gebruik Jev via je OpenRouter-account, met een aparte OpenRouter
            API-key.
          </p>
          <div className="system-model">Jev 1.13 · typesafe/jev-1.13</div>
          <CredentialForm
            target="openrouter_key"
            title="OpenRouter"
            configured={d?.credentials.openrouter_key.configured ?? false}
            ready={!!d?.storage_ready}
            changed={refresh}
          />
          <p className="system-footnote">
            Kosten komen uit de OpenRouter-respons. Jev gebruikt de Decisions
            API.
          </p>
          <a
            href="https://openrouter.ai/settings/keys"
            target="_blank"
            rel="noreferrer"
          >
            OpenRouter-sleutels openen ↗
          </a>
        </section>
      </div>
      <section className="studio-card system-ai">
        <div>
          <h3>Welke Jev moet worden gebruikt?</h3>
          <p>
            Deze keuze geldt voor handmatige voorstellen en Jev-nodes in bots.
            Er wordt alleen gebruikgemaakt van de gekozen provider.
          </p>
          <fieldset disabled={!d || busy}>
            <label>
              Jev-provider
              <select
                value={chosen}
                onChange={(e) => {
                  setSelection(e.target.value as Provider);
                  setConfirmed(false);
                }}
              >
                <option value="typesafe">Jev AI direct · TypeSafe</option>
                <option value="openrouter">Jev AI via OpenRouter</option>
              </select>
            </label>
            <button
              disabled={chosen === d?.provider}
              onClick={() =>
                void mutate(
                  "/api/settings/provider",
                  { provider: chosen },
                  "Provider opgeslagen. Activeer het kostenbudget voor deze provider wanneer je Jev wilt gebruiken.",
                )
              }
            >
              Provider opslaan
            </button>
          </fieldset>
          <p className="system-footnote">
            Een providerwissel of wijziging van een AI-sleutel schakelt Jev uit
            totdat je het kostenbudget opnieuw bevestigt.
          </p>
        </div>
        <div>
          <h3>AI-kostenbudget</h3>
          <div className="metric-line">
            <span>Gebruikt / totaalbudget</span>
            <strong>
              $ {d?.spent_usd ?? "?"} / $ {d?.budget_usd ?? "?"}
            </strong>
          </div>
          <div className="metric-line">
            <span>Gereserveerd / nog onbekend</span>
            <strong>$ {d?.reserved_usd ?? "?"}</strong>
          </div>
          <fieldset disabled={!d || busy}>
            <label>
              Totaalbudget in USD
              <input
                type="number"
                min="0"
                max="100"
                step="0.01"
                value={budget ?? d?.budget_usd ?? ""}
                onChange={(e) => setBudget(e.target.value)}
              />
            </label>
            <label className="system-checkbox">
              <input
                type="checkbox"
                checked={confirmed}
                onChange={(e) => setConfirmed(e.target.checked)}
              />
              Ik bevestig externe AI-kosten voor{" "}
              {d ? providerName(d.provider) : "de gekozen provider"}.
            </label>
            <div className="system-actions">
              <button
                className="primary"
                disabled={
                  !confirmed ||
                  !keyReady ||
                  chosen !== d?.provider ||
                  !Number.isFinite(Number(budget ?? d?.budget_usd)) ||
                  Number(budget ?? d?.budget_usd) <= 0
                }
                onClick={() =>
                  void mutate(
                    "/api/jev/config",
                    {
                      budget_usd: budget ?? d?.budget_usd,
                      enabled: true,
                      confirmed,
                    },
                    "Jev ingeschakeld binnen het bevestigde kostenbudget.",
                  )
                }
              >
                Activeer Jev
              </button>
              <button
                disabled={!d?.enabled}
                onClick={() =>
                  void mutate(
                    "/api/jev/config",
                    {
                      budget_usd: d?.budget_usd,
                      enabled: false,
                      confirmed: true,
                    },
                    "Jev uitgeschakeld.",
                  )
                }
              >
                Stop Jev
              </button>
            </div>
          </fieldset>
          {!keyReady && (
            <p className="system-footnote">
              Sla eerst de sleutel van de gekozen provider op.
            </p>
          )}
          <p className="system-footnote">
            Gemaakte en gereserveerde kosten blijven meetellen bij een
            providerwissel. Jev geeft alleen voorstellen.
          </p>
        </div>
        {message && (
          <p className="system-success" role="status">
            {message}
          </p>
        )}
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
      </section>
      <p className="system-storage">
        <ShieldCheck size={16} /> Sleutels worden lokaal versleuteld opgeslagen,
        buiten iCloud. Opgeslagen sleutels worden nooit teruggestuurd naar de
        browser. Lege invoervelden wijzigen niets.
      </p>
    </div>
  );
}
