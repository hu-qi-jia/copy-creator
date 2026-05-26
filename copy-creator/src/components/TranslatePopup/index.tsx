import { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface TranslateResponse {
  source_text: string;
  target_text: string;
  engine: string;
}

export default function TranslatePopup() {
  const { t } = useTranslation();
  const [sourceText, setSourceText] = useState("");
  const [result, setResult] = useState("");
  const [engine, setEngine] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [copied, setCopied] = useState(false);
  const [visible, setVisible] = useState(false);
  const pendingRef = useRef(false);

  useEffect(() => {
    invoke<string>("get_setting", { key: "theme" }).then((theme) => {
      document.documentElement.setAttribute("data-theme", theme);
    }).catch(() => {});

    let unlistenTheme: UnlistenFn;
    listen<{ theme: string }>("theme-changed", (e) => {
      document.documentElement.setAttribute("data-theme", e.payload.theme);
    }).then((fn) => { unlistenTheme = fn; });

    let unlistenText: UnlistenFn;

    const setup = async () => {
      unlistenText = await listen<string>("translate-popup-text", async (e) => {
        const text = e.payload;
        if (!text) return;

        pendingRef.current = true;
        setSourceText(text);
        setError("");
        setCopied(false);
        setVisible(true);
        setLoading(true);

        try {
          // Read persisted target language, default to zh
          let targetLang = "zh";
          try {
            const lang = await invoke<string>("get_setting", { key: "translate_target_lang" });
            if (lang) targetLang = lang;
          } catch { /* use default */ }

          const res = await invoke<TranslateResponse>("translate", {
            text,
            targetLang,
          });
          if (pendingRef.current) {
            setResult(res.target_text);
            setEngine(res.engine);
          }
        } catch (err) {
          if (pendingRef.current) {
            setError(String(err));
          }
        } finally {
          if (pendingRef.current) {
            setLoading(false);
          }
        }
      });
    };

    setup();

    const handleBlur = () => {
      setTimeout(() => {
        pendingRef.current = false;
        getCurrentWindow().hide();
        setVisible(false);
      }, 150);
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        pendingRef.current = false;
        getCurrentWindow().hide();
        setVisible(false);
      }
    };

    window.addEventListener("blur", handleBlur);
    document.addEventListener("keydown", handleKeyDown);

    return () => {
      pendingRef.current = false;
      if (unlistenText) unlistenText();
      if (unlistenTheme) unlistenTheme();
      window.removeEventListener("blur", handleBlur);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, []);

  const handleCopy = async () => {
    if (!result) return;
    try {
      await navigator.clipboard.writeText(result);
      setCopied(true);
      setTimeout(() => {
        setCopied(false);
        getCurrentWindow().hide();
        setVisible(false);
      }, 800);
    } catch (err) {
      console.error("Failed to copy:", err);
    }
  };

  if (!visible && !sourceText) return null;

  return (
    <div className="translate-popup">
      <div className="translate-popup-header">
        <span className="translate-popup-title">{t("tabs.translate")}</span>
        <div className="translate-popup-header-right">
          {engine && !loading && (
            <span className="engine-badge">
              {engine === "ai" ? "AI" : "Google"}
            </span>
          )}
          {result && !loading && (
            <button
              className={`translate-popup-copy-btn ${copied ? "copied" : ""}`}
              onClick={handleCopy}
              title={copied ? t("translate.copied") : t("translate.copy")}
            >
              {copied ? (
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="20 6 9 17 4 12" />
                </svg>
              ) : (
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                  <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
                </svg>
              )}
            </button>
          )}
        </div>
      </div>

      {sourceText && (
        <div className="translate-popup-source">
          <p className="translate-popup-source-text">{sourceText}</p>
        </div>
      )}

      <div className="translate-popup-body">
        {error ? (
          <div className="translate-popup-error">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <circle cx="12" cy="12" r="10" />
              <line x1="12" y1="8" x2="12" y2="12" />
              <line x1="12" y1="16" x2="12.01" y2="16" />
            </svg>
            <span>{error}</span>
          </div>
        ) : result ? (
          <div className={`translate-popup-result-area ${loading ? "dimmed" : ""}`}>
            <p className="translate-popup-result" onClick={handleCopy} title={t("translate.copy")}>
              {result}
              <span className="translate-popup-copy-hint">
                {copied ? t("translate.copied") : t("translate.copy")}
              </span>
            </p>
            {loading && <div className="translate-popup-updating">…</div>}
          </div>
        ) : loading ? (
          <div className="translate-popup-loading">
            <div className="translate-spinner" />
            <span>{t("translate.translating")}</span>
          </div>
        ) : (
          <div className="translate-popup-empty">{t("translate.noResult")}</div>
        )}
      </div>
    </div>
  );
}
