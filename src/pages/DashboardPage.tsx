import { useEffect, useMemo, useState } from "react";
import EquityChart from "../components/dashboard/EquityChart";
import KpiTiles from "../components/dashboard/KpiTiles";
import PnlBars from "../components/dashboard/PnlBars";
import RDistribution from "../components/dashboard/RDistribution";
import { getStats, listSymbols, type Stats } from "../lib/api";
import {
  EMPTY_DASH,
  PRESETS,
  curveRows,
  dashQuery,
  kpis,
  presetRange,
  rRows,
  type DashFilters,
  type Preset,
} from "../lib/dashboard";

/** Stats for the chosen period. Every number comes from Rust `get_stats`. */
export default function DashboardPage() {
  const [filters, setFilters] = useState<DashFilters>(EMPTY_DASH);
  const [stats, setStats] = useState<Stats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [symbols, setSymbols] = useState<string[]>([]);

  const query = useMemo(() => dashQuery(filters), [filters]);

  useEffect(() => {
    let cancelled = false;
    getStats(query).then(
      (s) => {
        if (cancelled) return;
        setStats(s);
        setError(null);
      },
      (err) => !cancelled && setError(String(err)),
    );
    return () => {
      cancelled = true;
    };
  }, [query]);

  useEffect(() => {
    listSymbols().then(setSymbols, () => setSymbols([]));
  }, []);

  const rows = useMemo(() => (stats ? curveRows(stats) : []), [stats]);

  function set<K extends keyof DashFilters>(key: K, value: DashFilters[K]) {
    setFilters((f) => ({ ...f, [key]: value }));
  }

  return (
    <main className="page dashboard">
      <div className="filters" role="search">
        <label>
          Period
          <select
            value=""
            onChange={(e) => {
              const range = presetRange(e.target.value as Preset, new Date());
              setFilters((f) => ({ ...f, ...range }));
            }}
          >
            <option value="" disabled>
              Pick…
            </option>
            {PRESETS.map(([id, label]) => (
              <option key={id} value={id}>
                {label}
              </option>
            ))}
          </select>
        </label>
        <label>
          From
          <input
            type="date"
            value={filters.from}
            onChange={(e) => set("from", e.target.value)}
          />
        </label>
        <label>
          To
          <input
            type="date"
            value={filters.to}
            onChange={(e) => set("to", e.target.value)}
          />
        </label>
        <label>
          Symbol
          <input
            list="dash-symbols"
            value={filters.symbol}
            placeholder="any"
            onChange={(e) => set("symbol", e.target.value)}
          />
          <datalist id="dash-symbols">
            {symbols.map((s) => (
              <option key={s} value={s} />
            ))}
          </datalist>
        </label>
        <label>
          Market
          <select
            value={filters.market}
            onChange={(e) =>
              set("market", e.target.value as DashFilters["market"])
            }
          >
            <option value="">Any</option>
            <option value="usdm">Futures</option>
            <option value="spot">Spot</option>
          </select>
        </label>
        <label>
          Side
          <select
            value={filters.direction}
            onChange={(e) =>
              set("direction", e.target.value as DashFilters["direction"])
            }
          >
            <option value="">Any</option>
            <option value="long">Long</option>
            <option value="short">Short</option>
          </select>
        </label>
        <button
          type="button"
          disabled={Object.values(filters).every((v) => v === "")}
          onClick={() => setFilters(EMPTY_DASH)}
        >
          Clear filters
        </button>
      </div>

      {error && <p className="status error">{error}</p>}

      {stats && (
        <>
          <KpiTiles items={kpis(stats)} />
          <div className="widgets">
            <section className="widget wide">
              <h2>Equity curve</h2>
              <EquityChart rows={rows} />
            </section>
            <section className="widget">
              <h2>Win / loss per trade</h2>
              <PnlBars rows={rows} />
            </section>
            <section className="widget">
              <h2>R distribution</h2>
              <RDistribution
                rows={rRows(stats.rBuckets)}
                withR={stats.tradesWithR}
                closed={stats.closed}
              />
            </section>
          </div>
        </>
      )}
    </main>
  );
}
