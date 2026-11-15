import { useEffect, useMemo, useState } from "react";
import { listSymbols, queryTrades, type TradePage } from "../lib/api";
import { pnlClass, signed } from "../lib/format";
import {
  DEFAULT_SORT,
  EMPTY_FILTERS,
  PAGE_SIZES,
  buildQuery,
  hasFilters,
  nextSort,
  type LogFilters,
  type LogSort,
} from "../lib/tradeLog";
import TradeList from "./TradeList";

interface Props {
  /** Bumped by the parent after a save/merge/split so the log refetches. */
  version: number;
  onDelete: (id: number) => void;
  onMerge: (id: number) => void;
  onSplit: (tradeId: number, fillId: number) => void;
  onReset: (tradeId: number) => void;
  onOpen: (id: number) => void;
}

/** Filter bar + sortable trade table + pagination. All querying is in Rust. */
export default function TradeLog({ version, ...actions }: Props) {
  const [filters, setFilters] = useState<LogFilters>(EMPTY_FILTERS);
  const [sort, setSort] = useState<LogSort>(DEFAULT_SORT);
  const [page, setPage] = useState(0);
  const [pageSize, setPageSize] = useState<number>(PAGE_SIZES[0]);
  const [result, setResult] = useState<TradePage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [symbols, setSymbols] = useState<string[]>([]);

  const query = useMemo(
    () => buildQuery(filters, sort, page, pageSize),
    [filters, sort, page, pageSize],
  );

  useEffect(() => {
    let cancelled = false;
    queryTrades(query).then(
      (r) => {
        if (cancelled) return;
        setResult(r);
        setError(null);
      },
      (err) => !cancelled && setError(String(err)),
    );
    return () => {
      cancelled = true;
    };
  }, [query, version]);

  useEffect(() => {
    listSymbols().then(setSymbols, () => setSymbols([]));
  }, [version]);

  // Any filter change starts again from the first page.
  function setFilter<K extends keyof LogFilters>(key: K, value: LogFilters[K]) {
    setFilters((f) => ({ ...f, [key]: value }));
    setPage(0);
  }

  return (
    <div className="trade-log">
      <div className="filters" role="search">
        <label>
          From
          <input
            type="date"
            value={filters.from}
            onChange={(e) => setFilter("from", e.target.value)}
          />
        </label>
        <label>
          To
          <input
            type="date"
            value={filters.to}
            onChange={(e) => setFilter("to", e.target.value)}
          />
        </label>
        <label>
          Symbol
          <input
            list="symbol-options"
            value={filters.symbol}
            placeholder="any"
            onChange={(e) => setFilter("symbol", e.target.value)}
          />
          <datalist id="symbol-options">
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
              setFilter("market", e.target.value as LogFilters["market"])
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
              setFilter("direction", e.target.value as LogFilters["direction"])
            }
          >
            <option value="">Any</option>
            <option value="long">Long</option>
            <option value="short">Short</option>
          </select>
        </label>
        <label>
          Status
          <select
            value={filters.status}
            onChange={(e) =>
              setFilter("status", e.target.value as LogFilters["status"])
            }
          >
            <option value="">Any</option>
            <option value="open">Open</option>
            <option value="closed">Closed</option>
          </select>
        </label>
        <label>
          Result
          <select
            value={filters.outcome}
            onChange={(e) =>
              setFilter("outcome", e.target.value as LogFilters["outcome"])
            }
          >
            <option value="">Any</option>
            <option value="win">Win</option>
            <option value="loss">Loss</option>
            <option value="breakeven">Breakeven</option>
          </select>
        </label>
        <button
          type="button"
          disabled={!hasFilters(filters)}
          onClick={() => {
            setFilters(EMPTY_FILTERS);
            setPage(0);
          }}
        >
          Clear filters
        </button>
      </div>

      {error && <p className="status error">{error}</p>}

      {result && (
        <>
          <p className="log-summary">
            {result.total} trade{result.total === 1 ? "" : "s"} · {result.wins}{" "}
            win{result.wins === 1 ? "" : "s"} / {result.losses} loss
            {result.losses === 1 ? "" : "es"} · closed net{" "}
            <span className={pnlClass(result.closedNetPnl)}>
              {signed(result.closedNetPnl)}
            </span>
          </p>

          <TradeList
            trades={result.items}
            sort={sort}
            onSort={(field) => {
              setSort((s) => nextSort(s, field));
              setPage(0);
            }}
            emptyText={
              hasFilters(filters) ? "No trades match these filters." : undefined
            }
            {...actions}
          />

          <div className="pager">
            <button
              type="button"
              disabled={result.page === 0}
              onClick={() => setPage(result.page - 1)}
            >
              ← Prev
            </button>
            <span>
              Page {result.page + 1} of {result.pageCount}
            </span>
            <button
              type="button"
              disabled={result.page >= result.pageCount - 1}
              onClick={() => setPage(result.page + 1)}
            >
              Next →
            </button>
            <label className="inline">
              Per page
              <select
                value={pageSize}
                onChange={(e) => {
                  setPageSize(Number(e.target.value));
                  setPage(0);
                }}
              >
                {PAGE_SIZES.map((n) => (
                  <option key={n} value={n}>
                    {n}
                  </option>
                ))}
              </select>
            </label>
          </div>
        </>
      )}
    </div>
  );
}
