import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import Chart from "./Chart";
import type { ChartPricePick } from "./chart-price";
import { timeframeLabel } from "./timeframes";
import { api } from "./types";
import type { Candles } from "./types";
import { usePreferences } from "./store";
import { post } from "./studio-ui";
import type { Doc } from "./studio-ui";
function Extra({
  symbol,
  bar,
  tickSize,
  onPricePick,
}: {
  symbol: string;
  bar: string;
  tickSize: string;
  onPricePick?: (pick: ChartPricePick) => void;
}) {
  const q = useQuery({
    queryKey: ["candles", symbol, bar],
    queryFn: () => api<Candles>(`/api/candles?instrument=${symbol}&bar=${bar}`),
    refetchInterval: 15000,
  });
  return (
    <div>
      {q.data ? (
        <Chart data={q.data} tickSize={tickSize} onPricePick={onPricePick} />
      ) : (
        <p>{q.error?.message || "Chart laden…"}</p>
      )}
      <span className="mini-label">
        {symbol} · {timeframeLabel(bar)}
      </span>
    </div>
  );
}
export default function ChartWorkspace({
  data,
  tickSize,
  onPricePick,
}: {
  data: Candles;
  tickSize: string;
  onPricePick?: (pick: ChartPricePick) => void;
}) {
  const pref = usePreferences();
  const qc = useQueryClient();
  const [count, setCount] = useState(
    +(localStorage.getItem("okx-chart-count") || 1),
  );
  const [name, setName] = useState("Mijn werkruimte");
  const [version, setVersion] = useState(0);
  const [msg, setMsg] = useState("");
  const [inputs, setInputs] = useState(false);
  const [settings, setSettings] = useState({
    ma: 20,
    oscillator: 14,
    macd_fast: 12,
    macd_slow: 26,
    macd_signal: 9,
    bb_multiplier: 2,
  });
  const [activeSettings, setActiveSettings] = useState<typeof settings>();
  const computed = useQuery({
    queryKey: ["custom-indicators", data.instrument, data.bar, activeSettings],
    queryFn: () =>
      post<Candles>("/api/studio/indicators", {
        instrument: data.instrument,
        bar: data.bar,
        settings: activeSettings,
      }),
    enabled: !!activeSettings,
    refetchInterval: 15000,
  });
  const custom = computed.data;
  const q = useQuery({
    queryKey: ["documents", "workspace"],
    queryFn: () => api<Doc[]>("/api/studio/documents?kind=workspace"),
  });
  const pineDocs = useQuery({
    queryKey: ["documents", "pine"],
    queryFn: () => api<Doc[]>("/api/studio/documents?kind=pine"),
  });
  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column" }}>
      <div className="workspace-tools">
        <select
          aria-label="Aantal charts"
          value={count}
          onChange={(e) => {
            setCount(+e.target.value);
            localStorage.setItem("okx-chart-count", e.target.value);
          }}
        >
          {[1, 2, 3, 4].map((n) => (
            <option value={n} key={n}>
              {n} chart{n > 1 ? "s" : ""}
            </option>
          ))}
        </select>
        <input
          aria-label="Werkruimtenaam"
          value={name}
          onChange={(e) => {
            setName(e.target.value);
            setVersion(0);
          }}
        />
        <button
          onClick={async () => {
            try {
              const { set, ...preferences } = pref;
              const r = await post<{ version: number }>(
                "/api/studio/documents",
                {
                  kind: "workspace",
                  name,
                  base_version: version,
                  payload: { preferences, count },
                },
              );
              setVersion(r.version);
              qc.invalidateQueries({ queryKey: ["documents", "workspace"] });
              setMsg("Bewaard");
            } catch (e) {
              setMsg(String(e));
            }
          }}
        >
          Bewaar
        </button>
        <select
          aria-label="Werkruimte laden"
          value=""
          onChange={(e) => {
            const d = q.data?.[+e.target.value];
            if (d) {
              pref.set(d.payload.preferences as Parameters<typeof pref.set>[0]);
              setCount(Number(d.payload.count) || 1);
              setName(d.name);
              setVersion(d.version);
              setMsg("Geladen");
            }
          }}
        >
          <option value="">Werkruimte</option>
          {q.data?.map((d, i) => (
            <option value={i} key={i}>
              {d.name} v{d.version}
            </option>
          ))}
        </select>
        <button onClick={() => setInputs(!inputs)}>Indicatorinputs</button>
        <select
          aria-label="Pine-script op chart"
          value=""
          onChange={(e) => {
            const d =
              e.target.value === ""
                ? undefined
                : pineDocs.data?.[+e.target.value];
            if (d)
              pref.set({
                pineScripts: [
                  ...pref.pineScripts.filter((p) => p.name !== d.name),
                  { name: d.name, version: d.version },
                ],
              });
          }}
        >
          <option value="">+ Mijn Pine-script</option>
          {pineDocs.data?.map((d, i) => (
            <option key={i} value={i}>
              {d.name} · v{d.version}
            </option>
          ))}
        </select>
        <span title={msg}>{msg.slice(0, 45)}</span>
      </div>
      {!!pref.pineScripts.length && (
        <div className="workspace-tools pine-chips">
          {pref.pineScripts.map((p) => (
            <button
              key={p.name}
              title="Verwijder script van de chart"
              onClick={() =>
                pref.set({
                  pineScripts: pref.pineScripts.filter(
                    (x) => x.name !== p.name,
                  ),
                })
              }
            >
              {p.name} v{p.version} ×
            </button>
          ))}
        </div>
      )}
      {inputs && (
        <form
          className="workspace-tools"
          onSubmit={async (e) => {
            e.preventDefault();
            try {
              setActiveSettings({ ...settings });
              setMsg("Inputs actief · elke 15 s vernieuwd");
            } catch (e) {
              setMsg(String(e));
            }
          }}
        >
          {Object.entries(settings).map(([k, v]) => (
            <label key={k}>
              {k}
              <input
                aria-label={k}
                type="number"
                min="0.5"
                max="100"
                step="any"
                value={v}
                onChange={(e) =>
                  setSettings({ ...settings, [k]: +e.target.value })
                }
              />
            </label>
          ))}
          <button>Bereken</button>
          <button type="button" onClick={() => setActiveSettings(undefined)}>
            Standaard
          </button>
        </form>
      )}
      <div
        className={`chart-multi ${count === 1 ? "one" : ""}`}
        style={{ flex: 1, minHeight: 0 }}
      >
        <div>
          <Chart
            data={
              custom &&
              custom.instrument === data.instrument &&
              custom.bar === data.bar
                ? custom
                : data
            }
            tickSize={tickSize}
            onPricePick={onPricePick}
          />
          {count > 1 && (
            <span className="mini-label">
              {data.instrument} · {timeframeLabel(data.bar)}
            </span>
          )}
        </div>
        {["1m", "1H", "4H"].slice(0, count - 1).map((bar) => (
          <Extra
            key={bar}
            symbol={data.instrument}
            bar={bar}
            tickSize={tickSize}
            onPricePick={onPricePick}
          />
        ))}
      </div>
    </div>
  );
}
