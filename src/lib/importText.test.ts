import { describe, expect, it } from "vitest";
import type { ImportPreview, InstrumentPreview, ImportResult } from "./api";
import {
  newRowCount,
  previewWarnings,
  regroupSummary,
  resultSummary,
} from "./importText";

function instrument(over: Partial<InstrumentPreview>): InstrumentPreview {
  return {
    exchange: "binance",
    market: "spot",
    symbol: "ETHUSDT",
    newRows: 3,
    tradesAfter: 1,
    openAfter: 0,
    orphanFills: 0,
    unassignedFunding: 0,
    unconvertedFees: {},
    ...over,
  };
}

function preview(over: Partial<ImportPreview>): ImportPreview {
  return {
    kind: "binance-spot",
    label: "Binance spot trade history",
    fileName: "spot.csv",
    fills: 8,
    funding: 0,
    skipped: 0,
    alreadyImported: 0,
    errors: [],
    instruments: [instrument({})],
    ...over,
  };
}

describe("import text", () => {
  it("counts only rows not imported before", () => {
    expect(newRowCount(preview({ fills: 8, alreadyImported: 3 }))).toBe(5);
    expect(newRowCount(preview({ fills: 0, funding: 6 }))).toBe(6);
  });

  it("has no warnings for a clean file", () => {
    expect(previewWarnings(preview({}))).toEqual([]);
  });

  it("warns about orphans, BNB fees, unmatched funding and skipped rows", () => {
    const w = previewWarnings(
      preview({
        skipped: 3,
        instruments: [
          instrument({ symbol: "ADAUSDT", orphanFills: 1 }),
          instrument({
            symbol: "SOLUSDT",
            unconvertedFees: { BNB: "0.0006125" },
          }),
          instrument({ symbol: "BTCUSDT", unassignedFunding: 3 }),
        ],
      }),
    );
    expect(w).toHaveLength(4);
    expect(w[0]).toContain("ADAUSDT: 1 sell without a matching buy");
    expect(w[1]).toContain("0.0006125 BNB");
    expect(w[2]).toContain("3 funding payments");
    expect(w[3]).toContain("3 rows skipped");
  });

  it("summarizes a regroup (merge/split)", () => {
    expect(
      regroupSummary({
        tradesCreated: 0,
        tradesUpdated: 1,
        tradesRemoved: 1,
        orphanFills: 0,
        unassignedFunding: 0,
      }),
    ).toBe("trades: 0 new, 1 updated, 1 removed");
  });

  it("summarizes the result", () => {
    const r: ImportResult = {
      import: { batchId: 1, rowsTotal: 8, inserted: 8, duplicates: 0 },
      regroup: {
        tradesCreated: 3,
        tradesUpdated: 0,
        tradesRemoved: 0,
        orphanFills: 0,
        unassignedFunding: 0,
      },
    };
    expect(resultSummary(r)).toBe(
      "Imported 8 new rows · trades: 3 new, 0 updated",
    );
    r.import = { batchId: 2, rowsTotal: 8, inserted: 1, duplicates: 7 };
    r.regroup.tradesRemoved = 1;
    expect(resultSummary(r)).toBe(
      "Imported 1 new row · 7 already imported (skipped) · trades: 3 new, 0 updated, 1 removed",
    );
  });
});
