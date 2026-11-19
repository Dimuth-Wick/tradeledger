import { useEffect, useRef, useState } from "react";
import ThemeToggle from "./components/shared/ThemeToggle";
import UnsavedProvider from "./components/shared/UnsavedProvider";
import UpdateDialog from "./components/shared/UpdateDialog";
import {
  appVersion,
  checkForUpdate,
  type AvailableUpdate,
} from "./connection/backend";
import { useGuard } from "./logic/unsaved";
import { useIndicator } from "./logic/useIndicator";
import CalendarPage from "./pages/CalendarPage";
import DashboardPage from "./pages/DashboardPage";
import HomePage from "./pages/HomePage";
import ImportPage from "./pages/ImportPage";
import PlaybooksPage from "./pages/PlaybooksPage";
import ReportsPage from "./pages/ReportsPage";
import SettingsPage from "./pages/SettingsPage";
import "./styles/app.css";

type Page =
  | "dashboard"
  | "calendar"
  | "reports"
  | "trades"
  | "import"
  | "playbooks"
  | "settings"; // Tags and Fields live inside Settings

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
  // The accent line slides to the open tab (and the tab scrolls into view
  // when the bar is too narrow to show them all).
  const tabsRef = useRef<HTMLElement>(null);
  const indicator = useIndicator(tabsRef, page);
  useEffect(() => {
    tabsRef.current
      ?.querySelector("[data-selected='true']")
      ?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [page]);
  // Once per launch: is a newer version published on GitHub? Offline or
  // any error → stay quiet (Settings → About can check by hand).
  const [update, setUpdate] = useState<AvailableUpdate | null>(null);
  const [version, setVersion] = useState<string | null>(null);
  useEffect(() => {
    checkForUpdate().then(setUpdate, () => {});
    appVersion().then(setVersion, () => {});
  }, []);

  return (
    <>
      <header className="appbar">
        <div className="appbar-inner">
          <div className="brand">
            <BrandMark />
            <span className="brand-name">TradeLedger</span>
          </div>
          <nav className="tabs" aria-label="Pages" ref={tabsRef}>
            <span className="tab-indicator" style={indicator} aria-hidden />
            {(
              [
                ["dashboard", "Dashboard"],
                ["calendar", "Calendar"],
                ["trades", "Trades"],
                ["reports", "Reports"],
                ["import", "Import"],
                ["playbooks", "Playbooks"],
                ["settings", "Settings"],
              ] as const
            ).map(([id, label]) => (
              <button
                key={id}
                type="button"
                className={page === id ? "active" : ""}
                data-selected={page === id}
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
      {update && (
        <UpdateDialog
          update={update}
          current={version}
          onClose={() => setUpdate(null)}
        />
      )}
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
      {page === "settings" && (
        <main className="page">
          <SettingsPage />
        </main>
      )}
    </>
  );
}

export default App;

/** Logo: white candlesticks climbing to an arrow on an orange tile (the
 *  same artwork as the app icon, backend/icons/app-icon.svg). */
function BrandMark() {
  return (
    <svg viewBox="0 0 1024 1024" width="28" height="28" aria-hidden="true">
      <defs>
        <linearGradient id="brand-tile" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#ffb21e" />
          <stop offset="0.55" stopColor="#ff7417" />
          <stop offset="1" stopColor="#f2450f" />
        </linearGradient>
      </defs>
      <rect
        x="32"
        y="32"
        width="960"
        height="960"
        rx="220"
        fill="url(#brand-tile)"
      />
      <g fill="#ffffff" stroke="#ffffff" strokeLinecap="round">
        <line x1="292" y1="548" x2="292" y2="812" strokeWidth="30" />
        <rect x="232" y="598" width="120" height="166" rx="28" stroke="none" />
        <line x1="512" y1="396" x2="512" y2="772" strokeWidth="30" />
        <rect x="452" y="446" width="120" height="250" rx="28" stroke="none" />
        <line x1="732" y1="560" x2="732" y2="732" strokeWidth="30" />
        <rect x="672" y="318" width="120" height="314" rx="28" stroke="none" />
        <path
          d="M732 168 L842 330 L622 330 Z"
          strokeWidth="24"
          strokeLinejoin="round"
        />
      </g>
    </svg>
  );
}
