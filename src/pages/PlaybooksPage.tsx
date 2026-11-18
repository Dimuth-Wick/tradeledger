import { useCallback, useEffect, useState } from "react";
import {
  createPlaybook,
  deletePlaybook,
  listPlaybooks,
  playbookStats,
  updatePlaybook,
  type Playbook,
  type PlaybookStats,
  type ReportRow,
} from "../lib/api";
import { formatR, pnlClass, round2, signed } from "../lib/format";
import {
  draftOf,
  moveRule,
  newRule,
  playbookDirty,
  toRuleInputs,
  type PlaybookDraft,
} from "../lib/playbooks";
import { profitFactorText } from "../lib/reports";
import { useUnsavedChanges } from "../lib/unsaved";

/** Setups with a rules checklist, and how each one performs. */
export default function PlaybooksPage() {
  const [playbooks, setPlaybooks] = useState<Playbook[] | null>(null);
  const [stats, setStats] = useState<PlaybookStats[]>([]);
  const [error, setError] = useState<string | null>(null);
  // null = list only; "new" or a playbook id = that editor is open.
  const [editing, setEditing] = useState<"new" | number | null>(null);

  const reload = useCallback(() => {
    listPlaybooks().then(setPlaybooks, (err) => setError(String(err)));
    playbookStats().then(setStats, () => setStats([]));
  }, []);
  useEffect(reload, [reload]);

  async function act(run: () => Promise<unknown>): Promise<boolean> {
    setError(null);
    try {
      await run();
      reload();
      return true;
    } catch (err) {
      setError(String(err));
      return false;
    }
  }

  async function save(p: Playbook | null, d: PlaybookDraft) {
    const rules = toRuleInputs(d.rules);
    const ok = await act(() =>
      p
        ? updatePlaybook(p.id, d.name, d.description, rules)
        : createPlaybook(
            d.name,
            d.description,
            rules.map((r) => r.text),
          ),
    );
    if (ok) setEditing(null);
  }

  return (
    <section className="playbooks-page">
      <div className="heading-row">
        <h2>Playbooks</h2>
        {editing === null && (
          <button type="button" onClick={() => setEditing("new")}>
            New playbook
          </button>
        )}
      </div>
      <p className="hint">
        A playbook is one setup you trade, with the rules it needs. On a trade's
        page, pick the playbook and tick the rules you followed. Here you see
        whether following every rule pays.
      </p>
      {error && <p className="status error">{error}</p>}

      {editing === "new" && (
        <PlaybookEditor
          playbook={null}
          onSave={(d) => save(null, d)}
          onCancel={() => setEditing(null)}
        />
      )}

      {playbooks?.length === 0 && editing !== "new" && (
        <p className="hint">No playbooks yet.</p>
      )}

      {playbooks?.map((p) =>
        editing === p.id ? (
          <PlaybookEditor
            key={p.id}
            playbook={p}
            onSave={(d) => save(p, d)}
            onCancel={() => setEditing(null)}
          />
        ) : (
          <PlaybookCard
            key={p.id}
            playbook={p}
            stats={stats.find((s) => s.playbookId === p.id)}
            canEdit={editing === null}
            onEdit={() => setEditing(p.id)}
            onDelete={() => act(() => deletePlaybook(p.id))}
          />
        ),
      )}
    </section>
  );
}

function PlaybookEditor({
  playbook,
  onSave,
  onCancel,
}: {
  playbook: Playbook | null;
  onSave: (d: PlaybookDraft) => void;
  onCancel: () => void;
}) {
  const [d, setD] = useState<PlaybookDraft>(() => draftOf(playbook));
  const dirty = playbookDirty(playbook, d);
  useUnsavedChanges(dirty);

  const setRule = (key: string, text: string) =>
    setD((x) => ({
      ...x,
      rules: x.rules.map((r) => (r.key === key ? { ...r, text } : r)),
    }));

  return (
    <form
      className="card playbook-editor"
      onSubmit={(e) => {
        e.preventDefault();
        onSave(d);
      }}
    >
      <label>
        Name
        <input
          aria-label="Playbook name"
          value={d.name}
          placeholder="e.g. Breakout retest"
          onChange={(e) => setD({ ...d, name: e.target.value })}
        />
      </label>
      <label>
        Description
        <textarea
          aria-label="Playbook description"
          rows={2}
          value={d.description}
          placeholder="When to use it, what the setup looks like"
          onChange={(e) => setD({ ...d, description: e.target.value })}
        />
      </label>
      <fieldset className="rules-edit">
        <legend>Rules (checklist, in order)</legend>
        {d.rules.map((r, i) => (
          <div key={r.key} className="rule-row">
            <span className="rule-num">{i + 1}.</span>
            <input
              aria-label={`Rule ${i + 1}`}
              value={r.text}
              placeholder="e.g. Higher timeframe trend agrees"
              onChange={(e) => setRule(r.key, e.target.value)}
            />
            <button
              type="button"
              aria-label={`Move rule ${i + 1} up`}
              disabled={i === 0}
              onClick={() => setD({ ...d, rules: moveRule(d.rules, i, -1) })}
            >
              ↑
            </button>
            <button
              type="button"
              aria-label={`Move rule ${i + 1} down`}
              disabled={i === d.rules.length - 1}
              onClick={() => setD({ ...d, rules: moveRule(d.rules, i, 1) })}
            >
              ↓
            </button>
            <button
              type="button"
              aria-label={`Remove rule ${i + 1}`}
              onClick={() =>
                setD({ ...d, rules: d.rules.filter((x) => x.key !== r.key) })
              }
            >
              ✕
            </button>
          </div>
        ))}
        <button
          type="button"
          className="link"
          onClick={() => setD({ ...d, rules: [...d.rules, newRule()] })}
        >
          + Add rule
        </button>
        {playbook && playbook.usedBy > 0 && (
          <p className="hint">
            Removing a rule also removes its tick from {playbook.usedBy} linked
            trade{playbook.usedBy === 1 ? "" : "s"}.
          </p>
        )}
      </fieldset>
      <div className="actions">
        <button
          type="submit"
          className="primary"
          disabled={!dirty || d.name.trim() === ""}
        >
          {playbook ? "Save playbook" : "Add playbook"}
        </button>
        <button type="button" onClick={onCancel}>
          Cancel
        </button>
      </div>
    </form>
  );
}

