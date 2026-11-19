import type {
  ImportPreview,
  ImportResult,
  RegroupSummary,
} from "../connection/backend";

// Turns import preview/result numbers into sentences for the Import page.
// Wording only: every number is computed in Rust.

/** Rows in the file that would actually be added. */
export function newRowCount(p: ImportPreview): number {
  return p.fills + p.funding - p.alreadyImported;
}

function plural(n: number, one: string, many: string = `${one}s`): string {
  return `${n} ${n === 1 ? one : many}`;
}

/** Things the user should know before confirming. */
export function previewWarnings(p: ImportPreview): string[] {
  const out: string[] = [];
  for (const i of p.instruments) {
    if (i.orphanFills > 0) {
      out.push(
        `${i.symbol}: ${plural(i.orphanFills, "sell")} without a matching buy ` +
          `(probably bought before this file's dates). Not counted as a trade.`,
      );
    }
    for (const [asset, amount] of Object.entries(i.unconvertedFees)) {
      out.push(
        `${i.symbol}: fees of ${amount} ${asset} can't be converted offline ` +
          `and aren't included in PnL.`,
      );
    }
    if (i.unassignedFunding > 0) {
      out.push(
        `${i.symbol}: ${plural(i.unassignedFunding, "funding payment")} ` +
          `don't match any trade yet. Import the futures trade history too.`,
      );
    }
  }
  if (p.skipped > 0) {
    out.push(
      `${plural(p.skipped, "row")} skipped on purpose (transfers, ` +
        `commissions and realized PnL already come from the trade history).`,
    );
  }
  return out;
}

/** "trades: 1 new, 2 updated, 1 removed" (removed only if any). */
export function regroupSummary(r: RegroupSummary): string {
  return (
    `trades: ${r.tradesCreated} new, ${r.tradesUpdated} updated` +
    (r.tradesRemoved > 0 ? `, ${r.tradesRemoved} removed` : "")
  );
}

export function resultSummary(r: ImportResult): string {
  const { inserted, duplicates } = r.import;
  const parts = [
    `Imported ${plural(inserted, "new row")}`,
    duplicates > 0 ? `${duplicates} already imported (skipped)` : null,
    regroupSummary(r.regroup),
  ];
  return parts.filter(Boolean).join(" · ");
}
