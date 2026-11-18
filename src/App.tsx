import { useState } from "react";
import UnsavedProvider from "./components/UnsavedProvider";
import { useGuard } from "./lib/unsaved";
import CalendarPage from "./pages/CalendarPage";
import DashboardPage from "./pages/DashboardPage";
import FieldsPage from "./pages/FieldsPage";
import HomePage from "./pages/HomePage";
import ImportPage from "./pages/ImportPage";
import ReportsPage from "./pages/ReportsPage";
import TagsPage from "./pages/TagsPage";
import "./App.css";

type Page =
  | "dashboard"
  | "calendar"
  | "reports"
  | "trades"
  | "import"
  | "tags"
  | "fields";

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
  // A trade to open when the Trades tab mounts (e.g. clicked in the calendar).
  const [openTrade, setOpenTrade] = useState<number | null>(null);

  return (
    <>
      <nav className="tabs" aria-label="Pages">
        {(
          [
            ["dashboard", "Dashboard"],
            ["calendar", "Calendar"],
            ["reports", "Reports"],
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
            onClick={() =>
              page !== id &&
              guard(() => {
                setOpenTrade(null);
                setPage(id);
              })
            }
          >
            {label}
          </button>
        ))}
      </nav>
      {page === "dashboard" && <DashboardPage />}
      {page === "calendar" && (
        <CalendarPage
          onOpenTrade={(id) => {
            setOpenTrade(id);
            setPage("trades");
          }}
        />
      )}
      {page === "reports" && <ReportsPage />}
      {page === "trades" && <HomePage initialOpenId={openTrade} />}
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
