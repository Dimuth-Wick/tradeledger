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

/**
 * Rounds to 2 decimals for display, working on the text (no floats):
 * "10.90559933" → "10.91", "-22.7688" → "-22.77", "2.50" → "2.5".
 * Halves round away from zero, like the Rust side.
 */
export function round2(d: Decimal): string {
  const m = /^(-?)(\d+)\.(\d{3,})$/.exec(d);
  if (!m) return d; // already 2 decimals or fewer
  const [, sign, int, frac] = m;
  let cents = BigInt(int + frac.slice(0, 2));
  if (Number(frac[2]) >= 5) cents += BigInt(1);
  if (cents === BigInt(0)) return "0";
  const s = cents.toString().padStart(3, "0");
  return sign + `${s.slice(0, -2)}.${s.slice(-2)}`.replace(/\.?0+$/, "");
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
