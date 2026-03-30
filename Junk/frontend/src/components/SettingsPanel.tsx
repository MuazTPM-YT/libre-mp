import { X, Monitor, Activity, Volume2 } from 'lucide-react';

interface Props {
  isOpen: boolean;
  onClose: () => void;
}

export function SettingsPanel({ isOpen, onClose }: Props) {
  return (
    <aside className={`settings-panel ${isOpen ? 'open' : ''}`}>
      <div className="settings-header">
        <h2>Settings</h2>
        <button className="icon-btn" onClick={onClose}>
          <X size={20} />
        </button>
      </div>
      
      <div className="settings-content">
        <div className="setting-group">
          <h3 className="group-title"><Monitor size={16}/> Display Mode</h3>
          <div className="radio-container">
            <label className="radio-label">
              <input type="radio" name="displayMode" value="movies" defaultChecked />
              <span className="radio-text">Movies (Smoother Video)</span>
            </label>
            <label className="radio-label">
              <input type="radio" name="displayMode" value="operations" />
              <span className="radio-text">Operations (Low Latency)</span>
            </label>
          </div>
        </div>

        <div className="setting-group">
          <h3 className="group-title"><Activity size={16}/> Bandwidth</h3>
          <select className="select-input" defaultValue="15">
            <option value="15">15 Mbps (Recommended)</option>
            <option value="10">10 Mbps (Stable)</option>
            <option value="5">5 Mbps (Low Quality)</option>
          </select>
        </div>

        <div className="setting-group">
          <h3 className="group-title"><Volume2 size={16}/> Audio</h3>
          <label className="checkbox-label">
            <input type="checkbox" defaultChecked />
            <span className="checkbox-text">Output audio from projector</span>
          </label>
        </div>
      </div>
    </aside>
  );
}
