import type { Decimal } from "./api";

// Display-only formatting of the decimal strings Rust sends. No arithmetic:
// the numbers are already computed, we only decorate them.

const ZERO = /^-?0*(\.0*)?$/;

export function isZero(d: Decimal): boolean {
  return ZERO.test(d);
}

export function isNegative(d: Decimal): boolean {
  return d.startsWith("-") && !isZero(d);
}

/** "19.5" → "+19.5", "-3" → "-3", "0" → "0". */
export function signed(d: Decimal): string {
  if (isZero(d) || d.startsWith("-")) return d;
  return `+${d}`;
}

/** R-multiple: "2" → "+2R", null → "—". */
export function formatR(d: Decimal | null): string {
  return d === null ? "—" : `${signed(d)}R`;
}

/** Planned reward-to-risk: "3" → "1 : 3", null → "—". */
export function formatRr(d: Decimal | null): string {
  return d === null ? "—" : `1 : ${d}`;
}

/** CSS class for coloring PnL / R values. */
export function pnlClass(d: Decimal | null): string {
  if (d === null || isZero(d)) return "";
  return isNegative(d) ? "neg" : "pos";
}

export function formatDateTime(ms: number): string {
  return new Date(ms).toLocaleString(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  });
}
