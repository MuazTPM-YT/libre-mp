import { X, Wifi } from 'lucide-react';

interface Props {
  isOpen: boolean;
  onClose: () => void;
  mode: 'quick' | 'advanced';
  setMode: (mode: 'quick' | 'advanced') => void;
}

/** Modal for selecting the connection mode between quick and advanced */
export function ConnectionModeModal({ isOpen, onClose, mode, setMode }: Props) {
  if (!isOpen) return null;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content connection-modal" onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <div className="modal-title-row">
            <Wifi size={16} />
            <h2>Select Connection Mode</h2>
          </div>
          <button className="icon-btn" onClick={onClose}><X size={18} /></button>
        </div>

        <div className="modal-body-pad">
          <p className="modal-instruction">
            When connecting wirelessly, make sure you select the connection mode
            specified in the [Network] menu.<br />
            (This is set to [Quick Connection Mode] by default.)
          </p>

          <div className="mode-options">
            <label className="radio-row">
              <input
                type="radio"
                name="connMode"
                value="quick"
                checked={mode === 'quick'}
                onChange={() => setMode('quick')}
              />
              <strong>Quick Connection Mode</strong>
            </label>
            <label className="radio-row">
              <input
                type="radio"
                name="connMode"
                value="advanced"
                checked={mode === 'advanced'}
                onChange={() => setMode('advanced')}
              />
              <strong>Advanced Connection Mode</strong>
            </label>
          </div>

          <div className="mode-description">
            Connect the computer and the projector over a wireless connection.
          </div>

          <label className="check-row">
            <input type="checkbox" />
            <span>Set the selected Connection Mode as the default mode for future connections.
              (Do not display this window again.)</span>
          </label>
        </div>

        <div className="modal-footer">
          <button className="action-btn primary" onClick={onClose}>OK</button>
          <button className="action-btn secondary" onClick={onClose}>Cancel</button>
        </div>
      </div>
    </div>
  );
}
