import {
  CartesianGrid,
  Line,
  LineChart,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type { CurveRow } from "../../lib/dashboard";
import { formatDateTime, pnlClass, signed } from "../../lib/format";

/** Cumulative closed net PnL, one point per closed trade (in close order). */
export default function EquityChart({ rows }: { rows: CurveRow[] }) {
  if (rows.length === 0) {
    return <p className="chart-empty">No closed trades yet.</p>;
  }
  const last = rows[rows.length - 1];
  return (
    <div
      className="chart"
      role="img"
      aria-label={`Equity curve over ${rows.length - 1} closed trades, ending at ${signed(last.equityText)}`}
    >
      <ResponsiveContainer width="100%" height="100%">
        <LineChart
          data={rows}
          margin={{ top: 8, right: 16, bottom: 4, left: 0 }}
        >
          <CartesianGrid vertical={false} />
          <XAxis
            dataKey="n"
            type="number"
            domain={[0, "dataMax"]}
            allowDecimals={false}
          />
          <YAxis width={64} />
          <ReferenceLine y={0} className="chart-zero" />
          <Tooltip
            content={({ active, payload }) => {
              const row = payload?.[0]?.payload as CurveRow | undefined;
              if (!active || !row) return null;
              if (row.time === null) {
                return <div className="chart-tip">Start · 0</div>;
              }
              return (
                <div className="chart-tip">
                  <div>
                    Trade #{row.n} · {formatDateTime(row.time)}
                  </div>
                  <div>
                    Equity{" "}
                    <strong className={pnlClass(row.equityText)}>
                      {signed(row.equityText)}
                    </strong>
                  </div>
                  <div>
                    This trade{" "}
                    <span className={pnlClass(row.pnlText)}>
                      {signed(row.pnlText)}
                    </span>
                  </div>
                </div>
              );
            }}
          />
          <Line
            type="linear"
            dataKey="equity"
            className="equity-line"
            strokeWidth={2}
            dot={false}
            activeDot={{ r: 4, className: "equity-dot" }}
            isAnimationActive={false}
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
