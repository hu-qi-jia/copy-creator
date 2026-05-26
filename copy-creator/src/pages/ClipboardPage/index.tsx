import { useEffect, useState, useRef, useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useClipboardStore } from "../../stores/clipboardStore";
import { Icons } from "../../components/Icons";
import SearchInput from "../../components/SearchInput";
import { ClipboardCard } from "./ClipboardCard";
import { TYPE_META } from "./utils";

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
    selectedRecordId,
    init,
    setSearch,
    setCategory,
    loadRecords,
    deleteRecord,
    pasteRecord,
    setSelectedRecord,
  } = useClipboardStore();

  const searchRef = useRef<HTMLInputElement>(null);
  const [hoverPreview, setHoverPreview] = useState<{ src: string; x: number; y: number } | null>(null);
  const hoverTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const categories: { key: ClipType; label: string }[] = [
    { key: "all", label: t("clipboard.all") },
    { key: "text", label: t("clipboard.text") },
    { key: "image", label: t("clipboard.image") },
    { key: "link", label: t("clipboard.link") },
    { key: "file", label: t("clipboard.file") },
  ];

  const labels: Record<string, string> = useMemo(
    () => ({
      text: t("clipboard.text"),
      image: t("clipboard.image"),
      link: t("clipboard.link"),
      file: t("clipboard.file"),
    }),
    [t],
  );

  const getTypeLabel = useCallback(
    (type: string): string => labels[type] || labels.text,
    [labels],
  );

  const handleSelect = useCallback(
    (r: typeof records[number]) => {
      if (selectedRecordId === r.id) {
        pasteRecord(r);
        setSelectedRecord(null);
      } else {
        setSelectedRecord(r.id);
      }
    },
    [selectedRecordId, pasteRecord, setSelectedRecord],
  );

  const handlePaste = useCallback(
    (r: typeof records[number]) => pasteRecord(r),
    [pasteRecord],
  );

  const handleDelete = useCallback(
    (id: string) => deleteRecord(id),
    [deleteRecord],
  );

  const filtered = useMemo(
    () => (category === "all" ? records : records.filter((r) => r.type === category)),
    [records, category],
  );

  useEffect(() => {
    init();
  }, []);

  // Auto-focus search input when window gains focus
  useEffect(() => {
    const unlisten = getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) {
        setTimeout(() => searchRef.current?.focus(), 50);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
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
          ref={searchRef}
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

      {loading && records.length === 0 ? (
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
          {filtered.map((r, i) => (
            <ClipboardCard
              key={r.id}
              record={r}
              index={i}
              selected={selectedRecordId === r.id}
              getTypeLabel={getTypeLabel}
              onSelect={handleSelect}
              onPaste={handlePaste}
              onDelete={handleDelete}
              onThumbHover={handleThumbHover}
              onThumbLeave={handleThumbLeave}
            />
          ))}
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
