// The "Exit" box on a saved trade: add the exit price when you close the
// trade, change it, or clear it to reopen. Only format checks here; Rust
// works out the profit and checks the rules.

import type { CloseInput, Trade } from "../connection/backend";
import { FormatError, decimal, time, toLocalInputValue } from "./tradeForm";

export interface ExitForm {
  exitPrice: string;
  /** `<input type="datetime-local">` value. */
  closeTime: string;
}

/** The box as saved: the trade's exit, or empty with the date set to now. */
export function exitFormFrom(t: Trade, now: Date = new Date()): ExitForm {
  return {
    exitPrice: t.avgExit ?? "",
    closeTime: toLocalInputValue(
      t.closeTime !== null ? new Date(t.closeTime) : now,
    ),
  };
}

/** Has the user changed the exit price or (for a closed trade) its date? */
export function exitDirty(form: ExitForm, t: Trade): boolean {
  const price = form.exitPrice.trim();
  if (price !== (t.avgExit ?? "")) return true;
  return (
    t.closeTime !== null &&
    form.closeTime !== toLocalInputValue(new Date(t.closeTime))
  );
}

export type ExitResult =
  { ok: true; input: CloseInput } | { ok: false; error: string };

export function toCloseInput(form: ExitForm): ExitResult {
  try {
    const exitPrice = decimal("Exit price", form.exitPrice);
    return {
      ok: true,
      input: {
        exitPrice,
        closeTime: exitPrice === null ? null : time("Closed", form.closeTime),
      },
    };
  } catch (err) {
    if (err instanceof FormatError) return { ok: false, error: err.message };
    throw err;
  }
}
