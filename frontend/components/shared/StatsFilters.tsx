import { useEffect, useState, type ReactNode } from "react";
import { listSymbols } from "../../connection/backend";
import {
  EMPTY_DASH,
  PRESETS,
  presetRange,
  type DashFilters,
  type Preset,
} from "../../logic/dashboard";

interface Props {
  filters: DashFilters;
  onChange: (next: DashFilters) => void;
  /** Extra controls at the right end of the row (e.g. Edit layout). */
  children?: ReactNode;
}

/** Period / symbol / market / side filters shared by Dashboard and Reports. */
export default function StatsFilters({ filters, onChange, children }: Props) {
  const [symbols, setSymbols] = useState<string[]>([]);

  useEffect(() => {
    listSymbols().then(setSymbols, () => setSymbols([]));
  }, []);

  function set<K extends keyof DashFilters>(key: K, value: DashFilters[K]) {
    onChange({ ...filters, [key]: value });
  }

  return (
    <div className="filters" role="search">
      <label>
        Period
        <select
          value=""
          onChange={(e) =>
            onChange({
              ...filters,
              ...presetRange(e.target.value as Preset, new Date()),
            })
          }
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
          list="stats-symbols"
          value={filters.symbol}
          placeholder="any"
          onChange={(e) => set("symbol", e.target.value)}
        />
        <datalist id="stats-symbols">
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
        onClick={() => onChange(EMPTY_DASH)}
      >
        Clear filters
      </button>
      {children && (
        <>
          <span className="spacer" />
          {children}
        </>
      )}
    </div>
  );
}
