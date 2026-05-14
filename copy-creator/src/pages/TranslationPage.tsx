import { useTranslation } from "react-i18next";
import { useTranslationStore } from "../stores/translationStore";
import { Icons } from "../icons";

const LANGUAGES = [
  { code: "zh", name: "中文", badge: "ZH" },
  { code: "en", name: "English", badge: "EN" },
  { code: "ja", name: "日本語", badge: "JA" },
  { code: "ko", name: "한국어", badge: "KO" },
  { code: "fr", name: "Français", badge: "FR" },
  { code: "de", name: "Deutsch", badge: "DE" },
  { code: "es", name: "Español", badge: "ES" },
  { code: "ru", name: "Русский", badge: "RU" },
  { code: "ar", name: "العربية", badge: "AR" },
  { code: "th", name: "ไทย", badge: "TH" },
  { code: "vi", name: "Tiếng Việt", badge: "VI" },
];

export default function TranslationPage() {
  const { t } = useTranslation();
  const {
    inputText,
    targetLang,
    result,
    engine,
    loading,
    error,
    setInputText,
    setTargetLang,
    translate,
  } = useTranslationStore();

  const selectedLang = LANGUAGES.find((l) => l.code === targetLang);

  return (
    <div className="translation-page">
      <div className="translation-input-card">
        <textarea
          className="translation-input"
          placeholder={t("translate.inputPlaceholder")}
          value={inputText}
          onChange={(e) => setInputText(e.target.value)}
        />
        <div className="translation-input-footer">
          <span className="char-count">{inputText.length}</span>
          <button
            className="translate-btn"
            onClick={translate}
            disabled={loading || !inputText.trim()}
          >
            {loading ? (
              <div className="translate-spinner" />
            ) : (
              <>
                {Icons.translate}
                <span>{t("translate.translate")}</span>
              </>
            )}
          </button>
        </div>
      </div>

      <div className="translation-lang-section">
        <span className="translation-lang-label">{t("translate.targetLang")}</span>
        <div className="translation-lang-select-wrapper">
          <span className="lang-bar-badge">{selectedLang?.badge}</span>
          <select
            className="translation-lang-select"
            value={targetLang}
            onChange={(e) => setTargetLang(e.target.value)}
          >
            {LANGUAGES.map((l) => (
              <option key={l.code} value={l.code}>
                {l.name}
              </option>
            ))}
          </select>
        </div>
      </div>

      {error && (
        <div className="translation-error">
          <div className="error-icon-svg">{Icons.delete}</div>
          <span>{error}</span>
        </div>
      )}

      <div className="translation-result-card">
        <div className="translation-result-header">
          <span className="section-label">{t("translate.result")}</span>
          {engine && (
            <span className="engine-badge">
              {engine === "ai" ? "AI" : engine === "google" ? "Google" : "Baidu"}
            </span>
          )}
        </div>
        <div className="translation-result">
          {result ? (
            <p className="result-text">{result}</p>
          ) : (
            <div className="result-placeholder">
              <div className="result-placeholder-icon">{Icons.translate}</div>
              <span>{t("translate.noResult")}</span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
