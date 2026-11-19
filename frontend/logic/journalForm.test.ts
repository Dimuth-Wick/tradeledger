import { describe, expect, it } from "vitest";
import type { Trade } from "../connection/backend";
import { formFromTrade, isDirty, toJournalInput } from "./journalForm";

// Fake trade, only the fields the journal form reads matter here.
const trade = {
  plannedEntry: null,
  stopLoss: "95",
  takeProfit: null,
  leverage: 10,
  notes: "breakout",
} as unknown as Trade;

describe("journal form", () => {
  it("fills the form from a saved trade", () => {
    expect(formFromTrade(trade)).toEqual({
      plannedEntry: "",
      stopLoss: "95",
      takeProfit: "",
      leverage: "10",
      notes: "breakout",
    });
  });

  it("converts empty boxes to null and keeps decimals as text", () => {
    const r = toJournalInput({
      plannedEntry: "",
      stopLoss: " 0.00001150 ",
      takeProfit: "",
      leverage: "",
      notes: "x",
    });
    expect(r).toEqual({
      ok: true,
      input: {
        plannedEntry: null,
        stopLoss: "0.00001150",
        takeProfit: null,
        leverage: null,
        notes: "x",
      },
    });
  });

  it("rejects bad numbers with the field name", () => {
    const bad = toJournalInput({
      ...formFromTrade(trade),
      takeProfit: "1,5",
    });
    expect(bad).toEqual({
      ok: false,
      error: "Take profit: enter a number like 0.5",
    });
    const lev = toJournalInput({ ...formFromTrade(trade), leverage: "2.5" });
    expect(lev.ok).toBe(false);
  });

  it("knows when the form has unsaved changes", () => {
    const form = formFromTrade(trade);
    expect(isDirty(form, trade)).toBe(false);
    expect(isDirty({ ...form, stopLoss: " 95 " }, trade)).toBe(false);
    expect(isDirty({ ...form, notes: "changed" }, trade)).toBe(true);
  });
});
