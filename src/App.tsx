import { useState } from "react";
import ThemeToggle from "./components/ThemeToggle";
import UnsavedProvider from "./components/UnsavedProvider";
import { useGuard } from "./lib/unsaved";
import CalendarPage from "./pages/CalendarPage";
import DashboardPage from "./pages/DashboardPage";
import FieldsPage from "./pages/FieldsPage";
import HomePage from "./pages/HomePage";
import ImportPage from "./pages/ImportPage";
import PlaybooksPage from "./pages/PlaybooksPage";
import ReportsPage from "./pages/ReportsPage";
import SettingsPage from "./pages/SettingsPage";
import TagsPage from "./pages/TagsPage";
import "./App.css";

type Page =
  | "dashboard"
  | "calendar"
  | "reports"
  | "trades"
  | "import"
  | "playbooks"
  | "tags"
  | "fields"
  | "settings";

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
      <header className="appbar">
        <div className="appbar-inner">
          <div className="brand">
            <BrandMark />
            <span className="brand-name">TradeLedger</span>
          </div>
          <nav className="tabs" aria-label="Pages">
            {(
              [
                ["dashboard", "Dashboard"],
                ["calendar", "Calendar"],
                ["reports", "Reports"],
                ["trades", "Trades"],
                ["import", "Import"],
                ["playbooks", "Playbooks"],
                ["tags", "Tags"],
                ["fields", "Fields"],
                ["settings", "Settings"],
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
          <ThemeToggle />
        </div>
      </header>
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
      {page === "playbooks" && (
        <main className="page">
          <PlaybooksPage />
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
      {page === "settings" && (
        <main className="page">
          <SettingsPage />
        </main>
      )}
    </>
  );
}

export default App;

/** Logo: two candlesticks, one up and one down, in the accent color. */
function BrandMark() {
  return (
    <svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true">
      <rect
        x="4"
        y="3"
        width="2"
        height="18"
        rx="1"
        fill="currentColor"
        opacity="0.45"
      />
      <rect x="2.5" y="7" width="5" height="9" rx="1.5" fill="currentColor" />
      <rect
        x="15"
        y="2"
        width="2"
        height="17"
        rx="1"
        fill="currentColor"
        opacity="0.45"
      />
      <rect
        x="13.5"
        y="5"
        width="5"
        height="7"
        rx="1.5"
        fill="currentColor"
        opacity="0.7"
      />
    </svg>
  );
}
