import type { Decimal } from "./api";
import { isNegative, isZero } from "./format";

// Calendar display helpers. Rust groups the trades and adds up the money;
// this file only picks names, colors and moves between months.

export const WEEKDAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

const MONTHS = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
];

/** "March 2026" (month 1–12). */
export function monthTitle(year: number, month: number): string {
  return `${MONTHS[month - 1]} ${year}`;
}

/** Month `delta` steps away; handles year ends. */
export function shiftMonth(
  year: number,
  month: number,
  delta: number,
): { year: number; month: number } {
  const index = year * 12 + (month - 1) + delta;
  return { year: Math.floor(index / 12), month: (index % 12) + 1 };
}

/**
 * Cell color: green for a profit day, red for a loss day, stronger for
 * bigger days (4 steps, relative to the month's biggest day). The numbers
 * only pick a shade; nothing is calculated for display.
 */
export function cellTone(
  net: Decimal,
  maxAbs: Decimal,
): { tone: "pos" | "neg" | ""; level: number } {
  if (isZero(net) || isZero(maxAbs)) return { tone: "", level: 0 };
  const share = Math.abs(Number(net)) / Number(maxAbs);
  const level = Math.min(4, Math.max(1, Math.ceil(share * 4)));
  return { tone: isNegative(net) ? "neg" : "pos", level };
}

/** "Mon 2 Mar 2026" from "2026-03-02". */
export function dayTitle(date: string): string {
  const [y, m, d] = date.split("-").map(Number);
  const weekday = WEEKDAYS[(new Date(y, m - 1, d).getDay() + 6) % 7];
  return `${weekday} ${d} ${MONTHS[m - 1].slice(0, 3)} ${y}`;
}

/** Minutes east of UTC for a zone value, or null for computer time. */
export function zoneOffset(value: string): number | null {
  if (value === "local") return null;
  if (value === "utc") return 0;
  const m = /^([+-])(\d{2}):(\d{2})$/.exec(value);
  if (!m) return null;
  const minutes = Number(m[2]) * 60 + Number(m[3]);
  return m[1] === "-" ? -minutes : minutes;
}

/** Clock time "14:05" of an instant in the calendar's zone. */
export function timeIn(ms: number, zoneValue: string): string {
  const offset = zoneOffset(zoneValue);
  const opts: Intl.DateTimeFormatOptions = {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  };
  if (offset === null) return new Date(ms).toLocaleTimeString(undefined, opts);
  // Shift the instant, then read it as UTC: gives the wall clock there.
  return new Date(ms + offset * 60_000).toLocaleTimeString(undefined, {
    ...opts,
    timeZone: "UTC",
  });
}
