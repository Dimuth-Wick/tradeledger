import type { Direction, Market, TradeInput } from "../connection/backend";

// Turns what the user typed into a TradeInput for Rust. Only checks *format*
// here (is it a number? is a required box empty?). Business rules — stop on
// the right side, close after open, etc. — are checked in Rust, so the rules
// live in one place.

/** Every box is a string, exactly as typed. */
export interface TradeFormState {
  exchange: string;
  market: Market;
  symbol: string;
  direction: Direction;
  /** `<input type="datetime-local">` value, e.g. "2026-11-14T10:30" (local time). */
  openTime: string;
  closeTime: string;
  /** Position size in USDT (quantity is worked out in Rust). */
  positionSize: string;
  avgEntry: string;
  avgExit: string;
  leverage: string;
  /** Empty = automatic, 0.1% of the position size. */
  fees: string;
  funding: string;
  plannedEntry: string;
  stopLoss: string;
  takeProfit: string;
  notes: string;
}

export type FormResult =
  | { ok: true; input: TradeInput }
  /** `incomplete` = a required box is still empty (show a hint, not an error). */
  | { ok: false; incomplete: boolean; error: string };

/** "2026-11-14T10:30" for a Date, in the computer's local time zone. */
export function toLocalInputValue(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return (
    `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}` +
    `T${pad(date.getHours())}:${pad(date.getMinutes())}`
  );
}

export function emptyForm(now: Date = new Date()): TradeFormState {
  return {
    exchange: "binance",
    market: "usdm",
    symbol: "",
    direction: "long",
    openTime: toLocalInputValue(now),
    closeTime: "",
    positionSize: "",
    avgEntry: "",
    avgExit: "",
    leverage: "",
    fees: "",
    funding: "",
    plannedEntry: "",
    stopLoss: "",
    takeProfit: "",
    notes: "",
  };
}

/** True once the user typed something worth keeping. The prefilled boxes
 *  (exchange, market, side, open time) don't count. */
export function hasInput(form: TradeFormState): boolean {
  const typed: (keyof TradeFormState)[] = [
    "symbol",
    "closeTime",
    "positionSize",
    "avgEntry",
    "avgExit",
    "leverage",
    "fees",
    "funding",
    "plannedEntry",
    "stopLoss",
    "takeProfit",
    "notes",
  ];
  return typed.some((k) => String(form[k]).trim() !== "");
}

const UNSIGNED = /^(\d+\.?\d*|\.\d+)$/;
const SIGNED = /^-?(\d+\.?\d*|\.\d+)$/;
const INTEGER = /^\d+$/;

class FormatError extends Error {}

/** Empty → null; otherwise must match `pattern`. Commas are not accepted
 *  because "1,5" means 1.5 in some countries and 15 in others. */
function decimal(
  label: string,
  raw: string,
  pattern = UNSIGNED,
): string | null {
  const v = raw.trim();
  if (v === "") return null;
  if (!pattern.test(v)) {
    throw new FormatError(`${label}: enter a number like 0.5`);
  }
  return v;
}

function time(label: string, raw: string): number | null {
  if (raw.trim() === "") return null;
  const ms = new Date(raw).getTime();
  if (Number.isNaN(ms)) throw new FormatError(`${label}: invalid date/time`);
  return ms;
}

export function toTradeInput(form: TradeFormState): FormResult {
  const missing = [
    form.symbol.trim() === "" && "symbol",
    form.openTime.trim() === "" && "open time",
    form.positionSize.trim() === "" && "position size",
    form.avgEntry.trim() === "" && "entry price",
  ].filter(Boolean);
  if (missing.length > 0) {
    return {
      ok: false,
      incomplete: true,
      error: `Fill in ${missing.join(", ")} to see the preview`,
    };
  }

  try {
    const leverage = form.leverage.trim();
    if (leverage !== "" && !INTEGER.test(leverage)) {
      throw new FormatError("Leverage: enter a whole number like 10");
    }
    // Leverage and funding only exist on futures; ignore leftovers if the
    // user switched the market back to spot.
    const futures = form.market === "usdm";

    const input: TradeInput = {
      exchange: form.exchange.trim(),
      market: form.market,
      symbol: form.symbol.trim(),
      direction: form.direction,
      openTime: time("Open time", form.openTime)!,
      closeTime: time("Close time", form.closeTime),
      positionSize: decimal("Position size", form.positionSize)!,
      avgEntry: decimal("Entry price", form.avgEntry)!,
      avgExit: decimal("Exit price", form.avgExit),
      leverage: futures && leverage !== "" ? Number(leverage) : null,
      // Empty fees box = let Rust charge 0.1% of the position size.
      fees: decimal("Fees", form.fees) ?? "0",
      autoFees: form.fees.trim() === "",
      funding: futures
        ? (decimal("Funding", form.funding, SIGNED) ?? "0")
        : "0",
      plannedEntry: decimal("Planned entry", form.plannedEntry),
      stopLoss: decimal("Stop loss", form.stopLoss),
      takeProfit: decimal("Take profit", form.takeProfit),
      notes: form.notes,
    };
    return { ok: true, input };
  } catch (err) {
    if (err instanceof FormatError) {
      return { ok: false, incomplete: false, error: err.message };
    }
    throw err;
  }
}
