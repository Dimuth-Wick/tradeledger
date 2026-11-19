import { useEffect, useState } from "react";
import {
  appVersion,
  checkForUpdate,
  type AvailableUpdate,
} from "../../connection/backend";
import { updateErrorText } from "../../logic/updates";
import UpdateDialog from "../shared/UpdateDialog";

/** The running version and a "Check for updates" button. */
export default function AboutSettings() {
  const [version, setVersion] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(
    null,
  );
  const [update, setUpdate] = useState<AvailableUpdate | null>(null);

  useEffect(() => {
    appVersion().then(setVersion, () => setVersion(null));
  }, []);

  async function checkNow() {
    setChecking(true);
    setMessage(null);
    try {
      const found = await checkForUpdate();
      if (found) setUpdate(found);
      else setMessage({ ok: true, text: "You have the latest version." });
    } catch (err) {
      setMessage({ ok: false, text: updateErrorText(err) });
    } finally {
      setChecking(false);
    }
  }

  return (
    <div className="card">
      <h3>About</h3>
      <p>
        TradeLedger <strong>{version ?? "…"}</strong>
      </p>
      <p className="hint">
        The app checks for a new version each time it opens. Updates come from
        the project's GitHub Releases and are signed, so only real TradeLedger
        updates can install.
      </p>
      <div className="data-actions">
        <button
          type="button"
          className="primary"
          disabled={checking}
          onClick={() => void checkNow()}
        >
          {checking ? "Checking…" : "Check for updates"}
        </button>
      </div>
      {message && (
        <p className={`status ${message.ok ? "ok" : "error"}`}>
          {message.text}
        </p>
      )}
      {update && (
        <UpdateDialog
          update={update}
          current={version}
          onClose={() => setUpdate(null)}
        />
      )}
    </div>
  );
}
