import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "./types";
import { post } from "./studio-ui";
import type { Doc } from "./studio-ui";
import type { PaperState } from "./Paper";
import type { Graph } from "./bot-model";
import { navigate } from "./bot-model";
import BotActivity, { decision, money } from "./BotActivity";
export default function BotOverview({
  onOpen,
  compact = false,
}: {
  onOpen?: (d: Doc) => void;
  compact?: boolean;
}) {
  const qc = useQueryClient();
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState("");
  const [selected, setSelected] = useState("");
  const state = useQuery({
    queryKey: ["paper"],
    queryFn: () => api<PaperState>("/api/paper"),
    refetchInterval: 3000,
  });
  const docs = useQuery({
    queryKey: ["documents", "graph"],
    queryFn: () => api<Doc[]>("/api/studio/documents?kind=graph"),
  });
  const latest = (docs.data || []).filter(
    (d, i, all) => all.findIndex((x) => x.name === d.name) === i,
  );
  const bots = Object.values(state.data?.account?.bots || {}).filter(
    (b) => b.id !== "manual",
  );
  const open = (d: Doc) => {
    if (onOpen) onOpen(d);
    else {
      sessionStorage.setItem("okx-open-bot", JSON.stringify(d));
      navigate("Bot Studio");
    }
  };
  async function control(id: string, running: boolean) {
    setBusy(id);
    try {
      await post("/api/paper/bots/control", { bot_id: id, running: !running });
      setSelected(id);
      await qc.invalidateQueries({ queryKey: ["paper"] });
      await qc.invalidateQueries({ queryKey: ["bot-activity", id] });
      setMessage(
        running
          ? "Bot gepauzeerd; bescherming blijft actief."
          : "Bot gestart op live data met virtueel geld. De volgende gesloten candle verschijnt bij Activiteit.",
      );
    } catch (e) {
      setMessage(String(e));
    } finally {
      setBusy("");
    }
  }
  const startButton = (b: (typeof bots)[number]) => (
    <button
      disabled={
        !!busy || state.isError || (!b.running && !state.data?.worker_alive)
      }
      onClick={() => control(b.id, b.running)}
    >
      {busy === b.id
        ? "Even wachten…"
        : b.running
          ? "Pauzeer"
          : "Start op live data"}
    </button>
  );
  return (
    <div className={compact ? "terminal-bots" : "bot-library"}>
      <div className="studio-toolbar">
        <strong>
          Mijn bots · {latest.length} ontwerpen ·{" "}
          {bots.filter((b) => b.running).length} actief
        </strong>
        {!onOpen && (
          <button onClick={() => navigate("Bot Studio")}>
            Open Bot Studio
          </button>
        )}
      </div>
      {(state.error || docs.error) && (
        <p role="alert">{String(state.error || docs.error)}</p>
      )}
      {message && <p role="status">{message}</p>}
      {!latest.length && !bots.length && (
        <p className="muted">
          Nog geen bots. Bouw en bewaar je eerste bot in Bot Studio.
        </p>
      )}
      {latest.map((d) => {
        const g = d.payload as unknown as Graph;
        const b = bots.find((x) => x.id === g.bot?.config.id);
        return (
          <article className="bot-row" key={d.name}>
            <div>
              <strong>{d.name}</strong>
              <small>
                {g.bot?.config.instrument || "Nog instellen"} · Ontwerp{" "}
                {g.bot?.mode === "live" ? "Live" : "Paper"} v{d.version}
              </small>
            </div>
            <span className="pill">
              {b?.running
                ? "Actief"
                : b
                  ? "Gepauzeerd"
                  : g.bot?.mode === "live"
                    ? "Live geblokkeerd"
                    : "Ontwerp"}
            </span>
            <div>
              <small>
                {b
                  ? b.ledger.paused
                    ? b.ledger.reason
                    : decision(b.last_signal)
                  : "Nog niet toegepast"}
              </small>
              {b && (
                <small>
                  Uitvoering: Paper ·{" "}
                  {b.version.startsWith("graph:")
                    ? "v" + b.version.split(":").at(-1)
                    : b.version}{" "}
                  · Vermogen {money(b.ledger.equity)}
                </small>
              )}
            </div>
            <div className="bot-row-actions">
              <button onClick={() => open(d)}>Bewerk ontwerp</button>
              {b && (
                <button
                  onClick={() => setSelected(b.id)}
                  aria-pressed={selected === b.id}
                >
                  Bekijk activiteit
                </button>
              )}
              {b && startButton(b)}
            </div>
          </article>
        );
      })}
      {bots
        .filter(
          (b) =>
            !latest.some(
              (d) => (d.payload as unknown as Graph).bot?.config.id === b.id,
            ),
        )
        .map((b) => (
          <article className="bot-row" key={b.id}>
            <div>
              <strong>{b.name}</strong>
              <small>
                {b.instrument} · Paper · {b.version}
              </small>
            </div>
            <span>{b.running ? "● Actief" : "Gepauzeerd"}</span>
            <small>
              {b.ledger.paused ? b.ledger.reason : decision(b.last_signal)}
            </small>
            <div className="bot-row-actions">
              <button onClick={() => setSelected(b.id)}>
                Bekijk activiteit
              </button>
              {startButton(b)}
            </div>
          </article>
        ))}
      {state.data && bots.find((b) => b.id === (selected || bots[0]?.id)) && (
        <BotActivity
          key={selected || bots[0]?.id}
          bot={bots.find((b) => b.id === (selected || bots[0]?.id))!}
          state={state.data}
        />
      )}
    </div>
  );
}
