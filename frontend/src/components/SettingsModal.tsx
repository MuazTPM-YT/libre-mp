import { useState, useEffect } from 'react';
import { X, Activity, Volume2, Settings, Truck, Check } from 'lucide-react';

export interface AppSettings {
  displayMode: 'movies' | 'operations';
  brightness: number;
  audioOutput: boolean;
  muteLocal: boolean;
  bandwidth: number;
  autoReconnect: boolean;
  minimizeToTray: boolean;
  showNotifications: boolean;
  sendContinuously: boolean;
  pauseOnChange: boolean;
}

export const defaultSettings: AppSettings = {
  displayMode: 'operations',
  brightness: 80,
  audioOutput: true,
  muteLocal: false,
  bandwidth: 15,
  autoReconnect: true,
  minimizeToTray: false,
  showNotifications: true,
  sendContinuously: true,
  pauseOnChange: false,
};

interface Props {
  isOpen: boolean;
  onClose: () => void;
  settings: AppSettings;
  onApply: (settings: AppSettings) => void;
}

type SettingsTab = 'general' | 'performance' | 'audio' | 'deliver';

export function SettingsModal({ isOpen, onClose, settings, onApply }: Props) {
  const [activeTab, setActiveTab] = useState<SettingsTab>('performance');
  const [draft, setDraft] = useState<AppSettings>(settings);
  const [saved, setSaved] = useState(false);
  
  const isDirty = JSON.stringify(draft) !== JSON.stringify(settings);

  // Sync draft when settings change externally or modal opens
  useEffect(() => {
    if (isOpen) {
      setDraft(settings);
    } else {
      setSaved(false);
    }
  }, [isOpen, settings]);

  if (!isOpen) return null;

  const tabs: { id: SettingsTab; label: string; icon: React.ReactNode }[] = [
    { id: 'general', label: 'General', icon: <Settings size={13} /> },
    { id: 'performance', label: 'Performance', icon: <Activity size={13} /> },
    { id: 'audio', label: 'Audio', icon: <Volume2 size={13} /> },
    { id: 'deliver', label: 'Delivery', icon: <Truck size={13} /> },
  ];

  const handleApply = () => {
    onApply(draft);
    setSaved(true);
  };

  const handleReset = () => {
    setDraft(defaultSettings);
    setSaved(false);
  };

  const handleClose = () => {
    onApply(draft);
    onClose();
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content settings-modal" onClick={e => e.stopPropagation()}>
        {/* Title */}
        <div className="modal-header">
          <div className="modal-title-row">
            <Settings size={16} />
            <h2>Settings</h2>
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

          {activeTab === 'general' && (
            <div className="tab-content">
              <div className="setting-group">
                <h4 className="setting-group-title">Connection</h4>
                <label className="toggle-row">
                  <div className="toggle-info">
                    <span className="toggle-label">Auto-reconnect</span>
                    <span className="toggle-desc">Automatically reconnect on disconnect</span>
                  </div>
                  <div className={`toggle-switch ${draft.autoReconnect ? 'on' : ''}`}
                    onClick={() => setDraft({ ...draft, autoReconnect: !draft.autoReconnect })}>
                    <div className="toggle-thumb" />
                  </div>
                </label>
                <label className="toggle-row">
                  <div className="toggle-info">
                    <span className="toggle-label">Show notifications</span>
                    <span className="toggle-desc">Display connection status alerts</span>
                  </div>
                  <div className={`toggle-switch ${draft.showNotifications ? 'on' : ''}`}
                    onClick={() => setDraft({ ...draft, showNotifications: !draft.showNotifications })}>
                    <div className="toggle-thumb" />
                  </div>
                </label>
              </div>
              <div className="setting-group">
                <h4 className="setting-group-title">System</h4>
                <label className="toggle-row">
                  <div className="toggle-info">
                    <span className="toggle-label">Minimize to tray</span>
                    <span className="toggle-desc">Keep running in system tray area</span>
                  </div>
                  <div className={`toggle-switch ${draft.minimizeToTray ? 'on' : ''}`}
                    onClick={() => setDraft({ ...draft, minimizeToTray: !draft.minimizeToTray })}>
                    <div className="toggle-thumb" />
                  </div>
                </label>
              </div>
            </div>
          )}

          {activeTab === 'performance' && (
            <div className="tab-content">
              <div className="setting-group">
                <h4 className="setting-group-title">Bandwidth</h4>
                <div className="segmented-control">
                  {[5, 10, 15].map(bw => (
                    <button
                      key={bw}
                      className={`seg-btn ${draft.bandwidth === bw ? 'active' : ''}`}
                      onClick={() => setDraft({ ...draft, bandwidth: bw })}
                    >
                      {bw} Mbps
                    </button>
                  ))}
                </div>
                <span className="setting-hint">Higher bandwidth = better quality, more network usage</span>
              </div>

              <div className="setting-group">
                <h4 className="setting-group-title">Projection Mode</h4>
                <div className="mode-cards">
                  <div
                    className={`mode-card ${draft.displayMode === 'movies' ? 'active' : ''}`}
                    onClick={() => setDraft({ ...draft, displayMode: 'movies' })}
                  >
                    <div className="mode-card-check">
                      {draft.displayMode === 'movies' && <Check size={14} />}
                    </div>
                    <div className="mode-card-content">
                      <strong>Movies</strong>
                      <span>Prioritizes image smoothness for video playback</span>
                    </div>
                  </div>
                  <div
                    className={`mode-card ${draft.displayMode === 'operations' ? 'active' : ''}`}
                    onClick={() => setDraft({ ...draft, displayMode: 'operations' })}
                  >
                    <div className="mode-card-check">
                      {draft.displayMode === 'operations' && <Check size={14} />}
                    </div>
                    <div className="mode-card-content">
                      <strong>Operations</strong>
                      <span>Prioritizes responsiveness for presentations</span>
                    </div>
                  </div>
                </div>
              </div>

              <div className="setting-group">
                <h4 className="setting-group-title">Brightness</h4>
                <div className="slider-row">
                  <input
                    type="range"
                    min="10"
                    max="100"
                    step="5"
                    value={draft.brightness}
                    onChange={(e) => setDraft({ ...draft, brightness: Number(e.target.value) })}
                    className="modern-slider"
                  />
                  <span className="slider-value">{draft.brightness}%</span>
                </div>
              </div>
            </div>
          )}

          {activeTab === 'audio' && (
            <div className="tab-content">
              <div className="setting-group">
                <h4 className="setting-group-title">Audio Output</h4>
                <label className="toggle-row">
                  <div className="toggle-info">
                    <span className="toggle-label">Output audio from projector</span>
                    <span className="toggle-desc">Route audio through the projector speakers</span>
                  </div>
                  <div className={`toggle-switch ${draft.audioOutput ? 'on' : ''}`}
                    onClick={() => setDraft({ ...draft, audioOutput: !draft.audioOutput })}>
                    <div className="toggle-thumb" />
                  </div>
                </label>
                <label className="toggle-row">
                  <div className="toggle-info">
                    <span className="toggle-label">Mute local audio</span>
                    <span className="toggle-desc">Silence your computer speakers while casting</span>
                  </div>
                  <div className={`toggle-switch ${draft.muteLocal ? 'on' : ''}`}
                    onClick={() => setDraft({ ...draft, muteLocal: !draft.muteLocal })}>
                    <div className="toggle-thumb" />
                  </div>
                </label>
              </div>
            </div>
          )}

          {activeTab === 'deliver' && (
            <div className="tab-content">
              <div className="setting-group">
                <h4 className="setting-group-title">Content Delivery</h4>
                <label className="toggle-row">
                  <div className="toggle-info">
                    <span className="toggle-label">Send screen continuously</span>
                    <span className="toggle-desc">Stream every frame in real-time</span>
                  </div>
                  <div className={`toggle-switch ${draft.sendContinuously ? 'on' : ''}`}
                    onClick={() => setDraft({ ...draft, sendContinuously: !draft.sendContinuously })}>
                    <div className="toggle-thumb" />
                  </div>
                </label>
                <label className="toggle-row">
                  <div className="toggle-info">
                    <span className="toggle-label">Pause on screen change</span>
                    <span className="toggle-desc">Briefly pause stream when switching windows</span>
                  </div>
                  <div className={`toggle-switch ${draft.pauseOnChange ? 'on' : ''}`}
                    onClick={() => setDraft({ ...draft, pauseOnChange: !draft.pauseOnChange })}>
                    <div className="toggle-thumb" />
                  </div>
                </label>
              </div>
            </div>
          )}
        </div>

        {/* Footer Buttons */}
        <div className="modal-footer">
          <button className="action-btn secondary" onClick={handleReset}>Reset</button>
          <button 
            className={`action-btn ${(saved && !isDirty) ? 'success' : isDirty ? 'primary' : 'secondary'}`} 
            onClick={handleApply} 
            disabled={!isDirty && !(saved && !isDirty)}
          >
            {(saved && !isDirty) ? <><Check size={14} /> Saved</> : 'Apply'}
          </button>
          <button className="action-btn primary" onClick={handleClose}>Close</button>
        </div>
      </div>
    </div>
  );
}
