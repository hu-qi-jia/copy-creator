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
      const retention = await invoke<string>("get_setting", {
        key: "clipboard_retention",
      });
      const engine = await invoke<string>("get_setting", {
        key: "default_translate_engine",
      });
      const apiUrl = await invoke<string>("get_setting", { key: "ai_api_url" });
      const apiKey = await invoke<string>("get_setting", { key: "ai_api_key" });
      const model = await invoke<string>("get_setting", { key: "ai_model" });
      const baiduAppId = await invoke<string>("get_setting", { key: "baidu_appid" });
      const baiduSecret = await invoke<string>("get_setting", { key: "baidu_secret" });
      const googleApiKey = await invoke<string>("get_setting", { key: "google_api_key" });
      const translateProxy = await invoke<string>("get_setting", { key: "translate_proxy" });
      const language = await invoke<string>("get_setting", { key: "language" });
      const shortcutKey = await invoke<string>("get_setting", { key: "shortcut_key" });
      const radialMenuEnabled = await invoke<string>("get_setting", { key: "radial_menu_enabled" });

      set({
        clipboardRetention: retention || "1month",
        defaultEngine: engine || "google",
        apiUrl: apiUrl || "",
        apiKey: apiKey || "",
        model: model || "",
        baiduAppId: baiduAppId || "",
        baiduSecret: baiduSecret || "",
        googleApiKey: googleApiKey || "",
        translateProxy: translateProxy || "",
        language: language || "zh-CN",
        shortcutKey: shortcutKey || "",
        radialMenuEnabled: radialMenuEnabled !== "0",
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
