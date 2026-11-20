import type {
  Decimal,
  LayoutItem,
  RBucket,
  Slice,
  Stats,
  TradeQuery,
} from "../connection/backend";
import { formatR, pnlClass, usd } from "./format";
import {
  DEFAULT_SORT,
  EMPTY_FILTERS,
  buildQuery,
  type LogFilters,
} from "./tradeLog";

// Dashboard helpers: Rust computes every number; this file only picks
// labels, formats text and shapes rows for the charts.

/** The dashboard filters: a subset of the trade log's. */
export type DashFilters = Pick<
  LogFilters,
  "from" | "to" | "symbol" | "market" | "direction"
>;

export const EMPTY_DASH: DashFilters = {
  from: "",
  to: "",
  symbol: "",
  market: "",
  direction: "",
};

/** Same translation as the trade log, so both show the same trades. */
export function dashQuery(f: DashFilters): TradeQuery {
  return buildQuery({ ...EMPTY_FILTERS, ...f }, DEFAULT_SORT, 0, 1);
}

export type Preset = "all" | "7d" | "30d" | "90d" | "ytd";

export const PRESETS: [Preset, string][] = [
  ["all", "All time"],
  ["7d", "Last 7 days"],
  ["30d", "Last 30 days"],
  ["90d", "Last 90 days"],
  ["ytd", "This year"],
];

