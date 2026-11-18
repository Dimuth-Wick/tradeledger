import type { Decimal, GroupBy, ReportRow } from "./api";

// Reports display helpers. Rust computes every figure; this file only
// labels them and sizes the bars.

export const GROUPS: [GroupBy, string][] = [
  ["tag", "Tag"],
  ["symbol", "Symbol"],
  ["weekday", "Weekday"],
  ["hour", "Hour"],
  ["market", "Market"],
  ["direction", "Side"],
  ["playbook", "Playbook"],
];

/** "2.5", "∞" (wins, no losses) or "—" (nothing to compare). */
export function profitFactorText(r: ReportRow): string {
  if (r.profitFactor !== null) return r.profitFactor;
  return r.wins > 0 && r.losses === 0 ? "∞" : "—";
}

/** Largest |net PnL| of the rows, for scaling the bars (display only). */
export function maxAbsNet(rows: ReportRow[]): number {
  return rows.reduce((m, r) => Math.max(m, Math.abs(Number(r.netPnl))), 0);
}

/** Bar length as a percent of half the bar track (0–100). */
export function barPercent(net: Decimal, maxAbs: number): number {
  if (maxAbs === 0) return 0;
  return Math.round((Math.abs(Number(net)) / maxAbs) * 100);
}
