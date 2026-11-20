import {
  Bar,
  BarChart,
  Cell,
  LabelList,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { chartMotion } from "../../logic/motion";
import { sliceSummary, type SliceRow } from "../../logic/dashboard";
import { pnlClass, usd, usdTick } from "../../logic/format";

/** The amount just past the end of the bar: right of a gain, left of a
 *  loss. */
function EndLabel(props: {
  x?: number | string;
  y?: number | string;
  width?: number | string;
  height?: number | string;
  value?: unknown;
}) {
  const x = Number(props.x ?? 0);
  const y = Number(props.y ?? 0);
  const w = Number(props.width ?? 0);
  const h = Number(props.height ?? 0);
  const text = String(props.value ?? "0");
  const loss = Number(text) < 0;
  const end = loss ? Math.min(x, x + w) : Math.max(x, x + w);
  return (
    <text
      x={end + (loss ? -6 : 6)}
      y={y + h / 2}
      dy="0.35em"
      textAnchor={loss ? "end" : "start"}
      className={`breakdown-value ${pnlClass(text)}`}
    >
      {usd(text, { round: true })}
    </text>
  );
}

/** Bar thickness + gap, so long coin lists scroll instead of squashing. */
const ROW_PX = 40;

/** Net PnL per group as horizontal bars from a zero line: right and green
 *  = earned, left and red = lost. Hover for details; click a bar to filter
 *  the dashboard to it (click again to clear). */
export default function BreakdownBars({
  rows,
  selected,
  onPick,
  label,
}: {
  rows: SliceRow[];
  /** Key of the group the dashboard is filtered to, if any. */
  selected: string | null;
  onPick: (key: string | null) => void;
  /** What the bars are, for screen readers ("Net PnL by market"). */
  label: string;
}) {
  if (rows.every((r) => r.closed === 0)) {
    return <p className="chart-empty">No closed trades yet.</p>;
  }
  const dim = (key: string) => selected !== null && selected !== key;
  return (
    <div className="chart breakdown-scroll">
      <div
        className="breakdown-inner"
        style={{ height: `max(100%, ${rows.length * ROW_PX + 24}px)` }}
        role="img"
        aria-label={`${label}: ${rows
          .map((r) => `${r.label} ${usd(r.pnl, { round: true })}`)
          .join(", ")}`}
      >
        <ResponsiveContainer width="100%" height="100%">
          <BarChart
            data={rows}
            layout="vertical"
            margin={{ top: 4, right: 8, bottom: 4, left: 4 }}
            barCategoryGap="22%"
          >
            {/* Room on both sides for the amount at each bar's end. */}
            <XAxis
              type="number"
              tickFormatter={usdTick}
              padding={{ left: 64, right: 64 }}
              hide
            />
            <YAxis
              type="category"
              dataKey="label"
              width={92}
              tickLine={false}
              axisLine={false}
              className="breakdown-axis"
            />
            <ReferenceLine x={0} className="chart-zero" />
            <Tooltip
              cursor={{ className: "chart-cursor" }}
              content={({ active, payload }) => {
                const r = payload?.[0]?.payload as SliceRow | undefined;
                if (!active || !r) return null;
                return (
                  <div className="chart-tip">
                    <div className="chart-tip-title">{r.label}</div>
                    <div>
                      Net{" "}
                      <strong className={pnlClass(r.pnl)}>
                        {usd(r.pnl, { round: true })}
                      </strong>
                    </div>
                    <div>
                      {sliceSummary(r)}
                      {r.closed > 0 && ` (${r.wins} W · ${r.losses} L)`}
                    </div>
                    <div className="chart-tip-hint">
                      {selected === r.key
                        ? "Click to show everything"
                        : "Click to filter the dashboard"}
                    </div>
                  </div>
                );
              }}
            />
            <Bar
              dataKey="value"
              radius={4}
              maxBarSize={28}
              cursor="pointer"
              onClick={(d) => {
                const key = (d as unknown as { payload?: SliceRow }).payload
                  ?.key;
                if (key) onPick(selected === key ? null : key);
              }}
              {...chartMotion(800)}
            >
              {rows.map((r) => (
                <Cell
                  key={r.key}
                  className={`bar-${r.tone}${dim(r.key) ? " bar-dim" : ""}`}
                />
              ))}
              <LabelList dataKey="pnl" content={EndLabel} />
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
