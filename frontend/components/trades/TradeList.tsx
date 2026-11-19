import { Fragment, useEffect, useState } from "react";
import {
  listTradeFills,
  type SortField,
  type Trade,
  type TradeFill,
  type WithId,
} from "../../connection/backend";
import { formatDateTime, formatR, pnlClass, usd } from "../../logic/format";
import type { TagInfo } from "../../logic/tagIndex";
import TagChip from "../shared/TagChip";

interface Props {
  trades: WithId<Trade>[];
  onDelete: (id: number) => void;
  onMerge: (id: number) => void;
  onSplit: (tradeId: number, fillId: number) => void;
  onReset: (tradeId: number) => void;
  /** Opens the trade detail page. */
  onOpen?: (id: number) => void;
  /** Trade id → tag ids, and tag lookup, to show chips under the symbol. */
  tradeTags?: Record<string, number[]>;
  tagInfo?: Map<number, TagInfo>;
  /** Current sort; when given with `onSort`, headers become clickable. */
  sort?: { field: SortField; descending: boolean };
  onSort?: (field: SortField) => void;
  emptyText?: string;
}

/** Column header that sorts when clicked (plain text if sorting is off). */
function SortHeader({
  label,
  field,
  sort,
  onSort,
  className,
}: {
  label: string;
  field: SortField;
  sort?: Props["sort"];
  onSort?: Props["onSort"];
  className?: string;
}) {
  if (!onSort) return <th className={className}>{label}</th>;
  const active = sort?.field === field;
  const arrow = active ? (sort.descending ? " ↓" : " ↑") : "";
  return (
    <th
      className={className}
      aria-sort={
        active ? (sort.descending ? "descending" : "ascending") : "none"
      }
    >
      <button
        type="button"
        className="sort-button"
        onClick={() => onSort(field)}
      >
        {label}
        {arrow}
      </button>
    </th>
  );
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
  onOpen,
  tradeTags,
  tagInfo,
  sort,
  onSort,
  emptyText = "No trades yet.",
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
    return <p className="hint">{emptyText}</p>;
  }

  const h = { sort, onSort };
  return (
    <table className="trade-list">
      <thead>
        <tr>
          <SortHeader label="Opened" field="openTime" {...h} />
          <SortHeader label="Symbol" field="symbol" {...h} />
          <th>Side</th>
          <th>Status</th>
          <SortHeader label="Net PnL" field="netPnl" className="num" {...h} />
          <SortHeader label="R" field="rMultiple" className="num" {...h} />
          <th aria-label="Actions" />
        </tr>
      </thead>
      <tbody>
        {trades.map((t) => (
          <Fragment key={t.id}>
            <tr>
              <td>{formatDateTime(t.openTime)}</td>
              <td>
                {onOpen ? (
                  <button
                    type="button"
                    className="link symbol-link"
                    title="Open trade details"
                    onClick={() => onOpen(t.id)}
                  >
                    {t.symbol}
                  </button>
                ) : (
                  t.symbol
                )}
                <span className="market">
                  {" "}
                  {t.market === "usdm" ? "Futures" : "Spot"}
                  {t.isManual ? " · manual" : ""}
                </span>
                {tagInfo && tradeTags?.[String(t.id)] && (
                  <div className="row-tags">
                    {tradeTags[String(t.id)].map((id) => {
                      const info = tagInfo.get(id);
                      return info ? (
                        <TagChip
                          key={id}
                          name={info.tag.name}
                          color={info.color}
                        />
                      ) : null;
                    })}
                  </div>
                )}
              </td>
              <td className={t.direction}>
                {t.direction === "long" ? "Long" : "Short"}
              </td>
              <td>{t.status === "open" ? "Open" : "Closed"}</td>
              <td className={`num ${pnlClass(t.netPnl)}`} title={usd(t.netPnl)}>
                {usd(t.netPnl, { round: true })}
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

/** A trade's fills. Without `onSplit`/`onReset` it's read-only (detail page). */
export function FillsPanel({
  tradeId,
  onSplit,
  onReset,
}: {
  tradeId: number;
  onSplit?: (fillId: number) => void;
  onReset?: () => void;
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
                {i > 0 && onSplit && (
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
      {onReset && (
        <button type="button" className="link" onClick={onReset}>
          Reset grouping for this coin
        </button>
      )}
    </div>
  );
}
