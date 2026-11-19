import { useState } from "react";
import AboutSettings from "../components/settings/AboutSettings";
import DataSettings from "../components/settings/DataSettings";
import FieldSettings from "../components/settings/FieldSettings";
import TagSettings from "../components/settings/TagSettings";
import ThemeSettings from "../components/settings/ThemeSettings";
import {
  SETTINGS_SECTIONS,
  loadSection,
  saveSection,
  type SettingsSection,
} from "../logic/settingsSections";
import { useGuard } from "../logic/unsaved";

/** Settings: a side menu on the left, the chosen section on the right. */
export default function SettingsPage() {
  const [section, setSection] = useState<SettingsSection>(loadSection);
  const guard = useGuard();

  function open(id: SettingsSection) {
    if (id === section) return;
    // Leaving Tags or Fields drops what's typed there, so ask first.
    guard(() => {
      setSection(id);
      saveSection(id);
    });
  }

  return (
    <section className="settings-page">
      <h2>Settings</h2>
      <div className="settings-layout">
        <nav className="settings-nav" aria-label="Settings sections">
          {SETTINGS_SECTIONS.map(([id, label]) => (
            <button
              key={id}
              type="button"
              className={section === id ? "active" : ""}
              aria-current={section === id ? "page" : undefined}
              onClick={() => open(id)}
            >
              {label}
            </button>
          ))}
        </nav>
        <div className="settings-pane">
          {section === "appearance" && <ThemeSettings />}
          {section === "tags" && <TagSettings />}
          {section === "fields" && <FieldSettings />}
          {section === "data" && <DataSettings />}
          {section === "about" && <AboutSettings />}
        </div>
      </div>
    </section>
  );
}
