import type {
  Direction,
  Market,
  Outcome,
  SortField,
  TradeQuery,
  TradeStatus,
} from "./api";

// Trade log UI state → the query Rust runs. Filtering, sorting and paging
// all happen in Rust; this file only translates form values.

/** Filter boxes exactly as shown in the UI ("" = any). */
export interface LogFilters {
  /** `<input type="date">` value, e.g. "2026-02-01" (local day). */
  from: string;
  to: string;
  symbol: string;
  market: Market | "";
  direction: Direction | "";
  status: TradeStatus | "";
  outcome: Outcome | "";
  /** Tag id as text ("" = any tag). */
  tag: string;
}

export const EMPTY_FILTERS: LogFilters = {
  tag: "",
  from: "",
  to: "",
  symbol: "",
  market: "",
  direction: "",
  status: "",
  outcome: "",
};

export interface LogSort {
  field: SortField;
  descending: boolean;
}

export const DEFAULT_SORT: LogSort = { field: "openTime", descending: true };
export const PAGE_SIZES = [25, 50, 100] as const;

/** Local midnight at the start of a "YYYY-MM-DD" day, in ms. */
export function startOfDay(date: string): number | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(date);
  if (!m) return null;
  return new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3])).getTime();
}

/** Local midnight of the NEXT day, so "to 2026-02-03" includes all of the 3rd. */
export function endOfDayExclusive(date: string): number | null {
  const start = startOfDay(date);
  if (start === null) return null;
  const d = new Date(start);
  d.setDate(d.getDate() + 1); // handles month ends and DST changes
  return d.getTime();
}

const orNull = <T extends string>(v: T | ""): T | null => (v === "" ? null : v);

export function buildQuery(
  f: LogFilters,
  sort: LogSort,
  page: number,
  pageSize: number,
): TradeQuery {
  return {
    from: startOfDay(f.from),
    to: endOfDayExclusive(f.to),
    symbol: f.symbol.trim() === "" ? null : f.symbol.trim(),
    market: orNull(f.market),
    direction: orNull(f.direction),
    status: orNull(f.status),
    outcome: orNull(f.outcome),
    tagIds: f.tag === "" ? [] : [Number(f.tag)],
    sort: sort.field,
    descending: sort.descending,
    page,
    pageSize,
  };
}

/** Clicking a column header: same column flips direction; a new column
 *  starts A→Z for symbol and biggest/newest first for everything else. */
export function nextSort(current: LogSort, field: SortField): LogSort {
  if (current.field === field) {
    return { field, descending: !current.descending };
  }
  return { field, descending: field !== "symbol" };
}

export function hasFilters(f: LogFilters): boolean {
  return Object.values(f).some((v) => v !== "");
}
