import { describe, expect, it } from "vitest";
import { applyTheme, parseCached, type ThemeTarget } from "./theme";

function fakeRoot() {
  const attrs: Record<string, string> = {};
  const vars: Record<string, string> = {};
  const root: ThemeTarget = {
    setAttribute: (n, v) => {
      attrs[n] = v;
    },
    removeAttribute: (n) => {
      delete attrs[n];
    },
    style: {
      setProperty: (n, v) => {
        vars[n] = v;
      },
    },
  };
  return { root, attrs, vars };
}

describe("applyTheme", () => {
  it("dark sets data-theme and the accent variables", () => {
    const { root, attrs, vars } = fakeRoot();
    applyTheme({ mode: "dark", accent: "#d99a00", onAccent: "#000000" }, root);
    expect(attrs["data-theme"]).toBe("dark");
    expect(vars).toEqual({ "--accent": "#d99a00", "--on-accent": "#000000" });
  });

  it("system removes data-theme so the OS setting decides", () => {
    const { root, attrs } = fakeRoot();
    applyTheme({ mode: "light", accent: "#396cd8", onAccent: "#ffffff" }, root);
    applyTheme(
      { mode: "system", accent: "#396cd8", onAccent: "#ffffff" },
      root,
    );
    expect(attrs["data-theme"]).toBeUndefined();
  });
});

describe("parseCached", () => {
  it("accepts a well-formed theme", () => {
    const t = { mode: "dark", accent: "#2e8540", onAccent: "#ffffff" };
    expect(parseCached(JSON.stringify(t))).toEqual(t);
  });

  it.each([
    null,
    "",
    "not json",
    '{"mode":"neon","accent":"#2e8540","onAccent":"#ffffff"}',
    '{"mode":"dark","accent":"red","onAccent":"#ffffff"}',
    '{"mode":"dark","accent":"#2e8540"}',
  ])("rejects %s", (text) => {
    expect(parseCached(text)).toBeNull();
  });
});
