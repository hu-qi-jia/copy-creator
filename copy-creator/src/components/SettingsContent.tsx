import { useState, useEffect, useRef, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { useSettingsStore } from "../stores/settingsStore";
import { StorageSection, LanguageSection, ShortcutSection, TranslationSection, StartupSection } from "./settings";

function createShortcutHandler(setKey: (k: string) => void, done: () => void): (e: KeyboardEvent) => void {
  return (e: KeyboardEvent) => {
    if (["Control", "Alt", "Shift", "Meta", "CapsLock", "NumLock", "ScrollLock", "Dead"].includes(e.key)) {
      return;
    }

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

    const code = e.code;
    let keyName: string;
    if (code.startsWith("Key")) {
      keyName = code[3];
    } else if (code.startsWith("Digit")) {
      keyName = code[5];
    } else if (code.startsWith("Numpad")) {
      keyName = "NumPad" + code.substring(6);
    } else {
      keyName = e.key;
      if (keyName === " ") keyName = "Space";
    }
    parts.push(keyName);

    setKey(parts.join("+"));
    done();
  };
}

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
  const [localMicrosoftApiKey, setLocalMicrosoftApiKey] = useState(settings.microsoftApiKey);
  const [localMicrosoftRegion, setLocalMicrosoftRegion] = useState(settings.microsoftRegion);
  const [localGoogleApiKey, setLocalGoogleApiKey] = useState(settings.googleApiKey);
  const [localTranslateProxy, setLocalTranslateProxy] = useState(settings.translateProxy);
  const [localLang, setLocalLang] = useState(i18n.language);
  const [localShortcutKey, setLocalShortcutKey] = useState(settings.shortcutKey);
  const [localRadialMenuEnabled, setLocalRadialMenuEnabled] = useState(settings.radialMenuEnabled);
  const [localAutostart, setLocalAutostart] = useState(settings.autostartEnabled);
  const [localHideDockIcon, setLocalHideDockIcon] = useState(settings.hideDockIcon);
  const [localRadialKeyboardShortcut, setLocalRadialKeyboardShortcut] = useState(settings.radialKeyboardShortcut);
  const [localTranslateShortcutKey, setLocalTranslateShortcutKey] = useState(settings.translateShortcutKey);

  const [recording, setRecording] = useState(false);
  const [radialKbRecording, setRadialKbRecording] = useState(false);
  const [translateRecording, setTranslateRecording] = useState(false);

  const [storagePath, setStoragePath] = useState("");
  const [saved, setSaved] = useState(false);

  // Use useRef for the active flag and handler so callbacks always see latest values
  const recordingActiveRef = useRef(false);
  const recordingHandlerRef = useRef<((e: KeyboardEvent) => void) | null>(null);
  const radialKbActiveRef = useRef(false);
  const radialKbHandlerRef = useRef<((e: KeyboardEvent) => void) | null>(null);
  const translateActiveRef = useRef(false);
  const translateHandlerRef = useRef<((e: KeyboardEvent) => void) | null>(null);

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
    setLocalMicrosoftApiKey(settings.microsoftApiKey);
    setLocalMicrosoftRegion(settings.microsoftRegion);
    setLocalGoogleApiKey(settings.googleApiKey);
    setLocalTranslateProxy(settings.translateProxy);
    setLocalLang(i18n.language);
    setLocalShortcutKey(settings.shortcutKey);
    setLocalRadialMenuEnabled(settings.radialMenuEnabled);
    setLocalAutostart(settings.autostartEnabled);
    setLocalHideDockIcon(settings.hideDockIcon);
    setLocalRadialKeyboardShortcut(settings.radialKeyboardShortcut);
    setLocalTranslateShortcutKey(settings.translateShortcutKey);
  }, [settings, i18n.language]);

  const startRecordingFor = useCallback(
    (
      setKey: (k: string) => void,
      activeRef: React.MutableRefObject<boolean>,
      handlerRef: React.MutableRefObject<((e: KeyboardEvent) => void) | null>,
      setActiveState: (v: boolean) => void,
    ) => {
      // Clean up any existing handler
      if (handlerRef.current) {
        document.removeEventListener("keydown", handlerRef.current, true);
        handlerRef.current = null;
      }

      activeRef.current = true;
      setActiveState(true);
      setKey("");

      const handler = createShortcutHandler(setKey, () => {
        activeRef.current = false;
        setActiveState(false);
        if (handlerRef.current) {
          document.removeEventListener("keydown", handlerRef.current, true);
          handlerRef.current = null;
        }
      });

      handlerRef.current = handler;
      document.addEventListener("keydown", handler, true);
    },
    [],
  );

  const stopRecordingFor = useCallback(
    (
      activeRef: React.MutableRefObject<boolean>,
      handlerRef: React.MutableRefObject<((e: KeyboardEvent) => void) | null>,
      setActiveState: (v: boolean) => void,
    ) => {
      activeRef.current = false;
      setActiveState(false);
      if (handlerRef.current) {
        document.removeEventListener("keydown", handlerRef.current, true);
        handlerRef.current = null;
      }
    },
    [],
  );

  const startRecording = () =>
    startRecordingFor(setLocalShortcutKey, recordingActiveRef, recordingHandlerRef, setRecording);
  const stopRecording = () =>
    stopRecordingFor(recordingActiveRef, recordingHandlerRef, setRecording);

  const startRadialKbRecording = () =>
    startRecordingFor(setLocalRadialKeyboardShortcut, radialKbActiveRef, radialKbHandlerRef, setRadialKbRecording);
  const stopRadialKbRecording = () =>
    stopRecordingFor(radialKbActiveRef, radialKbHandlerRef, setRadialKbRecording);

  const startTranslateRecording = () =>
    startRecordingFor(setLocalTranslateShortcutKey, translateActiveRef, translateHandlerRef, setTranslateRecording);
  const stopTranslateRecording = () =>
    stopRecordingFor(translateActiveRef, translateHandlerRef, setTranslateRecording);

  const handleSave = async () => {
    await settings.setSettingsBatch({
      clipboard_retention: localRetention,
      default_translate_engine: localEngine,
      ai_api_url: localApiUrl,
      ai_api_key: localApiKey,
      ai_model: localModel,
      microsoft_api_key: localMicrosoftApiKey,
      microsoft_region: localMicrosoftRegion,
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

    // Radial keyboard shortcut
    const oldRadialKb = settings.radialKeyboardShortcut;
    const newRadialKb = localRadialKeyboardShortcut;
    if (oldRadialKb !== newRadialKb) {
      try {
        await invoke("update_radial_keyboard_shortcut", { oldShortcut: oldRadialKb, newShortcut: newRadialKb });
        await settings.setSetting("radial_keyboard_shortcut", newRadialKb);
      } catch (e) {
        console.error("Failed to update radial keyboard shortcut:", e);
      }
    }

    // Translate shortcut
    const oldTranslateKey = settings.translateShortcutKey;
    const newTranslateKey = localTranslateShortcutKey;
    if (oldTranslateKey !== newTranslateKey) {
      try {
        await invoke("update_translate_shortcut", { oldShortcut: oldTranslateKey, newShortcut: newTranslateKey });
        await settings.setSetting("translate_shortcut_key", newTranslateKey);
      } catch (e) {
        console.error("Failed to update translate shortcut:", e);
      }
    }

    try {
      await invoke("set_radial_menu_enabled", { enabled: localRadialMenuEnabled });
    } catch (e) {
      console.error("Failed to set radial menu enabled:", e);
    }

    await settings.setAutostart(localAutostart);

    if (localHideDockIcon !== settings.hideDockIcon) {
      await settings.setHideDockIcon(localHideDockIcon);
    }

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
        localRadialKeyboardShortcut={localRadialKeyboardShortcut}
        setLocalRadialKeyboardShortcut={setLocalRadialKeyboardShortcut}
        radialKbRecording={radialKbRecording}
        startRadialKbRecording={startRadialKbRecording}
        stopRadialKbRecording={stopRadialKbRecording}
        localTranslateShortcutKey={localTranslateShortcutKey}
        setLocalTranslateShortcutKey={setLocalTranslateShortcutKey}
        translateRecording={translateRecording}
        startTranslateRecording={startTranslateRecording}
        stopTranslateRecording={stopTranslateRecording}
      />

      <StartupSection
        localAutostart={localAutostart}
        setLocalAutostart={setLocalAutostart}
        hideDockIcon={localHideDockIcon}
        setHideDockIcon={setLocalHideDockIcon}
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
        localMicrosoftApiKey={localMicrosoftApiKey}
        setLocalMicrosoftApiKey={setLocalMicrosoftApiKey}
        localMicrosoftRegion={localMicrosoftRegion}
        setLocalMicrosoftRegion={setLocalMicrosoftRegion}
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
