// Names and dialog settings for files the app writes. Rust does the work;
// this file only suggests sensible names.

import { isoDay } from "./dashboard";

export const BACKUP_FILTER = {
  name: "TradeLedger backup",
  extensions: ["zip"],
};
export const CSV_FILTER = { name: "CSV spreadsheet", extensions: ["csv"] };

/** "tradeledger-backup-2026-11-19.zip" (local date). */
export function backupFileName(now: Date): string {
  return `tradeledger-backup-${isoDay(now)}.zip`;
}

/** "tradeledger-trades-2026-11-19.csv" (local date). */
export function csvFileName(now: Date): string {
  return `tradeledger-trades-${isoDay(now)}.csv`;
}

/** The single path from an Open dialog (null when cancelled). */
export function onePath(picked: string | string[] | null): string | null {
  if (picked === null) return null;
  return Array.isArray(picked) ? (picked[0] ?? null) : picked;
}
