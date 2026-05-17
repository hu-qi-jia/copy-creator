import { useEffect, useRef, useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { useClipboardStore, type ClipType } from "../../stores/clipboardStore";
import { usePhraseStore } from "../../stores/phraseStore";
import { useHoverSwitch } from "./useHoverSwitch";
import { HoverProgress } from "./HoverProgress";

type TabKey = "clipboard" | "phrases";

const HOVER_DELAY = 500;
const MAX_ITEMS = 30;

function formatTime(dateStr: string): string {
  const date = new Date(dateStr);
  const month = date.getMonth() + 1;
  const day = date.getDate();
  const hours = date.getHours().toString().padStart(2, "0");
  const minutes = date.getMinutes().toString().padStart(2, "0");
  return `${month}/${day} ${hours}:${minutes}`;
}

function ImageThumb({ recordId }: { recordId: string }) {
  const [src, setSrc] = useState("");
  const { records, getThumbnail } = useClipboardStore();

  useEffect(() => {
    const record = records.find((r) => r.id === recordId);
    if (!record || record.type !== "image") return;
    let cancelled = false;
    getThumbnail(record).then((url) => {
      if (!cancelled && url) setSrc(url);
    });
    return () => { cancelled = true; };
  }, [recordId, records, getThumbnail]);

  if (!src) return <span className="radial-menu-item-text">…</span>;
  return (
    <img
      src={src}
      alt=""
      style={{ width: 48, height: 36, objectFit: "cover", borderRadius: 5 }}
    />
  );
}

export default function RadialMenu() {
  const { t } = useTranslation();

  const [visible, setVisible] = useState(false);
  const [activeTab, setActiveTab] = useState<TabKey>("clipboard");
  const [selectedItemId, setSelectedItemId] = useState<string | null>(null);
  const [clipboardCategory, setClipboardCategory] = useState<ClipType>("all");
  const [phraseGroupId, setPhraseGroupId] = useState<string | null>(null);

  const isRightDownRef = useRef(false);
  const visibleRef = useRef(false);
  const showTimestampRef = useRef(0);
  const lastFocusRef = useRef(0);
  const selectedItemIdRef = useRef<string | null>(null);
  const activeTabRef = useRef<TabKey>("clipboard");
  const clipboardCategoryRef = useRef<ClipType>("all");
  const phraseGroupIdRef = useRef<string | null>(null);
  const startPosRef = useRef({ x: 0, y: 0 });

  useEffect(() => { visibleRef.current = visible; }, [visible]);
  useEffect(() => { selectedItemIdRef.current = selectedItemId; }, [selectedItemId]);
  useEffect(() => { activeTabRef.current = activeTab; }, [activeTab]);
  useEffect(() => { clipboardCategoryRef.current = clipboardCategory; }, [clipboardCategory]);
  useEffect(() => { phraseGroupIdRef.current = phraseGroupId; }, [phraseGroupId]);

  useEffect(() => {
    // Initial theme load
    invoke<string>("get_setting", { key: "theme" }).then((theme) => {
      if (theme === "dark" || theme === "light") {
        document.documentElement.setAttribute("data-theme", theme);
      }
    }).catch(() => {});
    // Pre-load data so it's ready when the menu first shows
    useClipboardStore.getState().init();
    usePhraseStore.getState().loadGroups();
  }, []);

  const handleTabSwitch = useCallback((key: string) => {
    const tab = key as TabKey;
    setActiveTab(tab);
    setSelectedItemId(null);
    selectedItemIdRef.current = null;
    if (tab === "phrases") {
      const { groups, loadPhrases } = usePhraseStore.getState();
      if (groups.length > 0) {
        const firstId = groups[0].id;
        setPhraseGroupId(firstId);
        phraseGroupIdRef.current = firstId;
        loadPhrases(firstId);
      }
    }
  }, []);

  const handleCategorySwitch = useCallback((key: string) => {
    if (activeTabRef.current === "clipboard") {
      setClipboardCategory(key as ClipType);
      clipboardCategoryRef.current = key as ClipType;
    } else {
      setPhraseGroupId(key);
      phraseGroupIdRef.current = key;
      usePhraseStore.getState().loadPhrases(key);
    }
    setSelectedItemId(null);
    selectedItemIdRef.current = null;
  }, []);

  const navSwitch = useHoverSwitch(handleTabSwitch, HOVER_DELAY);
  const categorySwitch = useHoverSwitch(handleCategorySwitch, HOVER_DELAY);

  const navEnterRef = useRef(navSwitch.handleEnter);
  navEnterRef.current = navSwitch.handleEnter;
  const navLeaveRef = useRef(navSwitch.handleLeave);
  navLeaveRef.current = navSwitch.handleLeave;
  const catEnterRef = useRef(categorySwitch.handleEnter);
  catEnterRef.current = categorySwitch.handleEnter;
  const catLeaveRef = useRef(categorySwitch.handleLeave);
  catLeaveRef.current = categorySwitch.handleLeave;

  const resetState = useCallback(() => {
    isRightDownRef.current = false;
    visibleRef.current = false;
    setVisible(false);
    setSelectedItemId(null);
    selectedItemIdRef.current = null;
    navLeaveRef.current();
    catLeaveRef.current();
  }, []);

  const updateHoverFromPoint = useCallback((cssX: number, cssY: number) => {
    const el = document.elementFromPoint(cssX, cssY);
    if (!el) {
      selectedItemIdRef.current = null;
      setSelectedItemId(null);
      navLeaveRef.current();
      catLeaveRef.current();
      return;
    }

    const itemEl = (el as HTMLElement).closest("[data-radial-item-id]");
    const navEl = (el as HTMLElement).closest("[data-radial-nav]");
    const catEl = (el as HTMLElement).closest("[data-radial-category]");

    if (itemEl) {
      const id = itemEl.getAttribute("data-radial-item-id");
      selectedItemIdRef.current = id;
      setSelectedItemId(id);
      navLeaveRef.current();
      catLeaveRef.current();
    } else if (navEl) {
      const key = navEl.getAttribute("data-radial-nav");
      if (key && key !== activeTabRef.current) {
        navEnterRef.current(key);
      } else {
        navLeaveRef.current();
      }
      catLeaveRef.current();
      selectedItemIdRef.current = null;
      setSelectedItemId(null);
    } else if (catEl) {
      const key = catEl.getAttribute("data-radial-category");
      const activeCat = activeTabRef.current === "clipboard"
        ? clipboardCategoryRef.current
        : phraseGroupIdRef.current;
      if (key && key !== activeCat) {
        catEnterRef.current(key);
      } else {
        catLeaveRef.current();
      }
      navLeaveRef.current();
      selectedItemIdRef.current = null;
      setSelectedItemId(null);
    } else {
      selectedItemIdRef.current = null;
      setSelectedItemId(null);
      navLeaveRef.current();
      catLeaveRef.current();
    }
  }, []);

  useEffect(() => {
    let unlisteners: UnlistenFn[] = [];

    const setup = async () => {
      const unDown = await listen<{ x: number; y: number; theme: string }>("radial-menu-down", (e) => {
        console.log("[RadialMenu] radial-menu-down:", e.payload);
        // Apply theme synchronously from backend-provided value
        document.documentElement.setAttribute("data-theme", e.payload.theme);
        isRightDownRef.current = true;
        showTimestampRef.current = Date.now();
        startPosRef.current = { x: e.payload.x, y: e.payload.y };
        visibleRef.current = true;
        setVisible(true);
      });

      const unMove = await listen<{ x: number; y: number }>("radial-menu-move", (e) => {
        if (!isRightDownRef.current) return;

        const cssX = e.payload.x;
        const cssY = e.payload.y;

        if (!visibleRef.current) {
          showTimestampRef.current = Date.now();
          visibleRef.current = true;
          setVisible(true);
        }

        updateHoverFromPoint(cssX, cssY);
      });

      const unUp = await listen("radial-menu-up", async () => {
        console.log("[RadialMenu] radial-menu-up, visible:", visibleRef.current, "selected:", selectedItemIdRef.current);
        if (isRightDownRef.current) {
          if (visibleRef.current && selectedItemIdRef.current) {
            const itemId = selectedItemIdRef.current;
            const { records, pasteRecord } = useClipboardStore.getState();
            const record = records.find((r) => r.id === itemId);
            if (record) {
              await pasteRecord(record);
            } else {
              const { phrases, pastePhrase } = usePhraseStore.getState();
              const phrase = phrases.find((p) => p.id === itemId);
              if (phrase) {
                await pastePhrase(phrase);
              }
            }
          }
          resetState();
          // Hide the popup window after processing
          getCurrentWindow().hide();
        }
      });

      unlisteners = [unDown, unMove, unUp];
    };

    setup();

    const handleContextMenu = (e: Event) => {
      e.preventDefault();
    };

    const handleWheel = (e: WheelEvent) => {
      if (!isRightDownRef.current || !visibleRef.current) return;

      e.preventDefault();
      e.stopPropagation();

      const el = document.elementFromPoint(e.clientX, e.clientY);
      if (!el) return;

      const catContainer = (el as HTMLElement).closest("[data-radial-categories]");
      if (catContainer) {
        catContainer.scrollLeft += e.deltaY;
        return;
      }

      const listContainer = (el as HTMLElement).closest("[data-radial-list]");
      if (listContainer) {
        listContainer.scrollTop += e.deltaY;
      }
    };

    const handleFocus = () => {
      lastFocusRef.current = Date.now();
    };

    const handleBlur = () => {
      if (isRightDownRef.current) {
        // Ignore blurs within 1s of show — compositor initialization
        // can cause spurious focus/blur on first show of transparent window.
        if (Date.now() - showTimestampRef.current < 1000) return;
        resetState();
        getCurrentWindow().hide();
      }
    };

    document.addEventListener("contextmenu", handleContextMenu, true);
    document.addEventListener("wheel", handleWheel, { passive: false });
    window.addEventListener("focus", handleFocus);
    window.addEventListener("blur", handleBlur);

    return () => {
      unlisteners.forEach((fn) => fn());
      document.removeEventListener("contextmenu", handleContextMenu, true);
      document.removeEventListener("wheel", handleWheel);
      window.removeEventListener("focus", handleFocus);
      window.removeEventListener("blur", handleBlur);
    };
  }, [resetState, updateHoverFromPoint]);

  const clipboardStore = useClipboardStore();
  const phraseStore = usePhraseStore();

  useEffect(() => {
    if (visible && activeTab === "phrases" && !phraseGroupId && phraseStore.groups.length > 0) {
      const firstId = phraseStore.groups[0].id;
      setPhraseGroupId(firstId);
      phraseGroupIdRef.current = firstId;
      phraseStore.loadPhrases(firstId);
    }
  }, [visible, activeTab, phraseGroupId, phraseStore.groups, phraseStore.loadPhrases]);

  const filteredRecords = clipboardCategory === "all"
    ? clipboardStore.records
    : clipboardStore.records.filter((r) => r.type === clipboardCategory);

  const items = activeTab === "clipboard"
    ? filteredRecords.slice(0, MAX_ITEMS).map((r) => ({
        id: r.id,
        content: r.type === "image"
          ? `[${t("clipboard.image")}]`
          : r.type === "file"
            ? r.content.replace(/\\/g, "/").split("/").pop() || r.content
            : r.content,
        type: r.type,
        createdAt: r.created_at,
      }))
    : phraseStore.phrases.map((p) => ({
        id: p.id,
        content: p.content,
        type: "phrase" as string,
        title: p.title,
      }));

  const categories = activeTab === "clipboard"
    ? [
        { key: "all", label: t("clipboard.all") },
        { key: "text", label: t("clipboard.text") },
        { key: "image", label: t("clipboard.image") },
        { key: "link", label: t("clipboard.link") },
        { key: "file", label: t("clipboard.file") },
      ]
    : phraseStore.groups.map((g) => ({
        key: g.id,
        label: g.name,
      }));

  const activeCategory = activeTab === "clipboard" ? clipboardCategory : phraseGroupId;

  return (
    <div className={`radial-menu-overlay${visible ? "" : " radial-menu-hidden"}`}>
      <div className="radial-menu-popup">
        <div className="radial-menu-nav">
          {(["clipboard", "phrases"] as TabKey[]).map((tab) => (
            <button
              key={tab}
              className={`radial-menu-nav-tab ${activeTab === tab ? "active" : ""}`}
              data-radial-nav={tab}
            >
              <span className="radial-menu-nav-label">{t(`tabs.${tab}`)}</span>
              {navSwitch.progressKey === tab && (
                <HoverProgress progress={navSwitch.progress} />
              )}
            </button>
          ))}
        </div>

        {categories.length > 0 && (
          <div className="radial-menu-categories" data-radial-categories>
            {categories.map((cat) => (
              <button
                key={cat.key}
                className={`radial-menu-category-chip ${activeCategory === cat.key ? "active" : ""}`}
                data-radial-category={cat.key}
              >
                {cat.label}
                {categorySwitch.progressKey === cat.key && (
                  <HoverProgress progress={categorySwitch.progress} />
                )}
              </button>
            ))}
          </div>
        )}

        <div className="radial-menu-list" data-radial-list>
          {items.length === 0 ? (
            <div className="radial-menu-empty">{t("radialMenu.empty")}</div>
          ) : (
            items.map((item) => (
              <div
                key={item.id}
                className={`radial-menu-item ${selectedItemId === item.id ? "selected" : ""}`}
                data-radial-item-id={item.id}
              >
                {item.type === "image" ? (
                  <ImageThumb recordId={item.id} />
                ) : (
                  <span className="radial-menu-item-text">
                    {item.content.length > 80
                      ? item.content.slice(0, 80) + "…"
                      : item.content}
                  </span>
                )}
                {"createdAt" in item && item.createdAt && (
                  <span className="radial-menu-item-time">{formatTime(item.createdAt)}</span>
                )}
                {"title" in item && item.title && (
                  <span className="radial-menu-item-remark">{item.title}</span>
                )}
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
