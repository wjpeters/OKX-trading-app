export async function post<T = Record<string, unknown>>(
  path: string,
  body: unknown = {},
): Promise<T> {
  const r = await fetch(path, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Terminal-Request": "paper-v2",
    },
    body: JSON.stringify(body),
  });
  const data = await r.json();
  if (!r.ok) throw Error(data.error || "Opdracht afgewezen");
  return data;
}

export type Doc = {
  name: string;
  version: number;
  payload: Record<string, unknown>;
  created_ms: number;
};

export function Spark({ values }: { values: (number | null)[] }) {
  const finite = values.filter(
    (v): v is number => v !== null && Number.isFinite(v),
  );
  if (!finite.length) return <span>Geen reeks</span>;
  const min = Math.min(...finite),
    max = Math.max(...finite);
  const points = values
    .flatMap((v, i) =>
      v === null
        ? []
        : [
            `${(i / Math.max(1, values.length - 1)) * 400},${70 - ((v - min) / (max - min || 1)) * 65}`,
          ],
    )
    .join(" ");
  return (
    <svg viewBox="0 0 400 75" className="spark" aria-label="Berekende reeks">
      <polyline
        fill="none"
        stroke="currentColor"
        strokeWidth="1.4"
        points={points}
      />
    </svg>
  );
}
export function download(
  body: string,
  name: string,
  type = "application/json",
) {
  const u = URL.createObjectURL(new Blob([body], { type }));
  const a = document.createElement("a");
  a.href = u;
  a.download = name;
  a.click();
  URL.revokeObjectURL(u);
}
