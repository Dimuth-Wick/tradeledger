import { useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { AvailableUpdate, UpdateStep } from "../../connection/backend";
import {
  percent,
  releaseUrl,
  stepLabel,
  updateErrorText,
} from "../../logic/updates";

/** "Version X is available": Update now (backup → download → install →
 *  restart) or Later (asks again next time the app opens). */
export default function UpdateDialog({
  update,
  current,
  onClose,
}: {
  update: AvailableUpdate;
  current: string | null;
  onClose: () => void;
}) {
  const [step, setStep] = useState<UpdateStep | null>(null);
  const [error, setError] = useState<string | null>(null);
  const busy = step !== null && error === null;

  async function install() {
    setError(null);
    setStep({ step: "backup" });
    try {
      await update.install(setStep);
    } catch (err) {
      setError(updateErrorText(err));
    }
  }

  const pct = step?.step === "download" ? percent(step.done, step.total) : null;

  return (
    <div className="modal-backdrop">
      <div
        className="modal update-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="update-title"
      >
        <h2 id="update-title">Update available</h2>
        <p>
          TradeLedger <strong>{update.version}</strong> is ready to install
          {current && <> (you have {current})</>}.
        </p>
        <p className="hint">
          Your data is backed up first and stays where it is. The app restarts
          when the update is done.{" "}
          <button
            type="button"
            className="link"
            onClick={() => void openUrl(releaseUrl(update.version))}
          >
            See what's new
          </button>
        </p>

        {busy && step && (
          <div className="update-progress" aria-live="polite">
            <progress max={100} value={pct ?? undefined} />
            <span>{stepLabel(step)}</span>
          </div>
        )}
        {error && <p className="status error">{error}</p>}

        <div className="data-actions">
          <button
            type="button"
            className="primary"
            disabled={busy}
            onClick={() => void install()}
          >
            {error ? "Try again" : "Update now"}
          </button>
          <button type="button" disabled={busy} onClick={onClose}>
            Later
          </button>
        </div>
      </div>
    </div>
  );
}
