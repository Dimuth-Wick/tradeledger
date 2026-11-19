import { useMemo } from "react";
import ReactGridLayout, {
  useContainerWidth,
  type Layout,
} from "react-grid-layout";
import "react-grid-layout/css/styles.css";
import "react-resizable/css/styles.css";
import type { LayoutItem, Stats } from "../../connection/backend";
import {
  changedLayout,
  curveRows,
  dailyRows,
  kpis,
  rRows,
} from "../../logic/dashboard";
import EquityChart from "./EquityChart";
import KpiTiles from "./KpiTiles";
import PnlBars from "./PnlBars";
import RDistribution from "./RDistribution";

const TITLES: Record<string, string> = {
  kpis: "Key numbers",
  equity: "Equity and win rate",
  "pnl-bars": "Win / loss per trade",
  "r-dist": "R distribution",
};

interface Props {
  stats: Stats;
  /** From Rust; also decides which widgets exist. */
  layout: LayoutItem[];
  /** Drag (by the title bar) and resize only while editing. */
  editing: boolean;
  onLayout: (next: LayoutItem[]) => void;
}

/** The widgets on a 12-column grid that can be dragged and resized. */
export default function DashboardGrid({
  stats,
  layout,
  editing,
  onLayout,
}: Props) {
  const { width, containerRef, mounted } = useContainerWidth();
  const rows = useMemo(() => curveRows(stats), [stats]);
  const days = useMemo(() => dailyRows(stats), [stats]);

  function body(id: string) {
    switch (id) {
      case "kpis":
        return <KpiTiles items={kpis(stats)} />;
      case "equity":
        return <EquityChart rows={days} />;
      case "pnl-bars":
        return <PnlBars rows={rows} />;
      case "r-dist":
        return (
          <RDistribution
            rows={rRows(stats.rBuckets)}
            withR={stats.tradesWithR}
            closed={stats.closed}
          />
        );
      default:
        return null;
    }
  }

  // Called once when a drag or resize ends (not on every mouse move).
  const save = (grid: Layout) => {
    const next = changedLayout(layout, grid);
    if (next) onLayout(next);
  };

  return (
    <div ref={containerRef} className={editing ? "grid editing" : "grid"}>
      {mounted && (
        <ReactGridLayout
          width={width}
          layout={layout}
          gridConfig={{
            cols: 12,
            rowHeight: 40,
            margin: [12, 12],
            containerPadding: [0, 0],
          }}
          dragConfig={{ enabled: editing, handle: ".widget-head" }}
          resizeConfig={{ enabled: editing }}
          onDragStop={save}
          onResizeStop={save}
        >
          {layout.map((item) => (
            <section
              key={item.i}
              className="widget"
              aria-label={TITLES[item.i]}
            >
              <h2 className="widget-head">
                {TITLES[item.i] ?? item.i}
                {editing && <span className="drag-hint">drag to move</span>}
              </h2>
              <div className="widget-body">{body(item.i)}</div>
            </section>
          ))}
        </ReactGridLayout>
      )}
    </div>
  );
}
