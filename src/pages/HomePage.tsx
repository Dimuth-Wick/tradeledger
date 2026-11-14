import { useCallback, useEffect, useState } from "react";
import TradeForm from "../components/TradeForm";
import TradeList from "../components/TradeList";
import {
  dbInfo,
  deleteTrade,
  listTrades,
  mergeTradeWithPrevious,
  resetGrouping,
  splitTradeAtFill,
  type DbInfo,
  type RegroupSummary,
  type Trade,
  type WithId,
} from "../lib/api";
import { regroupSummary } from "../lib/importText";

export default function HomePage() {
  const [trades, setTrades] = useState<WithId<Trade>[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [db, setDb] = useState<DbInfo | null>(null);

  const reload = useCallback(() => {
    listTrades()
      .then((list) => {
        setTrades(list);
        setError(null);
      })
      .catch((err) => setError(String(err)));
  }, []);

  useEffect(() => {
    reload();
    dbInfo()
      .then(setDb)
      .catch((err) => setError(String(err)));
  }, [reload]);

  /** Runs a grouping action, then shows what changed and reloads the list. */
  async function regroupAction(
    label: string,
    run: () => Promise<RegroupSummary>,
  ) {
    setError(null);
    setNotice(null);
    try {
      setNotice(`${label}: ${regroupSummary(await run())}`);
      reload();
    } catch (err) {
      setError(String(err));
    }
  }

  async function remove(id: number) {
    try {
      await deleteTrade(id);
      reload();
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <main className="page">
      <header>
        <h1>TradeLedger</h1>
      </header>

      <section>
        <h2>New trade</h2>
        <TradeForm onSaved={reload} />
      </section>

      <section>
        <h2>Recent trades</h2>
        {error && <p className="status error">{error}</p>}
        {notice && <p className="status ok">{notice}</p>}
        <TradeList
          trades={trades}
          onDelete={remove}
          onMerge={(id) =>
            regroupAction("Merged", () => mergeTradeWithPrevious(id))
          }
          onSplit={(tradeId, fillId) =>
            regroupAction("Split", () => splitTradeAtFill(tradeId, fillId))
          }
          onReset={(tradeId) =>
            regroupAction("Grouping reset", () => resetGrouping(tradeId))
          }
        />
      </section>

      {db && (
        <footer className="db-info">
          Database: {db.path} (schema v{db.schemaVersion})
        </footer>
      )}
    </main>
  );
}
