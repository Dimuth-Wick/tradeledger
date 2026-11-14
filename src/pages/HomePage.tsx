import { useEffect, useState } from "react";
import { dbInfo, ping, type DbInfo } from "../lib/api";

type Status =
  | { kind: "idle" }
  | { kind: "ok"; message: string }
  | { kind: "error"; message: string };

export default function HomePage() {
  const [status, setStatus] = useState<Status>({ kind: "idle" });
  const [db, setDb] = useState<DbInfo | null>(null);
  const [dbError, setDbError] = useState<string | null>(null);

  // Load once on start so the user can see where their data is stored.
  useEffect(() => {
    dbInfo()
      .then(setDb)
      .catch((err) => setDbError(String(err)));
  }, []);

  async function checkCore() {
    try {
      setStatus({ kind: "ok", message: await ping() });
    } catch (err) {
      setStatus({ kind: "error", message: String(err) });
    }
  }

  return (
    <main className="container">
      <h1>TradeLedger</h1>
      <p>Offline crypto trading journal</p>

      <button type="button" onClick={checkCore}>
        Ping Rust core
      </button>

      {status.kind === "ok" && <p className="status ok">{status.message}</p>}
      {status.kind === "error" && (
        <p className="status error">Error: {status.message}</p>
      )}

      {db && (
        <p className="db-info">
          Database: {db.path} (schema v{db.schemaVersion})
        </p>
      )}
      {dbError && <p className="status error">Database error: {dbError}</p>}
    </main>
  );
}
