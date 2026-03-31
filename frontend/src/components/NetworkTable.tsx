import { Signal, RotateCcw } from 'lucide-react';

interface NetworkItem {
  id: string;
  name: string;
  ssid: string;
  signal: number;
  security: string;
  is_projector: boolean;
  ip?: string;
}

interface NetworkTableProps {
  networks: NetworkItem[];
  connectedSSID: string | null;
  connectingSSID: string | null;
  onConnect: (network: NetworkItem) => void;
  onDisconnect: () => void;
  isScanning: boolean;
  isCasting: boolean;
  onStartCast: () => void;
  onStopCast: () => void;
}

const signalLevel = (s: number) => s > 80 ? 5 : s > 60 ? 4 : s > 40 ? 3 : s > 20 ? 2 : 1;
const signalColor = (s: number) => s > 60 ? 'high' : s > 30 ? 'mid' : 'low';

/** Table component listing available Wi-Fi networks and projectors */
export function NetworkTable({
  networks,
  connectedSSID,
  connectingSSID,
  onConnect,
  onDisconnect,
  isScanning,
  isCasting,
  onStartCast,
  onStopCast
}: NetworkTableProps) {
  return (
    <section className="networks-section">
      <h3 className="section-heading">
        Available Networks
        <span className="heading-count">{networks.length}</span>
      </h3>

      {networks.length === 0 ? (
        <div className="empty-state">
          <div className="empty-icon"><Signal size={28} /></div>
          <h4>{isScanning ? 'Scanning for networks...' : 'No networks found'}</h4>
          <p>Ensure your Wi-Fi adapter is active and click refresh.</p>
        </div>
      ) : (
        <div className="network-table">
          <div className="net-table-header">
            <span className="col-status">Status</span>
            <span className="col-name">Projector Name</span>
            <span className="col-ssid">SSID</span>
            <span className="col-signal">Signal</span>
            <span className="col-action"></span>
          </div>

          <div className="network-list">
            {networks.map((n) => {
              const isConnected = n.ssid === connectedSSID;
              const isConnecting = n.ssid === connectingSSID;
              return (
                <div
                  key={n.id}
                  className={`net-row ${isConnected ? 'connected' : ''} ${n.is_projector ? 'projector' : ''}`}
                >
                  <div className="col-status">
                    <span className={`status-label ${isConnected ? 'on' : ''}`}>
                      {isConnected ? 'Connected' : isConnecting ? 'Connecting…' : n.is_projector ? 'Available' : n.security}
                    </span>
                  </div>

                  <span className="col-name" title={n.name}>{n.name}</span>
                  <span className="col-ssid" title={n.ssid}>{n.ssid}</span>

                  <div className="col-signal">
                    <div className={`signal-bars ${signalColor(n.signal)}`}>
                      {[1, 2, 3, 4, 5].map(i => (
                        <div key={i} className={`bar ${i <= signalLevel(n.signal) ? 'on' : ''}`} />
                      ))}
                    </div>
                  </div>

                  <div className="col-action" style={{ display: 'flex', gap: '8px', justifyContent: 'flex-end' }}>
                    {isConnected && (
                      <button
                        className={`connect-btn-sm ${isCasting ? 'active' : ''}`}
                        onClick={(e) => {
                          e.stopPropagation();
                          isCasting ? onStopCast() : onStartCast();
                        }}
                        style={{ backgroundColor: isCasting ? '#e53e3e' : '#3182ce', color: 'white' }}
                      >
                        {isCasting ? 'Stop Cast' : 'Cast'}
                      </button>
                    )}
                    <button
                      className={`connect-btn-sm ${isConnected ? 'active' : ''}`}
                      onClick={(e) => {
                        e.stopPropagation();
                        isConnected ? onDisconnect() : onConnect(n);
                      }}
                      disabled={isConnecting}
                    >
                      {isConnecting ? (
                        <RotateCcw size={14} className="spinning" />
                      ) : isConnected ? (
                        'Disconnect'
                      ) : (
                        'Connect'
                      )}
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </section>
  );
}
