import { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  backupNow,
  dbInfo,
  exportTradesCsv,
  inspectBackup,
  restoreBackup,
  type BackupManifest,
} from "../../connection/backend";
import {
  BACKUP_FILTER,
  CSV_FILTER,
  backupFileName,
  csvFileName,
  onePath,
} from "../../logic/dataFiles";
import { formatDateTime } from "../../logic/format";

const plural = (n: number, w: string) => `${n} ${w}${n === 1 ? "" : "s"}`;

/** Back up, restore and export. Everything happens in Rust. */
export default function DataSettings() {
  const [folder, setFolder] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(
    null,
  );
  // A backup picked for restore, waiting for the user to confirm.
  const [pending, setPending] = useState<{
    path: string;
    manifest: BackupManifest;
  } | null>(null);

  useEffect(() => {
    dbInfo().then(
      (i) => setFolder(i.path.replace(/[\\/][^\\/]+$/, "")),
      () => setFolder(null),
    );
  }, []);

  async function run(task: () => Promise<void>) {
    setBusy(true);
    setMessage(null);
    try {
      await task();
    } catch (err) {
      setMessage({ ok: false, text: String(err) });
    } finally {
      setBusy(false);
    }
  }

  const backUp = () =>
    run(async () => {
      const path = await save({
        defaultPath: backupFileName(new Date()),
        filters: [BACKUP_FILTER],
      });
      if (!path) return;
      const m = await backupNow(path);
      setMessage({
        ok: true,
        text: `Backed up ${plural(m.trades, "trade")} and ${plural(m.screenshots, "screenshot")} to ${path}`,
      });
    });

  const pickRestore = () =>
    run(async () => {
      const path = onePath(
        await open({ multiple: false, filters: [BACKUP_FILTER] }),
      );
      if (!path) return;
      setPending({ path, manifest: await inspectBackup(path) });
    });

  const confirmRestore = () =>
    run(async () => {
      if (!pending) return;
      const r = await restoreBackup(pending.path);
      setPending(null);
      setMessage({
        ok: true,
        text: `Restored ${plural(r.restored.trades, "trade")}. Your previous data was saved to ${r.safetyBackup}. Reloading…`,
      });
      // Every page re-reads the restored data.
      setTimeout(() => window.location.reload(), 1800);
    });

  const exportAll = () =>
    run(async () => {
      const path = await save({
        defaultPath: csvFileName(new Date()),
        filters: [CSV_FILTER],
      });
      if (!path) return;
      const rows = await exportTradesCsv({}, path);
      setMessage({
        ok: true,
        text: `Exported ${plural(rows, "trade")} to ${path}`,
      });
    });

  return (
    <div className="card data-settings">
      <h3>Your data</h3>
      <p className="hint">
        Everything stays on this computer
        {folder && (
          <>
            , in <code>{folder}</code>
          </>
        )}
        . A backup is one .zip file with your trades, journal and screenshots;
        keep it somewhere safe, like a USB drive or cloud folder.
      </p>

      <div className="data-actions">
        <button
          type="button"
          className="primary"
          disabled={busy}
          onClick={backUp}
        >
          Back up now
        </button>
        <button type="button" disabled={busy} onClick={pickRestore}>
          Restore from backup…
        </button>
        <button type="button" disabled={busy} onClick={exportAll}>
          Export all trades (CSV)
        </button>
      </div>

      {pending && (
        <div
          className="restore-confirm"
          role="alertdialog"
          aria-label="Confirm restore"
        >
          <p>
            <strong>Restore this backup?</strong> It's from{" "}
            {formatDateTime(pending.manifest.createdAt)} and holds{" "}
            {plural(pending.manifest.trades, "trade")} and{" "}
            {plural(pending.manifest.screenshots, "screenshot")}.
          </p>
          <p className="hint">
            It replaces everything in TradeLedger now. Your current data is
            saved first as a safety copy, so you can go back.
          </p>
          <div className="data-actions">
            <button
              type="button"
              className="danger"
              disabled={busy}
              onClick={confirmRestore}
            >
              Restore
            </button>
            <button
              type="button"
              disabled={busy}
              onClick={() => setPending(null)}
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {busy && <p className="hint">Working…</p>}
      {message && (
        <p className={`status ${message.ok ? "ok" : "error"}`}>
          {message.text}
        </p>
      )}
    </div>
  );
}
