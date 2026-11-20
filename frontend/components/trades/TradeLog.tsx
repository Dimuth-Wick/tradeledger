import { useEffect, useMemo, useState } from "react";
import {
  exportTradesCsv,
  listTagGroups,
  queryTrades,
  type TagGroup,
  type TradePage,
} from "../../connection/backend";
import { pnlClass, usd } from "../../logic/format";
import { indexTags } from "../../logic/tagIndex";
import {
  DEFAULT_SORT,
  EMPTY_FILTERS,
  PAGE_SIZES,
  buildQuery,
  hasFilters,
  nextSort,
  type LogFilters,
  type LogSort,
} from "../../logic/tradeLog";
import TradeList from "./TradeList";
import { save } from "@tauri-apps/plugin-dialog";
import { CSV_FILTER, csvFileName } from "../../logic/dataFiles";

interface Props {
  /** Bumped by the parent after a save/merge/split so the log refetches. */
  version: number;
  onDelete: (id: number) => void;
  onMerge: (id: number) => void;
  onSplit: (tradeId: number, fillId: number) => void;
  onReset: (tradeId: number) => void;
  onOpen: (id: number) => void;
}

/** Date and tag filters + sortable trade table + pagination. All querying
 *  is in Rust. Market, side, coin and result live on the Dashboard as charts. */
export default function TradeLog({ version, ...actions }: Props) {
  const [filters, setFilters] = useState<LogFilters>(EMPTY_FILTERS);
  const [sort, setSort] = useState<LogSort>(DEFAULT_SORT);
  const [page, setPage] = useState(0);
  const [pageSize, setPageSize] = useState<number>(PAGE_SIZES[0]);
  const [result, setResult] = useState<TradePage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tagGroups, setTagGroups] = useState<TagGroup[]>([]);
  const tagInfo = useMemo(() => indexTags(tagGroups), [tagGroups]);
  const [exporting, setExporting] = useState(false);
  const [exported, setExported] = useState<string | null>(null);

  const query = useMemo(
    () => buildQuery(filters, sort, page, pageSize),
    [filters, sort, page, pageSize],
  );

  /** Saves every trade matching the filters (all pages), via Rust. */
  async function exportCsv() {
    setExporting(true);
    setExported(null);
    try {
      const path = await save({
        defaultPath: csvFileName(new Date()),
        filters: [CSV_FILTER],
      });
      if (!path) return;
      const rows = await exportTradesCsv(query, path);
      setExported(`Exported ${rows} trade${rows === 1 ? "" : "s"} to ${path}`);
    } catch (err) {
      setError(String(err));
    } finally {
      setExporting(false);
    }
  }

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
    listTagGroups().then(setTagGroups, () => setTagGroups([]));
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
          Tag
          <select
            value={filters.tag}
            onChange={(e) => setFilter("tag", e.target.value)}
          >
            <option value="">Any</option>
            {tagGroups
              .filter((g) => g.tags.length > 0)
              .map((g) => (
                <optgroup key={g.id} label={g.name}>
                  {g.tags.map((t) => (
                    <option key={t.id} value={String(t.id)}>
                      {t.name}
                    </option>
                  ))}
                </optgroup>
              ))}
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
        <button
          type="button"
          disabled={exporting || (result?.total ?? 0) === 0}
          title="Save the trades matching these filters as a CSV file"
          onClick={exportCsv}
        >
          Export CSV
        </button>
      </div>

      {error && <p className="status error">{error}</p>}
      {exported && <p className="status ok">{exported}</p>}

      {result && (
        <>
          <p className="log-summary">
            {result.total} trade{result.total === 1 ? "" : "s"} · {result.wins}{" "}
            win{result.wins === 1 ? "" : "s"} / {result.losses} loss
            {result.losses === 1 ? "" : "es"} · closed net{" "}
            <span className={pnlClass(result.closedNetPnl)}>
              {usd(result.closedNetPnl, { round: true })}
            </span>
          </p>

          <TradeList
            trades={result.items}
            tradeTags={result.tags}
            tagInfo={tagInfo}
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
