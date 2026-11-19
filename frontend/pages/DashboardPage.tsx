import { useEffect, useMemo, useState } from "react";
import DashboardGrid from "../components/dashboard/DashboardGrid";
import {
  getDashboardLayout,
  getStats,
  resetDashboardLayout,
  saveDashboardLayout,
  type LayoutItem,
  type Stats,
} from "../connection/backend";
import StatsFilters from "../components/shared/StatsFilters";
import { EMPTY_DASH, dashQuery, type DashFilters } from "../logic/dashboard";

/** Stats for the chosen period. Every number comes from Rust `get_stats`. */
export default function DashboardPage() {
  const [filters, setFilters] = useState<DashFilters>(EMPTY_DASH);
  const [stats, setStats] = useState<Stats | null>(null);
  const [error, setError] = useState<string | null>(null);
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

  return (
    <main className="page dashboard">
      <StatsFilters filters={filters} onChange={setFilters}>
        <button
          type="button"
          disabled={editing}
          onClick={() => setEditing(true)}
        >
          Edit layout
        </button>
      </StatsFilters>

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
