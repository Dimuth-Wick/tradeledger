import { describe, expect, it } from "vitest";
import type { ReportRow } from "../connection/backend";
import { barPercent, maxAbsNet, profitFactorText } from "./reports";

const row = (over: Partial<ReportRow>): ReportRow => ({
  key: "k",
  label: "k",
  trades: 2,
  wins: 1,
  losses: 1,
  winRate: "50",
  netPnl: "6",
  avgPnl: "3",
  profitFactor: "2.5",
  avgR: null,
  tradesWithR: 0,
  ...over,
});

describe("reports helpers", () => {
  it("profit factor text", () => {
    expect(profitFactorText(row({}))).toBe("2.5");
    expect(profitFactorText(row({ profitFactor: null, losses: 0 }))).toBe("∞");
    expect(
      profitFactorText(row({ profitFactor: null, wins: 0, losses: 0 })),
    ).toBe("—");
  });

  it("bars scale to the biggest row, either sign", () => {
    const rows = [row({ netPnl: "6" }), row({ netPnl: "-12" })];
    const max = maxAbsNet(rows);
    expect(max).toBe(12);
    expect(barPercent("6", max)).toBe(50);
    expect(barPercent("-12", max)).toBe(100);
    expect(barPercent("0", 0)).toBe(0);
  });
});
