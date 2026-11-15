import { useState } from "react";
import UnsavedProvider from "./components/UnsavedProvider";
import { useGuard } from "./lib/unsaved";
import HomePage from "./pages/HomePage";
import ImportPage from "./pages/ImportPage";
import TagsPage from "./pages/TagsPage";
import "./App.css";

type Page = "trades" | "import" | "tags";

function App() {
  return (
    <UnsavedProvider>
      <Shell />
    </UnsavedProvider>
  );
}

// Simple tab switch; a router can come later if pages need URLs.
// Switching tabs remounts the page, so the trade list reloads after an import.
function Shell() {
  const [page, setPage] = useState<Page>("trades");
  const guard = useGuard();

  return (
    <>
      <nav className="tabs" aria-label="Pages">
        {(
          [
            ["trades", "Trades"],
            ["import", "Import"],
            ["tags", "Tags"],
          ] as const
        ).map(([id, label]) => (
          <button
            key={id}
            type="button"
            className={page === id ? "active" : ""}
            aria-current={page === id ? "page" : undefined}
            // Leaving a page drops what's typed on it, so ask first.
            onClick={() => page !== id && guard(() => setPage(id))}
          >
            {label}
          </button>
        ))}
      </nav>
      {page === "trades" && <HomePage />}
      {page === "import" && (
        <main className="page">
          <ImportPage />
        </main>
      )}
      {page === "tags" && (
        <main className="page">
          <TagsPage />
        </main>
      )}
    </>
  );
}

export default App;
