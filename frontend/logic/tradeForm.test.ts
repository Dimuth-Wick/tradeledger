import { describe, expect, it } from "vitest";
import {
  emptyForm,
  hasInput,
  toLocalInputValue,
  toPlanInput,
  toTradeInput,
  withEntryPrice,
  type TradeFormState,
} from "./tradeForm";

// Fake values only.
function filled(overrides: Partial<TradeFormState> = {}): TradeFormState {
  return {
    ...emptyForm(new Date(2026, 10, 14, 9, 30)),
    symbol: "btcusdt",
    positionSize: "500",
    avgEntry: "36500.10",
    ...overrides,
  };
}

describe("hasInput", () => {
  it("ignores the prefilled boxes", () => {
    const form = emptyForm();
    expect(hasInput(form)).toBe(false);
    expect(hasInput({ ...form, market: "spot", direction: "short" })).toBe(
      false,
    );
  });

  it("counts anything the user typed", () => {
    expect(hasInput({ ...emptyForm(), symbol: "btc" })).toBe(true);
    expect(hasInput({ ...emptyForm(), notes: "idea" })).toBe(true);
    expect(hasInput({ ...emptyForm(), positionSize: "  " })).toBe(false);
  });
});

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
      expect(result.error).toContain("position size");
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
      positionSize: "500",
      avgEntry: "36500.10",
      avgExit: null,
      leverage: null,
      fees: "0",
      autoFees: true, // always 0.1% of size, worked out in Rust
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

  it("parses a closed futures trade with leverage", () => {
    const result = toTradeInput(
      filled({
        closeTime: "2026-11-14T11:00",
        avgExit: "37010.55",
        leverage: "10",
      }),
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.input.closeTime).toBe(
      new Date(2026, 10, 14, 11, 0).getTime(),
    );
    expect(result.input.leverage).toBe(10);
    expect(result.input).toMatchObject({
      exchange: "binance",
      autoFees: true,
      funding: "0",
    });
  });

  it("ignores leverage on spot", () => {
    const result = toTradeInput(filled({ market: "spot", leverage: "10" }));
    expect(result.ok && result.input.leverage).toBeNull();
  });

  it.each([
    ["positionSize", "abc", "Position size"],
    ["avgEntry", "1,5", "Entry price"],
    ["stopLoss", "-5", "Stop loss"],
    ["leverage", "2.5", "Leverage"],
  ] as const)("rejects %s = %j with a %s message", (field, value, label) => {
    const result = toTradeInput(filled({ [field]: value }));
    expect(result).toMatchObject({ ok: false, incomplete: false });
    if (!result.ok) expect(result.error).toContain(label);
  });
});

describe("withEntryPrice", () => {
  it("fills the planned entry while it follows the entry", () => {
    let form = withEntryPrice(emptyForm(), "100");
    expect(form.plannedEntry).toBe("100");
    form = withEntryPrice(form, "101.5");
    expect(form).toMatchObject({ avgEntry: "101.5", plannedEntry: "101.5" });
  });

  it("keeps a planned entry the user typed", () => {
    const form = withEntryPrice(
      { ...emptyForm(), avgEntry: "100", plannedEntry: "99" },
      "102",
    );
    expect(form).toMatchObject({ avgEntry: "102", plannedEntry: "99" });
  });

  it("starts following again after the planned entry is cleared", () => {
    const form = withEntryPrice(
      { ...emptyForm(), avgEntry: "100", plannedEntry: "" },
      "103",
    );
    expect(form.plannedEntry).toBe("103");
  });
});

describe("toPlanInput", () => {
  const base = { ...emptyForm(), avgEntry: "100", plannedEntry: "100" };

  it("needs an entry and a stop loss or take profit", () => {
    expect(toPlanInput(emptyForm())).toBeNull();
    expect(toPlanInput(base)).toBeNull();
    expect(
      toPlanInput({ ...base, avgEntry: "", plannedEntry: "", stopLoss: "95" }),
    ).toBeNull();
  });

  it("sends the plan with the position size when given", () => {
    expect(
      toPlanInput({
        ...base,
        direction: "short",
        stopLoss: "105",
        takeProfit: "90",
        positionSize: "500",
      }),
    ).toEqual({
      direction: "short",
      entry: "100",
      stopLoss: "105",
      takeProfit: "90",
      positionSize: "500",
    });
    expect(toPlanInput({ ...base, takeProfit: "110" })).toMatchObject({
      stopLoss: null,
      takeProfit: "110",
      positionSize: null,
    });
  });

  it("uses the entry price when the planned entry is blank", () => {
    expect(
      toPlanInput({ ...base, plannedEntry: "", stopLoss: "95" })?.entry,
    ).toBe("100");
  });

  it("waits while a box isn't a number", () => {
    expect(toPlanInput({ ...base, stopLoss: "9,5" })).toBeNull();
    expect(
      toPlanInput({ ...base, stopLoss: "95", positionSize: "x" }),
    ).toBeNull();
    expect(
      toPlanInput({ ...base, plannedEntry: "abc", stopLoss: "95" }),
    ).toBeNull();
  });
});
