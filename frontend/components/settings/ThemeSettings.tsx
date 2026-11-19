import { useEffect, useRef, useState } from "react";
import Segmented from "../shared/Segmented";
import {
  getTheme,
  listAccentPresets,
  setTheme,
  type AccentPreset,
  type Theme,
  type ThemeMode,
} from "../../connection/backend";
import { THEME_EVENT, switchTheme } from "../../logic/theme";

const MODES: [ThemeMode, string][] = [
  ["system", "Follow Windows"],
  ["light", "Light"],
  ["dark", "Dark"],
];

/** Settings → Appearance: light/dark mode and the accent color. Changes
 *  apply and save at once. */
export default function ThemeSettings() {
  const [theme, setThemeState] = useState<Theme | null>(null);
  const [presets, setPresets] = useState<AccentPreset[]>([]);
  const [error, setError] = useState<string | null>(null);
  // Only the newest answer is applied (the color picker sends many).
  const latest = useRef(0);

  useEffect(() => {
    getTheme().then(setThemeState, (err) => setError(String(err)));
    listAccentPresets().then(setPresets, () => setPresets([]));
    // The tab-bar toggle can change the theme while this page is open.
    const onTheme = (e: Event) =>
      setThemeState((e as CustomEvent<Theme>).detail);
    window.addEventListener(THEME_EVENT, onTheme);
    return () => window.removeEventListener(THEME_EVENT, onTheme);
  }, []);

  function change(mode: ThemeMode, accent: string) {
    const id = ++latest.current;
    setTheme(mode, accent).then(
      (t) => {
        if (id !== latest.current) return;
        setThemeState(t);
        switchTheme(t);
        setError(null);
      },
      (err) => id === latest.current && setError(String(err)),
    );
  }

  return (
    <div className="card">
      <h3>Appearance</h3>
      {error && <p className="status error">{error}</p>}
      {theme && (
        <>
          <div className="setting-row">
            <span className="setting-label">Colors</span>
            <Segmented
              label="Color mode"
              options={MODES}
              value={theme.mode}
              onChange={(id) => change(id, theme.accent)}
            />
          </div>

          <div className="setting-row">
            <span className="setting-label">Accent</span>
            <div className="swatches" role="group" aria-label="Accent color">
              {presets.map((p) => (
                <button
                  key={p.hex}
                  type="button"
                  className="swatch"
                  title={p.name}
                  aria-label={`Accent ${p.name}`}
                  aria-pressed={theme.accent === p.hex}
                  style={{ background: p.hex }}
                  onClick={() => change(theme.mode, p.hex)}
                />
              ))}
              <label className="inline custom-color">
                Custom
                <input
                  type="color"
                  aria-label="Custom accent color"
                  value={theme.accent}
                  onChange={(e) => change(theme.mode, e.target.value)}
                />
              </label>
            </div>
          </div>

          <div className="setting-row">
            <span className="setting-label">Preview</span>
            <div className="theme-preview">
              <button type="button" className="primary">
                Primary button
              </button>
              <span className="pos">+12.50 win</span>
              <span className="neg">-4.20 loss</span>
              <a href="#settings" onClick={(e) => e.preventDefault()}>
                A link
              </a>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
