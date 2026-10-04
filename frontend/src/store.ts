import { create } from "zustand";
import { persist } from "zustand/middleware";
interface Preferences {
  symbol: string;
  bar: string;
  chartType: string;
  overlay: string;
  oscillator: string;
  light: boolean;
  timezone: string;
  set: (next: Partial<Omit<Preferences, "set">>) => void;
}
export const usePreferences = create<Preferences>()(
  persist(
    (set) => ({
      symbol: "BTC-EUR",
      bar: "15m",
      chartType: "Candles",
      overlay: "ema",
      oscillator: "rsi",
      light: false,
      timezone: "UTC",
      set: (next) => set(next),
    }),
    { name: "okx-new-terminal-v1" },
  ),
);
