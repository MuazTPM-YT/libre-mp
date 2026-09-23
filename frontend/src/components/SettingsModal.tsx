import { SlidersHorizontal } from 'lucide-react';
import { Modal } from './Modal';

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

const OPTIONS: { key: keyof AppSettings; label: string; hint: string }[] = [
  {
    key: 'autoReconnect',
    label: 'Reconnect on launch',
    hint: 'Rejoin the last projector automatically when the app opens.',
  },
  {
    key: 'showNotifications',
    label: 'Notifications',
    hint: 'Show toasts for connection and casting events.',
  },
];

/** Application settings. Only options that actually change behavior live here. */
export function SettingsModal({ isOpen, onClose, settings, onApply }: Props) {
  const toggle = (key: keyof AppSettings) => onApply({ ...settings, [key]: !settings[key] });

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title="Settings"
      icon={<SlidersHorizontal size={16} />}
      footer={
        <button className="lm-btn signal" onClick={onClose}>
          Done
        </button>
      }
    >
      {OPTIONS.map(({ key, label, hint }) => (
        <div className="lm-toggle" key={key}>
          <div className="lm-toggle-txt">
            <strong>{label}</strong>
            <span>{hint}</span>
          </div>
          <button
            className={`lm-switch ${settings[key] ? 'on' : ''}`}
            onClick={() => toggle(key)}
            role="switch"
            aria-checked={settings[key]}
            aria-label={label}
          />
        </div>
      ))}
    </Modal>
  );
}
