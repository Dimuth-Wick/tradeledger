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

export function ping(): Promise<string> {
  return invoke<string>("ping");
}

export function dbInfo(): Promise<DbInfo> {
  return invoke<DbInfo>("db_info");
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
