import { useTranslation } from "react-i18next";

interface StartupSectionProps {
  localAutostart: boolean;
  setLocalAutostart: (enabled: boolean) => void;
  hideDockIcon: boolean;
  setHideDockIcon: (hide: boolean) => void;
}

export function StartupSection({
  localAutostart,
  setLocalAutostart,
  hideDockIcon,
  setHideDockIcon,
}: StartupSectionProps) {
  const { t } = useTranslation();

  return (
    <div className="settings-section">
      <div className="settings-section-title">{t("settings.startup")}</div>
      <div className="settings-card">
        <div className="settings-row">
          <div className="settings-row-label">{t("settings.startup")}</div>
          <button
            className={`toggle-switch ${localAutostart ? "on" : "off"}`}
            onClick={() => setLocalAutostart(!localAutostart)}
            title={localAutostart ? t("common.on") : t("common.off")}
          >
            <span className="toggle-thumb" />
          </button>
        </div>
        <div className="settings-row">
          <div className="settings-row-label">{t("settings.hideDock")}</div>
          <button
            className={`toggle-switch ${hideDockIcon ? "on" : "off"}`}
            onClick={() => setHideDockIcon(!hideDockIcon)}
            title={hideDockIcon ? t("common.on") : t("common.off")}
          >
            <span className="toggle-thumb" />
          </button>
        </div>
      </div>
    </div>
  );
}
