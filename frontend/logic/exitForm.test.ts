import { describe, expect, it } from "vitest";
import type { Trade } from "../connection/backend";
import { exitDirty, exitFormFrom, toCloseInput } from "./exitForm";
import { toLocalInputValue } from "./tradeForm";

const open = { avgExit: null, closeTime: null } as unknown as Trade;
const closedAt = new Date(2026, 10, 20, 9, 30).getTime();
const closed = { avgExit: "160", closeTime: closedAt } as unknown as Trade;

describe("exitFormFrom", () => {
  it("starts empty with today for an open trade", () => {
    const now = new Date(2026, 10, 20, 8, 41);
    expect(exitFormFrom(open, now)).toEqual({
      exitPrice: "",
      closeTime: "2026-11-20T08:41",
    });
  });
  it("shows the saved exit for a closed trade", () => {
    expect(exitFormFrom(closed)).toEqual({
      exitPrice: "160",
      closeTime: "2026-11-20T09:30",
    });
  });
});

describe("exitDirty", () => {
  it("notices a new or changed exit", () => {
    const f = exitFormFrom(open);
    expect(exitDirty(f, open)).toBe(false);
    expect(exitDirty({ ...f, exitPrice: "160" }, open)).toBe(true);
    // Moving the date on an open trade with no price changes nothing.
    expect(exitDirty({ ...f, closeTime: "2020-01-01T00:00" }, open)).toBe(
      false,
    );
    const c = exitFormFrom(closed);
    expect(exitDirty(c, closed)).toBe(false);
    expect(exitDirty({ ...c, closeTime: "2026-11-21T09:30" }, closed)).toBe(
      true,
    );
    expect(exitDirty({ ...c, exitPrice: "" }, closed)).toBe(true);
  });
});

describe("toCloseInput", () => {
  it("sends the price and date", () => {
    const at = new Date(2026, 10, 20, 9, 30);
    expect(
      toCloseInput({ exitPrice: " 160 ", closeTime: toLocalInputValue(at) }),
    ).toEqual({
      ok: true,
      input: { exitPrice: "160", closeTime: at.getTime() },
    });
  });
  it("an empty price reopens", () => {
    expect(
      toCloseInput({ exitPrice: "", closeTime: "2026-11-20T09:30" }),
    ).toEqual({ ok: true, input: { exitPrice: null, closeTime: null } });
  });
  it("rejects a non-number", () => {
    const r = toCloseInput({ exitPrice: "1,5", closeTime: "2026-11-20T09:30" });
    expect(r.ok).toBe(false);
  });
});
