import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  LabelList,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type { RRow } from "../../lib/dashboard";

/** How many closed trades ended in each R-multiple range. */
export default function RDistribution({
  rows,
  withR,
  closed,
}: {
  rows: RRow[];
  withR: number;
  closed: number;
}) {
  if (withR === 0) {
    return (
      <p className="chart-empty">
        No closed trades with an R yet. Add a stop loss to a trade to get one.
      </p>
    );
  }
  return (
    <>
      <div
        className="chart"
        role="img"
        aria-label={rows.map((r) => `${r.label}: ${r.count}`).join(", ")}
      >
        <ResponsiveContainer width="100%" height={240}>
          <BarChart
            data={rows}
            margin={{ top: 20, right: 16, bottom: 4, left: 0 }}
          >
            <CartesianGrid vertical={false} />
            <XAxis dataKey="label" interval={0} />
            <YAxis width={40} allowDecimals={false} />
            <Tooltip
              cursor={{ className: "chart-cursor" }}
              content={({ active, payload }) => {
                const row = payload?.[0]?.payload as RRow | undefined;
                if (!active || !row) return null;
                return (
                  <div className="chart-tip">
                    {row.label}: <strong>{row.count}</strong> trade
                    {row.count === 1 ? "" : "s"}
                  </div>
                );
              }}
            />
            <Bar
              dataKey="count"
              maxBarSize={48}
              radius={2}
              isAnimationActive={false}
            >
              {rows.map((r) => (
                <Cell key={r.label} className={`bar-${r.tone}`} />
              ))}
              <LabelList dataKey="count" position="top" className="bar-label" />
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>
      {withR < closed && (
        <p className="chart-note">
          {withR} of {closed} closed trades have an R (the rest have no stop
          loss).
        </p>
      )}
    </>
  );
}
