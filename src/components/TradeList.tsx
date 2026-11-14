import { useState } from "react";
import type { Trade, WithId } from "../lib/api";
import { formatDateTime, formatR, pnlClass, signed } from "../lib/format";

interface Props {
  trades: WithId<Trade>[];
  onDelete: (id: number) => void;
}

// Simple list so saved trades are visible; the full trade log with sorting,
// filters and paging comes in T5.1.
export default function TradeList({ trades, onDelete }: Props) {
  // Two-step delete: first click arms, second click deletes.
  const [armed, setArmed] = useState<number | null>(null);

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
          <tr key={t.id}>
            <td>{formatDateTime(t.openTime)}</td>
            <td>
              {t.symbol}
              <span className="market">
                {" "}
                {t.market === "usdm" ? "Futures" : "Spot"}
              </span>
            </td>
            <td className={t.direction}>
              {t.direction === "long" ? "Long" : "Short"}
            </td>
            <td>{t.status === "open" ? "Open" : "Closed"}</td>
            <td className={`num ${pnlClass(t.netPnl)}`}>{signed(t.netPnl)}</td>
            <td className={`num ${pnlClass(t.rMultiple)}`}>
              {formatR(t.rMultiple)}
            </td>
            <td className="num">
              {armed === t.id ? (
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
              )}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
