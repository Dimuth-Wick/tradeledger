import { describe, expect, it } from "vitest";
import {
  cellTone,
  dayTitle,
  monthTitle,
  shiftMonth,
  timeIn,
  zoneOffset,
} from "./calendar";

describe("months", () => {
  it.each([
    [2026, 3, 1, 2026, 4],
    [2026, 12, 1, 2027, 1],
    [2026, 1, -1, 2025, 12],
    [2026, 3, -14, 2025, 1],
  ])("%i-%i %+i → %i-%i", (y, m, d, ey, em) => {
    expect(shiftMonth(y, m, d)).toEqual({ year: ey, month: em });
  });

  it("titles", () => {
    expect(monthTitle(2026, 3)).toBe("March 2026");
    expect(dayTitle("2026-03-02")).toBe("Mon 2 Mar 2026");
    expect(dayTitle("2026-03-01")).toBe("Sun 1 Mar 2026");
  });
});

describe("cellTone", () => {
  it.each([
    ["0", "11", "", 0],
    ["5", "0", "", 0],
    ["11", "11", "pos", 4],
    ["-7", "11", "neg", 3], // 7/11 = 0.64 → step 3
    ["0.01", "11", "pos", 1], // tiny days still get the lightest shade
    ["-2.75", "11", "neg", 1], // exactly 1/4
  ])("%s of max %s → %s %i", (net, max, tone, level) => {
    expect(cellTone(net, max)).toEqual({ tone, level });
  });
});

describe("zones", () => {
  it("parses offsets", () => {
    expect(zoneOffset("local")).toBeNull();
    expect(zoneOffset("utc")).toBe(0);
    expect(zoneOffset("+05:30")).toBe(330);
    expect(zoneOffset("-03:00")).toBe(-180);
  });

  it("shows the wall clock of the chosen zone", () => {
    const t = Date.UTC(2026, 2, 2, 22, 30); // 22:30 UTC
    expect(timeIn(t, "utc")).toBe("22:30");
    expect(timeIn(t, "+05:30")).toBe("04:00");
    expect(timeIn(t, "-03:00")).toBe("19:30");
  });
});
