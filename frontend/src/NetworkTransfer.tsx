import { useRef } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Download, Upload, FolderOpen } from "lucide-react";
import { api } from "./types";
import { post, download } from "./studio-ui";
import type { NetworkBundle, SavedNetwork, ImportedNetwork } from "./training-types";

type Props = {
  exportPath?: string;
  selection?: ImportedNetwork;
  busy: boolean;
  onSelect: (selection?: ImportedNetwork) => void;
  act: (fn: () => Promise<unknown>, success: string) => Promise<void>;
};
export default function NetworkTransfer({ exportPath, selection, busy, onSelect, act }: Props) {
  const fileInput = useRef<HTMLInputElement>(null);
  const qc = useQueryClient();
  const query = useQuery({ queryKey: ["training-networks"], queryFn: () => api<{ networks: SavedNetwork[] }>("/api/training/networks") });
  async function exportNetwork() {
    const bundle = selection?.bundle ?? await api<NetworkBundle>(exportPath!);
    const a = bundle.artifact;
    download(JSON.stringify(bundle, null, 2), `okx-${a.instrument}-${a.bar}-seed${a.seed}-gen${a.generation}-${bundle.artifact_sha256.slice(0, 8)}.json`);
  }
  async function importFile(file: File) {
    if (file.size > 128_000) throw Error("Netwerkbestand is te groot. Maximaal 128 KB.");
    let bundle: unknown;
    try { bundle = JSON.parse(await file.text()); } catch { throw Error("Dit bestand bevat geen geldige JSON. Kies een netwerkexport uit Training Lab."); }
    const result = await post<{ id: string }>("/api/training/networks/import", { bundle, name: file.name.replace(/\.json$/i, "").slice(0, 100) || "Geïmporteerd netwerk" });
    const saved = await api<NetworkBundle>(`/api/training/networks/${result.id}`);
    onSelect({ id: result.id, name: file.name, bundle: saved });
    await qc.invalidateQueries({ queryKey: ["training-networks"] });
  }
  return <section className="lab-transfer" aria-label="Netwerk bewaren en terugladen">
    <div className="lab-transfer-intro"><span className="lab-eyebrow"><FolderOpen size={13}/> NETWERKARCHIEF</span><strong>Bewaar het brein. Ook tussentijds.</strong><p>{selection ? `${selection.bundle.artifact.instrument} · ${selection.bundle.artifact.bar} · seed ${selection.bundle.artifact.seed} · generatie ${selection.bundle.artifact.generation}. Geïmporteerde onderzoekskopie.` : "Exporteer de kandidaat of het geselecteerde generatiecheckpoint, met alle 340 gewichten en biases, normalisatie en strategieparameters."}</p></div>
    <div className="lab-transfer-controls">
      <label>Bewaard netwerk<select value={selection?.id ?? ""} disabled={busy} onChange={e => { const id = e.target.value; if (!id) {onSelect(undefined);return;} void act(async () => { const item = query.data?.networks.find(x => x.id === id); const bundle = await api<NetworkBundle>(`/api/training/networks/${id}`);onSelect({ id, name: item?.name ?? "Netwerk", bundle }); }, "Opgeslagen netwerk geladen. Gewichten en instellingen zijn zichtbaar."); }}><option value="">Campagne / huidig netwerk</option>{query.data?.networks.map(n => <option key={n.id} value={n.id}>{n.name} · {n.instrument} · generatie {n.generation}</option>)}</select></label>
      <div className="lab-transfer-buttons"><button disabled={busy || !(selection || exportPath)} onClick={() => void act(exportNetwork, "Netwerkexport gemaakt. Het bestand bevat alle gewichten en instellingen.")}><Download size={14}/>Netwerk exporteren</button><button disabled={busy} onClick={() => fileInput.current?.click()}><Upload size={14}/>Netwerk importeren</button></div>
      <input ref={fileInput} type="file" accept=".json,application/json" hidden aria-label="Netwerkbestand" onChange={e => { const file = e.target.files?.[0];e.target.value = "";if (file) void act(() => importFile(file), "Netwerk geïmporteerd. Alle 340 gewichten en instellingen zijn teruggeladen."); }}/>
      <small>{selection ? "Deze kopie heeft geen geïmporteerde testbewijzen of handelsvrijgave." : exportPath ? "Ook beschikbaar vóór afronding, zodra een generatie is berekend. Een eerdere generatie kies je bij Netwerkcheckpoint." : "Vóór de eerste berekende generatie zijn er nog geen getrainde gewichten om te exporteren."} Het bestand bewaart het netwerk; de volledige trainingsvoortgang blijft in de campagne.</small>
      {query.error && <small role="status">Het netwerkarchief kon niet worden geladen: {String(query.error)}</small>}
    </div>
  </section>;
}
