import { useState } from "react";
import UnsavedProvider from "./components/UnsavedProvider";
import { useGuard } from "./lib/unsaved";
import DashboardPage from "./pages/DashboardPage";
import FieldsPage from "./pages/FieldsPage";
import HomePage from "./pages/HomePage";
import ImportPage from "./pages/ImportPage";
import TagsPage from "./pages/TagsPage";
import "./App.css";

type Page = "dashboard" | "trades" | "import" | "tags" | "fields";

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
  const [page, setPage] = useState<Page>("dashboard");
  const guard = useGuard();

  return (
    <>
      <nav className="tabs" aria-label="Pages">
        {(
          [
            ["dashboard", "Dashboard"],
            ["trades", "Trades"],
            ["import", "Import"],
            ["tags", "Tags"],
            ["fields", "Fields"],
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
      {page === "dashboard" && <DashboardPage />}
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
      {page === "fields" && (
        <main className="page">
          <FieldsPage />
        </main>
      )}
    </>
  );
}

export default App;
