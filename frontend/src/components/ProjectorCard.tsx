import { Signal, MonitorDot } from 'lucide-react';

export interface Projector {
  id: string;
  name: string;
  ssid: string;
  signal: number;
  mode: 'Quick Connection' | 'Advanced Mode';
}

interface Props {
  projector: Projector;
  onConnect: (id: string) => void;
}

export function ProjectorCard({ projector, onConnect }: Props) {
  const getSignalClass = (signal: number) => {
    if (signal > 80) return 'high';
    if (signal > 40) return 'med';
    return 'low';
  };

  return (
    <div className="projector-card">
      <div className="card-header">
        <div>
          <h3 className="projector-name">{projector.name}</h3>
          <span className="projector-ssid">SSID: {projector.ssid}</span>
        </div>
        <div className={`signal-strength ${getSignalClass(projector.signal)}`}>
          <Signal size={20} />
          <span>{projector.signal}%</span>
        </div>
      </div>
      <div className="card-footer">
        <div className="mode-badge">{projector.mode}</div>
        <button className="btn btn-primary" onClick={() => onConnect(projector.id)}>
          <MonitorDot size={18} />
          Connect
        </button>
      </div>
    </div>
  );
}
