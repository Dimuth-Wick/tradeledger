import { useEffect, useMemo, useState } from "react";
import Notes from "../components/Notes";
import Screenshots from "../components/Screenshots";
import TradeTags from "../components/TradeTags";
import { FillsPanel } from "../components/TradeList";
import {
  getCustomValues,
  getTrade,
  getTradePlaybook,
  listCustomFields,
  listPlaybooks,
  previewJournal,
  setCustomValues,
  setTradePlaybook,
  updateJournal,
  type CustomField,
  type FieldValue,
  type Playbook,
  type TradePlaybook,
  type Trade,
  type WithId,
} from "../lib/api";
import {
  changedValues,
  draftsFrom,
  type FieldDrafts,
} from "../lib/customFields";
import {
  formatDateTime,
  formatR,
  formatRr,
  pnlClass,
  usd,
} from "../lib/format";
import {
  formFromTrade,
  isDirty,
  toJournalInput,
  type JournalForm,
} from "../lib/journalForm";
import { checklistDirty, followedText } from "../lib/playbooks";
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

  // Custom fields: definitions, what's saved, and what's typed.
  const [fields, setFields] = useState<CustomField[]>([]);
  const [savedValues, setSavedValues] = useState<FieldValue[]>([]);
  const [drafts, setDrafts] = useState<FieldDrafts>({});

  // Playbook: the choices, what's saved, and what's picked/ticked now.
  const [playbooks, setPlaybooks] = useState<Playbook[]>([]);
  const [savedPb, setSavedPb] = useState<TradePlaybook>({
    playbookId: null,
    checked: [],
  });
  const [pbId, setPbId] = useState<number | null>(null);
  const [checked, setChecked] = useState<number[]>([]);

  useEffect(() => {
    let cancelled = false;
    Promise.all([listPlaybooks(), getTradePlaybook(t.id)]).then(
      ([list, saved]) => {
        if (cancelled) return;
        setPlaybooks(list);
        setSavedPb(saved);
        setPbId(saved.playbookId);
        setChecked(saved.checked);
      },
      (err) => !cancelled && setMessage({ ok: false, text: String(err) }),
    );
    return () => {
      cancelled = true;
    };
  }, [t.id]);

  useEffect(() => {
    let cancelled = false;
    Promise.all([listCustomFields(), getCustomValues(t.id)]).then(
      ([f, v]) => {
        if (cancelled) return;
        setFields(f);
        setSavedValues(v);
        setDrafts(draftsFrom(f, v));
      },
      (err) => !cancelled && setMessage({ ok: false, text: String(err) }),
    );
    return () => {
      cancelled = true;
    };
  }, [t.id]);

  const parsed = useMemo(() => toJournalInput(form), [form]);
  const inputKey = parsed.ok ? JSON.stringify(parsed.input) : null;
  const journalDirty = isDirty(form, t);
  const changed = changedValues(fields, savedValues, drafts);
  const customDirty = changed.length > 0;
  const pbDirty = checklistDirty(savedPb, pbId, checked);
  const dirty = journalDirty || customDirty || pbDirty;
  useUnsavedChanges(dirty);

  // Live R:R / R from Rust while editing the plan (same rules as saving).
  useEffect(() => {
    if (!parsed.ok || !journalDirty) return;
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
  }, [parsed, journalDirty, t.id]);

  const current = journalDirty && preview?.key === inputKey ? preview : null;
  const shown = current?.trade ?? t;
  const planError = !parsed.ok ? parsed.error : (current?.error ?? null);
  // A broken plan only blocks saving if the plan was actually edited.
  const canSave =
    dirty && !saving && (!journalDirty || (parsed.ok && !planError));

  function set<K extends keyof JournalForm>(key: K, value: string) {
    setForm((f) => ({ ...f, [key]: value }));
    setMessage(null);
  }

  async function save() {
    setSaving(true);
    try {
      if (journalDirty && parsed.ok) {
        const saved = await updateJournal(t.id, parsed.input);
        onSaved(saved);
        setForm(formFromTrade(saved));
      }
      if (customDirty) {
        const values = await setCustomValues(t.id, changed);
        setSavedValues(values);
        setDrafts(draftsFrom(fields, values));
      }
      if (pbDirty) {
        const pb = await setTradePlaybook(t.id, pbId, checked);
        setSavedPb(pb);
        setPbId(pb.playbookId);
        setChecked(pb.checked);
      }
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
          value={usd(t.grossPnl)}
          cls={pnlClass(t.grossPnl)}
        />
        <Stat label="Fees" value={usd(t.fees, { sign: false })} />
        <Stat label="Funding" value={usd(t.funding)} />
        <Stat label="Net PnL" value={usd(t.netPnl)} cls={pnlClass(t.netPnl)} />
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
        <h2>Playbook</h2>
        <PlaybookChecklist
          playbooks={playbooks}
          playbookId={pbId}
          checked={checked}
          onPick={(id) => {
            setPbId(id);
            // Ticks belong to one playbook's rules; going back to the saved
            // playbook restores its saved ticks.
            setChecked(id === savedPb.playbookId ? savedPb.checked : []);
            setMessage(null);
          }}
          onToggle={(ruleId, on) => {
            setChecked((c) =>
              on ? [...c, ruleId] : c.filter((x) => x !== ruleId),
            );
            setMessage(null);
          }}
        />
      </section>

      {fields.length > 0 && (
        <section className="card">
          <h2>Custom fields</h2>
          <div className="custom-fields">
            {fields.map((f) => (
              <CustomFieldInput
                key={f.id}
                field={f}
                value={drafts[f.id] ?? ""}
                onChange={(v) => {
                  setDrafts((d) => ({ ...d, [f.id]: v }));
                  setMessage(null);
                }}
              />
            ))}
          </div>
        </section>
      )}

      <section className="card">
        <h2>Tags</h2>
        <TradeTags tradeId={t.id} />
      </section>

      <section className="card">
        <h2>Screenshots</h2>
        <Screenshots tradeId={t.id} />
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
            setDrafts(draftsFrom(fields, savedValues));
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

/** One custom field, shown as the input that fits its type. */
function CustomFieldInput({
  field,
  value,
  onChange,
}: {
  field: CustomField;
  value: string;
  onChange: (v: string) => void;
}) {
  if (field.fieldType === "checkbox") {
    return (
      <label className="check">
        <input
          type="checkbox"
          checked={value === "true"}
          onChange={(e) => onChange(e.target.checked ? "true" : "")}
        />
        {field.name}
      </label>
    );
  }
  if (field.fieldType === "select") {
    return (
      <label>
        {field.name}
        <select value={value} onChange={(e) => onChange(e.target.value)}>
          <option value="">—</option>
          {field.options.map((o) => (
            <option key={o} value={o}>
              {o}
            </option>
          ))}
        </select>
      </label>
    );
  }
  return (
    <label>
      {field.name}
      <input
        inputMode={field.fieldType === "number" ? "decimal" : undefined}
        value={value}
        maxLength={500}
        onChange={(e) => onChange(e.target.value)}
      />
    </label>
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

function PlaybookChecklist({
  playbooks,
  playbookId,
  checked,
  onPick,
  onToggle,
}: {
  playbooks: Playbook[];
  playbookId: number | null;
  checked: number[];
  onPick: (id: number | null) => void;
  onToggle: (ruleId: number, on: boolean) => void;
}) {
  const pb = playbooks.find((p) => p.id === playbookId);
  if (playbooks.length === 0) {
    return (
      <p className="hint">No playbooks yet. Create one on the Playbooks tab.</p>
    );
  }
  return (
    <div className="playbook-check">
      <label className="inline">
        Setup
        <select
          aria-label="Playbook"
          value={playbookId ?? ""}
          onChange={(e) =>
            onPick(e.target.value === "" ? null : Number(e.target.value))
          }
        >
          <option value="">None</option>
          {playbooks.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      </label>
      {pb && pb.rules.length > 0 && (
        <>
          <ul className="checklist">
            {pb.rules.map((r) => (
              <li key={r.id}>
                <label>
                  <input
                    type="checkbox"
                    checked={checked.includes(r.id)}
                    onChange={(e) => onToggle(r.id, e.target.checked)}
                  />
                  {r.text}
                </label>
              </li>
            ))}
          </ul>
          <p className="hint">
            {followedText(
              pb.rules.filter((r) => checked.includes(r.id)).length,
              pb.rules.length,
            )}
          </p>
        </>
      )}
      {pb && pb.rules.length === 0 && (
        <p className="hint">This playbook has no rules yet.</p>
      )}
    </div>
  );
}
