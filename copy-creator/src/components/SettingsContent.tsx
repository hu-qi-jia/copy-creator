import { useState, useEffect, useRef, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "../stores/settingsStore";

interface Props {
  embedded?: boolean;
  onClose?: () => void;
}

export default function SettingsContent({ embedded, onClose }: Props) {
  const { t, i18n } = useTranslation();
  const settings = useSettingsStore();

  const [localRetention, setLocalRetention] = useState(settings.clipboardRetention);
  const [localEngine, setLocalEngine] = useState(settings.defaultEngine);
  const [localApiUrl, setLocalApiUrl] = useState(settings.apiUrl);
  const [localApiKey, setLocalApiKey] = useState(settings.apiKey);
  const [localModel, setLocalModel] = useState(settings.model);
  const [localBaiduAppId, setLocalBaiduAppId] = useState(settings.baiduAppId);
  const [localBaiduSecret, setLocalBaiduSecret] = useState(settings.baiduSecret);
  const [localGoogleApiKey, setLocalGoogleApiKey] = useState(settings.googleApiKey);
  const [localLang, setLocalLang] = useState(i18n.language);
  const [localShortcutKey, setLocalShortcutKey] = useState(settings.shortcutKey);
  const [recording, setRecording] = useState(false);
  const recordRef = useRef(false);
  const [storagePath, setStoragePath] = useState("");
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    settings.loadSettings();
    invoke<string>("get_storage_path").then(setStoragePath).catch(console.error);
  }, []);

  useEffect(() => {
    setLocalRetention(settings.clipboardRetention);
    setLocalEngine(settings.defaultEngine);
    setLocalApiUrl(settings.apiUrl);
    setLocalApiKey(settings.apiKey);
    setLocalModel(settings.model);
    setLocalBaiduAppId(settings.baiduAppId);
    setLocalBaiduSecret(settings.baiduSecret);
    setLocalGoogleApiKey(settings.googleApiKey);
    setLocalLang(i18n.language);
    setLocalShortcutKey(settings.shortcutKey);
  }, [settings, i18n.language]);

  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if (!recordRef.current) return;
    e.preventDefault();
    e.stopPropagation();

    const parts: string[] = [];
    if (e.ctrlKey) parts.push("Ctrl");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");
    if (e.metaKey) parts.push("Super");

    const key = e.key;
    if (!["Control", "Alt", "Shift", "Meta"].includes(key)) {
      let keyName = key;
      if (key === " ") keyName = "Space";
      else if (key.length === 1) keyName = key.toUpperCase();
      parts.push(keyName);
    }

    if (parts.length > 1 || (parts.length === 1 && !["Ctrl", "Alt", "Shift", "Super"].includes(parts[0]))) {
      setLocalShortcutKey(parts.join("+"));
      setRecording(false);
      recordRef.current = false;
    }
  }, []);

  const startRecording = () => {
    setRecording(true);
    recordRef.current = true;
    setLocalShortcutKey("");
  };

  const stopRecording = () => {
    setRecording(false);
    recordRef.current = false;
  };

  useEffect(() => {
    if (recording) {
      window.addEventListener("keydown", handleKeyDown, true);
      return () => window.removeEventListener("keydown", handleKeyDown, true);
    }
  }, [recording, handleKeyDown]);

  const handleSave = async () => {
    await settings.setSetting("clipboard_retention", localRetention);
    await settings.setSetting("default_translate_engine", localEngine);
    await settings.setSetting("ai_api_url", localApiUrl);
    await settings.setSetting("ai_api_key", localApiKey);
    await settings.setSetting("ai_model", localModel);
    await settings.setSetting("baidu_appid", localBaiduAppId);
    await settings.setSetting("baidu_secret", localBaiduSecret);
    await settings.setSetting("google_api_key", localGoogleApiKey);
    await settings.setSetting("language", localLang);

    const oldKey = settings.shortcutKey;
    const newKey = localShortcutKey;
    if (oldKey !== newKey) {
      try {
        await invoke("update_shortcut", { oldShortcut: oldKey, newShortcut: newKey });
        await settings.setSetting("shortcut_key", newKey);
      } catch (e) {
        console.error("Failed to update shortcut:", e);
      }
    }

    if (localLang !== i18n.language) {
      i18n.changeLanguage(localLang);
    }

    setSaved(true);
    setTimeout(() => setSaved(false), 1500);
  };

  const content = (
    <>
      <div className="settings-section">
        <div className="settings-section-title">{t("settings.storage")}</div>
        <div className="settings-card">
          <div className="settings-row vertical">
            <div className="settings-row-label">{t("settings.storagePath")}</div>
            <div className="settings-storage-row">
              <span className="settings-storage-path">{storagePath}</span>
              <button
                className="settings-storage-btn"
                onClick={async () => {
                  try {
                    const folder = await invoke<string>("select_storage_folder");
                    await invoke("set_setting", { key: "storage_path", value: folder });
                    setStoragePath(folder);
                  } catch {}
                }}
              >
                {t("settings.changeFolder")}
              </button>
            </div>
          </div>
        </div>
      </div>

      <div className="settings-section">
        <div className="settings-section-title">{t("settings.language")}</div>
        <div className="settings-card">
          <div className="settings-row">
            <div className="settings-row-label">{t("settings.language")}</div>
            <div className="settings-lang-toggle">
              <button
                className={`lang-toggle-btn${localLang === "zh-CN" ? " active" : ""}`}
                onClick={() => setLocalLang("zh-CN")}
              >
                ZH
              </button>
              <button
                className={`lang-toggle-btn${localLang === "en" ? " active" : ""}`}
                onClick={() => setLocalLang("en")}
              >
                EN
              </button>
            </div>
          </div>
          <div className="settings-row">
            <div className="settings-row-label">{t("settings.shortcut")}</div>
            <div className="shortcut-setting">
              <div className="shortcut-keyboard-row">
                <span className={`shortcut-display${recording ? " recording" : ""}`}>
                  {recording ? t("settings.recording") : (localShortcutKey || t("settings.shortcutPlaceholder"))}
                </span>
                <button
                  className="shortcut-record-btn"
                  onClick={recording ? stopRecording : startRecording}
                >
                  {recording ? t("settings.stopRecord") : t("settings.recordShortcut")}
                </button>
              </div>
            </div>
          </div>
          <div className="settings-row">
            <div className="settings-row-label">{t("settings.clipboardRetention")}</div>
            <select
              className="settings-select"
              value={localRetention}
              onChange={(e) => setLocalRetention(e.target.value)}
            >
              <option value="1week">{t("settings.retention1week")}</option>
              <option value="1month">{t("settings.retention1month")}</option>
              <option value="3months">{t("settings.retention3months")}</option>
            </select>
          </div>
        </div>
      </div>

      <div className="settings-section">
        <div className="settings-section-title">{t("settings.translation")}</div>
        <div className="settings-card">
          <div className="settings-row">
            <div className="settings-row-label">{t("settings.defaultEngine")}</div>
            <select
              className="settings-select"
              value={localEngine}
              onChange={(e) => setLocalEngine(e.target.value)}
            >
              <option value="google">{t("settings.googleTranslation")}</option>
              <option value="ai">{t("settings.aiTranslation")}</option>
            </select>
          </div>
          <div className="settings-row vertical">
            <div className="settings-row-label">{t("settings.googleApiKey")}</div>
            <input
              className="settings-input"
              type="password"
              value={localGoogleApiKey}
              onChange={(e) => setLocalGoogleApiKey(e.target.value)}
              placeholder={t("settings.googleNote")}
            />
          </div>
          <div className="settings-row vertical">
            <div className="settings-row-label">{t("settings.apiUrl")}</div>
            <input
              className="settings-input"
              value={localApiUrl}
              onChange={(e) => setLocalApiUrl(e.target.value)}
              placeholder={t("settings.apiUrl")}
            />
          </div>
          <div className="settings-row vertical">
            <div className="settings-row-label">{t("settings.apiKey")}</div>
            <input
              className="settings-input"
              type="password"
              value={localApiKey}
              onChange={(e) => setLocalApiKey(e.target.value)}
              placeholder={t("settings.apiKey")}
            />
          </div>
          <div className="settings-row vertical">
            <div className="settings-row-label">{t("settings.model")}</div>
            <input
              className="settings-input"
              value={localModel}
              onChange={(e) => setLocalModel(e.target.value)}
              placeholder={t("settings.model")}
            />
          </div>
        </div>
      </div>

      <div className="settings-actions">
        <button className={`settings-save-btn${saved ? " saved" : ""}`} onClick={handleSave}>
          {saved ? "✓" : t("common.save")}
        </button>
      </div>
    </>
  );

  if (embedded) {
    return <div className="settings-panel-content">{content}</div>;
  }

  return content;
}
