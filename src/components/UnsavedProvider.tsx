import { useMemo, useRef, useState, type ReactNode } from "react";
import { UnsavedContext, type UnsavedApi } from "../lib/unsaved";

/** Holds which forms are dirty and shows the discard popup when needed. */
export default function UnsavedProvider({ children }: { children: ReactNode }) {
  const dirty = useRef(new Set<symbol>());
  const [pending, setPending] = useState<(() => void) | null>(null);

  const api = useMemo<UnsavedApi>(
    () => ({
      register: (id, isDirty) => {
        if (isDirty) dirty.current.add(id);
        else dirty.current.delete(id);
      },
      guard: (action) => {
        if (dirty.current.size === 0) action();
        // Functional form: store the action itself, don't call it.
        else setPending(() => action);
      },
    }),
    [],
  );

  return (
    <UnsavedContext.Provider value={api}>
      {children}
      {pending && (
        <div className="modal-backdrop">
          <div
            className="modal"
            role="alertdialog"
            aria-modal="true"
            aria-labelledby="discard-title"
            aria-describedby="discard-text"
          >
            <h2 id="discard-title">Discard unsaved changes?</h2>
            <p id="discard-text">
              You have entered data that isn't saved yet. If you leave now, it
              will be lost.
            </p>
            <div className="actions">
              <button type="button" autoFocus onClick={() => setPending(null)}>
                Keep editing
              </button>
              <button
                type="button"
                className="danger"
                onClick={() => {
                  const action = pending;
                  setPending(null);
                  dirty.current.clear();
                  action();
                }}
              >
                Leave without saving
              </button>
            </div>
          </div>
        </div>
      )}
    </UnsavedContext.Provider>
  );
}
