import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { enable, disable, isEnabled } from "@tauri-apps/plugin-autostart";

type ThemeMode = "light" | "dark";

interface SettingsState {
  themeMode: ThemeMode;
  clipboardRetention: string;
  defaultEngine: string;
  apiUrl: string;
  apiKey: string;
  model: string;
  baiduAppId: string;
  baiduSecret: string;
  googleApiKey: string;
  translateProxy: string;
  language: string;
  shortcutKey: string;
  radialMenuEnabled: boolean;
  autostartEnabled: boolean;

  toggleTheme: () => void;
  loadSettings: () => Promise<void>;
  setSetting: (key: string, value: string) => Promise<void>;
  setSettingsBatch: (settings: Record<string, string>) => Promise<void>;
  setAutostart: (enabled: boolean) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  themeMode: "light",
  clipboardRetention: "1month",
  defaultEngine: "google",
  apiUrl: "",
  apiKey: "",
  model: "",
  baiduAppId: "",
  baiduSecret: "",
  googleApiKey: "",
  translateProxy: "",
  language: "zh-CN",
  shortcutKey: "",
  radialMenuEnabled: true,
  autostartEnabled: false,

  toggleTheme: () => {
    const next = get().themeMode === "light" ? "dark" : "light";
    set({ themeMode: next });
    // Persist to DB so radial menu reads the correct theme on re-open
    get().setSetting("theme", next);
    emit("theme-changed", { theme: next });
  },

  loadSettings: async () => {
    try {
      const settings = await invoke<Record<string, string>>("get_all_settings");

      set({
        clipboardRetention: settings.clipboard_retention || "1month",
        defaultEngine: settings.default_translate_engine || "google",
        apiUrl: settings.ai_api_url || "",
        apiKey: settings.ai_api_key || "",
        model: settings.ai_model || "",
        baiduAppId: settings.baidu_appid || "",
        baiduSecret: settings.baidu_secret || "",
        googleApiKey: settings.google_api_key || "",
        translateProxy: settings.translate_proxy || "",
        language: settings.language || "zh-CN",
        shortcutKey: settings.shortcut_key || "",
        radialMenuEnabled: settings.radial_menu_enabled !== "0",
      });

      // Read autostart state from the OS (plugin)
      try {
        const auto = await isEnabled();
        set({ autostartEnabled: auto });
      } catch { /* plugin not available */ }
    } catch {
      // Settings not yet initialized, use defaults
    }
  },

  setSetting: async (key: string, value: string) => {
    try {
      await invoke("set_setting", { key, value });
    } catch (e) {
      console.error("Failed to save setting:", e);
    }
  },

  setSettingsBatch: async (settings: Record<string, string>) => {
    try {
      await invoke("set_settings_batch", { settings });
    } catch (e) {
      console.error("Failed to batch save settings:", e);
    }
  },

  setAutostart: async (enabled: boolean) => {
    try {
      if (enabled) {
        await enable();
      } else {
        await disable();
      }
      set({ autostartEnabled: enabled });
    } catch (e) {
      console.error("Failed to set autostart:", e);
    }
  },
}));
