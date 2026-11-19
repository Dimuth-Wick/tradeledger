import { describe, expect, it } from "vitest";
import {
  DEFAULT_SORT,
  EMPTY_FILTERS,
  buildQuery,
  endOfDayExclusive,
  hasFilters,
  nextSort,
  startOfDay,
} from "./tradeLog";

describe("trade log helpers", () => {
  it("turns date inputs into a local, end-exclusive range", () => {
    expect(startOfDay("2026-02-03")).toBe(new Date(2026, 1, 3).getTime());
    expect(endOfDayExclusive("2026-02-03")).toBe(
      new Date(2026, 1, 4).getTime(),
    );
    expect(endOfDayExclusive("2026-01-31")).toBe(
      new Date(2026, 1, 1).getTime(),
    );
    expect(startOfDay("")).toBeNull();
    expect(startOfDay("03/02/2026")).toBeNull();
  });

  it("builds an empty query from empty filters", () => {
    expect(buildQuery(EMPTY_FILTERS, DEFAULT_SORT, 0, 25)).toEqual({
      from: null,
      to: null,
      symbol: null,
      market: null,
      direction: null,
      status: null,
      outcome: null,
      tagIds: [],
      sort: "openTime",
      descending: true,
      page: 0,
      pageSize: 25,
    });
  });

  it("passes every filter through", () => {
    const q = buildQuery(
      {
        from: "2026-02-01",
        to: "2026-02-07",
        symbol: " btc ",
        market: "usdm",
        direction: "short",
        status: "closed",
        outcome: "loss",
        tag: "12",
      },
      { field: "netPnl", descending: false },
      2,
      50,
    );
    expect(q).toMatchObject({
      from: new Date(2026, 1, 1).getTime(),
      to: new Date(2026, 1, 8).getTime(),
      symbol: "btc",
      market: "usdm",
      direction: "short",
      status: "closed",
      outcome: "loss",
      tagIds: [12],
      sort: "netPnl",
      descending: false,
      page: 2,
      pageSize: 50,
    });
  });

  it("toggles sort on the same column and picks sensible defaults", () => {
    expect(nextSort(DEFAULT_SORT, "openTime")).toEqual({
      field: "openTime",
      descending: false,
    });
    expect(nextSort(DEFAULT_SORT, "symbol")).toEqual({
      field: "symbol",
      descending: false,
    });
    expect(nextSort(DEFAULT_SORT, "netPnl")).toEqual({
      field: "netPnl",
      descending: true,
    });
  });

  it("knows when any filter is set", () => {
    expect(hasFilters(EMPTY_FILTERS)).toBe(false);
    expect(hasFilters({ ...EMPTY_FILTERS, outcome: "win" })).toBe(true);
  });
});
