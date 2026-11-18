import {
  CartesianGrid,
  Area,
  AreaChart,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { chartMotion } from "../../lib/motion";
import type { CurveRow } from "../../lib/dashboard";
import { formatDateTime, pnlClass, usd, usdTick } from "../../lib/format";

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
      aria-label={`Equity curve over ${rows.length - 1} closed trades, ending at ${usd(last.equityText)}`}
    >
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart
          data={rows}
          margin={{ top: 8, right: 16, bottom: 4, left: 0 }}
        >
          <defs>
            <linearGradient id="equity-fill" x1="0" y1="0" x2="0" y2="1">
              <stop
                offset="0%"
                style={{ stopColor: "var(--accent)" }}
                stopOpacity={0.28}
              />
              <stop
                offset="100%"
                style={{ stopColor: "var(--accent)" }}
                stopOpacity={0}
              />
            </linearGradient>
          </defs>
          <CartesianGrid vertical={false} />
          <XAxis
            dataKey="n"
            type="number"
            domain={[0, "dataMax"]}
            allowDecimals={false}
          />
          <YAxis width={64} tickFormatter={usdTick} />
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
                      {usd(row.equityText, { round: true })}
                    </strong>
                  </div>
                  <div>
                    This trade{" "}
                    <span className={pnlClass(row.pnlText)}>
                      {usd(row.pnlText, { round: true })}
                    </span>
                  </div>
                </div>
              );
            }}
          />
          <Area
            type="linear"
            dataKey="equity"
            className="equity-line"
            strokeWidth={2}
            dot={false}
            fill="url(#equity-fill)"
            activeDot={{ r: 4, className: "equity-dot" }}
            {...chartMotion(900)}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}
