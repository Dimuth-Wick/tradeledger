import { invoke } from "@tauri-apps/api/core";

// The single place where the UI talks to Rust. Components import these typed
// functions instead of calling invoke("...") with raw strings, so a renamed
// command breaks in one file instead of silently failing at runtime.

/**
 * Exact decimal as text, e.g. "36500.10". Rust sends money this way so
 * JavaScript never rounds it; the UI only formats it for display.
 */
export type Decimal = string;
/** Milliseconds since 1970-01-01 UTC, like Date.now(). */
export type TimestampMs = number;

export type Market = "spot" | "usdm";
export type Direction = "long" | "short";
export type TradeStatus = "open" | "closed";

/** A stored row: Rust `WithId<T>` flattens `id` into the object. */
export type WithId<T> = T & { id: number };

/** Mirrors Rust `commands::DbInfo` (serde camelCase). */
export interface DbInfo {
  path: string;
  schemaVersion: number;
}

/** Mirrors Rust `models::Trade`. */
export interface Trade {
  exchange: string;
  market: Market;
  symbol: string;
  direction: Direction;
  status: TradeStatus;
  openTime: TimestampMs;
  closeTime: TimestampMs | null;
  qty: Decimal;
  avgEntry: Decimal;
  avgExit: Decimal | null;
  leverage: number | null;
  grossPnl: Decimal;
  fees: Decimal;
  funding: Decimal;
  netPnl: Decimal;
  plannedEntry: Decimal | null;
  stopLoss: Decimal | null;
  takeProfit: Decimal | null;
  plannedRr: Decimal | null;
  rMultiple: Decimal | null;
  notes: string;
  playbookId: number | null;
  isManual: boolean;
}

/**
 * Mirrors Rust `trade_entry::TradeInput`: only what the user types.
 * Status, PnL, R:R and R-multiple are computed in Rust.
 */
export interface TradeInput {
  exchange: string;
  market: Market;
  symbol: string;
  direction: Direction;
  openTime: TimestampMs;
  closeTime?: TimestampMs | null;
  qty: Decimal;
  avgEntry: Decimal;
  avgExit?: Decimal | null;
  leverage?: number | null;
  fees?: Decimal;
  /** Positive = paid, negative = received. */
  funding?: Decimal;
  plannedEntry?: Decimal | null;
  stopLoss?: Decimal | null;
  takeProfit?: Decimal | null;
  notes?: string;
  playbookId?: number | null;
}

export type Outcome = "win" | "loss" | "breakeven";
export type SortField =
  "openTime" | "closeTime" | "symbol" | "netPnl" | "rMultiple";

/** Mirrors Rust `trade_query::TradeQuery`. Every filter is optional. */
export interface TradeQuery {
  /** Opened at or after (ms, inclusive). */
  from?: TimestampMs | null;
  /** Opened before (ms, exclusive). */
  to?: TimestampMs | null;
  /** Part of the symbol, any case. */
  symbol?: string | null;
  market?: Market | null;
  direction?: Direction | null;
  status?: TradeStatus | null;
  /** Implies closed trades only. */
  outcome?: Outcome | null;
  tagIds?: number[];
  sort?: SortField;
  descending?: boolean;
  /** 0-based. */
  page?: number;
  pageSize?: number;
}

/** Mirrors Rust `trade_query::TradePage`. */
export interface TradePage {
  items: WithId<Trade>[];
  total: number;
  page: number;
  pageSize: number;
  pageCount: number;
  /** Net PnL of all matching closed trades (every page). */
  closedNetPnl: Decimal;
  wins: number;
  losses: number;
}

/** Mirrors Rust `importers::RowError`. */
export interface RowError {
  /** Line in the file (header = 1). */
  line: number;
  message: string;
}

/** Mirrors Rust `import_flow::InstrumentPreview`. */
export interface InstrumentPreview {
  exchange: string;
  market: Market;
  symbol: string;
  newRows: number;
  tradesAfter: number;
  openAfter: number;
  orphanFills: number;
  unassignedFunding: number;
  /** Asset → amount, e.g. { BNB: "0.0006125" }. */
  unconvertedFees: Record<string, Decimal>;
}

