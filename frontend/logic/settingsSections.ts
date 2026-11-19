// The parts of the Settings page, in the order of its side menu.

export type SettingsSection =
  "appearance" | "tags" | "fields" | "data" | "about";

export const SETTINGS_SECTIONS: readonly [SettingsSection, string][] = [
  ["appearance", "Appearance"],
  ["tags", "Tags"],
  ["fields", "Fields"],
  ["data", "Your data"],
  ["about", "About & updates"],
];

const STORE_KEY = "tradeledger.settingsSection";

/** Turns a stored value back into a section; anything unknown → Appearance. */
export function toSection(value: string | null | undefined): SettingsSection {
  return SETTINGS_SECTIONS.some(([id]) => id === value)
    ? (value as SettingsSection)
    : "appearance";
}

/** The section open last time (so Settings reopens where you left it). */
export function loadSection(): SettingsSection {
  try {
    return toSection(sessionStorage.getItem(STORE_KEY));
  } catch {
    return "appearance";
  }
}

export function saveSection(section: SettingsSection): void {
  try {
    sessionStorage.setItem(STORE_KEY, section);
  } catch {
    // Storage blocked: Settings just opens on Appearance next time.
  }
}
