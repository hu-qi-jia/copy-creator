import { useEffect, useState, useRef, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { useClipboardStore } from "../stores/clipboardStore";
import { Icons } from "../icons";
import SearchInput from "../components/SearchInput";

type ClipType = "all" | "text" | "image" | "link" | "file";

function formatTime(dateStr: string): string {
  const date = new Date(dateStr);
  const month = date.getMonth() + 1;
  const day = date.getDate();
  const hours = date.getHours().toString().padStart(2, "0");
  const minutes = date.getMinutes().toString().padStart(2, "0");
  return `${month}/${day} ${hours}:${minutes}`;
}

const TYPE_META: Record<string, { icon: React.ReactElement; color: string }> = {
  text: { icon: Icons.clipboard, color: "#007AFF" },
  image: { icon: Icons.image, color: "#34C759" },
  link: { icon: Icons.link, color: "#FF9500" },
  file: { icon: Icons.file, color: "#AF52DE" },
};

function getFileName(path: string): string {
  const parts = path.replace(/\\/g, "/").split("/");
  return parts[parts.length - 1] || path;
}

function ImageThumb({ record, onHover, onLeave, onClick }: {
  record: { id: string; content: string };
  onHover: (src: string, rect: DOMRect) => void;
  onLeave: () => void;
  onClick: (e: React.MouseEvent) => void;
}) {
  const { getThumbnail, thumbnailCache } = useClipboardStore();
  const [src, setSrc] = useState<string | null>(null);
  const [visible, setVisible] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!visible) return;
    const cached = thumbnailCache[record.id];
    if (cached) {
      setSrc(cached);
      return;
    }
    getThumbnail(record as any).then((dataUrl) => {
      if (dataUrl) setSrc(dataUrl);
    });
  }, [visible, record.id]);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) setVisible(true);
      },
      { rootMargin: "200px" }
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  return (
    <div
      ref={ref}
      className="clipboard-card-thumb"
      onMouseEnter={(e) => {
        if (!src) return;
        const rect = e.currentTarget.getBoundingClientRect();
        onHover(src, rect);
      }}
      onMouseLeave={onLeave}
      onClick={onClick}
    >
      {src ? (
        <img src={src} alt="" />
      ) : (
        <div className="thumb-spinner" />
      )}
    </div>
  );
}

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

  const filtered =
    category === "all" ? records : records.filter((r) => r.type === category);

  useEffect(() => {
    init();
  }, []);

  useEffect(() => {
    loadRecords();
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
                  <div className="notititle">
                    <span className="noti-type-label">
                      <span className="noti-type-icon">{meta.icon}</span>
                      {formatTime(r.created_at)}
                    </span>
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
                  <div className="notibody">
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
                      r.content
                    )}
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
