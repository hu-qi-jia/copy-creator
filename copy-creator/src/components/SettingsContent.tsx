import { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { useSettingsStore } from "../stores/settingsStore";
import { StorageSection, LanguageSection, ShortcutSection, TranslationSection, StartupSection } from "./settings";

interface Props {
  embedded?: boolean;
}

export default function SettingsContent({ embedded }: Props) {
  const { i18n, t } = useTranslation();
  const settings = useSettingsStore();

  const [localRetention, setLocalRetention] = useState(settings.clipboardRetention);
  const [localEngine, setLocalEngine] = useState(settings.defaultEngine);
  const [localApiUrl, setLocalApiUrl] = useState(settings.apiUrl);
  const [localApiKey, setLocalApiKey] = useState(settings.apiKey);
  const [localModel, setLocalModel] = useState(settings.model);
  const [localGoogleApiKey, setLocalGoogleApiKey] = useState(settings.googleApiKey);
  const [localTranslateProxy, setLocalTranslateProxy] = useState(settings.translateProxy);
  const [localLang, setLocalLang] = useState(i18n.language);
  const [localShortcutKey, setLocalShortcutKey] = useState(settings.shortcutKey);
  const [localRadialMenuEnabled, setLocalRadialMenuEnabled] = useState(settings.radialMenuEnabled);
  const [localAutostart, setLocalAutostart] = useState(settings.autostartEnabled);
  const [recording, setRecording] = useState(false);
  const recordingRef = useRef(false);
  const keydownHandlerRef = useRef<((e: KeyboardEvent) => void) | null>(null);
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
    setLocalGoogleApiKey(settings.googleApiKey);
    setLocalTranslateProxy(settings.translateProxy);
    setLocalLang(i18n.language);
    setLocalShortcutKey(settings.shortcutKey);
    setLocalRadialMenuEnabled(settings.radialMenuEnabled);
    setLocalAutostart(settings.autostartEnabled);
  }, [settings, i18n.language]);

  const startRecording = () => {
    recordingRef.current = true;
    setRecording(true);
    setLocalShortcutKey("");

    const cleanup = () => {
      document.removeEventListener("keydown", handler, true);
      keydownHandlerRef.current = null;
    };

    const handler = (e: KeyboardEvent) => {
      if (!recordingRef.current) {
        cleanup();
        return;
      }

      // Ignore modifier-only presses
      if (["Control", "Alt", "Shift", "Meta", "CapsLock", "NumLock", "ScrollLock", "Dead"].includes(e.key)) {
        return;
      }

      // Require at least one modifier
      if (!e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey) {
        return;
      }

      e.preventDefault();
      e.stopPropagation();

      const parts: string[] = [];
      if (e.ctrlKey) parts.push("Ctrl");
      if (e.altKey) parts.push("Alt");
      if (e.shiftKey) parts.push("Shift");
      if (e.metaKey) parts.push("Super");

      // Map physical key code to layout-independent name
      const code = e.code;
      let keyName: string;
      if (code.startsWith("Key")) {
        keyName = code[3]; // KeyA → A
      } else if (code.startsWith("Digit")) {
        keyName = code[5]; // Digit1 → 1
      } else if (code.startsWith("Numpad")) {
        keyName = "NumPad" + code.substring(6);
      } else {
        keyName = e.key;
        if (keyName === " ") keyName = "Space";
      }
      parts.push(keyName);

      const shortcut = parts.join("+");
      setLocalShortcutKey(shortcut);
      recordingRef.current = false;
      setRecording(false);
      cleanup();
    };

    keydownHandlerRef.current = handler;
    document.addEventListener("keydown", handler, true);
  };

  const stopRecording = () => {
    recordingRef.current = false;
    setRecording(false);
    if (keydownHandlerRef.current) {
      document.removeEventListener("keydown", keydownHandlerRef.current, true);
      keydownHandlerRef.current = null;
    }
  };

  const handleSave = async () => {
    await settings.setSettingsBatch({
      clipboard_retention: localRetention,
      default_translate_engine: localEngine,
      ai_api_url: localApiUrl,
      ai_api_key: localApiKey,
      ai_model: localModel,
      google_api_key: localGoogleApiKey,
      translate_proxy: localTranslateProxy,
      language: localLang,
    });

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

    try {
      await invoke("set_radial_menu_enabled", { enabled: localRadialMenuEnabled });
    } catch (e) {
      console.error("Failed to set radial menu enabled:", e);
    }

    await settings.setAutostart(localAutostart);

    if (localLang !== i18n.language) {
      i18n.changeLanguage(localLang);
      emit("language-changed", { language: localLang });
      invoke("update_tray_language").catch(console.error);
    }

    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  const content = (
    <>
      <StorageSection
        storagePath={storagePath}
        setStoragePath={setStoragePath}
        localRetention={localRetention}
        setLocalRetention={setLocalRetention}
      />

      <LanguageSection
        localLang={localLang}
        setLocalLang={setLocalLang}
      />

      <ShortcutSection
        localShortcutKey={localShortcutKey}
        setLocalShortcutKey={setLocalShortcutKey}
        recording={recording}
        startRecording={startRecording}
        stopRecording={stopRecording}
        localRadialMenuEnabled={localRadialMenuEnabled}
        setLocalRadialMenuEnabled={setLocalRadialMenuEnabled}
      />

      <StartupSection
        localAutostart={localAutostart}
        setLocalAutostart={setLocalAutostart}
      />

      <TranslationSection
        localEngine={localEngine}
        setLocalEngine={setLocalEngine}
        localApiUrl={localApiUrl}
        setLocalApiUrl={setLocalApiUrl}
        localApiKey={localApiKey}
        setLocalApiKey={setLocalApiKey}
        localModel={localModel}
        setLocalModel={setLocalModel}
        localGoogleApiKey={localGoogleApiKey}
        setLocalGoogleApiKey={setLocalGoogleApiKey}
        localTranslateProxy={localTranslateProxy}
        setLocalTranslateProxy={setLocalTranslateProxy}
      />

      <div className="settings-actions">
        <button className={`settings-save-btn${saved ? " saved" : ""}`} onClick={handleSave}>
          {saved ? t("common.saved") : t("common.save")}
        </button>
      </div>
    </>
  );

  if (embedded) {
    return <div className="settings-panel-content">{content}</div>;
  }

  return content;
}
