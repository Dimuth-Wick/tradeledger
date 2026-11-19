import { createContext, useContext, useEffect, useState } from "react";

// "You have unsaved changes" guard.
//
// Forms report whether they hold unsaved input with `useUnsavedChanges`.
// Navigation (Back, tab switch) goes through `useGuard()`: if any form is
// dirty, a confirm popup asks before the action runs and the input is lost.

export interface UnsavedApi {
  /** A form says whether it currently has unsaved input. */
  register: (id: symbol, dirty: boolean) => void;
  /** Runs `action` now, or after the user confirms discarding changes. */
  guard: (action: () => void) => void;
}

export const UnsavedContext = createContext<UnsavedApi | null>(null);

export function useUnsavedChanges(dirty: boolean) {
  const api = useContext(UnsavedContext);
  const [id] = useState(() => Symbol("form"));
  useEffect(() => {
    api?.register(id, dirty);
    // Unmounted forms no longer hold anything to lose.
    return () => api?.register(id, false);
  }, [api, id, dirty]);
}

export function useGuard(): (action: () => void) => void {
  const api = useContext(UnsavedContext);
  return api ? api.guard : (action) => action();
}
