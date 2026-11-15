import { useCallback, useEffect, useRef, useState } from "react";
import {
  addScreenshot,
  deleteScreenshot,
  listScreenshots,
  screenshotImage,
  setScreenshotCaption,
  type Screenshot,
} from "../lib/api";
import { formatDateTime } from "../lib/format";
import { imageFiles, mimeFromFileName, rejectReason } from "../lib/images";
import { useUnsavedChanges } from "../lib/unsaved";

/**
 * Screenshots of one trade: paste (Ctrl+V, e.g. after Win+Shift+S),
 * drag & drop, or choose files. Click a thumbnail for the full view.
 */
export default function Screenshots({ tradeId }: { tradeId: number }) {
  const [shots, setShots] = useState<Screenshot[]>([]);
  const [urls, setUrls] = useState<Record<number, string>>({});
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [dragging, setDragging] = useState(false);
  const [viewing, setViewing] = useState<number | null>(null);
  const picker = useRef<HTMLInputElement>(null);
  // Blob URLs hold memory until revoked; remember them to clean up.
  const created = useRef<string[]>([]);

  const loadImage = useCallback(async (shot: Screenshot) => {
    const bytes = await screenshotImage(shot.id);
    const url = URL.createObjectURL(
      new Blob([bytes], { type: mimeFromFileName(shot.fileName) }),
    );
    created.current.push(url);
    setUrls((u) => ({ ...u, [shot.id]: url }));
  }, []);

  useEffect(() => {
    let cancelled = false;
    listScreenshots(tradeId).then(
      (list) => {
        if (cancelled) return;
        setShots(list);
        list.forEach((s) => loadImage(s).catch((err) => setError(String(err))));
      },
      (err) => !cancelled && setError(String(err)),
    );
    return () => {
      cancelled = true;
    };
  }, [tradeId, loadImage]);

  useEffect(
    () => () => created.current.forEach((u) => URL.revokeObjectURL(u)),
    [],
  );

  const upload = useCallback(
    async (files: File[]) => {
      if (files.length === 0) return;
      setError(null);
      setBusy(true);
      try {
        for (const file of files) {
          const reason = rejectReason(file);
          if (reason) {
            setError(`${file.name || "Pasted image"}: ${reason}`);
            continue;
          }
          const bytes = new Uint8Array(await file.arrayBuffer());
          const shot = await addScreenshot(tradeId, bytes);
          setShots((s) => [...s, shot]);
          await loadImage(shot);
        }
      } catch (err) {
        setError(String(err));
      } finally {
        setBusy(false);
      }
    },
    [tradeId, loadImage],
  );

  // Paste anywhere on the page. Text pastes (e.g. into notes) pass through.
  useEffect(() => {
    function onPaste(e: ClipboardEvent) {
      const files = imageFiles(e.clipboardData?.files ?? []);
      if (files.length === 0) return;
      e.preventDefault();
      void upload(files);
    }
    window.addEventListener("paste", onPaste);
    return () => window.removeEventListener("paste", onPaste);
  }, [upload]);

  async function remove(id: number) {
    try {
      await deleteScreenshot(id);
      setShots((s) => s.filter((x) => x.id !== id));
      setViewing(null);
    } catch (err) {
      setError(String(err));
    }
  }

  async function saveCaption(id: number, caption: string) {
    await setScreenshotCaption(id, caption);
    setShots((s) =>
      s.map((x) => (x.id === id ? { ...x, caption: caption.trim() } : x)),
    );
  }

  const index = shots.findIndex((s) => s.id === viewing);

  return (
    <div
      className={`shots ${dragging ? "dragging" : ""}`}
      onDragOver={(e) => {
        e.preventDefault();
        setDragging(true);
      }}
      onDragLeave={() => setDragging(false)}
      onDrop={(e) => {
        e.preventDefault();
        setDragging(false);
        void upload([...e.dataTransfer.files]);
      }}
    >
      <div className="shots-grid">
        {shots.map((s) => (
          <figure key={s.id}>
            <button
              type="button"
              className="thumb"
              title="Open full size"
              onClick={() => setViewing(s.id)}
            >
              {urls[s.id] ? (
                <img src={urls[s.id]} alt={s.caption || "Trade screenshot"} />
              ) : (
                <span className="hint">Loading…</span>
              )}
            </button>
            {s.caption && <figcaption>{s.caption}</figcaption>}
          </figure>
        ))}
      </div>

      <div className="shots-add">
        <span className="hint">
          {busy
            ? "Saving image…"
            : "Paste a screenshot (Ctrl+V), drop images here, or"}
        </span>
        <button
          type="button"
          disabled={busy}
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
            void upload([...(e.target.files ?? [])]);
            e.target.value = "";
          }}
        />
      </div>
      {error && <p className="status error">{error}</p>}

      {index >= 0 && (
        <Viewer
          key={shots[index].id}
          shot={shots[index]}
          url={urls[shots[index].id]}
          position={`${index + 1} / ${shots.length}`}
          onPrev={index > 0 ? () => setViewing(shots[index - 1].id) : undefined}
          onNext={
            index < shots.length - 1
              ? () => setViewing(shots[index + 1].id)
              : undefined
          }
          onClose={() => setViewing(null)}
          onDelete={() => remove(shots[index].id)}
          onSaveCaption={(c) => saveCaption(shots[index].id, c)}
        />
      )}
    </div>
  );
}

