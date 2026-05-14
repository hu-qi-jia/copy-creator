import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { usePhraseStore } from "../stores/phraseStore";
import { Icons } from "../icons";
import SearchInput from "../components/SearchInput";

export default function PhrasePage() {
  const { t } = useTranslation();
  const [groupDialogOpen, setGroupDialogOpen] = useState(false);
  const [phraseDialogOpen, setPhraseDialogOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [groupName, setGroupName] = useState("");
  const [phraseRemark, setPhraseRemark] = useState("");
  const [phraseContent, setPhraseContent] = useState("");
  const [phraseError, setPhraseError] = useState(false);
  const [manageGroupsOpen, setManageGroupsOpen] = useState(false);
  const [renameId, setRenameId] = useState<string | null>(null);
  const [renameName, setRenameName] = useState("");

  const {
    groups,
    phrases,
    selectedGroupId,
    search,
    loading,
    setSearch,
    setSelectedGroup,
    loadGroups,
    loadPhrases,
    createGroup,
    updateGroup,
    createPhrase,
    updatePhrase,
    deletePhrase,
    deleteGroup,
    pastePhrase,
  } = usePhraseStore();

  useEffect(() => {
    loadGroups();
  }, []);

  useEffect(() => {
    if (selectedGroupId) {
      loadPhrases(selectedGroupId);
    }
  }, [selectedGroupId]);

  const groupsScrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = groupsScrollRef.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      if (Math.abs(e.deltaY) > Math.abs(e.deltaX)) {
        e.preventDefault();
        el.scrollLeft += e.deltaY;
      }
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, []);

  const openNewGroup = () => {
    setEditingId(null);
    setGroupName("");
    setGroupDialogOpen(true);
  };

  const handleSaveGroup = async () => {
    if (groupName.trim()) {
      if (editingId) {
        await updateGroup(editingId, groupName.trim());
      } else {
        await createGroup(groupName.trim());
      }
    }
    setGroupDialogOpen(false);
  };

  const openNewPhrase = () => {
    setEditingId(null);
    setPhraseRemark("");
    setPhraseContent("");
    setPhraseError(false);
    setPhraseDialogOpen(true);
  };

  const openEditPhrase = (p: { id: string; title: string; content: string }) => {
    setEditingId(p.id);
    setPhraseRemark(p.title);
    setPhraseContent(p.content);
    setPhraseError(false);
    setPhraseDialogOpen(true);
  };

  const handleSavePhrase = async () => {
    if (!phraseContent.trim()) {
      setPhraseError(true);
      return;
    }
    setPhraseError(false);
    if (editingId) {
      await updatePhrase(editingId, phraseRemark.trim(), phraseContent.trim());
    } else if (selectedGroupId) {
      await createPhrase(selectedGroupId, phraseRemark.trim(), phraseContent.trim());
    }
    setPhraseDialogOpen(false);
  };

  const openManageGroups = () => {
    setRenameId(null);
    setRenameName("");
    setManageGroupsOpen(true);
  };

  const startRename = (id: string, name: string) => {
    setRenameId(id);
    setRenameName(name);
  };

  const handleRename = async () => {
    if (renameId && renameName.trim()) {
      await updateGroup(renameId, renameName.trim());
    }
    setRenameId(null);
    setRenameName("");
  };

  const handleDeleteGroup = async (id: string) => {
    await deleteGroup(id);
    if (groups.length <= 1) {
      setManageGroupsOpen(false);
    }
  };

  return (
    <div className="phrase-page">
      <div className="page-search">
        <SearchInput
          placeholder={t("phrases.search")}
          value={search}
          onChange={setSearch}
        />
      </div>

      <div className="phrase-groups">
        <div className="groups-scroll" ref={groupsScrollRef}>
          {groups.map((g) => (
            <button
              key={g.id}
              className={`group-chip ${g.id === selectedGroupId ? "active" : ""}`}
              onClick={() => setSelectedGroup(g.id)}
            >
              {g.name}
            </button>
          ))}
        </div>
        <button className="group-add-btn" onClick={openNewGroup}>
          {Icons.add}
        </button>
        <button className="group-add-btn" onClick={openManageGroups} title={t("phrases.manageGroups")}>
          {Icons.edit}
        </button>
        {selectedGroupId && (
          <button className="phrase-add-btn" onClick={openNewPhrase}>
            {Icons.add}
            <span>{t("phrases.newPhrase")}</span>
          </button>
        )}
      </div>

      {loading && phrases.length === 0 ? (
        <div className="phrase-list">
          {[1, 2, 3, 4].map((i) => (
            <div key={i} className="notification skeleton">
              <div className="notibar" />
              <div className="noticontent">
                <div className="notibody">
                  <div className="skeleton-line" style={{ width: `${40 + ((i * 13) % 30)}%` }} />
                </div>
                <div className="notititle">
                  <div className="skeleton-line short" />
                </div>
              </div>
            </div>
          ))}
        </div>
      ) : !selectedGroupId ? (
        <div className="page-empty-compact">
          <div className="empty-icon-compact">{Icons.phrases}</div>
          <span>{t("phrases.empty")}</span>
        </div>
      ) : phrases.length === 0 && !loading ? (
        <div className="page-empty-compact">
          <span>{t("phrases.emptyGroupPhrases")}</span>
        </div>
      ) : (
        <div className="phrase-list">
          {phrases.map((p, i) => (
            <div
              key={p.id}
              className="notification phrase-card"
              style={{ "--enter-delay": i } as React.CSSProperties}
              onClick={() => pastePhrase(p)}
            >
              <div className="notibar" />
              <div className="noticontent">
                <div className="notibody phrase-card-body">{p.content}</div>
                <div className="notititle phrase-card-footer">
                  <span className="phrase-card-remark">{p.title}</span>
                  <div className="phrase-card-actions">
                    <button
                      className="card-edit-btn"
                      onClick={(e) => {
                        e.stopPropagation();
                        openEditPhrase(p);
                      }}
                    >
                      {Icons.edit}
                    </button>
                    <button
                      className="card-delete-btn"
                      onClick={(e) => {
                        e.stopPropagation();
                        deletePhrase(p.id);
                      }}
                    >
                      {Icons.delete}
                    </button>
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {groupDialogOpen && (
        <div className="dialog-overlay" onClick={() => setGroupDialogOpen(false)}>
          <div className="dialog-content" onClick={(e) => e.stopPropagation()}>
            <h3 className="dialog-title">
              {editingId ? t("common.edit") : t("phrases.newGroup")}
            </h3>
            <input
              className="dialog-input"
              autoFocus
              placeholder={t("phrases.groupName")}
              value={groupName}
              onChange={(e) => setGroupName(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleSaveGroup()}
            />
            <div className="dialog-actions">
              <button
                className="dialog-btn secondary"
                onClick={() => setGroupDialogOpen(false)}
              >
                {t("common.cancel")}
              </button>
              <button className="dialog-btn save" onClick={handleSaveGroup}>
                {t("common.save")}
              </button>
            </div>
          </div>
        </div>
      )}

      {phraseDialogOpen && (
        <div className="dialog-overlay" onClick={() => setPhraseDialogOpen(false)}>
          <div className="dialog-content large" onClick={(e) => e.stopPropagation()}>
            <h3 className="dialog-title">
              {editingId ? t("common.edit") : t("phrases.newPhrase")}
            </h3>
            <textarea
              className={`dialog-textarea${phraseError ? " error" : ""}`}
              autoFocus
              placeholder={t("phrases.content")}
              value={phraseContent}
              onChange={(e) => {
                setPhraseContent(e.target.value);
                if (e.target.value.trim()) setPhraseError(false);
              }}
            />
            {phraseError && (
              <span className="dialog-error-text">{t("phrases.contentRequired")}</span>
            )}
            <input
              className="dialog-input"
              placeholder={t("phrases.remark")}
              value={phraseRemark}
              onChange={(e) => setPhraseRemark(e.target.value)}
            />
            <div className="dialog-actions">
              <button
                className="dialog-btn secondary"
                onClick={() => setPhraseDialogOpen(false)}
              >
                {t("common.cancel")}
              </button>
              <button className="dialog-btn save" onClick={handleSavePhrase}>
                {t("common.save")}
              </button>
            </div>
          </div>
        </div>
      )}

      {manageGroupsOpen && (
        <div className="dialog-overlay" onClick={() => setManageGroupsOpen(false)}>
          <div className="dialog-content large" onClick={(e) => e.stopPropagation()}>
            <h3 className="dialog-title">{t("phrases.manageGroups")}</h3>
            <div className="phrase-group-manage-list">
              {groups.map((g) => (
                <div key={g.id} className="phrase-group-manage-row">
                  {renameId === g.id ? (
                    <input
                      className="dialog-input"
                      autoFocus
                      value={renameName}
                      onChange={(e) => setRenameName(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") handleRename();
                        if (e.key === "Escape") {
                          setRenameId(null);
                          setRenameName("");
                        }
                      }}
                      onBlur={handleRename}
                    />
                  ) : (
                    <span className="phrase-group-manage-name">{g.name}</span>
                  )}
                  <div className="phrase-group-manage-actions">
                    <button
                      className="card-edit-btn"
                      style={{ opacity: 1 }}
                      onClick={() => startRename(g.id, g.name)}
                      title={t("phrases.rename")}
                    >
                      {Icons.edit}
                    </button>
                    <button
                      className="card-delete-btn"
                      style={{ opacity: 1 }}
                      onClick={() => handleDeleteGroup(g.id)}
                      title={t("common.delete")}
                    >
                      {Icons.delete}
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