/** Mirrors Rust `import_flow::ImportPreview`. */
export interface ImportPreview {
  kind: string;
  label: string;
  fileName: string;
  fills: number;
  funding: number;
  skipped: number;
  alreadyImported: number;
  errors: RowError[];
  instruments: InstrumentPreview[];
}

/** Mirrors Rust `grouping::persist::RegroupSummary`. */
export interface RegroupSummary {
  tradesCreated: number;
  tradesUpdated: number;
  tradesRemoved: number;
  orphanFills: number;
  unassignedFunding: number;
}

/** Mirrors Rust `import_flow::ImportResult`. */
export interface ImportResult {
  import: {
    batchId: number;
    rowsTotal: number;
    inserted: number;
    duplicates: number;
  };
  regroup: RegroupSummary;
}

/** Mirrors Rust `grouping::edit::TradeFill`: one fill as used by a trade. */
export interface TradeFill {
  fillId: number;
  time: TimestampMs;
  side: "buy" | "sell";
  price: Decimal;
  /** The fill's full quantity. */
  fillQty: Decimal;
  /** The part used by this trade (smaller for a fill that flips the position). */
  qty: Decimal;
  isEntry: boolean;
  fee: Decimal;
  feeAsset: string;
}

export function ping(): Promise<string> {
  return invoke<string>("ping");
}

export function dbInfo(): Promise<DbInfo> {
  return invoke<DbInfo>("db_info");
}

/** Derived values for the form preview; nothing is saved. */
export function previewTrade(input: TradeInput): Promise<Trade> {
  return invoke<Trade>("preview_trade", { input });
}

export function createTrade(input: TradeInput): Promise<WithId<Trade>> {
  return invoke<WithId<Trade>>("create_trade", { input });
}

export function updateTrade(
  id: number,
  input: TradeInput,
): Promise<WithId<Trade>> {
  return invoke<WithId<Trade>>("update_trade", { id, input });
}

export function deleteTrade(id: number): Promise<void> {
  return invoke<void>("delete_trade", { id });
}

export function getTrade(id: number): Promise<WithId<Trade> | null> {
  return invoke<WithId<Trade> | null>("get_trade", { id });
}

export function listTrades(): Promise<WithId<Trade>[]> {
  return invoke<WithId<Trade>[]>("list_trades");
}

/** Trade log: filtered, sorted, paginated in Rust. */
export function queryTrades(query: TradeQuery): Promise<TradePage> {
  return invoke<TradePage>("query_trades", { query });
}

export function listSymbols(): Promise<string[]> {
  return invoke<string[]>("list_symbols");
}

export function listTradeFills(tradeId: number): Promise<TradeFill[]> {
  return invoke<TradeFill[]>("list_trade_fills", { tradeId });
}

/** Joins an imported trade to the previous one (same coin and direction). */
export function mergeTradeWithPrevious(id: number): Promise<RegroupSummary> {
  return invoke<RegroupSummary>("merge_trade_with_previous", { id });
}

/** Starts a new trade at this fill (also undoes a merge). */
export function splitTradeAtFill(
  tradeId: number,
  fillId: number,
): Promise<RegroupSummary> {
  return invoke<RegroupSummary>("split_trade_at_fill", { tradeId, fillId });
}

/** Removes all manual merge/split rules for this trade's coin. */
export function resetGrouping(tradeId: number): Promise<RegroupSummary> {
  return invoke<RegroupSummary>("reset_grouping", { tradeId });
}

/** Parses the CSV text and shows what importing would do. Saves nothing. */
export function previewImport(
  fileName: string,
  content: string,
): Promise<ImportPreview> {
  return invoke<ImportPreview>("preview_import", { fileName, content });
}

/** Saves the file's valid rows and regroups the affected trades. */
export function confirmImport(
  fileName: string,
  content: string,
): Promise<ImportResult> {
  return invoke<ImportResult>("confirm_import", { fileName, content });
}
