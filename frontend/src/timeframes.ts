export const timeframes = [
  { value: "1m", label: "1m", name: "1 minuut", group: "Minuten", quick: true },
  {
    value: "3m",
    label: "3m",
    name: "3 minuten",
    group: "Minuten",
    quick: false,
  },
  {
    value: "5m",
    label: "5m",
    name: "5 minuten",
    group: "Minuten",
    quick: true,
  },
  {
    value: "15m",
    label: "15m",
    name: "15 minuten",
    group: "Minuten",
    quick: true,
  },
  {
    value: "30m",
    label: "30m",
    name: "30 minuten",
    group: "Minuten",
    quick: false,
  },
  { value: "1H", label: "1H", name: "1 uur", group: "Uren", quick: true },
  { value: "2H", label: "2H", name: "2 uur", group: "Uren", quick: false },
  { value: "4H", label: "4H", name: "4 uur", group: "Uren", quick: true },
  { value: "6Hutc", label: "6H", name: "6 uur", group: "Uren", quick: false },
  {
    value: "12Hutc",
    label: "12H",
    name: "12 uur",
    group: "Uren",
    quick: false,
  },
  {
    value: "1Dutc",
    label: "1D",
    name: "1 dag",
    group: "Dag en langer",
    quick: true,
  },
  {
    value: "1Wutc",
    label: "1W",
    name: "1 week",
    group: "Dag en langer",
    quick: true,
  },
  {
    value: "1Mutc",
    label: "1M",
    name: "1 maand",
    group: "Dag en langer",
    quick: true,
  },
  {
    value: "3Mutc",
    label: "3M",
    name: "3 maanden (kwartaal)",
    group: "Dag en langer",
    quick: false,
  },
] as const;
export const timeframeGroups = ["Minuten", "Uren", "Dag en langer"] as const;

export const timeframeLabel = (value: string) =>
  timeframes.find((t) => t.value === value)?.label ?? value;
