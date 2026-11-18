import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { chartMotion } from "../../lib/motion";
import type { CurveRow } from "../../lib/dashboard";
import { formatDateTime, pnlClass, usd, usdTick } from "../../lib/format";

/** One bar per closed trade: up = win, down = loss. */
export default function PnlBars({ rows }: { rows: CurveRow[] }) {
  // Row 0 is the curve's starting point, not a trade.
  const trades = rows.slice(1);
  if (trades.length === 0) {
    return <p className="chart-empty">No closed trades yet.</p>;
  }
  return (
    <div
      className="chart"
      role="img"
      aria-label={`Net PnL of each of ${trades.length} closed trades`}
    >
      <ResponsiveContainer width="100%" height="100%">
        <BarChart
          data={trades}
          margin={{ top: 8, right: 16, bottom: 4, left: 0 }}
        >
          <CartesianGrid vertical={false} />
          <XAxis dataKey="n" />
          <YAxis width={64} tickFormatter={usdTick} />
          <ReferenceLine y={0} className="chart-zero" />
          <Tooltip
            cursor={{ className: "chart-cursor" }}
            content={({ active, payload }) => {
              const row = payload?.[0]?.payload as CurveRow | undefined;
              if (!active || !row || row.time === null) return null;
              return (
                <div className="chart-tip">
                  <div>
                    Trade #{row.n} · {formatDateTime(row.time)}
                  </div>
                  <div>
                    Net{" "}
                    <strong className={pnlClass(row.pnlText)}>
                      {usd(row.pnlText, { round: true })}
                    </strong>
                  </div>
                </div>
              );
            }}
          />
          <Bar dataKey="pnl" maxBarSize={24} radius={2} {...chartMotion(650)}>
            {trades.map((r) => (
              <Cell key={r.n} className={r.pnl < 0 ? "bar-neg" : "bar-pos"} />
            ))}
          </Bar>
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}
