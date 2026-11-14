import { useCallback, useEffect, useState } from "react";
import TradeForm from "../components/TradeForm";
import TradeList from "../components/TradeList";
import {
  dbInfo,
  deleteTrade,
  listTrades,
  type DbInfo,
  type Trade,
  type WithId,
} from "../lib/api";

export default function HomePage() {
  const [trades, setTrades] = useState<WithId<Trade>[]>([]);
  const [error, setError] = useState<string | null>(null);
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
        <TradeList trades={trades} onDelete={remove} />
      </section>

      {db && (
        <footer className="db-info">
          Database: {db.path} (schema v{db.schemaVersion})
        </footer>
      )}
    </main>
  );
}
