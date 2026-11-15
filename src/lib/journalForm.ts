import type { JournalInput, Trade } from "./api";

// Journal form boxes ↔ JournalInput. Format checks only; the plan rules
// (stop on the right side, etc.) are checked in Rust.

export interface JournalForm {
  plannedEntry: string;
  stopLoss: string;
  takeProfit: string;
  leverage: string;
  notes: string;
}

export function formFromTrade(t: Trade): JournalForm {
  return {
    plannedEntry: t.plannedEntry ?? "",
    stopLoss: t.stopLoss ?? "",
    takeProfit: t.takeProfit ?? "",
    leverage: t.leverage === null ? "" : String(t.leverage),
    notes: t.notes,
  };
}

const UNSIGNED = /^(\d+\.?\d*|\.\d+)$/;

export type JournalResult =
  { ok: true; input: JournalInput } | { ok: false; error: string };

export function toJournalInput(f: JournalForm): JournalResult {
  const dec = (label: string, raw: string): string | null => {
    const v = raw.trim();
    if (v === "") return null;
    if (!UNSIGNED.test(v)) throw new Error(`${label}: enter a number like 0.5`);
    return v;
  };
  try {
    const lev = f.leverage.trim();
    if (lev !== "" && !/^\d+$/.test(lev)) {
      throw new Error("Leverage: enter a whole number like 10");
    }
    return {
      ok: true,
      input: {
        plannedEntry: dec("Planned entry", f.plannedEntry),
        stopLoss: dec("Stop loss", f.stopLoss),
        takeProfit: dec("Take profit", f.takeProfit),
        leverage: lev === "" ? null : Number(lev),
        notes: f.notes,
      },
    };
  } catch (err) {
    return { ok: false, error: (err as Error).message };
  }
}

/** True if the form differs from what's saved (enables the Save button). */
export function isDirty(f: JournalForm, t: Trade): boolean {
  const saved = formFromTrade(t);
  return (Object.keys(saved) as (keyof JournalForm)[]).some(
    (k) => f[k].trim() !== saved[k].trim(),
  );
}
