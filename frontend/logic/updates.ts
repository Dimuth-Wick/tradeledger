// Text for the update pop-up and Settings → About. No Tauri calls here.

import type { UpdateStep } from "../connection/backend";

const RELEASES = "https://github.com/Dimuth-Wick/tradeledger/releases";

/** The GitHub page with that version's notes. */
export function releaseUrl(version: string): string {
  return `${RELEASES}/tag/v${version.replace(/^v/, "")}`;
}

/** 1536 → "1.5 KB", 3_250_000 → "3.1 MB". */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** Whole percent 0–100, or null when the size is unknown. */
export function percent(done: number, total: number | null): number | null {
  if (!total || total <= 0) return null;
  return Math.min(100, Math.floor((done / total) * 100));
}

/** What the pop-up says while an update installs. */
export function stepLabel(step: UpdateStep): string {
  switch (step.step) {
    case "backup":
      return "Backing up your data…";
    case "download": {
      const pct = percent(step.done, step.total);
      return pct === null
        ? `Downloading… ${formatBytes(step.done)}`
        : `Downloading… ${pct}%`;
    }
    case "install":
      return "Installing… the app will restart.";
  }
}

/** Turns an updater error into one line the user can act on. */
export function updateErrorText(err: unknown): string {
  const text = String(err instanceof Error ? err.message : err);
  if (/network|connect|dns|timed? ?out|offline/i.test(text)) {
    return "Couldn't reach GitHub. Check your internet connection and try again.";
  }
  if (/release JSON/i.test(text)) {
    return "No update information on GitHub yet. Try again later.";
  }
  if (/signature/i.test(text)) {
    return "The download failed its safety check, so nothing was installed.";
  }
  return `Update failed: ${text}`;
}
