import { X } from 'lucide-react';

interface StatusBannerProps {
  connectedSSID: string | null;
  connectingSSID: string | null;
  connectionError: string | null;
  statusDetail?: string | null;
  onDismissError: () => void;
}

export function StatusBanner({
  connectedSSID,
  connectingSSID,
  connectionError,
  statusDetail,
  onDismissError,
}: StatusBannerProps) {
  // If connected, close the bar immediately (as requested)
  if (connectedSSID && !connectionError) {
    return null;
  }
  
  // If not idle and not connected, show the banner
  if (!connectingSSID && !connectionError) {
    return null;
  }

  const statusClass = connectingSSID ? 'connecting' : connectionError ? 'error' : 'idle';
  const dotClass = connectingSSID ? 'pulse' : connectionError ? 'error' : '';

  return (
    <header className={`status-banner ${statusClass}`}>
      <div className="banner-left">
        <div className={`banner-dot ${dotClass}`} />
        {connectingSSID ? (
          <span>
            {statusDetail || `Connecting to ${connectingSSID}...`}
          </span>
        ) : connectionError ? (
          <span className="banner-error-text">{connectionError}</span>
        ) : (
          <span>Not connected</span>
        )}
      </div>
      <div className="banner-right">
        {connectionError && (
          <button className="banner-dismiss" onClick={onDismissError} title="Dismiss">
            <X size={14} />
          </button>
        )}
      </div>
    </header>
  );
}
