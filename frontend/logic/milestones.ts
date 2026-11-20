// Milestone badges: text and colors only. Rust counts the trades and
// remembers which badges were celebrated.

import type { Milestone, MilestoneProgress } from "../connection/backend";

/** Metal or gem of each badge, lowest to highest (CSS classes). */
export const TIERS: Record<number, string> = {
  50: "bronze",
  100: "silver",
  300: "gold",
  500: "emerald",
  1000: "sapphire",
  2000: "ruby",
  5000: "legend",
};

export function tierOf(trades: number): string {
  return TIERS[trades] ?? "bronze";
}

/** 1000 → "1K", 50 → "50" (fits inside the badge). */
export function shortCount(n: number): string {
  return n >= 1000 ? `${n / 1000}K` : String(n);
}

/** What the pop-up celebrates: the highest new badge, and how many other
 *  badges came with it (a big import can unlock several). */
export function celebration(
  p: MilestoneProgress,
): { badge: Milestone; alsoUnlocked: number } | null {
  if (p.unseen.length === 0) return null;
  const top = Math.max(...p.unseen);
  const badge = p.milestones.find((m) => m.trades === top);
  return badge ? { badge, alsoUnlocked: p.unseen.length - 1 } : null;
}

/** The highest badge earned so far, or null before the first one. */
export function currentBadge(p: MilestoneProgress): Milestone | null {
  const earned = p.milestones.filter((m) => m.reached);
  return earned.length > 0 ? earned[earned.length - 1] : null;
}

/** The badge being worked towards, or null when all are earned. */
export function nextBadge(p: MilestoneProgress): Milestone | null {
  return p.milestones.find((m) => m.trades === p.next) ?? null;
}

/** "27 more trades to Committed" / "Every badge earned". */
export function nextText(p: MilestoneProgress): string {
  if (p.next === null) return "Every badge earned. Legendary.";
  const left = p.next - p.trades;
  const name = p.milestones.find((m) => m.trades === p.next)?.name ?? "";
  return `${left} more trade${left === 1 ? "" : "s"} to ${name}`;
}

// Pages tell the app shell that trades were added or removed, so it can
// check for a new badge right away.
const EVENT = "tradeledger:trades-changed";

export function notifyTradesChanged(): void {
  window.dispatchEvent(new Event(EVENT));
}

export function onTradesChanged(handler: () => void): () => void {
  window.addEventListener(EVENT, handler);
  return () => window.removeEventListener(EVENT, handler);
}
