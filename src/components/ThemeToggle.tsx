import { useEffect, useState } from "react";
import { getTheme, setTheme, type Theme } from "../lib/api";
import {
  THEME_EVENT,
  switchTheme,
  shownMode,
  systemPrefersDark,
  toggledMode,
} from "../lib/theme";

/** Sun/moon button in the tab bar: one click flips light ↔ dark. */
export default function ThemeToggle() {
  const [theme, setThemeState] = useState<Theme | null>(null);
  const [systemDark, setSystemDark] = useState(systemPrefersDark);

  useEffect(() => {
    getTheme().then(setThemeState, () => {});
    // Stay in sync when the Settings page changes the theme…
    const onTheme = (e: Event) =>
      setThemeState((e as CustomEvent<Theme>).detail);
    window.addEventListener(THEME_EVENT, onTheme);
    // …and when Windows switches light/dark (matters for "Follow Windows").
    const mq = window.matchMedia?.("(prefers-color-scheme: dark)");
    const onSystem = () => setSystemDark(mq?.matches ?? false);
    mq?.addEventListener("change", onSystem);
    return () => {
      window.removeEventListener(THEME_EVENT, onTheme);
      mq?.removeEventListener("change", onSystem);
    };
  }, []);

  if (!theme) return null;
  const dark = shownMode(theme.mode, systemDark) === "dark";
  const label = dark ? "Switch to light mode" : "Switch to dark mode";

  return (
    <button
      type="button"
      className="theme-toggle"
      aria-label={label}
      title={label}
      onClick={() =>
        setTheme(toggledMode(theme.mode, systemDark), theme.accent).then(
          switchTheme,
          () => {},
        )
      }
    >
      {/* Shows what you switch TO: a sun in dark mode, a moon in light
          mode. Both stay rendered so CSS can animate between them. */}
      <SunIcon hidden={!dark} />
      <MoonIcon hidden={dark} />
    </button>
  );
}

function SunIcon({ hidden }: { hidden: boolean }) {
  return (
    <svg
      className={`icon sun${hidden ? " hidden" : ""}`}
      viewBox="0 0 24 24"
      width="18"
      height="18"
      aria-hidden="true"
    >
      <circle cx="12" cy="12" r="4.5" fill="currentColor" />
      <g stroke="currentColor" strokeWidth="2" strokeLinecap="round">
        <line x1="12" y1="2" x2="12" y2="4.5" />
        <line x1="12" y1="19.5" x2="12" y2="22" />
        <line x1="2" y1="12" x2="4.5" y2="12" />
        <line x1="19.5" y1="12" x2="22" y2="12" />
        <line x1="4.9" y1="4.9" x2="6.6" y2="6.6" />
        <line x1="17.4" y1="17.4" x2="19.1" y2="19.1" />
        <line x1="4.9" y1="19.1" x2="6.6" y2="17.4" />
        <line x1="17.4" y1="6.6" x2="19.1" y2="4.9" />
      </g>
    </svg>
  );
}

function MoonIcon({ hidden }: { hidden: boolean }) {
  return (
    <svg
      className={`icon moon${hidden ? " hidden" : ""}`}
      viewBox="0 0 24 24"
      width="18"
      height="18"
      aria-hidden="true"
    >
      <path
        fill="currentColor"
        d="M20.5 14.4A8.5 8.5 0 0 1 9.6 3.5a8.5 8.5 0 1 0 10.9 10.9Z"
      />
    </svg>
  );
}
