import { memo, useCallback } from "react";
import type { ClipboardRecord } from "../../types";
import { Icons } from "../../components/Icons";
import { ImageThumb } from "./ImageThumb";
import { formatTime, getFileName, TYPE_META } from "./utils";

interface ClipboardCardProps {
  record: ClipboardRecord;
  index: number;
  selected: boolean;
  getTypeLabel: (type: string) => string;
  onSelect: (r: ClipboardRecord) => void;
  onPaste: (r: ClipboardRecord) => void;
  onDelete: (id: string) => void;
  onThumbHover: (thumbSrc: string, rect: DOMRect) => void;
  onThumbLeave: () => void;
}

function ClipboardCardInner({
  record,
  index,
  selected,
  getTypeLabel,
  onSelect,
  onPaste,
  onDelete,
  onThumbHover,
  onThumbLeave,
}: ClipboardCardProps) {
  const meta = TYPE_META[record.type] || TYPE_META.text;

  const handleClick = useCallback(() => {
    if (selected) {
      onPaste(record);
    } else {
      onSelect(record);
    }
  }, [selected, onSelect, onPaste, record]);
  const handleDelete = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      onDelete(record.id);
    },
    [onDelete, record.id],
  );

  return (
    <div
      className={`notification clipboard-card type-${record.type}${selected ? " selected" : ""}`}
      style={{ "--color": meta.color, "--enter-delay": index } as React.CSSProperties}
      onClick={handleClick}
    >
      <div className="notibar" />
      <div className="noticontent">
        <div className="notititle clipboard-card-header">
          <span className="noti-type-label">
            <span className="noti-type-icon">{meta.icon}</span>
            <span className="noti-type-text">{getTypeLabel(record.type)}</span>
          </span>
        </div>
        <div className="notibody clipboard-card-body">
          {record.type === "image" ? (
            <ImageThumb
              record={record}
              onHover={onThumbHover}
              onLeave={onThumbLeave}
              onClick={handleClick}
            />
          ) : record.type === "link" ? (
            <span className="clipboard-link-content">{record.content}</span>
          ) : record.type === "file" ? (
            <span className="clipboard-file-content">{getFileName(record.content)}</span>
          ) : (
            <span className="clipboard-text-content">{record.content}</span>
          )}
        </div>
        <div className="notititle clipboard-card-footer">
          <span className="clipboard-card-time">{formatTime(record.created_at)}</span>
          <div className="clipboard-card-actions">
            <button className="card-delete-btn" onClick={handleDelete}>
              {Icons.delete}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

export const ClipboardCard = memo(ClipboardCardInner);
