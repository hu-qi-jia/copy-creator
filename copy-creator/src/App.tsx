import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { getCurrentWindow } from "@tauri-apps/api/window";
import ClipboardPage from "./pages/ClipboardPage";
import PhrasePage from "./pages/PhrasePage";
import TranslationPage from "./pages/TranslationPage";
import SettingsContent from "./components/SettingsContent";
import GlassIcons from "./components/GlassIcons";
import { useSettingsStore } from "./stores/settingsStore";
import { useClipboardStore } from "./stores/clipboardStore";
import { Icons } from "./icons";

const PANEL_MAP: Record<string, { titleKey: string; component: React.ReactNode }> = {
  clipboard: { titleKey: "tabs.clipboard", component: <ClipboardPage /> },
  phrases: { titleKey: "tabs.phrases", component: <PhrasePage /> },
  translate: { titleKey: "tabs.translate", component: <TranslationPage /> },
};

function App() {
  const { t } = useTranslation();
  const [activePanel, setActivePanel] = useState<string>("clipboard");
  const { themeMode, toggleTheme } = useSettingsStore();

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", themeMode);
  }, [themeMode]);

  const navItems = [
    { icon: Icons.clipboard, color: "blue", label: t("tabs.clipboard"), panelType: "clipboard", customClass: "clipboard" },
    { icon: Icons.phrases, color: "purple", label: t("tabs.phrases"), panelType: "phrases", customClass: "phrases" },
    { icon: Icons.translate, color: "green", label: t("tabs.translate"), panelType: "translate", customClass: "translate" },
  ];

  const handleNavChange = (index: number | null) => {
    if (index !== null) {
      setActivePanel(navItems[index].panelType!);
    }
  };

  const handleSettingsClick = () => setActivePanel("settings");

  const handleHide = async () => {
    await getCurrentWindow().hide();
  };

  const panelInfo = activePanel !== "settings" ? PANEL_MAP[activePanel] : null;
  const isSettingsPanel = activePanel === "settings";

  return (
    <div className="app-container">
      <div className="sidebar" data-tauri-drag-region>
        <div className="sidebar-nav">
          <GlassIcons
            items={navItems}
            activePanelType={isSettingsPanel ? null : activePanel}
            onActiveChange={handleNavChange}
          />
        </div>

        <div className="sidebar-footer">
          <button
            className={`sidebar-tool-btn ${isSettingsPanel ? "active" : ""}`}
            onClick={handleSettingsClick}
            title={t("settings.title")}
          >
            {Icons.settings}
          </button>
          <button
            className="sidebar-tool-btn"
            onClick={toggleTheme}
            title={themeMode === "light" ? t("settings.darkMode") : t("settings.lightMode")}
          >
            {themeMode === "light" ? Icons.moon : Icons.sun}
          </button>
        </div>
      </div>

      <div className="panel-area">
        <div className="panel-window-header" data-tauri-drag-region>
          <h3 className="panel-window-title" data-tauri-drag-region>
            {isSettingsPanel ? t("settings.title") : panelInfo ? t(panelInfo.titleKey) : ""}
          </h3>
          <button className="window-close-btn" onClick={handleHide} title={t("common.hide")}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>
        <div className="panel-window-body">
          {isSettingsPanel ? (
            <SettingsContent embedded />
          ) : (
            panelInfo?.component
          )}
        </div>
      </div>

    </div>
  );
}

export default App;
