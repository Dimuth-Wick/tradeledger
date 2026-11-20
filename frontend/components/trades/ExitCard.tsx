import { useEffect, useMemo, useState } from "react";
import {
  closeTrade,
  previewClose,
  type Trade,
  type WithId,
} from "../../connection/backend";
import { exitDirty, exitFormFrom, toCloseInput } from "../../logic/exitForm";
import { formatR, pnlClass, usd } from "../../logic/format";
import { useUnsavedChanges } from "../../logic/unsaved";

const PREVIEW_DELAY_MS = 250;

/** Exit price for a saved manual trade: close it later, change the exit,
 *  or clear the price to reopen it. Profit and R come from Rust. */
export default function ExitCard({
  trade,
  onSaved,
}: {
  trade: WithId<Trade>;
  onSaved: (t: WithId<Trade>) => void;
}) {
  const t = trade;
  const [form, setForm] = useState(() => exitFormFrom(t));
  const [preview, setPreview] = useState<{
    key: string;
    trade?: Trade;
    error?: string;
  } | null>(null);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(
    null,
  );

  const parsed = useMemo(() => toCloseInput(form), [form]);
  const dirty = exitDirty(form, t);
  useUnsavedChanges(dirty);
  const key = parsed.ok ? JSON.stringify(parsed.input) : null;

  useEffect(() => {
    if (!parsed.ok || !dirty || parsed.input.exitPrice === null) return;
    const k = JSON.stringify(parsed.input);
    let cancelled = false;
    const timer = setTimeout(() => {
      previewClose(t.id, parsed.input).then(
        (p) => !cancelled && setPreview({ key: k, trade: p }),
        (err) => !cancelled && setPreview({ key: k, error: String(err) }),
      );
    }, PREVIEW_DELAY_MS);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [parsed, dirty, t.id]);

  const current = dirty && preview?.key === key ? preview : null;
  // Before any edit, a closed trade shows its saved result.
  const shown: Trade | null =
    current?.trade ?? (!dirty && t.status === "closed" ? t : null);
  const error = !parsed.ok ? parsed.error : (current?.error ?? null);
  const reopening = parsed.ok && parsed.input.exitPrice === null;
  const canSave = dirty && !saving && parsed.ok && !current?.error;

  async function save() {
    if (!parsed.ok) return;
    setSaving(true);
    setMessage(null);
    try {
      const saved = await closeTrade(t.id, parsed.input);
      onSaved(saved);
      setForm(exitFormFrom(saved));
      setPreview(null);
      setMessage({
        ok: true,
        text: saved.status === "closed" ? "Trade closed" : "Trade reopened",
      });
    } catch (err) {
      setMessage({ ok: false, text: String(err) });
    } finally {
      setSaving(false);
    }
  }

  return (
    <section className="card exit-card">
      <h2>{t.status === "open" ? "Close trade" : "Exit"}</h2>
      <p className="hint">
        {t.status === "open"
          ? "Enter the exit price when you close the trade. The profit or loss is worked out from your entry."
          : "Change the exit price, or clear it to mark the trade open again."}
      </p>
      <div className="row">
        <label>
          Exit price
          <input
            inputMode="decimal"
            value={form.exitPrice}
            placeholder={`entry was ${t.avgEntry}`}
            onChange={(e) => {
              setForm((f) => ({ ...f, exitPrice: e.target.value }));
              setMessage(null);
            }}
          />
        </label>
        <label>
          Closed on
          <input
            type="datetime-local"
            value={form.closeTime}
            onChange={(e) => {
              setForm((f) => ({ ...f, closeTime: e.target.value }));
              setMessage(null);
            }}
          />
        </label>
      </div>

      {shown && shown.status === "closed" && (
        <div className="plan-summary">
          <dl className="exit-result">
            <div>
              <dt>Profit / loss at exit</dt>
              <dd className={pnlClass(shown.netPnl)}>
                {usd(shown.netPnl, { round: true })}
              </dd>
            </div>
            <div>
              <dt>R-multiple</dt>
              <dd className={pnlClass(shown.rMultiple)}>
                {formatR(shown.rMultiple)}
              </dd>
            </div>
          </dl>
        </div>
      )}
      {error && <p className="status error">{error}</p>}

      <div className="actions">
        {message && (
          <span className={`status ${message.ok ? "ok" : "error"}`}>
            {message.text}
          </span>
        )}
        <button
          type="button"
          className="primary"
          disabled={!canSave}
          onClick={() => void save()}
        >
          {saving
            ? "Saving…"
            : reopening && t.status === "closed"
              ? "Reopen trade"
              : t.status === "open"
                ? "Close trade"
                : "Save exit"}
        </button>
      </div>
    </section>
  );
}
