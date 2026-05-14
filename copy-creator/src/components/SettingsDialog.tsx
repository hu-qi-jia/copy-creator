import SettingsContent from "./SettingsContent";

interface Props {
  open: boolean;
  onClose: () => void;
}

export default function SettingsDialog({ open, onClose }: Props) {
  if (!open) return null;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal settings-panel"
        onClick={(e) => e.stopPropagation()}
        style={{ width: 340, maxHeight: "85vh", overflowY: "auto", padding: 0 }}
      >
        <div style={{ padding: "20px 20px 0" }}>
          <div className="modal-title">Settings</div>
        </div>
        <SettingsContent onClose={onClose} />
      </div>
    </div>
  );
}
