import { useState } from "react";
import HomePage from "./pages/HomePage";
import ImportPage from "./pages/ImportPage";
import "./App.css";

type Page = "trades" | "import";

// Simple tab switch; a router can come later if pages need URLs.
// Switching tabs remounts the page, so the trade list reloads after an import.
function App() {
  const [page, setPage] = useState<Page>("trades");

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
            onClick={() => setPage(id)}
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
