import { describe, expect, it } from "vitest";
import {
  formatR,
  formatRr,
  isZero,
  pnlClass,
  round2,
  signed,
  usd,
  usdTick,
} from "./format";

describe("format helpers", () => {
  it("detects zero in any spelling", () => {
    for (const z of ["0", "0.00", "-0", "-0.0", ".0"])
      expect(isZero(z)).toBe(true);
    expect(isZero("0.01")).toBe(false);
  });

  it("adds a plus sign to positive values only", () => {
    expect(signed("19.5")).toBe("+19.5");
    expect(signed("-3")).toBe("-3");
    expect(signed("0")).toBe("0");
  });

  it("formats R values", () => {
    expect(formatR("2")).toBe("+2R");
    expect(formatR("-1.5")).toBe("-1.5R");
    expect(formatR(null)).toBe("—");
    expect(formatRr("3.33")).toBe("1 : 3.33");
    expect(formatRr(null)).toBe("—");
  });

  it("picks a color class from the sign", () => {
    expect(pnlClass("7.09")).toBe("pos");
    expect(pnlClass("-0.5")).toBe("neg");
    expect(pnlClass("0.00")).toBe("");
    expect(pnlClass(null)).toBe("");
  });
});

describe("round2", () => {
  it.each([
    ["10.90559933", "10.91"],
    ["-22.7688", "-22.77"],
    ["2.555", "2.56"], // half rounds away from zero
    ["-2.555", "-2.56"],
    ["9.999", "10"],
    ["0.004", "0"],
    ["-0.004", "0"],
    ["2.501", "2.5"],
    ["4", "4"],
    ["4.1", "4.1"],
    ["12.75", "12.75"],
  ])("%s → %s", (d, out) => {
    expect(round2(d)).toBe(out);
  });
});

describe("usd", () => {
  it.each([
    ["65.67", {}, "+$65.67"],
    ["-23.21", {}, "-$23.21"],
    ["0", {}, "$0"],
    ["-0.004", { round: true }, "$0"],
    ["1234567.891", { round: true }, "+$1,234,567.89"],
    ["-1500", {}, "-$1,500"],
    ["71.51", { sign: false }, "$71.51"],
    ["12.3456789", {}, "+$12.3456789"], // exact unless rounded
  ] as const)("%s %o → %s", (d, opts, out) => {
    expect(usd(d, opts)).toBe(out);
  });

  it("axis ticks", () => {
    expect(usdTick(300)).toBe("$300");
    expect(usdTick(-300)).toBe("-$300");
    expect(usdTick(0)).toBe("$0");
  });
});
