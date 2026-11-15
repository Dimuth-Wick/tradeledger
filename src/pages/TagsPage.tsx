import { useCallback, useEffect, useState } from "react";
import {
  createTag,
  createTagGroup,
  deleteTag,
  deleteTagGroup,
  listTagGroups,
  updateTag,
  updateTagGroup,
  type TagGroup,
} from "../lib/api";
import { FALLBACK_COLOR } from "../lib/tagIndex";
import { useUnsavedChanges } from "../lib/unsaved";

/** Manage tag groups (e.g. Setup, Mistake, Emotion) and their tags. */
export default function TagsPage() {
  const [groups, setGroups] = useState<TagGroup[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(() => {
    listTagGroups().then(setGroups, (err) => setError(String(err)));
  }, []);
  useEffect(reload, [reload]);

  /** Runs a change, shows its error if any, then reloads the list. */
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

  return (
    <section className="tags-page">
      <h2>Tags</h2>
      <p className="hint">
        Group your tags, e.g. <b>Setup</b> (what you traded), <b>Mistake</b>{" "}
        (what went wrong), <b>Emotion</b> (how you felt). Tags are added to
        trades on the trade detail page.
      </p>
      {error && <p className="status error">{error}</p>}

      <AddRow
        label="New group"
        placeholder="e.g. Market condition"
        initialColor="#6b7280"
        onAdd={(name, color) => act(() => createTagGroup(name, color))}
      />

      {groups?.length === 0 && <p className="hint">No tag groups yet.</p>}
      {groups?.map((g) => (
        <div className="card tag-group" key={g.id}>
          <EditRow
            name={g.name}
            color={g.color || FALLBACK_COLOR}
            deleteText={
              g.tags.length > 0
                ? `Delete group and its ${g.tags.length} tag(s)? They are removed from all trades.`
                : "Delete this group?"
            }
            onSave={(name, color) =>
              act(() => updateTagGroup(g.id, name, color))
            }
            onDelete={() => act(() => deleteTagGroup(g.id))}
            heading
          />
          <ul className="tag-rows">
            {g.tags.map((t) => (
              <li key={t.id}>
                <EditRow
                  name={t.name}
                  color={t.color || g.color || FALLBACK_COLOR}
                  deleteText="Delete this tag? It's removed from all trades."
                  onSave={(name, color) =>
                    act(() => updateTag(t.id, name, color))
                  }
                  onDelete={() => act(() => deleteTag(t.id))}
                />
              </li>
            ))}
          </ul>
          <AddRow
            label={`New tag in ${g.name}`}
            placeholder="e.g. Breakout"
            initialColor={g.color || FALLBACK_COLOR}
            onAdd={(name, color) => act(() => createTag(g.id, name, color))}
          />
        </div>
      ))}
    </section>
  );
}

/** Name + color inputs with an Add button. */
function AddRow({
  label,
  placeholder,
  initialColor,
  onAdd,
}: {
  label: string;
  placeholder: string;
  initialColor: string;
  onAdd: (name: string, color: string) => Promise<boolean>;
}) {
  const [name, setName] = useState("");
  const [color, setColor] = useState(initialColor);
  useUnsavedChanges(name.trim() !== "");

  return (
    <form
      className="tag-add"
      onSubmit={async (e) => {
        e.preventDefault();
        if (await onAdd(name, color)) setName("");
      }}
    >
      <input
        aria-label={label}
        placeholder={placeholder}
        value={name}
        onChange={(e) => setName(e.target.value)}
      />
      <input
        type="color"
        aria-label={`${label} color`}
        value={color}
        onChange={(e) => setColor(e.target.value)}
      />
      <button type="submit" disabled={name.trim() === ""}>
        Add
      </button>
    </form>
  );
}

/** Editable name + color with Save (when changed) and two-step Delete. */
function EditRow({
  name,
  color,
  deleteText,
  onSave,
  onDelete,
  heading,
}: {
  name: string;
  color: string;
  deleteText: string;
  onSave: (name: string, color: string) => Promise<boolean>;
  onDelete: () => Promise<boolean>;
  heading?: boolean;
}) {
  const [draftName, setDraftName] = useState(name);
  const [draftColor, setDraftColor] = useState(color);
  const [confirming, setConfirming] = useState(false);
  const dirty = draftName.trim() !== name || draftColor !== color;
  useUnsavedChanges(dirty);

  return (
    <div className={`tag-edit ${heading ? "heading" : ""}`}>
      <input
        aria-label={heading ? "Group name" : "Tag name"}
        value={draftName}
        onChange={(e) => setDraftName(e.target.value)}
      />
      <input
        type="color"
        aria-label={heading ? "Group color" : "Tag color"}
        value={draftColor}
        onChange={(e) => setDraftColor(e.target.value)}
      />
      {dirty && (
        <>
          <button
            type="button"
            className="primary small"
            onClick={() => onSave(draftName, draftColor)}
          >
            Save
          </button>
          <button
            type="button"
            onClick={() => {
              setDraftName(name);
              setDraftColor(color);
            }}
          >
            Undo
          </button>
        </>
      )}
      {confirming ? (
        <span className="confirm-inline">
          {deleteText}
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
          {heading ? "Delete group" : "Delete"}
        </button>
      )}
    </div>
  );
}