/** "YYYY-MM-DD" of a local date (what `<input type="date">` uses). */
export function isoDay(d: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** From/to dates for a preset; "last 7 days" includes today. */
export function presetRange(
  preset: Preset,
  today: Date,
): { from: string; to: string } {
  if (preset === "all") return { from: "", to: "" };
  const to = isoDay(today);
  if (preset === "ytd") {
    return { from: isoDay(new Date(today.getFullYear(), 0, 1)), to };
  }
  const days = { "7d": 7, "30d": 30, "90d": 90 }[preset];
  const from = new Date(today);
  from.setDate(from.getDate() - (days - 1));
  return { from: isoDay(from), to };
}

/** "2d 4h", "3h 12m", "45m", "30s". */
export function formatHold(ms: number | null): string {
  if (ms === null) return "—";
  const s = Math.round(ms / 1000);
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m`;
  return `${s}s`;
}

const plural = (n: number, one: string, many: string) =>
  `${n} ${n === 1 ? one : many}`;

const shortDate = (ms: number) =>
  new Date(ms).toLocaleDateString(undefined, { dateStyle: "medium" });

export interface Kpi {
  label: string;
  value: string;
  /** Exact value on hover, when `value` is rounded. */
  title?: string;
  detail: string;
  /** CSS class: "pos", "neg" or "". */
  tone: string;
}

/** Tiles show 2 decimals; the exact value is in the tile's hover title. */
const money = (d: Decimal | null) =>
  d === null ? "—" : usd(d, { round: true });
const exact = (d: Decimal | null) => (d === null ? "—" : usd(d));
const plain = (d: Decimal) => usd(d, { round: true, sign: false });

/** The KPI tiles, in display order. */
export function kpis(s: Stats): Kpi[] {
  const pf = s.profitFactor ?? (s.wins > 0 && s.losses === 0 ? "∞" : "—");
  const tiles: Kpi[] = [
    {
      label: "Net PnL",
      value: money(s.netPnl),
      title: exact(s.netPnl),
      detail: `gross ${money(s.grossPnl)} · fees ${plain(s.fees)} · funding ${money(s.funding)}`,
      tone: pnlClass(s.netPnl),
    },
    {
      label: "Win rate",
      value: s.winRate === null ? "—" : `${s.winRate}%`,
      detail: `${s.wins}W · ${s.losses}L · ${s.breakeven}BE`,
      tone: "",
    },
    {
      label: "Profit factor",
      value: pf,
      detail: `payoff ${s.payoffRatio ?? "—"}`,
      tone: "",
    },
    {
      label: "Expectancy",
      value: money(s.expectancy),
      title: exact(s.expectancy),
      detail: "per closed trade",
      tone: pnlClass(s.expectancy),
    },
    {
      label: "Average R",
      value: formatR(s.avgR),
      detail: `${s.tradesWithR} of ${plural(s.closed, "trade", "trades")} have an R`,
      tone: pnlClass(s.avgR),
    },
    {
      label: "Avg win / loss",
      value: `${money(s.avgWin)} / ${money(s.avgLoss)}`,
      title: `${exact(s.avgWin)} / ${exact(s.avgLoss)}`,
      detail: `largest ${money(s.largestWin)} / ${money(s.largestLoss)}`,
      tone: "",
    },
    {
      label: "Max drawdown",
      value: money(`-${s.maxDrawdown}`),
      title: exact(`-${s.maxDrawdown}`),
      detail:
        s.maxDrawdownStart !== null && s.maxDrawdownEnd !== null
          ? `${shortDate(s.maxDrawdownStart)} → ${shortDate(s.maxDrawdownEnd)}`
          : "no drop yet",
      tone: s.maxDrawdown === "0" ? "" : "neg",
    },
    {
      label: "Longest streaks",
      value: `${s.maxWinStreak}W / ${s.maxLossStreak}L`,
      detail: "wins / losses in a row",
      tone: "",
    },
    {
      label: "Trades",
      value: String(s.trades),
      detail: `${s.closed} closed · ${s.open} open`,
      tone: "",
    },
    {
      label: "Avg hold time",
      value: formatHold(s.avgHoldMs),
      detail: "open to close",
      tone: "",
    },
  ];
  return tiles;
}

const MONTHS_SHORT = [
  "Jan",
  "Feb",
  "Mar",
  "Apr",
  "May",
  "Jun",
  "Jul",
  "Aug",
  "Sep",
  "Oct",
  "Nov",
  "Dec",
];

/** "2026-09-05" → "Sep 5" (axis label). */
export function shortDay(date: string): string {
  const [, m, d] = date.split("-").map(Number);
  return `${MONTHS_SHORT[m - 1]} ${d}`;
}

/** A day on the equity / win-rate charts. Numbers only position the marks. */
export interface DailyRow {
  date: string;
  equity: number;
  equityText: Decimal;
  pnlText: Decimal;
  trades: number;
  wins: number;
  closed: number;
  winRate: number | null;
  winRateText: Decimal | null;
}

export function dailyRows(s: Stats): DailyRow[] {
  return s.daily.map((d) => ({
    date: d.date,
    equity: Number(d.equity),
    equityText: d.equity,
    pnlText: d.pnl,
    trades: d.trades,
    wins: d.wins,
    closed: d.closed,
    winRate: d.winRate === null ? null : Number(d.winRate),
    winRateText: d.winRate,
  }));
}

/** A chart row. Numbers are for drawing only; text keeps Rust's exact value. */
export interface CurveRow {
  /** Trade number in close order; 0 = the starting point. */
  n: number;
  time: number | null;
  equity: number;
  equityText: Decimal;
  pnl: number;
  pnlText: Decimal;
  tradeId: number | null;
}

/** Equity curve rows, starting from 0 before the first closed trade. */
export function curveRows(s: Stats): CurveRow[] {
  if (s.equityCurve.length === 0) return [];
  const start: CurveRow = {
    n: 0,
    time: null,
    equity: 0,
    equityText: "0",
    pnl: 0,
    pnlText: "0",
    tradeId: null,
  };
  return [
    start,
    ...s.equityCurve.map((p, i) => ({
      n: i + 1,
      time: p.time,
      equity: Number(p.equity),
      equityText: p.equity,
      pnl: Number(p.pnl),
      pnlText: p.pnl,
      tradeId: p.tradeId,
    })),
  ];
}

/** What the grid reports after a drag/resize (it adds extra fields). */
interface GridItem {
  i: string;
  x: number;
  y: number;
  w: number;
  h: number;
}

/**
 * The grid's layout → our saved shape, in the order of `current` (minimum
 * sizes come from Rust). Returns null when nothing moved, so we skip saving.
 */
export function changedLayout(
  current: LayoutItem[],
  grid: readonly GridItem[],
): LayoutItem[] | null {
  const byId = new Map(grid.map((g) => [g.i, g]));
  let changed = false;
  const next = current.map((item) => {
    const g = byId.get(item.i);
    if (!g) return item;
    if (g.x !== item.x || g.y !== item.y || g.w !== item.w || g.h !== item.h) {
      changed = true;
    }
    return { ...item, x: g.x, y: g.y, w: g.w, h: g.h };
  });
  return changed ? next : null;
}

/** One bar of a "where the money comes from" chart. */
export interface SliceRow {
  key: string;
  label: string;
  /** Net PnL as a number, only for bar length (text comes from `pnl`). */
  value: number;
  pnl: Decimal;
  closed: number;
  wins: number;
  losses: number;
  winRate: Decimal | null;
  tone: "pos" | "neg" | "flat";
}

export function sliceRows(slices: Slice[]): SliceRow[] {
  return slices.map((s) => {
    const value = Number(s.netPnl);
    return {
      key: s.key,
      label: s.label,
      value,
      pnl: s.netPnl,
      closed: s.closed,
      wins: s.wins,
      losses: s.losses,
      winRate: s.winRate,
      tone: value > 0 ? "pos" : value < 0 ? "neg" : "flat",
    };
  });
}

/** "12 trades · 58% won" (or "No closed trades"). */
export function sliceSummary(r: SliceRow): string {
  if (r.closed === 0) return "No closed trades";
  const trades = `${r.closed} trade${r.closed === 1 ? "" : "s"}`;
  return r.winRate === null
    ? trades
    : `${trades} · ${Number(r.winRate).toFixed(0)}% won`;
}

/** Ring segments for wins / losses / breakeven, empty ones left out. */
export interface OutcomeRow {
  key: "wins" | "losses" | "breakeven";
  label: string;
  count: number;
}

export function outcomeRows(s: Stats): OutcomeRow[] {
  const rows: OutcomeRow[] = [
    { key: "wins", label: "Wins", count: s.wins },
    { key: "losses", label: "Losses", count: s.losses },
    { key: "breakeven", label: "Breakeven", count: s.breakeven },
  ];
  return rows.filter((r) => r.count > 0);
}

export interface RRow {
  label: string;
  count: number;
  /** Buckets below 0R are losses. */
  tone: "pos" | "neg";
}

export function rRows(buckets: RBucket[]): RRow[] {
  return buckets.map((b) => ({
    label: b.label,
    count: b.count,
    tone: b.to !== null && b.to <= 0 ? "neg" : "pos",
  }));
}
