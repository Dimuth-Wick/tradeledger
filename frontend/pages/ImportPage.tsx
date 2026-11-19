import { useRef, useState } from "react";
import {
  confirmImport,
  previewImport,
  type ImportPreview,
  type ImportResult,
} from "../connection/backend";
import {
  newRowCount,
  previewWarnings,
  resultSummary,
} from "../logic/importText";

/** Show at most this many row errors; the rest are summarized. */
const MAX_ERRORS_SHOWN = 20;

type State =
  | { step: "pick" }
  | { step: "reading" }
  | {
      step: "preview";
      fileName: string;
      content: string;
      preview: ImportPreview;
    }
  | { step: "importing" }
  | { step: "done"; result: ImportResult };

export default function ImportPage() {
  const [state, setState] = useState<State>({ step: "pick" });
  const [error, setError] = useState<string | null>(null);
  const input = useRef<HTMLInputElement>(null);

  async function onFile(file: File | undefined) {
    if (!file) return;
    setError(null);
    setState({ step: "reading" });
    try {
      // The file is read here and its text sent to Rust; the path never
      // leaves the browser picker, and nothing is copied anywhere.
      const content = await file.text();
      const preview = await previewImport(file.name, content);
      setState({ step: "preview", fileName: file.name, content, preview });
    } catch (err) {
      setError(String(err));
      setState({ step: "pick" });
    } finally {
      // Allow picking the same file again.
      if (input.current) input.current.value = "";
    }
  }

  async function onConfirm(fileName: string, content: string) {
    setError(null);
    setState({ step: "importing" });
    try {
      setState({
        step: "done",
        result: await confirmImport(fileName, content),
      });
    } catch (err) {
      setError(String(err));
      setState({ step: "pick" });
    }
  }

  return (
    <section className="import-page">
      <h2>Import trades</h2>
      <p className="hint">
        Binance exports: spot trade history, USDⓈ-M futures trade history, or
        USDⓈ-M transaction history (funding). The file is only read; nothing is
        saved until you confirm.
      </p>

      {(state.step === "pick" || state.step === "reading") && (
        <label className="file-pick">
          <span>Choose CSV file</span>
          <input
            ref={input}
            type="file"
            accept=".csv,text/csv"
            disabled={state.step === "reading"}
            onChange={(e) => onFile(e.target.files?.[0])}
          />
        </label>
      )}
      {state.step === "reading" && <p className="hint">Reading file…</p>}
      {error && <p className="status error">{error}</p>}

      {state.step === "preview" && (
        <PreviewCard
          preview={state.preview}
          onCancel={() => setState({ step: "pick" })}
          onConfirm={() => onConfirm(state.fileName, state.content)}
        />
      )}

      {state.step === "importing" && <p className="hint">Importing…</p>}

      {state.step === "done" && (
        <div className="card">
          <p className="status ok">{resultSummary(state.result)}</p>
          <button type="button" onClick={() => setState({ step: "pick" })}>
            Import another file
          </button>
        </div>
      )}
    </section>
  );
}

function PreviewCard({
  preview: p,
  onCancel,
  onConfirm,
}: {
  preview: ImportPreview;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const fresh = newRowCount(p);
  const warnings = previewWarnings(p);

  return (
    <div className="card">
      <h3>
        {p.fileName} <span className="hint">· {p.label}</span>
      </h3>
      <dl className="counts">
        <dt>Rows read</dt>
        <dd>{p.fills + p.funding}</dd>
        <dt>Already imported</dt>
        <dd>{p.alreadyImported}</dd>
        <dt>New</dt>
        <dd>{fresh}</dd>
        <dt>Rows with errors</dt>
        <dd className={p.errors.length > 0 ? "neg" : ""}>{p.errors.length}</dd>
      </dl>

      {p.instruments.length > 0 && (
        <table className="trade-list">
          <thead>
            <tr>
              <th>Symbol</th>
              <th>Market</th>
              <th className="num">New rows</th>
              <th className="num">Trades after import</th>
              <th className="num">Still open</th>
            </tr>
          </thead>
          <tbody>
            {p.instruments.map((i) => (
              <tr key={`${i.exchange}/${i.market}/${i.symbol}`}>
                <td>{i.symbol}</td>
                <td>{i.market === "usdm" ? "Futures" : "Spot"}</td>
                <td className="num">{i.newRows}</td>
                <td className="num">{i.tradesAfter}</td>
                <td className="num">{i.openAfter}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {warnings.length > 0 && (
        <ul className="warnings">
          {warnings.map((w) => (
            <li key={w}>{w}</li>
          ))}
        </ul>
      )}

      {p.errors.length > 0 && (
        <details className="row-errors" open={p.errors.length <= 5}>
          <summary>
            {p.errors.length} row(s) can't be read and will be left out
          </summary>
          <ul>
            {p.errors.slice(0, MAX_ERRORS_SHOWN).map((e) => (
              <li key={e.line}>
                Line {e.line}: {e.message}
              </li>
            ))}
            {p.errors.length > MAX_ERRORS_SHOWN && (
              <li>…and {p.errors.length - MAX_ERRORS_SHOWN} more</li>
            )}
          </ul>
        </details>
      )}

      <div className="actions">
        <button type="button" onClick={onCancel}>
          Cancel
        </button>
        <button
          type="button"
          className="primary"
          disabled={fresh === 0}
          onClick={onConfirm}
        >
          {fresh === 0 ? "Nothing new to import" : `Import ${fresh} new rows`}
        </button>
      </div>
    </div>
  );
}
