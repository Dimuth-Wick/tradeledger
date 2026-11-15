import { useEffect, useMemo, useState } from "react";
import { createTrade, previewTrade, type Trade } from "../lib/api";
import { formatR, formatRr, pnlClass, signed } from "../lib/format";
import {
  emptyForm,
  hasInput,
  toTradeInput,
  type TradeFormState,
} from "../lib/tradeForm";
import { useUnsavedChanges } from "../lib/unsaved";

/** Wait this long after the last keystroke before asking Rust for a preview. */
const PREVIEW_DELAY_MS = 250;

type Preview = { key: string } & (
  { ok: true; trade: Trade } | { ok: false; error: string }
);

interface Props {
  onSaved: () => void;
}

export default function TradeForm({ onSaved }: Props) {
  const [form, setForm] = useState<TradeFormState>(() => emptyForm());
  const [preview, setPreview] = useState<Preview | null>(null);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [savedMsg, setSavedMsg] = useState<string | null>(null);

  const parsed = useMemo(() => toTradeInput(form), [form]);
  useUnsavedChanges(hasInput(form));
  // Identifies the exact input a preview was computed for, so a slow reply
  // for old values is never shown next to newer values.
  const inputKey = parsed.ok ? JSON.stringify(parsed.input) : null;

  useEffect(() => {
    if (!parsed.ok) return;
    const key = JSON.stringify(parsed.input);
    let cancelled = false;
    const timer = setTimeout(() => {
      previewTrade(parsed.input).then(
        (trade) => !cancelled && setPreview({ key, ok: true, trade }),
        (err) =>
          !cancelled && setPreview({ key, ok: false, error: String(err) }),
      );
    }, PREVIEW_DELAY_MS);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [parsed]);

  const current = preview && preview.key === inputKey ? preview : null;
  const canSave = parsed.ok && current?.ok === true && !saving;

  function set<K extends keyof TradeFormState>(
    key: K,
    value: TradeFormState[K],
  ) {
    setForm((f) => ({ ...f, [key]: value }));
    setSavedMsg(null);
    setSaveError(null);
  }

  async function save(e: React.FormEvent) {
    e.preventDefault();
    if (!parsed.ok) return;
    setSaving(true);
    setSaveError(null);
    try {
      const saved = await createTrade(parsed.input);
      // Keep the choices people repeat (exchange, market, direction).
      setForm({
        ...emptyForm(),
        exchange: form.exchange,
        market: form.market,
        direction: form.direction,
      });
      setSavedMsg(`Saved trade #${saved.id} (${saved.symbol})`);
      onSaved();
    } catch (err) {
      setSaveError(String(err));
    } finally {
      setSaving(false);
    }
  }

  const futures = form.market === "usdm";

  return (
    <form className="trade-form" onSubmit={save}>
      <div className="fields">
        <fieldset>
          <legend>Trade</legend>
          <div className="row">
            <label>
              Market
              <select
                value={form.market}
                onChange={(e) =>
                  set("market", e.target.value as TradeFormState["market"])
                }
              >
                <option value="usdm">USDT-M futures</option>
                <option value="spot">Spot</option>
              </select>
            </label>
            <label>
              Symbol
              <input
                value={form.symbol}
                onChange={(e) => set("symbol", e.target.value)}
                placeholder="BTCUSDT"
              />
            </label>
            <label>
              Exchange
              <input
                value={form.exchange}
                onChange={(e) => set("exchange", e.target.value)}
              />
            </label>
          </div>
          <div className="direction" role="group" aria-label="Direction">
            {(["long", "short"] as const).map((d) => (
              <button
                key={d}
                type="button"
                className={form.direction === d ? `active ${d}` : ""}
                aria-pressed={form.direction === d}
                onClick={() => set("direction", d)}
              >
                {d === "long" ? "Long" : "Short"}
              </button>
            ))}
          </div>
          <div className="row">
            <label>
              Quantity
              <input
                inputMode="decimal"
                value={form.qty}
                onChange={(e) => set("qty", e.target.value)}
                placeholder="0.015"
              />
            </label>
            <label>
              Entry price
              <input
                inputMode="decimal"
                value={form.avgEntry}
                onChange={(e) => set("avgEntry", e.target.value)}
              />
            </label>
            <label>
              Exit price
              <input
                inputMode="decimal"
                value={form.avgExit}
                onChange={(e) => set("avgExit", e.target.value)}
                placeholder="empty = still open"
              />
            </label>
          </div>
          <div className="row">
            <label>
              Open time
              <input
                type="datetime-local"
                value={form.openTime}
                onChange={(e) => set("openTime", e.target.value)}
              />
            </label>
            <label>
              Close time
              <input
                type="datetime-local"
                value={form.closeTime}
                onChange={(e) => set("closeTime", e.target.value)}
              />
            </label>
          </div>
          <div className="row">
            <label>
              Fees
              <input
                inputMode="decimal"
                value={form.fees}
                onChange={(e) => set("fees", e.target.value)}
                placeholder="0"
              />
            </label>
            {futures && (
              <>
                <label>
                  Funding
                  <input
                    inputMode="decimal"
                    value={form.funding}
                    onChange={(e) => set("funding", e.target.value)}
                    placeholder="paid +, received −"
                  />
                </label>
                <label>
                  Leverage
                  <input
                    inputMode="numeric"
                    value={form.leverage}
                    onChange={(e) => set("leverage", e.target.value)}
                    placeholder="10"
                  />
                </label>
              </>
            )}
          </div>
        </fieldset>

        <fieldset>
          <legend>Risk plan</legend>
          <div className="row">
            <label>
              Planned entry
              <input
                inputMode="decimal"
                value={form.plannedEntry}
                onChange={(e) => set("plannedEntry", e.target.value)}
                placeholder="= entry price"
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
          </div>
        </fieldset>

        <label className="notes">
          Notes
          <textarea
            rows={3}
            value={form.notes}
            onChange={(e) => set("notes", e.target.value)}
          />
        </label>
      </div>

      <aside className="preview" aria-live="polite">
        <h2>Preview</h2>
        {!parsed.ok ? (
          <p className={parsed.incomplete ? "hint" : "status error"}>
            {parsed.error}
          </p>
        ) : !current ? (
          <p className="hint">Calculating…</p>
        ) : !current.ok ? (
          <p className="status error">{current.error}</p>
        ) : (
          <dl>
            <dt>Status</dt>
            <dd>{current.trade.status === "open" ? "Open" : "Closed"}</dd>
            <dt>Planned R:R</dt>
            <dd>{formatRr(current.trade.plannedRr)}</dd>
            <dt>R-multiple</dt>
            <dd className={pnlClass(current.trade.rMultiple)}>
              {formatR(current.trade.rMultiple)}
            </dd>
            <dt>Gross PnL</dt>
            <dd className={pnlClass(current.trade.grossPnl)}>
              {signed(current.trade.grossPnl)}
            </dd>
            <dt>Net PnL</dt>
            <dd className={pnlClass(current.trade.netPnl)}>
              {signed(current.trade.netPnl)}
            </dd>
          </dl>
        )}

        <button type="submit" className="primary" disabled={!canSave}>
          {saving ? "Saving…" : "Save trade"}
        </button>
        {saveError && <p className="status error">{saveError}</p>}
        {savedMsg && <p className="status ok">{savedMsg}</p>}
      </aside>
    </form>
  );
}