function Viewer({
  shot,
  url,
  position,
  onPrev,
  onNext,
  onClose,
  onDelete,
  onSaveCaption,
}: {
  shot: Screenshot;
  url: string | undefined;
  position: string;
  onPrev?: () => void;
  onNext?: () => void;
  onClose: () => void;
  onDelete: () => void;
  onSaveCaption: (caption: string) => Promise<void>;
}) {
  const [caption, setCaption] = useState(shot.caption);
  const [confirming, setConfirming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const dirty = caption.trim() !== shot.caption;
  useUnsavedChanges(dirty);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape" && !dirty) onClose();
      if (e.key === "ArrowLeft" && onPrev && !dirty) onPrev();
      if (e.key === "ArrowRight" && onNext && !dirty) onNext();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [dirty, onClose, onPrev, onNext]);

  return (
    <div className="modal-backdrop viewer-backdrop">
      <div
        className="viewer"
        role="dialog"
        aria-modal="true"
        aria-label="Screenshot"
      >
        <div className="viewer-bar">
          <span className="hint">
            {position} · added {formatDateTime(shot.createdAt)}
          </span>
          <span>
            <button type="button" disabled={!onPrev || dirty} onClick={onPrev}>
              ← Prev
            </button>
            <button type="button" disabled={!onNext || dirty} onClick={onNext}>
              Next →
            </button>
            <button type="button" disabled={dirty} onClick={onClose}>
              Close
            </button>
          </span>
        </div>
        {url ? (
          <img
            className="viewer-img"
            src={url}
            alt={shot.caption || "Trade screenshot"}
          />
        ) : (
          <p className="hint">Loading…</p>
        )}
        <div className="viewer-bar">
          <input
            aria-label="Caption"
            className="viewer-caption"
            placeholder="Caption, e.g. entry on retest of the range high"
            value={caption}
            maxLength={200}
            onChange={(e) => setCaption(e.target.value)}
          />
          {dirty && (
            <>
              <button
                type="button"
                className="primary small"
                onClick={() =>
                  onSaveCaption(caption).then(
                    () => setError(null),
                    (err) => setError(String(err)),
                  )
                }
              >
                Save caption
              </button>
              <button type="button" onClick={() => setCaption(shot.caption)}>
                Undo
              </button>
            </>
          )}
          {confirming ? (
            <span className="confirm-inline">
              Delete this screenshot?
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
              Delete screenshot
            </button>
          )}
        </div>
        {error && <p className="status error">{error}</p>}
      </div>
    </div>
  );
}
