import { useState } from "react";
import UnsavedProvider from "./components/UnsavedProvider";
import { useGuard } from "./lib/unsaved";
import HomePage from "./pages/HomePage";
import ImportPage from "./pages/ImportPage";
import "./App.css";

type Page = "trades" | "import";

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
      {page === "trades" ? (
        <HomePage />
      ) : (
        <main className="page">
          <ImportPage />
        </main>
      )}
    </>
  );
}

export default App;
