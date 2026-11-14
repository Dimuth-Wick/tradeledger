import { describe, expect, it } from "vitest";
import {
  emptyForm,
  toLocalInputValue,
  toTradeInput,
  type TradeFormState,
} from "./tradeForm";

// Fake values only.
function filled(overrides: Partial<TradeFormState> = {}): TradeFormState {
  return {
    ...emptyForm(new Date(2026, 10, 14, 9, 30)),
    symbol: "btcusdt",
    qty: "0.015",
    avgEntry: "36500.10",
    ...overrides,
  };
}

describe("toLocalInputValue", () => {
  it("formats a local date for datetime-local inputs", () => {
    expect(toLocalInputValue(new Date(2026, 0, 5, 7, 3))).toBe(
      "2026-01-05T07:03",
    );
  });
});

describe("toTradeInput", () => {
  it("reports missing required fields as incomplete", () => {
    const result = toTradeInput(emptyForm());
    expect(result).toMatchObject({ ok: false, incomplete: true });
    if (!result.ok) {
      expect(result.error).toContain("symbol");
      expect(result.error).toContain("quantity");
      expect(result.error).toContain("entry price");
    }
  });

  it("builds an open trade input with empty optionals as null", () => {
    const result = toTradeInput(filled());
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.input).toMatchObject({
      exchange: "binance",
      market: "usdm",
      symbol: "btcusdt", // Rust normalizes case
      direction: "long",
      openTime: new Date(2026, 10, 14, 9, 30).getTime(),
      closeTime: null,
      qty: "0.015",
      avgEntry: "36500.10",
      avgExit: null,
      leverage: null,
      fees: "0",
      funding: "0",
      stopLoss: null,
      takeProfit: null,
    });
  });

  it("keeps decimals as exact strings", () => {
    const result = toTradeInput(
      filled({ avgEntry: "0.00001234", stopLoss: " .5 " }),
    );
    expect(result.ok && result.input.avgEntry).toBe("0.00001234");
    expect(result.ok && result.input.stopLoss).toBe(".5");
  });

  it("parses a closed futures trade with leverage and negative funding", () => {
    const result = toTradeInput(
      filled({
        closeTime: "2026-11-14T11:00",
        avgExit: "37010.55",
        leverage: "10",
        fees: "0.55",
        funding: "-0.01",
      }),
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.input.closeTime).toBe(
      new Date(2026, 10, 14, 11, 0).getTime(),
    );
    expect(result.input.leverage).toBe(10);
    expect(result.input.funding).toBe("-0.01");
  });

  it("ignores leverage and funding on spot", () => {
    const result = toTradeInput(
      filled({ market: "spot", leverage: "10", funding: "1" }),
    );
    expect(result.ok && result.input.leverage).toBeNull();
    expect(result.ok && result.input.funding).toBe("0");
  });

  it.each([
    ["qty", "abc", "Quantity"],
    ["avgEntry", "1,5", "Entry price"],
    ["stopLoss", "-5", "Stop loss"],
    ["fees", "1.2.3", "Fees"],
    ["leverage", "2.5", "Leverage"],
  ] as const)("rejects %s = %j with a %s message", (field, value, label) => {
    const result = toTradeInput(filled({ [field]: value }));
    expect(result).toMatchObject({ ok: false, incomplete: false });
    if (!result.ok) expect(result.error).toContain(label);
  });
});