function PlaybookCard({
  playbook: p,
  stats,
  canEdit,
  onEdit,
  onDelete,
}: {
  playbook: Playbook;
  stats: PlaybookStats | undefined;
  canEdit: boolean;
  onEdit: () => void;
  onDelete: () => Promise<boolean>;
}) {
  const [confirming, setConfirming] = useState(false);
  return (
    <article className="card playbook-card">
      <div className="heading-row">
        <h3>{p.name}</h3>
        <span className="hint">
          used on {p.usedBy} trade{p.usedBy === 1 ? "" : "s"}
        </span>
        <span className="spacer" />
        {canEdit && (
          <button type="button" onClick={onEdit}>
            Edit
          </button>
        )}
        {confirming ? (
          <span className="confirm-inline">
            {p.usedBy > 0
              ? `Delete it? ${p.usedBy} trade(s) keep existing, without a playbook.`
              : "Delete this playbook?"}
            <button type="button" className="danger" onClick={onDelete}>
              Delete
            </button>
            <button type="button" onClick={() => setConfirming(false)}>
              Cancel
            </button>
          </span>
        ) : (
          <button
            type="button"
            className="link danger-link"
            onClick={() => setConfirming(true)}
          >
            Delete playbook
          </button>
        )}
      </div>
      {p.description && <p className="playbook-desc">{p.description}</p>}
      {p.rules.length > 0 ? (
        <ol className="rules-list">
          {p.rules.map((r) => (
            <li key={r.id}>{r.text}</li>
          ))}
        </ol>
      ) : (
        <p className="hint">No rules yet. Edit the playbook to add them.</p>
      )}

      {stats && stats.all.trades > 0 ? (
        <>
          <table className="trade-list report-table playbook-stats">
            <thead>
              <tr>
                <th scope="col" />
                <th scope="col" className="num">
                  Trades
                </th>
                <th scope="col" className="num">
                  Win rate
                </th>
                <th scope="col" className="num">
                  Net PnL
                </th>
                <th scope="col" className="num">
                  Avg / trade
                </th>
                <th scope="col" className="num">
                  Profit factor
                </th>
                <th scope="col" className="num">
                  Avg R
                </th>
              </tr>
            </thead>
            <tbody>
              <StatsRow label="All closed trades" r={stats.all} />
              <StatsRow label="Followed every rule" r={stats.followed} />
              <StatsRow label="Skipped a rule" r={stats.broke} />
            </tbody>
          </table>
          {stats.adherence !== null && (
            <p className="hint">
              On average {stats.adherence}% of the rules were followed.
            </p>
          )}
        </>
      ) : (
        <p className="hint">
          No closed trades with this playbook yet. Pick it on a trade's page.
        </p>
      )}
    </article>
  );
}

function StatsRow({ label, r }: { label: string; r: ReportRow }) {
  const money = (d: string | null) => (d === null ? "—" : signed(round2(d)));
  const empty = r.trades === 0;
  return (
    <tr className={empty ? "empty" : ""}>
      <th scope="row">{label}</th>
      <td className="num">{r.trades}</td>
      <td className="num">{r.winRate === null ? "—" : `${r.winRate}%`}</td>
      <td className={`num ${pnlClass(r.netPnl)}`}>
        {empty ? "—" : money(r.netPnl)}
      </td>
      <td className={`num ${pnlClass(r.avgPnl)}`}>{money(r.avgPnl)}</td>
      <td className="num">{profitFactorText(r)}</td>
      <td className={`num ${pnlClass(r.avgR)}`}>{formatR(r.avgR)}</td>
    </tr>
  );
}
