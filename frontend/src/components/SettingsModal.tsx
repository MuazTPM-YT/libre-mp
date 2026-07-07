import { X, SlidersHorizontal } from 'lucide-react';

export interface AppSettings {
  /** Reconnect to the most recently used projector automatically on launch. */
  autoReconnect: boolean;
  /** Show toast notifications for connection and casting events. */
  showNotifications: boolean;
}

export const defaultSettings: AppSettings = {
  autoReconnect: false,
  showNotifications: true,
};

interface Props {
  isOpen: boolean;
  onClose: () => void;
  settings: AppSettings;
  onApply: (settings: AppSettings) => void;
}

/** Application settings. Only options that actually change behavior live here. */
export function SettingsModal({ isOpen, onClose, settings, onApply }: Props) {
  if (!isOpen) return null;

  const toggle = (key: keyof AppSettings) => onApply({ ...settings, [key]: !settings[key] });

  return (
    <div className="lm-modal-overlay" onClick={onClose}>
      <div className="lm-modal" onClick={(e) => e.stopPropagation()}>
        <div className="lm-modal-head">
          <div className="lm-modal-title">
            <SlidersHorizontal size={16} />
            <span>Settings</span>
          </div>
          <button className="lm-iconbtn" onClick={onClose} aria-label="Close">
            <X size={18} />
          </button>
        </div>

        <div className="lm-modal-body">
          <div className="lm-toggle">
            <div className="lm-toggle-txt">
              <strong>Reconnect on launch</strong>
              <span>Rejoin the last projector automatically when the app opens.</span>
            </div>
            <button
              className={`lm-switch ${settings.autoReconnect ? 'on' : ''}`}
              onClick={() => toggle('autoReconnect')}
              role="switch"
              aria-checked={settings.autoReconnect}
              aria-label="Reconnect on launch"
            />
          </div>

          <div className="lm-toggle">
            <div className="lm-toggle-txt">
              <strong>Notifications</strong>
              <span>Show toasts for connection and casting events.</span>
            </div>
            <button
              className={`lm-switch ${settings.showNotifications ? 'on' : ''}`}
              onClick={() => toggle('showNotifications')}
              role="switch"
              aria-checked={settings.showNotifications}
              aria-label="Notifications"
            />
          </div>
        </div>

        <div className="lm-modal-foot">
          <button className="lm-btn signal" onClick={onClose}>
            Done
          </button>
        </div>
      </div>
    </div>
  );
}
