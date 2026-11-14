import { Fragment, useEffect, useState } from "react";
import {
  listTradeFills,
  type Trade,
  type TradeFill,
  type WithId,
} from "../lib/api";
import { formatDateTime, formatR, pnlClass, signed } from "../lib/format";

interface Props {
  trades: WithId<Trade>[];
  onDelete: (id: number) => void;
  onMerge: (id: number) => void;
  onSplit: (tradeId: number, fillId: number) => void;
  onReset: (tradeId: number) => void;
}

const COLUMNS = 7;

// Simple list so saved trades are visible; the full trade log with sorting,
// filters and paging comes in T5.1.
//
// Imported trades are rebuilt from their fills on every import, so instead of
// Delete they offer Fills (with "Split here") and Merge with the previous
// trade. Manual trades can be deleted.
export default function TradeList({
  trades,
  onDelete,
  onMerge,
  onSplit,
  onReset,
}: Props) {
  // Two-step delete: first click arms, second click deletes.
  const [armed, setArmed] = useState<number | null>(null);
  const [expanded, setExpanded] = useState<number | null>(null);

  // Merge/split/reset rebuild trades, so an open fills panel would show
  // stale fills. Close it before running the action.
  function closingPanel<A extends unknown[]>(action: (...args: A) => void) {
    return (...args: A) => {
      setExpanded(null);
      action(...args);
    };
  }

  if (trades.length === 0) {
    return <p className="hint">No trades yet.</p>;
  }

  return (
    <table className="trade-list">
      <thead>
        <tr>
          <th>Opened</th>
          <th>Symbol</th>
          <th>Side</th>
          <th>Status</th>
          <th className="num">Net PnL</th>
          <th className="num">R</th>
          <th aria-label="Actions" />
        </tr>
      </thead>
      <tbody>
        {trades.map((t) => (
          <Fragment key={t.id}>
            <tr>
              <td>{formatDateTime(t.openTime)}</td>
              <td>
                {t.symbol}
                <span className="market">
                  {" "}
                  {t.market === "usdm" ? "Futures" : "Spot"}
                  {t.isManual ? " · manual" : ""}
                </span>
              </td>
              <td className={t.direction}>
                {t.direction === "long" ? "Long" : "Short"}
              </td>
              <td>{t.status === "open" ? "Open" : "Closed"}</td>
              <td className={`num ${pnlClass(t.netPnl)}`}>
                {signed(t.netPnl)}
              </td>
              <td className={`num ${pnlClass(t.rMultiple)}`}>
                {formatR(t.rMultiple)}
              </td>
              <td className="num actions-cell">
                {t.isManual ? (
                  armed === t.id ? (
                    <>
                      <button
                        type="button"
                        className="danger"
                        onClick={() => onDelete(t.id)}
                      >
                        Confirm delete
                      </button>
                      <button type="button" onClick={() => setArmed(null)}>
                        Cancel
                      </button>
                    </>
                  ) : (
                    <button type="button" onClick={() => setArmed(t.id)}>
                      Delete
                    </button>
                  )
                ) : (
                  <>
                    <button
                      type="button"
                      aria-expanded={expanded === t.id}
                      onClick={() =>
                        setExpanded(expanded === t.id ? null : t.id)
                      }
                    >
                      {expanded === t.id ? "Hide fills" : "Fills"}
                    </button>
                    <button
                      type="button"
                      title="Join this trade to the previous trade on the same coin"
                      onClick={closingPanel(() => onMerge(t.id))}
                    >
                      Merge ↑
                    </button>
                  </>
                )}
              </td>
            </tr>
            {expanded === t.id && (
              <tr className="fills-row">
                <td colSpan={COLUMNS}>
                  <FillsPanel
                    tradeId={t.id}
                    onSplit={closingPanel((fillId: number) =>
                      onSplit(t.id, fillId),
                    )}
                    onReset={closingPanel(() => onReset(t.id))}
                  />
                </td>
              </tr>
            )}
          </Fragment>
        ))}
      </tbody>
    </table>
  );
}

function FillsPanel({
  tradeId,
  onSplit,
  onReset,
}: {
  tradeId: number;
  onSplit: (fillId: number) => void;
  onReset: () => void;
}) {
  const [fills, setFills] = useState<TradeFill[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    listTradeFills(tradeId).then(
      (f) => !cancelled && setFills(f),
      (err) => !cancelled && setError(String(err)),
    );
    return () => {
      cancelled = true;
    };
  }, [tradeId]);

  if (error) return <p className="status error">{error}</p>;
  if (!fills) return <p className="hint">Loading fills…</p>;

  return (
    <div className="fills-panel">
      <table>
        <thead>
          <tr>
            <th>Time</th>
            <th>Side</th>
            <th className="num">Price</th>
            <th className="num">Qty used</th>
            <th className="num">Fee</th>
            <th aria-label="Actions" />
          </tr>
        </thead>
        <tbody>
          {fills.map((f, i) => (
            <tr key={f.fillId}>
              <td>{formatDateTime(f.time)}</td>
              <td>
                {f.side === "buy" ? "Buy" : "Sell"}
                <span className="market">
                  {" "}
                  {f.isEntry ? "· adds" : "· reduces"}
                </span>
              </td>
              <td className="num">{f.price}</td>
              <td className="num">
                {f.qty}
                {f.qty !== f.fillQty && (
                  <span className="market"> of {f.fillQty}</span>
                )}
              </td>
              <td className="num">
                {f.fee} {f.feeAsset}
              </td>
              <td className="num">
                {i > 0 && (
                  <button
                    type="button"
                    title="Start a new trade at this fill"
                    onClick={() => onSplit(f.fillId)}
                  >
                    Split here
                  </button>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <button type="button" className="link" onClick={onReset}>
        Reset grouping for this coin
      </button>
    </div>
  );
}
