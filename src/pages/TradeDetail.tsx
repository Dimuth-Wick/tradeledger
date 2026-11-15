import { useEffect, useMemo, useState } from "react";
import Notes from "../components/Notes";
import { FillsPanel } from "../components/TradeList";
import {
  getTrade,
  previewJournal,
  updateJournal,
  type Trade,
  type WithId,
} from "../lib/api";
import {
  formatDateTime,
  formatR,
  formatRr,
  pnlClass,
  signed,
} from "../lib/format";
import {
  formFromTrade,
  isDirty,
  toJournalInput,
  type JournalForm,
} from "../lib/journalForm";
import { useUnsavedChanges } from "../lib/unsaved";

const PREVIEW_DELAY_MS = 250;

interface Props {
  tradeId: number;
  onBack: () => void;
}

export default function TradeDetail({ tradeId, onBack }: Props) {
  const [trade, setTrade] = useState<WithId<Trade> | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    getTrade(tradeId).then(
      (t) => {
        if (cancelled) return;
        if (t) setTrade(t);
        else setLoadError(`Trade ${tradeId} not found`);
      },
      (err) => !cancelled && setLoadError(String(err)),
    );
    return () => {
      cancelled = true;
    };
  }, [tradeId]);

  return (
    <main className="page trade-detail">
      <button type="button" className="link" onClick={onBack}>
        ← Back to trades
      </button>
      {loadError && <p className="status error">{loadError}</p>}
      {/* key: a fresh editor per loaded trade, so the form starts from it */}
      {trade && <Detail key={trade.id} trade={trade} onSaved={setTrade} />}
    </main>
  );
}

