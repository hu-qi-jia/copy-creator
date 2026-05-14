import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

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
  language: string;
  shortcutKey: string;

  toggleTheme: () => void;
  loadSettings: () => Promise<void>;
  setSetting: (key: string, value: string) => Promise<void>;
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
  language: "zh-CN",
  shortcutKey: "",

  toggleTheme: async () => {
    const next = get().themeMode === "light" ? "dark" : "light";
    set({ themeMode: next });
    await get().setSetting("theme", next);
  },

  loadSettings: async () => {
    try {
      const theme = await invoke<string>("get_setting", { key: "theme" });
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
      const language = await invoke<string>("get_setting", { key: "language" });
      const shortcutKey = await invoke<string>("get_setting", { key: "shortcut_key" });

      set({
        themeMode: (theme as ThemeMode) || "light",
        clipboardRetention: retention || "1month",
        defaultEngine: engine || "google",
        apiUrl: apiUrl || "",
        apiKey: apiKey || "",
        model: model || "",
        baiduAppId: baiduAppId || "",
        baiduSecret: baiduSecret || "",
        googleApiKey: googleApiKey || "",
        language: language || "zh-CN",
        shortcutKey: shortcutKey || "",
      });
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
}));
