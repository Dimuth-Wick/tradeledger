import { afterEach, describe, expect, it, vi } from "vitest";
import {
  SETTINGS_SECTIONS,
  loadSection,
  saveSection,
  toSection,
} from "./settingsSections";

describe("settings sections", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("lists each section once, Appearance first", () => {
    const ids = SETTINGS_SECTIONS.map(([id]) => id);
    expect(ids[0]).toBe("appearance");
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids).toEqual(["appearance", "tags", "fields", "data"]);
  });

  it("keeps known values and falls back to Appearance", () => {
    expect(toSection("tags")).toBe("tags");
    expect(toSection("data")).toBe("data");
    expect(toSection("nope")).toBe("appearance");
    expect(toSection(null)).toBe("appearance");
    expect(toSection(undefined)).toBe("appearance");
  });

  it("remembers the last section", () => {
    const store = new Map<string, string>();
    vi.stubGlobal("sessionStorage", {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => store.set(k, v),
    });
    expect(loadSection()).toBe("appearance");
    saveSection("fields");
    expect(loadSection()).toBe("fields");
  });

  it("works when storage is blocked", () => {
    vi.stubGlobal("sessionStorage", {
      getItem: () => {
        throw new Error("blocked");
      },
      setItem: () => {
        throw new Error("blocked");
      },
    });
    expect(() => saveSection("tags")).not.toThrow();
    expect(loadSection()).toBe("appearance");
  });
});
