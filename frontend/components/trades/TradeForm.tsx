import { useEffect, useMemo, useState } from "react";
import PendingShots, { type PendingShot } from "./PendingShots";
import {
  addScreenshot,
  createTrade,
  previewPlan,
  previewTrade,
  symbolChoices,
  type PlanPreview,
  type SymbolChoice,
  type Trade,
} from "../../connection/backend";
import { formatR, formatRr, pnlClass, usd } from "../../logic/format";
import {
  emptyForm,
  hasInput,
  toPlanInput,
  toTradeInput,
  withEntryPrice,
  type TradeFormState,
} from "../../logic/tradeForm";
import { useUnsavedChanges } from "../../logic/unsaved";

/** Wait this long after the last keystroke before asking Rust for a preview. */
const PREVIEW_DELAY_MS = 250;

type Preview = { key: string } & (
  { ok: true; trade: Trade } | { ok: false; error: string }
);

type PlanResult = { key: string } & (
  { ok: true; plan: PlanPreview } | { ok: false; error: string }
);

/** The symbol menu's "type your own" entry. */
const OTHER = "__other__";

interface Props {
  onSaved: () => void;
}

export default function TradeForm({ onSaved }: Props) {
  const [form, setForm] = useState<TradeFormState>(() => emptyForm());
  const [preview, setPreview] = useState<Preview | null>(null);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [savedMsg, setSavedMsg] = useState<string | null>(null);
  // Screenshots chosen before the trade exists; attached after saving.
  const [shots, setShots] = useState<PendingShot[]>([]);
  // The symbol menu for the chosen market (from Rust).
  const [choices, setChoices] = useState<SymbolChoice[]>([]);
  // True after "Other" is picked: the symbol is typed instead.
  const [typingSymbol, setTypingSymbol] = useState(false);
  const [plan, setPlan] = useState<PlanResult | null>(null);

  useEffect(() => {
    let cancelled = false;
    symbolChoices(form.market).then(
      (list) => !cancelled && setChoices(list),
      () => !cancelled && setChoices([]),
    );
    return () => {
      cancelled = true;
    };
  }, [form.market]);

  const parsed = useMemo(() => toTradeInput(form), [form]);
  useUnsavedChanges(hasInput(form) || shots.length > 0);
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

  // R:R and profit/loss at target/stop, as soon as the risk plan has them.
  const planInput = useMemo(() => toPlanInput(form), [form]);
  const planKey = planInput ? JSON.stringify(planInput) : null;
  useEffect(() => {
    if (!planInput) return;
    const key = JSON.stringify(planInput);
    let cancelled = false;
    const timer = setTimeout(() => {
      previewPlan(planInput).then(
        (p) => !cancelled && setPlan({ key, ok: true, plan: p }),
        (err) => !cancelled && setPlan({ key, ok: false, error: String(err) }),
      );
    }, PREVIEW_DELAY_MS);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [planInput]);
  const currentPlan = plan && plan.key === planKey ? plan : null;

  const current = preview && preview.key === inputKey ? preview : null;
  // A symbol not in the menu (e.g. a spot pair after switching to futures)
  // is shown in the typed box so it's never hidden.
  const inMenu = choices.some((c) => c.symbol === form.symbol);
  const showSymbolBox = typingSymbol || (form.symbol !== "" && !inMenu);
  const autoFee = current?.ok
    ? current.trade.fees
    : currentPlan?.ok
      ? currentPlan.plan.fees
      : null;
  const canSave = parsed.ok && current?.ok === true && !saving;

  function set<K extends keyof TradeFormState>(
    key: K,
    value: TradeFormState[K],
  ) {
    change((f) => ({ ...f, [key]: value }));
  }

  function change(next: (f: TradeFormState) => TradeFormState) {
    setForm(next);
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
      // Then attach the screenshots, one by one. A failed image doesn't undo
      // the trade; the message says which ones to add again.
      let added = 0;
      const failed: string[] = [];
      for (const shot of shots) {
        try {
          const bytes = new Uint8Array(await shot.file.arrayBuffer());
          await addScreenshot(saved.id, bytes);
          added += 1;
        } catch (err) {
          failed.push(`${shot.file.name || "pasted image"} (${String(err)})`);
        }
      }
      shots.forEach((s) => URL.revokeObjectURL(s.url));
      setShots([]);
      // Keep the choices people repeat (market, direction).
      setTypingSymbol(false);
      setForm({
        ...emptyForm(),
        market: form.market,
        direction: form.direction,
      });
      const withShots =
        added > 0 ? ` with ${added} screenshot${added === 1 ? "" : "s"}` : "";
      setSavedMsg(`Saved trade #${saved.id} (${saved.symbol})${withShots}`);
      if (failed.length > 0) {
        setSaveError(
          `Trade saved, but these images weren't added (open the trade to add them again): ${failed.join("; ")}`,
        );
      }
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
              <select
                value={showSymbolBox ? OTHER : form.symbol}
                onChange={(e) => {
                  const v = e.target.value;
                  setTypingSymbol(v === OTHER);
                  set("symbol", v === OTHER ? "" : v);
                }}
              >
                <option value="" disabled>
                  Choose a pair…
                </option>
                {choices.map((c) => (
                  <option key={c.symbol} value={c.symbol}>
                    {c.quote ? `${c.base} / ${c.quote}` : c.symbol}
                  </option>
                ))}
                <option value={OTHER}>Other (type it)…</option>
              </select>
              {showSymbolBox && (
                <input
                  aria-label="Symbol (typed)"
                  value={form.symbol}
                  onChange={(e) => set("symbol", e.target.value)}
                  placeholder="e.g. WIFUSDT"
                  autoFocus
                />
              )}
            </label>
            <label>
              Exchange
              <input
                value="Binance"
                readOnly
                className="locked"
                tabIndex={-1}
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
              Position size (USDT)
              <input
                inputMode="decimal"
                value={form.positionSize}
                onChange={(e) => set("positionSize", e.target.value)}
                placeholder="e.g. 500"
              />
            </label>
            <label>
              Entry price
              <input
                inputMode="decimal"
                value={form.avgEntry}
                onChange={(e) => {
                  const v = e.target.value;
                  change((f) => withEntryPrice(f, v));
                }}
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
              Fees (0.1%)
              <input
                readOnly
                className="locked"
                tabIndex={-1}
                value={
                  autoFee === null
                    ? ""
                    : usd(autoFee, { sign: false, round: true })
                }
                placeholder="worked out from position size"
              />
            </label>
            {futures && (
              <label>
                Leverage
                <input
                  inputMode="numeric"
                  value={form.leverage}
                  onChange={(e) => set("leverage", e.target.value)}
                  placeholder="10"
                />
              </label>
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
                placeholder="fills from entry price"
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
          <PlanSummary hasPlan={planInput !== null} result={currentPlan} />
        </fieldset>

        <fieldset>
          <legend>Screenshots</legend>
          <PendingShots
            shots={shots}
            onChange={(next) => {
              setShots(next);
              setSavedMsg(null);
            }}
            disabled={saving}
          />
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
            <dt>Quantity</dt>
            <dd>{current.trade.qty}</dd>
            <dt>Fees</dt>
            <dd>{usd(current.trade.fees, { sign: false })}</dd>
            <dt>Planned R:R</dt>
            <dd>{formatRr(current.trade.plannedRr)}</dd>
            <dt>R-multiple</dt>
            <dd className={pnlClass(current.trade.rMultiple)}>
              {formatR(current.trade.rMultiple)}
            </dd>
            <dt>Gross PnL</dt>
            <dd className={pnlClass(current.trade.grossPnl)}>
              {usd(current.trade.grossPnl)}
            </dd>
            <dt>Net PnL</dt>
            <dd className={pnlClass(current.trade.netPnl)}>
              {usd(current.trade.netPnl)}
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

/** The risk plan's numbers: R:R, and what you would make or lose at the
 *  take profit and the stop loss (after the 0.1% fee). Worked out in Rust. */
function PlanSummary({
  hasPlan,
  result,
}: {
  hasPlan: boolean;
  result: PlanResult | null;
}) {
  if (!hasPlan) {
    return (
      <p className="hint plan-summary">
        Add a stop loss and take profit to see the R:R and your profit or loss.
      </p>
    );
  }
  if (!result) return <p className="hint plan-summary">Calculating…</p>;
  if (!result.ok) {
    return <p className="status error plan-summary">{result.error}</p>;
  }
  const p = result.plan;
  return (
    <div className="plan-summary">
      <dl>
        <div>
          <dt>R:R</dt>
          <dd>{formatRr(p.plannedRr)}</dd>
        </div>
        <div>
          <dt>Profit at take profit</dt>
          <dd className={pnlClass(p.profitAtTarget)}>
            {p.profitAtTarget === null
              ? "—"
              : usd(p.profitAtTarget, { round: true })}
          </dd>
        </div>
        <div>
          <dt>Loss at stop loss</dt>
          <dd className={pnlClass(p.lossAtStop)}>
            {p.lossAtStop === null ? "—" : usd(p.lossAtStop, { round: true })}
          </dd>
        </div>
      </dl>
      {p.fees === null && (
        <p className="hint">Add a position size to see profit and loss in $.</p>
      )}
    </div>
  );
}
