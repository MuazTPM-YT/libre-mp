import { Settings } from 'lucide-react';

interface Props {
  isSettingsOpen: boolean;
  onToggleSettings: () => void;
  statusText?: string;
  isScanning?: boolean;
}

export function ConnectionStatusBanner({ 
  isSettingsOpen, 
  onToggleSettings, 
  statusText = "Scanning for Epson Projectors...",
  isScanning = true 
}: Props) {
  return (
    <header className="status-banner">
      <div className="status-indicator">
        {isScanning ? (
          <div className="pulse-dot"></div>
        ) : (
          <div className="connected-dot" style={{ width: 10, height: 10, borderRadius: '50%', backgroundColor: 'var(--accent-success)' }}></div>
        )}
        <span className="status-text">{statusText}</span>
      </div>
      <div className="top-actions">
        <button 
          className={`icon-btn ${isSettingsOpen ? 'active' : ''}`} 
          onClick={onToggleSettings}
          title="Settings"
          aria-label="Settings"
        >
          <Settings size={20} />
        </button>
      </div>
    </header>
  );
}
