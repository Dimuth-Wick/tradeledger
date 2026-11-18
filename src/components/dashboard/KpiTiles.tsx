import type { CSSProperties } from "react";
import type { Kpi } from "../../lib/dashboard";

export default function KpiTiles({ items }: { items: Kpi[] }) {
  return (
    <dl className="kpis">
      {items.map((k, i) => (
        <div key={k.label} style={{ "--i": i } as CSSProperties}>
          <dt>{k.label}</dt>
          <dd key={k.value} className={`kpi-value ${k.tone}`} title={k.title}>
            {k.value}
          </dd>
          <dd className="kpi-detail">{k.detail}</dd>
        </div>
      ))}
    </dl>
  );
}
