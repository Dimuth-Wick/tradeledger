import { describe, expect, it } from "vitest";
import type { LayoutItem, Stats } from "./api";
import {
  EMPTY_DASH,
  changedLayout,
  curveRows,
  dashQuery,
  formatHold,
  kpis,
  presetRange,
  rRows,
} from "./dashboard";

// Same fake sample as the Rust stats tests: closed nets +9, −4, +10, 0, −2.25
// and one open trade.
const stats: Stats = {
  trades: 6,
  open: 1,
  closed: 5,
  wins: 2,
  losses: 2,
  breakeven: 1,
  winRate: "40",
  grossPnl: "17.75",
  fees: "4",
  funding: "1",
  netPnl: "12.75",
  avgWin: "9.5",
  avgLoss: "-3.125",
  largestWin: "10",
  largestLoss: "-4",
  payoffRatio: "3.04",
  profitFactor: "3.04",
  expectancy: "2.55",
  avgR: "0",
  tradesWithR: 4,
  maxDrawdown: "4",
  maxDrawdownStart: 2_000,
  maxDrawdownEnd: 3_000,
  maxWinStreak: 1,
  maxLossStreak: 1,
  avgHoldMs: 1000,
  equityCurve: [
    { time: 2_000, equity: "9", pnl: "9", tradeId: 1 },
    { time: 3_000, equity: "5", pnl: "-4", tradeId: 2 },
  ],
  rBuckets: [
    { label: "< -2R", from: null, to: -2, count: 0 },
    { label: "-1R to 0R", from: -1, to: 0, count: 2 },
    { label: "0R to 1R", from: 0, to: 1, count: 1 },
    { label: "≥ 3R", from: 3, to: null, count: 1 },
  ],
};

const byLabel = (s: Stats) =>
  Object.fromEntries(kpis(s).map((k) => [k.label, k]));

describe("kpis", () => {
  it("formats the tiles from Rust's numbers", () => {
    const k = byLabel(stats);
    expect(k["Net PnL"]).toMatchObject({ value: "+12.75", tone: "pos" });
    expect(k["Net PnL"].detail).toBe("gross +17.75 · fees 4 · funding 1");
    expect(k["Win rate"]).toMatchObject({
      value: "40%",
      detail: "2W · 2L · 1BE",
    });
    expect(k["Profit factor"].value).toBe("3.04");
    expect(k["Average R"]).toMatchObject({
      value: "0R",
      detail: "4 of 5 trades have an R",
    });
    // Tiles show 2 decimals; the exact value is the hover title.
    expect(k["Avg win / loss"]).toMatchObject({
      value: "+9.5 / -3.13",
      title: "+9.5 / -3.125",
    });
    expect(k["Max drawdown"]).toMatchObject({ value: "-4", tone: "neg" });
    expect(k["Max drawdown"].detail).toContain("→");
    expect(k["Trades"].detail).toBe("5 closed · 1 open");
  });

  it("shows ∞ profit factor with wins and no losses, — with nothing", () => {
    const noLoss = { ...stats, losses: 0, profitFactor: null };
    expect(byLabel(noLoss)["Profit factor"].value).toBe("∞");
    const empty = { ...noLoss, wins: 0, winRate: null, maxDrawdown: "0" };
    const k = byLabel(empty);
    expect(k["Profit factor"].value).toBe("—");
    expect(k["Win rate"].value).toBe("—");
    expect(k["Max drawdown"]).toMatchObject({ value: "0", tone: "" });
  });
});

describe("formatHold", () => {
  it.each([
    [null, "—"],
    [30_000, "30s"],
    [45 * 60_000, "45m"],
    [(3 * 60 + 12) * 60_000, "3h 12m"],
    [(2 * 24 + 4) * 3_600_000, "2d 4h"],
  ])("%s ms → %s", (ms, text) => {
    expect(formatHold(ms)).toBe(text);
  });
});

describe("presetRange", () => {
  const today = new Date(2026, 2, 5); // 5 Mar 2026, local

  it.each([
    ["all", "", ""],
    ["7d", "2026-02-27", "2026-03-05"], // crosses the month end
    ["30d", "2026-02-04", "2026-03-05"],
    ["ytd", "2026-01-01", "2026-03-05"],
  ] as const)("%s → %s .. %s", (preset, from, to) => {
    expect(presetRange(preset, today)).toEqual({ from, to });
  });
});

describe("chart rows", () => {
  it("equity curve starts at 0 and keeps exact text", () => {
    const rows = curveRows(stats);
    expect(rows.map((r) => r.equity)).toEqual([0, 9, 5]);
    expect(rows[2]).toMatchObject({ n: 2, pnl: -4, pnlText: "-4", tradeId: 2 });
    expect(curveRows({ ...stats, equityCurve: [] })).toEqual([]);
  });

  it("R buckets below 0R are losses", () => {
    expect(rRows(stats.rBuckets).map((r) => r.tone)).toEqual([
      "neg",
      "neg",
      "pos",
      "pos",
    ]);
  });
});

describe("dashQuery", () => {
  it("uses the trade log's date translation", () => {
    const q = dashQuery({ ...EMPTY_DASH, from: "2026-03-01", market: "usdm" });
    expect(q.from).toBe(new Date(2026, 2, 1).getTime());
    expect(q.to).toBeNull();
    expect(q.market).toBe("usdm");
    expect(q.outcome).toBeNull();
  });
});

describe("changedLayout", () => {
  const current: LayoutItem[] = [
    { i: "kpis", x: 0, y: 0, w: 12, h: 6, minW: 3, minH: 3 },
    { i: "equity", x: 0, y: 6, w: 12, h: 7, minW: 3, minH: 4 },
  ];

  it("returns null when nothing moved (grid may add extra fields)", () => {
    const grid = current.map((c) => ({ ...c, moved: false, static: false }));
    expect(changedLayout(current, grid)).toBeNull();
  });

  it("copies new positions and keeps Rust's minimum sizes and order", () => {
    const grid = [
      { i: "equity", x: 0, y: 0, w: 8, h: 7 },
      { i: "kpis", x: 8, y: 0, w: 4, h: 10 },
    ];
    expect(changedLayout(current, grid)).toEqual([
      { i: "kpis", x: 8, y: 0, w: 4, h: 10, minW: 3, minH: 3 },
      { i: "equity", x: 0, y: 0, w: 8, h: 7, minW: 3, minH: 4 },
    ]);
  });
});
