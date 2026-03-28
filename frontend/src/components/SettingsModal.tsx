import { useState } from 'react';
import { X, Activity, Volume2, Settings, Users, Truck } from 'lucide-react';

interface Props {
  isOpen: boolean;
  onClose: () => void;
}

type SettingsTab = 'user' | 'general' | 'performance' | 'audio' | 'deliver';

export function SettingsModal({ isOpen, onClose }: Props) {
  const [activeTab, setActiveTab] = useState<SettingsTab>('performance');

  if (!isOpen) return null;

  const tabs: { id: SettingsTab; label: string; icon: React.ReactNode }[] = [
    { id: 'user', label: 'User Settings', icon: <Users size={13} /> },
    { id: 'general', label: 'General', icon: <Settings size={13} /> },
    { id: 'performance', label: 'Adjust performance', icon: <Activity size={13} /> },
    { id: 'audio', label: 'Audio Output', icon: <Volume2 size={13} /> },
    { id: 'deliver', label: 'Deliver', icon: <Truck size={13} /> },
  ];

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content settings-modal" onClick={e => e.stopPropagation()}>
        {/* Title */}
        <div className="modal-header">
          <div className="modal-title-row">
            <Settings size={16} />
            <h2>Set options</h2>
          </div>
          <button className="icon-btn" onClick={onClose}><X size={18} /></button>
        </div>

        {/* Tab Bar */}
        <div className="settings-tabs">
          {tabs.map(tab => (
            <button
              key={tab.id}
              className={`settings-tab ${activeTab === tab.id ? 'active' : ''}`}
              onClick={() => setActiveTab(tab.id)}
            >
              {tab.icon}
              {tab.label}
            </button>
          ))}
        </div>

        {/* Tab Content */}
        <div className="settings-body">
          {activeTab === 'user' && (
            <div className="tab-content">
              <fieldset className="field-group">
                <legend>Receiver Settings</legend>
                <label className="check-row">
                  <input type="checkbox" defaultChecked />
                  <span>Display received images in the Viewer</span>
                </label>
                <div className="field-row">
                  <span className="field-label">Save Location Folder</span>
                  <input type="text" className="text-input" defaultValue="C:\Users\User\Documents\LibreMP" readOnly />
                </div>
                <label className="check-row">
                  <input type="checkbox" defaultChecked />
                  <span>Allow Moderator Monitoring</span>
                </label>
              </fieldset>
            </div>
          )}

          {activeTab === 'general' && (
            <div className="tab-content">
              <fieldset className="field-group">
                <legend>General Settings</legend>
                <label className="check-row">
                  <input type="checkbox" defaultChecked />
                  <span>Auto-reconnect on disconnect</span>
                </label>
                <label className="check-row">
                  <input type="checkbox" />
                  <span>Minimize to system tray</span>
                </label>
                <label className="check-row">
                  <input type="checkbox" defaultChecked />
                  <span>Show connection notifications</span>
                </label>
              </fieldset>
            </div>
          )}

          {activeTab === 'performance' && (
            <div className="tab-content">
              <div className="perf-row">
                <span className="field-label">Use Bandwidth</span>
                <select className="select-input" defaultValue="15">
                  <option value="15">15Mbps</option>
                  <option value="10">10Mbps</option>
                  <option value="5">5Mbps</option>
                </select>
              </div>
              <p className="hint-text">For One Projector</p>

              <fieldset className="field-group">
                <legend>Projection Mode</legend>
                <label className="radio-row">
                  <input type="radio" name="projMode" value="movies" />
                  <div>
                    <strong>Movies</strong>
                    <p className="radio-desc">Suitable for watching videos. Prioritizes image smoothness.</p>
                  </div>
                </label>
                <label className="radio-row">
                  <input type="radio" name="projMode" value="operations" defaultChecked />
                  <div>
                    <strong>Operations</strong>
                    <p className="radio-desc">Suitable for projecting and operating images using a computer. Prioritizes operation smoothness.</p>
                  </div>
                </label>
              </fieldset>
            </div>
          )}

          {activeTab === 'audio' && (
            <div className="tab-content">
              <fieldset className="field-group">
                <legend>Audio Output</legend>
                <label className="check-row">
                  <input type="checkbox" defaultChecked />
                  <span>Output audio from projector</span>
                </label>
                <label className="check-row">
                  <input type="checkbox" />
                  <span>Mute local audio while projecting</span>
                </label>
              </fieldset>
            </div>
          )}

          {activeTab === 'deliver' && (
            <div className="tab-content">
              <fieldset className="field-group">
                <legend>Content Delivery</legend>
                <label className="check-row">
                  <input type="checkbox" defaultChecked />
                  <span>Send screen continuously</span>
                </label>
                <label className="check-row">
                  <input type="checkbox" />
                  <span>Pause on screen change</span>
                </label>
              </fieldset>
            </div>
          )}
        </div>

        {/* Footer Buttons */}
        <div className="modal-footer">
          <button className="action-btn secondary" onClick={onClose}>Reset</button>
          <button className="action-btn secondary">Apply</button>
          <button className="action-btn primary" onClick={onClose}>Close</button>
        </div>
      </div>
    </div>
  );
}
