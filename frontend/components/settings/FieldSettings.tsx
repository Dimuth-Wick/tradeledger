import { useCallback, useEffect, useState } from "react";
import {
  createCustomField,
  deleteCustomField,
  listCustomFields,
  updateCustomField,
  type CustomField,
  type FieldType,
} from "../../connection/backend";
import { FIELD_TYPE_LABELS, parseOptions } from "../../logic/customFields";
import { useUnsavedChanges } from "../../logic/unsaved";

/** Settings → Fields: extra fields to fill in on every trade. */
export default function FieldSettings() {
  const [fields, setFields] = useState<CustomField[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(() => {
    listCustomFields().then(setFields, (err) => setError(String(err)));
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

  return (
    <section className="fields-page">
      <h3>Custom fields</h3>
      <p className="hint">
        Extra things to record on every trade, e.g. <b>Session</b> (choice:
        Asia, London, NY), <b>Confidence</b> (number), <b>Followed plan?</b>{" "}
        (checkbox). Fill them in on the trade detail page.
      </p>
      {error && <p className="status error">{error}</p>}

      <NewField
        onAdd={(name, type, options) =>
          act(() => createCustomField(name, type, options))
        }
      />

      {fields?.length === 0 && <p className="hint">No custom fields yet.</p>}
      {fields?.map((f) => (
        <FieldRow
          key={f.id}
          field={f}
          onSave={(name, options) =>
            act(() => updateCustomField(f.id, name, options))
          }
          onDelete={() => act(() => deleteCustomField(f.id))}
        />
      ))}
    </section>
  );
}

function NewField({
  onAdd,
}: {
  onAdd: (name: string, type: FieldType, options: string[]) => Promise<boolean>;
}) {
  const [name, setName] = useState("");
  const [type, setType] = useState<FieldType>("text");
  const [options, setOptions] = useState("");
  useUnsavedChanges(name.trim() !== "" || options.trim() !== "");

  return (
    <form
      className="card field-new"
      onSubmit={async (e) => {
        e.preventDefault();
        if (await onAdd(name, type, parseOptions(options))) {
          setName("");
          setOptions("");
        }
      }}
    >
      <label>
        New field
        <input
          aria-label="New field name"
          value={name}
          placeholder="e.g. Session"
          onChange={(e) => setName(e.target.value)}
        />
      </label>
      <label>
        Type
        <select
          aria-label="New field type"
          value={type}
          onChange={(e) => setType(e.target.value as FieldType)}
        >
          {(Object.keys(FIELD_TYPE_LABELS) as FieldType[]).map((t) => (
            <option key={t} value={t}>
              {FIELD_TYPE_LABELS[t]}
            </option>
          ))}
        </select>
      </label>
      {type === "select" && (
        <label className="grow">
          Options (comma-separated)
          <input
            aria-label="New field options"
            value={options}
            placeholder="Asia, London, NY"
            onChange={(e) => setOptions(e.target.value)}
          />
        </label>
      )}
      <button type="submit" disabled={name.trim() === ""}>
        Add field
      </button>
    </form>
  );
}

function FieldRow({
  field,
  onSave,
  onDelete,
}: {
  field: CustomField;
  onSave: (name: string, options: string[]) => Promise<boolean>;
  onDelete: () => Promise<boolean>;
}) {
  const savedOptions = field.options.join(", ");
  const [name, setName] = useState(field.name);
  const [options, setOptions] = useState(savedOptions);
  const [confirming, setConfirming] = useState(false);
  const dirty =
    name.trim() !== field.name ||
    (field.fieldType === "select" &&
      parseOptions(options).join(", ") !== savedOptions);
  useUnsavedChanges(dirty);

  return (
    <div className="card field-row">
      <div className="tag-edit heading">
        <input
          aria-label="Field name"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <span className="field-type">{FIELD_TYPE_LABELS[field.fieldType]}</span>
        <span className="hint">
          used on {field.usedBy} trade{field.usedBy === 1 ? "" : "s"}
        </span>
        {dirty && (
          <>
            <button
              type="button"
              className="primary small"
              onClick={() => onSave(name, parseOptions(options))}
            >
              Save
            </button>
            <button
              type="button"
              onClick={() => {
                setName(field.name);
                setOptions(savedOptions);
              }}
            >
              Undo
            </button>
          </>
        )}
        {confirming ? (
          <span className="confirm-inline">
            {field.usedBy > 0
              ? `Delete it and its values on ${field.usedBy} trade(s)?`
              : "Delete this field?"}
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
            Delete field
          </button>
        )}
      </div>
      {field.fieldType === "select" && (
        <label>
          Options (comma-separated)
          <input
            aria-label={`${field.name} options`}
            value={options}
            onChange={(e) => setOptions(e.target.value)}
          />
        </label>
      )}
    </div>
  );
}
