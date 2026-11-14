import { useState } from "react";
import { ping } from "../lib/api";

type Status =
  | { kind: "idle" }
  | { kind: "ok"; message: string }
  | { kind: "error"; message: string };

export default function HomePage() {
  const [status, setStatus] = useState<Status>({ kind: "idle" });

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
    </main>
  );
}
