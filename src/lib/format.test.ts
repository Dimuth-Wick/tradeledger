import { describe, expect, it } from "vitest";
import { formatR, formatRr, isZero, pnlClass, signed } from "./format";

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
