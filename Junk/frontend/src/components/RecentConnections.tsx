import { Clock } from 'lucide-react';

export function RecentConnections() {
  return (
    <section className="recent-sector">
      <h2 className="section-title">
        <Clock size={18} />
        Recent
      </h2>
      <div className="recent-scroll-area">
        <div className="recent-card">
          <span className="recent-name">Conference Room A</span>
          <button className="btn btn-secondary btn-sm">Reconnect</button>
        </div>
      </div>
    </section>
  );
}
