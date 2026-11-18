import type { Theme } from "./api";

// Puts the theme Rust returns onto the page: data-theme on <html> picks the
// light or dark colors in App.css, and the accent goes into CSS variables.
// A copy is kept in localStorage so the next start shows the right colors
// before Rust answers (no white flash in dark mode).

const CACHE_KEY = "tradeledger.theme";

/** The bits of <html> we touch (so tests can pass a fake). */
export interface ThemeTarget {
  setAttribute(name: string, value: string): void;
  removeAttribute(name: string): void;
  style: { setProperty(name: string, value: string): void };
}

export function applyTheme(t: Theme, root: ThemeTarget): void {
  if (t.mode === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", t.mode);
  root.style.setProperty("--accent", t.accent);
  root.style.setProperty("--on-accent", t.onAccent);
}

const HEX = /^#[0-9a-f]{6}$/;

/** A cached theme, or null if missing or not in the expected shape. */
export function parseCached(text: string | null): Theme | null {
  if (!text) return null;
  try {
    const t = JSON.parse(text) as Partial<Theme>;
    const modeOk =
      t.mode === "system" || t.mode === "light" || t.mode === "dark";
    if (
      modeOk &&
      typeof t.accent === "string" &&
      HEX.test(t.accent) &&
      typeof t.onAccent === "string" &&
      HEX.test(t.onAccent)
    ) {
      return t as Theme;
    }
  } catch {
    // broken cache: ignore
  }
  return null;
}

/** Apply and remember for the next start. */
export function rememberTheme(t: Theme): void {
  applyTheme(t, document.documentElement);
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify(t));
  } catch {
    // storage may be blocked; the theme still applies this session
  }
}

/** Apply the theme from the last run, if any (called before first render). */
export function applyCachedTheme(): void {
  let text: string | null;
  try {
    text = localStorage.getItem(CACHE_KEY);
  } catch {
    return;
  }
  const t = parseCached(text);
  if (t) applyTheme(t, document.documentElement);
}
