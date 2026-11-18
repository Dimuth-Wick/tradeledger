import { useEffect, useMemo, useState } from "react";
import DashboardGrid from "../components/dashboard/DashboardGrid";
import {
  getDashboardLayout,
  getStats,
  listSymbols,
  resetDashboardLayout,
  saveDashboardLayout,
  type LayoutItem,
  type Stats,
} from "../lib/api";
import {
  EMPTY_DASH,
  PRESETS,
  dashQuery,
  presetRange,
  type DashFilters,
  type Preset,
} from "../lib/dashboard";

/** Stats for the chosen period. Every number comes from Rust `get_stats`. */
export default function DashboardPage() {
  const [filters, setFilters] = useState<DashFilters>(EMPTY_DASH);
  const [stats, setStats] = useState<Stats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [symbols, setSymbols] = useState<string[]>([]);
  const [layout, setLayout] = useState<LayoutItem[] | null>(null);
  const [layoutError, setLayoutError] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);

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

  useEffect(() => {
    getDashboardLayout().then(setLayout, (err) => setLayoutError(String(err)));
  }, []);

  // Show the move at once; Rust checks and stores it. On error, reload
  // what is really saved.
  function saveLayout(next: LayoutItem[]) {
    setLayout(next);
    saveDashboardLayout(next).then(
      (saved) => {
        setLayout(saved);
        setLayoutError(null);
      },
      (err) => {
        setLayoutError(`Layout not saved: ${String(err)}`);
        getDashboardLayout().then(setLayout, () => {});
      },
    );
  }

  function resetLayout() {
    resetDashboardLayout().then(
      (l) => {
        setLayout(l);
        setLayoutError(null);
      },
      (err) => setLayoutError(String(err)),
    );
  }

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
        <span className="spacer" />
        <button
          type="button"
          disabled={editing}
          onClick={() => setEditing(true)}
        >
          Edit layout
        </button>
      </div>

      {editing && (
        <div className="edit-bar" role="status">
          <span>
            Drag a widget by its title. Resize it from the bottom-right corner.
            Changes save on their own.
          </span>
          <button type="button" onClick={resetLayout}>
            Reset layout
          </button>
          <button
            type="button"
            className="primary"
            onClick={() => setEditing(false)}
          >
            Done
          </button>
        </div>
      )}

      {error && <p className="status error">{error}</p>}
      {layoutError && <p className="status error">{layoutError}</p>}

      {stats && layout && (
        <DashboardGrid
          stats={stats}
          layout={layout}
          editing={editing}
          onLayout={saveLayout}
        />
      )}
    </main>
  );
}
