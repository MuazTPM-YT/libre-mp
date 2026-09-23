import { Modal } from './Modal';

export type Appearance = 'system' | 'light' | 'dark';

export interface AppSettings {
  // follow os, or force light / dark
  appearance: Appearance;
  // rejoin last projector when app opens
  autoReconnect: boolean;
  // small notices for things that happen out of view
  showNotifications: boolean;
}

export const defaultSettings: AppSettings = {
  appearance: 'system',
  autoReconnect: false,
  showNotifications: true,
};

interface Props {
  isOpen: boolean;
  onClose: () => void;
  settings: AppSettings;
  onApply: (settings: AppSettings) => void;
}

const SWITCHES: { key: 'autoReconnect' | 'showNotifications'; label: string; hint: string }[] = [
  { key: 'autoReconnect', label: 'Reconnect on launch', hint: 'Connect to the last projector when LibreMP opens.' },
  { key: 'showNotifications', label: 'Notifications', hint: 'Show a short notice when casting stops.' },
];

const APPEARANCES: [Appearance, string][] = [
  ['system', 'System'],
  ['light', 'Light'],
  ['dark', 'Dark'],
];

// settings sheet: appearance + two switches
export function SettingsModal({ isOpen, onClose, settings, onApply }: Props) {
  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      onSubmit={onClose}
      title="Settings"
      footer={
        <button type="submit" className="lm-btn primary">
          Done
        </button>
      }
    >
      <div className="lm-setting">
        <div>
          <strong id="lm-appearance">Appearance</strong>
        </div>
        <div className="lm-seg" role="radiogroup" aria-labelledby="lm-appearance">
          {APPEARANCES.map(([value, label]) => (
            <button
              key={value}
              type="button"
              role="radio"
              aria-checked={settings.appearance === value}
              onClick={() => onApply({ ...settings, appearance: value })}
            >
              {label}
            </button>
          ))}
        </div>
      </div>
      {SWITCHES.map(({ key, label, hint }) => (
        <div className="lm-setting" key={key}>
          <div>
            <strong>{label}</strong>
            <span>{hint}</span>
          </div>
          <button
            type="button"
            className="lm-switch"
            role="switch"
            aria-checked={settings[key]}
            aria-label={label}
            onClick={() => onApply({ ...settings, [key]: !settings[key] })}
          />
        </div>
      ))}
    </Modal>
  );
}
