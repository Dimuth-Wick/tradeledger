import { useEffect, useRef, useState } from "react";
import {
  getTheme,
  listAccentPresets,
  setTheme,
  type AccentPreset,
  type Theme,
  type ThemeMode,
} from "../lib/api";
import { rememberTheme } from "../lib/theme";

const MODES: [ThemeMode, string][] = [
  ["system", "Follow Windows"],
  ["light", "Light"],
  ["dark", "Dark"],
];

/** App preferences. Changes apply and save at once. */
export default function SettingsPage() {
  const [theme, setThemeState] = useState<Theme | null>(null);
  const [presets, setPresets] = useState<AccentPreset[]>([]);
  const [error, setError] = useState<string | null>(null);
  // Only the newest answer is applied (the color picker sends many).
  const latest = useRef(0);

  useEffect(() => {
    getTheme().then(setThemeState, (err) => setError(String(err)));
    listAccentPresets().then(setPresets, () => setPresets([]));
  }, []);

  function change(mode: ThemeMode, accent: string) {
    const id = ++latest.current;
    setTheme(mode, accent).then(
      (t) => {
        if (id !== latest.current) return;
        setThemeState(t);
        rememberTheme(t);
        setError(null);
      },
      (err) => id === latest.current && setError(String(err)),
    );
  }

  return (
    <section className="settings-page">
      <h2>Settings</h2>
      {error && <p className="status error">{error}</p>}

      {theme && (
        <div className="card">
          <h3>Theme</h3>
          <div className="setting-row">
            <span className="setting-label">Colors</span>
            <div className="segmented" role="group" aria-label="Color mode">
              {MODES.map(([id, label]) => (
                <button
                  key={id}
                  type="button"
                  aria-pressed={theme.mode === id}
                  className={theme.mode === id ? "active" : ""}
                  onClick={() => change(id, theme.accent)}
                >
                  {label}
                </button>
              ))}
            </div>
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
        </div>
      )}
    </section>
  );
}
