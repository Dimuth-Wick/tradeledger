import { describe, expect, it } from "vitest";
import { backupFileName, csvFileName, onePath } from "./dataFiles";

describe("data files", () => {
  it("suggests dated file names", () => {
    const d = new Date(2026, 10, 19, 23, 59); // local time
    expect(backupFileName(d)).toBe("tradeledger-backup-2026-11-19.zip");
    expect(csvFileName(d)).toBe("tradeledger-trades-2026-11-19.csv");
  });

  it("takes one path from an Open dialog", () => {
    expect(onePath(null)).toBeNull();
    expect(onePath("C:/a.zip")).toBe("C:/a.zip");
    expect(onePath(["C:/a.zip", "C:/b.zip"])).toBe("C:/a.zip");
    expect(onePath([])).toBeNull();
  });
});
