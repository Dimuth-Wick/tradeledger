import {
  Area,
  AreaChart,
  CartesianGrid,
  Line,
  LineChart,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { chartMotion } from "../../logic/motion";
import { shortDay, type DailyRow } from "../../logic/dashboard";
import { pnlClass, usd, usdTick } from "../../logic/format";
import { dayTitle } from "../../logic/calendar";

/** Both charts share these so their plot areas line up exactly. */
const MARGIN = { top: 8, right: 16, bottom: 0, left: 0 };
const Y_WIDTH = 64;
/** Hovering one chart shows the same day on the other. */
const SYNC = "daily";

const plural = (n: number, w: string) => `${n} ${w}${n === 1 ? "" : "s"}`;

/**
 * Account equity at the end of each day, and below it the running win rate
 * on the same dates. Two charts, not one with two y-axes: dollars and
 * percent don't share a scale, and a second axis invents crossings.
 */
export default function EquityChart({ rows }: { rows: DailyRow[] }) {
  if (rows.length === 0) {
    return <p className="chart-empty">No closed trades yet.</p>;
  }
  const last = rows[rows.length - 1];
  const first = rows[0];

  return (
    <div className="equity-stack">
      <div
        className="chart equity-main"
        role="img"
        aria-label={`Equity by day from ${dayTitle(first.date)} to ${dayTitle(last.date)}, ending at ${usd(last.equityText, { round: true })}`}
      >
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={rows} margin={MARGIN} syncId={SYNC}>
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
            <XAxis dataKey="date" hide />
            <YAxis width={Y_WIDTH} tickFormatter={usdTick} />
            <ReferenceLine y={0} className="chart-zero" />
            <Tooltip
              content={({ active, payload }) => {
                const row = payload?.[0]?.payload as DailyRow | undefined;
                if (!active || !row) return null;
                return (
                  <div className="chart-tip">
                    <div className="chart-tip-title">{dayTitle(row.date)}</div>
                    <div>
                      Equity{" "}
                      <strong className={pnlClass(row.equityText)}>
                        {usd(row.equityText, { round: true })}
                      </strong>
                    </div>
                    <div>
                      {row.trades > 0 ? (
                        <>
                          This day{" "}
                          <span className={pnlClass(row.pnlText)}>
                            {usd(row.pnlText, { round: true })}
                          </span>{" "}
                          · {plural(row.trades, "trade")}
                        </>
                      ) : (
                        <span className="muted">No trades this day</span>
                      )}
                    </div>
                  </div>
                );
              }}
            />
            <Area
              type="monotone"
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

      <div className="winrate-head">
        <span>Win rate</span>
        <strong>
          {last.winRateText === null ? "—" : `${last.winRateText}%`}
        </strong>
        <span className="muted">
          {last.wins} of {plural(last.closed, "trade")} so far
        </span>
      </div>
      <div
        className="chart equity-winrate"
        role="img"
        aria-label={`Running win rate by day, now ${last.winRateText ?? "—"}%`}
      >
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={rows} margin={MARGIN} syncId={SYNC}>
            <CartesianGrid vertical={false} />
            <XAxis
              dataKey="date"
              tickFormatter={shortDay}
              minTickGap={40}
              tickMargin={6}
            />
            <YAxis
              width={Y_WIDTH}
              domain={[0, 100]}
              ticks={[0, 50, 100]}
              tickFormatter={(v: number) => `${v}%`}
            />
            <ReferenceLine
              y={50}
              className="chart-zero"
              strokeDasharray="4 4"
            />
            <Tooltip
              content={({ active, payload }) => {
                const row = payload?.[0]?.payload as DailyRow | undefined;
                if (!active || !row || row.winRateText === null) return null;
                return (
                  <div className="chart-tip">
                    Win rate <strong>{row.winRateText}%</strong>
                    <div className="muted">
                      {row.wins} of {plural(row.closed, "trade")} won
                    </div>
                  </div>
                );
              }}
            />
            <Line
              type="monotone"
              dataKey="winRate"
              className="winrate-line"
              strokeWidth={2}
              dot={false}
              activeDot={{ r: 4, className: "winrate-dot" }}
              connectNulls
              {...chartMotion(900)}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
