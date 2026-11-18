import { useEffect, useState } from "react";
import TradeForm from "../components/TradeForm";
import TradeLog from "../components/TradeLog";
import TradeDetail from "./TradeDetail";
import {
  dbInfo,
  deleteTrade,
  mergeTradeWithPrevious,
  resetGrouping,
  splitTradeAtFill,
  type DbInfo,
  type RegroupSummary,
} from "../lib/api";
import { regroupSummary } from "../lib/importText";
import { useGuard } from "../lib/unsaved";

export default function HomePage({
  initialOpenId = null,
}: {
  /** Show this trade first (opened from another tab). */
  initialOpenId?: number | null;
}) {
  // Bumping this tells the trade log to fetch again (after save/merge/...).
  const [version, setVersion] = useState(0);
  const refresh = () => setVersion((v) => v + 1);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [db, setDb] = useState<DbInfo | null>(null);
  // Trade shown on the detail page; null = form + log.
  const [openId, setOpenId] = useState<number | null>(initialOpenId);
  const guard = useGuard();

  useEffect(() => {
    dbInfo()
      .then(setDb)
      .catch((err) => setError(String(err)));
  }, []);

  /** Runs a grouping action, then shows what changed and refreshes the log. */
  async function regroupAction(
    label: string,
    run: () => Promise<RegroupSummary>,
  ) {
    setError(null);
    setNotice(null);
    try {
      setNotice(`${label}: ${regroupSummary(await run())}`);
      refresh();
    } catch (err) {
      setError(String(err));
    }
  }

  async function remove(id: number) {
    setError(null);
    try {
      await deleteTrade(id);
      refresh();
    } catch (err) {
      setError(String(err));
    }
  }

  if (openId !== null) {
    return (
      <TradeDetail
        tradeId={openId}
        onBack={() =>
          guard(() => {
            setOpenId(null);
            refresh(); // journal edits change R values shown in the log
          })
        }
      />
    );
  }

  return (
    <main className="page">
      <header>
        <h1>Trades</h1>
      </header>

      <section>
        <h2>New trade</h2>
        <TradeForm onSaved={refresh} />
      </section>

      <section>
        <h2>Trade log</h2>
        {error && <p className="status error">{error}</p>}
        {notice && <p className="status ok">{notice}</p>}
        <TradeLog
          version={version}
          // Opening a trade leaves the half-filled New trade form behind.
          onOpen={(id) => guard(() => setOpenId(id))}
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
