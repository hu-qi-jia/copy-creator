import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { getCurrentWindow } from "@tauri-apps/api/window";
import ClipboardPage from "./pages/ClipboardPage";
import PhrasePage from "./pages/PhrasePage";
import TranslationPage from "./pages/TranslationPage";
import SettingsContent from "./components/SettingsContent";
import { useSettingsStore } from "./stores/settingsStore";
import { Icons } from "./components/Icons";
import i18n from "./i18n";

const PANEL_MAP: Record<string, { titleKey: string; component: React.ReactNode }> = {
  clipboard: { titleKey: "tabs.clipboard", component: <ClipboardPage /> },
  phrases: { titleKey: "tabs.phrases", component: <PhrasePage /> },
  translate: { titleKey: "tabs.translate", component: <TranslationPage /> },
};

const NAV_ITEMS = [
  { panelType: "clipboard" },
  { panelType: "phrases" },
  { panelType: "translate" },
] as const;

function App() {
  const { t } = useTranslation();
  const [activePanel, setActivePanel] = useState<string>("clipboard");
  const { themeMode, toggleTheme, loadSettings } = useSettingsStore();

  useEffect(() => {
    loadSettings().then(() => {
      const lang = useSettingsStore.getState().language;
      if (lang && lang !== i18n.language) {
        i18n.changeLanguage(lang);
      }
    });
  }, []);

  const SIDEBAR_MIN = 60;
  const SIDEBAR_MAX = 130;
  const SIDEBAR_DEFAULT = 60;
  const COLLAPSE_THRESHOLD = 80;
  const [sidebarWidth, setSidebarWidth] = useState(SIDEBAR_DEFAULT);
  const [isCollapsed, setIsCollapsed] = useState(true);
  const sidebarRef = useRef<HTMLDivElement>(null);
  const isDragging = useRef(false);
  const dragStartX = useRef(0);
  const dragStartWidth = useRef(0);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", themeMode);
  }, [themeMode]);

  useEffect(() => {
    if (navigator.platform.startsWith("Mac")) {
      document.documentElement.setAttribute("data-platform", "macos");
    }
  }, []);

  const handleResizeMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    isDragging.current = true;
    dragStartX.current = e.clientX;
    dragStartWidth.current = sidebarWidth;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  }, [sidebarWidth]);

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isDragging.current) return;
      const delta = e.clientX - dragStartX.current;
      const newWidth = Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, dragStartWidth.current + delta));
      const el = sidebarRef.current;
      if (el) {
        el.style.width = `${newWidth}px`;
        el.style.minWidth = `${newWidth}px`;
        if (newWidth <= COLLAPSE_THRESHOLD) {
          el.classList.add("collapsed");
        } else {
          el.classList.remove("collapsed");
        }
      }
    };

    const handleMouseUp = () => {
      if (!isDragging.current) return;
      isDragging.current = false;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      const el = sidebarRef.current;
      if (el) {
        const finalWidth = parseFloat(el.style.width);
        if (!isNaN(finalWidth)) {
          setSidebarWidth(finalWidth);
          setIsCollapsed(finalWidth <= COLLAPSE_THRESHOLD);
        }
      }
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, []);

  const handleSettingsClick = () => setActivePanel("settings");

  const handleHide = async () => {
    await getCurrentWindow().hide();
  };

  const handleMinimize = async () => {
    await getCurrentWindow().minimize();
  };

  const handleZoom = async () => {
    await getCurrentWindow().toggleMaximize();
  };

  const panelInfo = activePanel !== "settings" ? PANEL_MAP[activePanel] : null;
  const isSettingsPanel = activePanel === "settings";

  return (
    <div className="app-container">
      <div
        ref={sidebarRef}
        className={`sidebar ${isCollapsed ? "collapsed" : ""}`}
        style={{ width: sidebarWidth, minWidth: sidebarWidth }}
        data-tauri-drag-region
      >
        <div className="sidebar-header" data-tauri-drag-region>
          <img className="sidebar-logo" src="/logo_top.png" alt="logo" />
          <span className="sidebar-brand">{t("brand.name")}</span>
        </div>

        <div className="sidebar-nav">
          {NAV_ITEMS.map((item) => {
            const iconKey = item.panelType as keyof typeof Icons;
            const titleKey = `tabs.${item.panelType}`;
            const isActive = !isSettingsPanel && activePanel === item.panelType;
            return (
              <button
                key={item.panelType}
                className={`sidebar-nav-item ${isActive ? "active" : ""}`}
                onClick={() => setActivePanel(item.panelType)}
                title={t(titleKey)}
              >
                <span className="sidebar-nav-icon">{Icons[iconKey]}</span>
                <span className="sidebar-nav-label">{t(titleKey)}</span>
              </button>
            );
          })}
        </div>

        <div className="sidebar-footer">
          <button
            className={`sidebar-footer-item ${isSettingsPanel ? "active" : ""}`}
            onClick={handleSettingsClick}
            title={t("settings.title")}
          >
            <span className="sidebar-footer-icon">{Icons.settings}</span>
            <span className="sidebar-footer-label">{t("settings.title")}</span>
          </button>
          <button
            className="sidebar-footer-item"
            onClick={toggleTheme}
            title={themeMode === "light" ? t("settings.dark") : t("settings.light")}
          >
            <span className="sidebar-footer-icon">
              {themeMode === "light" ? Icons.moon : Icons.sun}
            </span>
            <span className="sidebar-footer-label">
              {themeMode === "light" ? t("settings.dark") : t("settings.light")}
            </span>
          </button>
        </div>

        <div
          className="sidebar-resize-handle"
          onMouseDown={handleResizeMouseDown}
        />
      </div>

      <div className="panel-area">
        <div className="panel-window-header" data-tauri-drag-region>
          <div className="macos-traffic-lights">
            <button className="macos-traffic-btn macos-traffic-close" onClick={handleHide} title={t("common.hide")} />
            <button className="macos-traffic-btn macos-traffic-minimize" onClick={handleMinimize} title="Minimize" />
            <button className="macos-traffic-btn macos-traffic-zoom" onClick={handleZoom} title="Zoom" />
          </div>
          <h3 className="panel-window-title" data-tauri-drag-region>
            {isSettingsPanel ? t("settings.title") : panelInfo ? t(panelInfo.titleKey) : ""}
          </h3>
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
