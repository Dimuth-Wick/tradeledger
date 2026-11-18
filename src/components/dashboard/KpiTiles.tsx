import type { Kpi } from "../../lib/dashboard";

export default function KpiTiles({ items }: { items: Kpi[] }) {
  return (
    <dl className="kpis">
      {items.map((k) => (
        <div key={k.label}>
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
