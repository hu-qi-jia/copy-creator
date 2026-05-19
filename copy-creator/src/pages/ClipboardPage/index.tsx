import { useEffect, useState, useRef, useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useClipboardStore } from "../../stores/clipboardStore";
import { Icons } from "../../components/Icons";
import SearchInput from "../../components/SearchInput";
import { ImageThumb } from "./ImageThumb";
import { formatTime, getFileName, TYPE_META } from "./utils";

type ClipType = "all" | "text" | "image" | "link" | "file";

TYPE_META.text.icon = Icons.clipboard;
TYPE_META.image.icon = Icons.image;
TYPE_META.link.icon = Icons.link;
TYPE_META.file.icon = Icons.file;

export default function ClipboardPage() {
  const { t } = useTranslation();
  const {
    records,
    search,
    loading,
    category,
    init,
    setSearch,
    setCategory,
    loadRecords,
    deleteRecord,
    pasteRecord,
  } = useClipboardStore();

  const [hoverPreview, setHoverPreview] = useState<{ src: string; x: number; y: number } | null>(null);
  const hoverTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const categories: { key: ClipType; label: string }[] = [
    { key: "all", label: t("clipboard.all") },
    { key: "text", label: t("clipboard.text") },
    { key: "image", label: t("clipboard.image") },
    { key: "link", label: t("clipboard.link") },
    { key: "file", label: t("clipboard.file") },
  ];

  const labels: Record<string, string> = {
    text: t("clipboard.text"),
    image: t("clipboard.image"),
    link: t("clipboard.link"),
    file: t("clipboard.file"),
  };

  const getTypeLabel = (type: string): string => labels[type] || labels.text;

  const filtered = useMemo(
    () => (category === "all" ? records : records.filter((r) => r.type === category)),
    [records, category],
  );

  useEffect(() => {
    init();
  }, []);

  useEffect(() => {
    const timer = setTimeout(() => loadRecords(), 300);
    return () => clearTimeout(timer);
  }, [search]);

  const handleThumbHover = useCallback((thumbSrc: string, rect: DOMRect) => {
    if (hoverTimerRef.current) clearTimeout(hoverTimerRef.current);
    setHoverPreview({ src: thumbSrc, x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 });
  }, []);

  const handleThumbLeave = useCallback(() => {
    hoverTimerRef.current = setTimeout(() => setHoverPreview(null), 150);
  }, []);

  return (
    <div className="clipboard-page">
      <div className="page-search">
        <SearchInput
          placeholder={t("clipboard.search")}
          value={search}
          onChange={setSearch}
        />
      </div>

      <div className="clipboard-categories">
        {categories.map((c) => (
          <button
            key={c.key}
            className={`category-chip ${category === c.key ? "active" : ""}`}
            onClick={() => setCategory(c.key)}
          >
            {c.label}
          </button>
        ))}
      </div>

      {loading ? (
        <div className="clipboard-list">
          {[1, 2, 3, 4].map((i) => (
            <div key={i} className="notification skeleton">
              <div className="notibar" />
              <div className="noticontent">
                <div className="notititle">
                  <div className="skeleton-line short" />
                </div>
                <div className="notibody">
                  <div
                    className="skeleton-line"
                    style={{ width: `${55 + ((i * 17) % 35)}%` }}
                  />
                </div>
              </div>
            </div>
          ))}
        </div>
      ) : filtered.length === 0 ? (
        <div className="page-empty-compact">
          <div className="empty-icon-compact">{Icons.clipboard}</div>
          <span>{t("clipboard.empty")}</span>
        </div>
      ) : (
        <div className="clipboard-list">
          {filtered.map((r, i) => {
            const meta = TYPE_META[r.type] || TYPE_META.text;
            return (
              <div
                key={r.id}
                className={`notification clipboard-card type-${r.type}`}
                style={{ "--color": meta.color, "--enter-delay": i } as React.CSSProperties}
                onClick={() => pasteRecord(r)}
              >
                <div className="notibar" />
                <div className="noticontent">
                  <div className="notititle clipboard-card-header">
                    <span className="noti-type-label">
                      <span className="noti-type-icon">{meta.icon}</span>
                      <span className="noti-type-text">{getTypeLabel(r.type)}</span>
                    </span>
                  </div>
                  <div className="notibody clipboard-card-body">
                    {r.type === "image" ? (
                      <ImageThumb
                        record={r}
                        onHover={handleThumbHover}
                        onLeave={handleThumbLeave}
                        onClick={(e) => {
                          e.stopPropagation();
                          pasteRecord(r);
                        }}
                      />
                    ) : r.type === "link" ? (
                      <span className="clipboard-link-content">{r.content}</span>
                    ) : r.type === "file" ? (
                      <span className="clipboard-file-content">
                        {getFileName(r.content)}
                      </span>
                    ) : (
                      <span className="clipboard-text-content">{r.content}</span>
                    )}
                  </div>
                  <div className="notititle clipboard-card-footer">
                    <span className="clipboard-card-time">{formatTime(r.created_at)}</span>
                    <div className="clipboard-card-actions">
                      <button
                        className="card-delete-btn"
                        onClick={(e) => {
                          e.stopPropagation();
                          deleteRecord(r.id);
                        }}
                      >
                        {Icons.delete}
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {hoverPreview && (
        <div className="thumb-hover-overlay">
          <img src={hoverPreview.src} alt="" />
        </div>
      )}

    </div>
  );
}
