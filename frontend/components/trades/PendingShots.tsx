import { useEffect, useRef, useState } from "react";
import { imageFiles, rejectReason } from "../../logic/images";

/** An image chosen in the New trade form, not saved yet. */
export interface PendingShot {
  key: string;
  file: File;
  /** Blob URL for the thumbnail (revoked when removed). */
  url: string;
}

let nextKey = 0;

interface Props {
  shots: PendingShot[];
  onChange: (next: PendingShot[]) => void;
  disabled?: boolean;
}

/**
 * Screenshots for a trade that doesn't exist yet: they wait here (paste,
 * drop or pick) and the form attaches them right after the trade is saved.
 */
export default function PendingShots({ shots, onChange, disabled }: Props) {
  const [error, setError] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);
  const picker = useRef<HTMLInputElement>(null);
  // Latest props for event handlers (the paste listener lives across
  // renders); refs are only written in an effect, only read in handlers.
  const latest = useRef({ shots, onChange, disabled });
  useEffect(() => {
    latest.current = { shots, onChange, disabled };
  });

  function add(files: File[]) {
    const now = latest.current;
    if (files.length === 0 || now.disabled) return;
    setError(null);
    const fresh: PendingShot[] = [];
    for (const file of files) {
      const reason = rejectReason(file);
      if (reason) {
        setError(`${file.name || "Pasted image"}: ${reason}`);
        continue;
      }
      fresh.push({
        key: `p${nextKey++}`,
        file,
        url: URL.createObjectURL(file),
      });
    }
    if (fresh.length > 0) now.onChange([...now.shots, ...fresh]);
  }
  const addFiles = useRef(add);
  useEffect(() => {
    addFiles.current = add;
  });

  function remove(key: string) {
    const gone = shots.find((s) => s.key === key);
    if (gone) URL.revokeObjectURL(gone.url);
    onChange(shots.filter((s) => s.key !== key));
  }

  // Paste anywhere on the page; text pastes (notes, symbol) pass through.
  useEffect(() => {
    function onPaste(e: ClipboardEvent) {
      const files = imageFiles(e.clipboardData?.files ?? []);
      if (files.length === 0) return;
      e.preventDefault();
      addFiles.current(files);
    }
    window.addEventListener("paste", onPaste);
    return () => window.removeEventListener("paste", onPaste);
  }, []);

  // Free the thumbnails when the form goes away.
  useEffect(
    () => () => latest.current.shots.forEach((s) => URL.revokeObjectURL(s.url)),
    [],
  );

  return (
    <div
      className={`shots pending ${dragging ? "dragging" : ""}`}
      onDragOver={(e) => {
        e.preventDefault();
        setDragging(true);
      }}
      onDragLeave={() => setDragging(false)}
      onDrop={(e) => {
        e.preventDefault();
        setDragging(false);
        add([...e.dataTransfer.files]);
      }}
    >
      {shots.length > 0 && (
        <div className="shots-grid">
          {shots.map((s, i) => (
            <figure key={s.key}>
              <div className="thumb">
                <img src={s.url} alt={`Screenshot ${i + 1} to add`} />
              </div>
              <button
                type="button"
                className="shot-remove"
                aria-label={`Remove screenshot ${i + 1}`}
                title="Remove"
                disabled={disabled}
                onClick={() => remove(s.key)}
              >
                ✕
              </button>
            </figure>
          ))}
        </div>
      )}
      <div className="shots-add">
        <span className="hint">
          {shots.length === 0
            ? "Paste a screenshot (Ctrl+V), drop images here, or"
            : `${shots.length} image${shots.length === 1 ? "" : "s"} will be added when you save. Add more:`}
        </span>
        <button
          type="button"
          disabled={disabled}
          onClick={() => picker.current?.click()}
        >
          Choose images
        </button>
        <input
          ref={picker}
          type="file"
          accept="image/png,image/jpeg,image/webp,image/gif"
          multiple
          hidden
          onChange={(e) => {
            add([...(e.target.files ?? [])]);
            e.target.value = "";
          }}
        />
      </div>
      {error && <p className="status error">{error}</p>}
    </div>
  );
}
