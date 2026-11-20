import { useState } from "react";
import { Cell, Pie, PieChart, ResponsiveContainer } from "recharts";
import type { Stats } from "../../connection/backend";
import { outcomeRows, type OutcomeRow } from "../../logic/dashboard";
import { chartMotion } from "../../logic/motion";

/** Wins, losses and breakeven as a ring, total trades in the middle.
 *  Hover a part (or its legend row) to see its share. */
export default function OutcomeRing({ stats }: { stats: Stats }) {
  const rows = outcomeRows(stats);
  const [active, setActive] = useState<number | null>(null);
  if (stats.trades === 0) {
    return <p className="chart-empty">No trades yet.</p>;
  }
  const hovered: OutcomeRow | null = active === null ? null : rows[active];
  const share = (n: number) =>
    stats.closed === 0 ? 0 : Math.round((n / stats.closed) * 100);

  return (
    <div className="outcome">
      <div
        className="chart outcome-ring"
        role="img"
        aria-label={`${stats.trades} trades: ${rows
          .map((r) => `${r.count} ${r.label.toLowerCase()}`)
          .join(", ")}, ${stats.open} open`}
      >
        {rows.length > 0 && (
          <ResponsiveContainer width="100%" height="100%">
            <PieChart>
              <Pie
                data={rows}
                dataKey="count"
                nameKey="label"
                innerRadius="64%"
                outerRadius="88%"
                paddingAngle={rows.length > 1 ? 2 : 0}
                cornerRadius={4}
                startAngle={90}
                endAngle={-270}
                stroke="none"
                onMouseEnter={(_, i) => setActive(i)}
                onMouseLeave={() => setActive(null)}
                {...chartMotion(900)}
              >
                {rows.map((r, i) => (
                  <Cell
                    key={r.key}
                    className={`ring-${r.key}${
                      active !== null && active !== i ? " ring-dim" : ""
                    }`}
                  />
                ))}
              </Pie>
            </PieChart>
          </ResponsiveContainer>
        )}
        <div className="outcome-center" aria-hidden>
          {hovered ? (
            <>
              <strong>{share(hovered.count)}%</strong>
              <span>{hovered.label.toLowerCase()}</span>
            </>
          ) : (
            <>
              <strong>{stats.trades}</strong>
              <span>trade{stats.trades === 1 ? "" : "s"}</span>
            </>
          )}
        </div>
      </div>
      <ul className="outcome-legend">
        {rows.map((r, i) => (
          <li
            key={r.key}
            className={active === i ? "active" : ""}
            onMouseEnter={() => setActive(i)}
            onMouseLeave={() => setActive(null)}
          >
            <span className={`dot ring-${r.key}`} aria-hidden />
            {r.label}
            <strong>{r.count}</strong>
          </li>
        ))}
        {stats.open > 0 && (
          <li className="muted">
            <span className="dot ring-open" aria-hidden />
            Open
            <strong>{stats.open}</strong>
          </li>
        )}
      </ul>
    </div>
  );
}
