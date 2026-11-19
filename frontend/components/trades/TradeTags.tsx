import { useEffect, useState } from "react";
import {
  getTradeTags,
  listTagGroups,
  setTradeTags,
  type TagGroup,
} from "../../connection/backend";
import { FALLBACK_COLOR, toggleTag } from "../../logic/tagIndex";
import TagChip from "../shared/TagChip";

/** Tag picker on the trade detail page. Each click saves immediately. */
export default function TradeTags({ tradeId }: { tradeId: number }) {
  const [groups, setGroups] = useState<TagGroup[] | null>(null);
  const [selected, setSelected] = useState<number[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    Promise.all([listTagGroups(), getTradeTags(tradeId)]).then(
      ([g, ids]) => {
        if (cancelled) return;
        setGroups(g);
        setSelected(ids);
      },
      (err) => !cancelled && setError(String(err)),
    );
    return () => {
      cancelled = true;
    };
  }, [tradeId]);

  async function toggle(id: number) {
    const before = selected;
    const next = toggleTag(selected, id);
    setSelected(next); // show it right away; undo below if saving fails
    setError(null);
    try {
      setSelected(await setTradeTags(tradeId, next));
    } catch (err) {
      setSelected(before);
      setError(String(err));
    }
  }

  if (!groups) return error ? <p className="status error">{error}</p> : null;
  const withTags = groups.filter((g) => g.tags.length > 0);

  return (
    <div className="trade-tags">
      {withTags.length === 0 && (
        <p className="hint">No tags yet. Create some in Settings → Tags.</p>
      )}
      {withTags.map((g) => (
        <div className="tag-line" key={g.id}>
          <span className="tag-group-name">{g.name}</span>
          {g.tags.map((t) => (
            <TagChip
              key={t.id}
              name={t.name}
              color={t.color || g.color || FALLBACK_COLOR}
              selected={selected.includes(t.id)}
              onClick={() => toggle(t.id)}
            />
          ))}
        </div>
      ))}
      {error && <p className="status error">{error}</p>}
    </div>
  );
}