function Detail({
  trade,
  onSaved,
}: {
  trade: WithId<Trade>;
  onSaved: (t: WithId<Trade>) => void;
}) {
  const t = trade;
  const [form, setForm] = useState<JournalForm>(() => formFromTrade(t));
  const [notesTab, setNotesTab] = useState<"write" | "preview">(
    t.notes ? "preview" : "write",
  );
  const [preview, setPreview] = useState<{
    key: string;
    trade?: Trade;
    error?: string;
  } | null>(null);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(
    null,
  );

  const parsed = useMemo(() => toJournalInput(form), [form]);
  const inputKey = parsed.ok ? JSON.stringify(parsed.input) : null;
  const dirty = isDirty(form, t);
  useUnsavedChanges(dirty);

  // Live R:R / R from Rust while editing the plan (same rules as saving).
  useEffect(() => {
    if (!parsed.ok || !dirty) return;
    const key = JSON.stringify(parsed.input);
    let cancelled = false;
    const timer = setTimeout(() => {
      previewJournal(t.id, parsed.input).then(
        (p) => !cancelled && setPreview({ key, trade: p }),
        (err) => !cancelled && setPreview({ key, error: String(err) }),
      );
    }, PREVIEW_DELAY_MS);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [parsed, dirty, t.id]);

  const current = dirty && preview?.key === inputKey ? preview : null;
  const shown = current?.trade ?? t;
  const planError = !parsed.ok ? parsed.error : (current?.error ?? null);
  const canSave = dirty && parsed.ok && !planError && !saving;

  function set<K extends keyof JournalForm>(key: K, value: string) {
    setForm((f) => ({ ...f, [key]: value }));
    setMessage(null);
  }

  async function save() {
    if (!parsed.ok) return;
    setSaving(true);
    try {
      const saved = await updateJournal(t.id, parsed.input);
      onSaved(saved);
      setForm(formFromTrade(saved));
      setMessage({ ok: true, text: "Saved" });
    } catch (err) {
      setMessage({ ok: false, text: String(err) });
    } finally {
      setSaving(false);
    }
  }

  return (
    <>
      <h1>
        {t.symbol}{" "}
        <span className={t.direction}>
          {t.direction === "long" ? "Long" : "Short"}
        </span>{" "}
        <span className="hint">
          {t.market === "usdm" ? "Futures" : "Spot"} ·{" "}
          {t.status === "open" ? "Open" : "Closed"}
          {t.isManual ? " · manual" : ""}
        </span>
      </h1>
      <p className="hint">
        Opened {formatDateTime(t.openTime)}
        {t.closeTime !== null && ` · closed ${formatDateTime(t.closeTime)}`}
      </p>

      <dl className="stats">
        <Stat label="Quantity" value={t.qty} />
        <Stat label="Avg entry" value={t.avgEntry} />
        <Stat label="Avg exit" value={t.avgExit ?? "—"} />
        <Stat
          label="Gross PnL"
          value={signed(t.grossPnl)}
          cls={pnlClass(t.grossPnl)}
        />
        <Stat label="Fees" value={t.fees} />
        <Stat label="Funding" value={signed(t.funding)} />
        <Stat
          label="Net PnL"
          value={signed(t.netPnl)}
          cls={pnlClass(t.netPnl)}
        />
        <Stat label="Planned R:R" value={formatRr(shown.plannedRr)} />
        <Stat
          label="R-multiple"
          value={formatR(shown.rMultiple)}
          cls={pnlClass(shown.rMultiple)}
        />
      </dl>

      <section className="card">
        <h2>Plan</h2>
        <div className="row">
          <label>
            Planned entry
            <input
              inputMode="decimal"
              value={form.plannedEntry}
              placeholder={`= avg entry ${t.avgEntry}`}
              onChange={(e) => set("plannedEntry", e.target.value)}
            />
          </label>
          <label>
            Stop loss
            <input
              inputMode="decimal"
              value={form.stopLoss}
              onChange={(e) => set("stopLoss", e.target.value)}
            />
          </label>
          <label>
            Take profit
            <input
              inputMode="decimal"
              value={form.takeProfit}
              onChange={(e) => set("takeProfit", e.target.value)}
            />
          </label>
          <label>
            Leverage
            <input
              inputMode="numeric"
              value={form.leverage}
              placeholder="e.g. 10"
              onChange={(e) => set("leverage", e.target.value)}
            />
          </label>
        </div>
        {planError && <p className="status error">{planError}</p>}
      </section>

      <section className="card">
        <div className="notes-head">
          <h2>Notes</h2>
          <div className="direction" role="tablist" aria-label="Notes view">
            {(["write", "preview"] as const).map((tab) => (
              <button
                key={tab}
                type="button"
                role="tab"
                aria-selected={notesTab === tab}
                className={notesTab === tab ? "active long" : ""}
                onClick={() => setNotesTab(tab)}
              >
                {tab === "write" ? "Write" : "Preview"}
              </button>
            ))}
          </div>
        </div>
        {notesTab === "write" ? (
          <textarea
            className="notes-input"
            aria-label="Notes"
            rows={10}
            value={form.notes}
            placeholder={
              "Why did you take it? What went well or wrong?\n\n" +
              "Markdown works: **bold**, - lists, ## headings, [links](https://…)"
            }
            onChange={(e) => set("notes", e.target.value)}
          />
        ) : (
          <Notes text={form.notes} />
        )}
      </section>

      <div className="actions">
        {message && (
          <span className={`status ${message.ok ? "ok" : "error"}`}>
            {message.text}
          </span>
        )}
        <button
          type="button"
          disabled={!dirty || saving}
          onClick={() => {
            setForm(formFromTrade(t));
            setMessage(null);
          }}
        >
          Discard changes
        </button>
        <button
          type="button"
          className="primary"
          disabled={!canSave}
          onClick={save}
        >
          {saving ? "Saving…" : "Save"}
        </button>
      </div>

      {!t.isManual && (
        <section className="card">
          <h2>Fills</h2>
          <FillsPanel tradeId={t.id} />
        </section>
      )}
    </>
  );
}

function Stat({
  label,
  value,
  cls,
}: {
  label: string;
  value: string;
  cls?: string;
}) {
  return (
    <div>
      <dt>{label}</dt>
      <dd className={cls}>{value}</dd>
    </div>
  );
}
