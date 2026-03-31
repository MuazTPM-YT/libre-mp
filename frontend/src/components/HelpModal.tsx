import { X, HelpCircle } from 'lucide-react';

interface Props {
  isOpen: boolean;
  onClose: () => void;
}

/** Modal displaying troubleshooting and usage instructions */
export function HelpModal({ isOpen, onClose }: Props) {
  if (!isOpen) return null;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <div className="modal-title-row">
            <HelpCircle size={16} />
            <h2>Help & Assistant</h2>
          </div>
          <button className="icon-btn" onClick={onClose}>
            <X size={18} />
          </button>
        </div>
        <div className="modal-body-pad">
          <h3>How to connect:</h3>
          <ol className="help-steps">
            <li>Turn on your Epson projector and wait for Wi-Fi broadcast.</li>
            <li>Click <strong>Refresh</strong> to scan for available projectors.</li>
            <li>Select a projector from the list and click <strong>Connect</strong>.</li>
          </ol>
          <div className="help-note">
            <strong>Note:</strong> You will temporarily lose internet access while connected directly to the projector's Quick Connection network.
          </div>
        </div>
      </div>
    </div>
  );
}
