import { Search, RefreshCw, Settings, Sun, Moon, Radio } from 'lucide-react';

interface TopHeaderProps {
  searchQuery: string;
  onSearchChange: (query: string) => void;
  onRefresh: () => void;
  onOpenSettings: () => void;
  onToggleTheme: () => void;
  theme: 'light' | 'dark';
  isScanning: boolean;
}

export function TopHeader({
  searchQuery,
  onSearchChange,
  onRefresh,
  onOpenSettings,
  onToggleTheme,
  theme,
  isScanning
}: TopHeaderProps) {
  return (
    <div className="top-header">
      <div className="brand">
        <div className="brand-icon">
          <Radio size={20} />
        </div>
        <span className="brand-name">LibreMP</span>
      </div>

      <div className="header-controls">
        <div className="search-bar">
          <Search size={18} />
          <input
            type="text"
            placeholder="Search networks..."
            value={searchQuery}
            onChange={(e) => onSearchChange(e.target.value)}
          />
        </div>

        <div className="header-actions">
          <button className="icon-btn-rounded" onClick={onRefresh} disabled={isScanning} title="Refresh">
            <RefreshCw size={20} className={isScanning ? 'spinning' : ''} />
          </button>
          <button className="icon-btn-rounded" onClick={onOpenSettings} title="Settings">
            <Settings size={20} />
          </button>
          <button 
            className="theme-toggle" 
            onClick={onToggleTheme} 
            title={theme === 'light' ? 'Switch to dark mode' : 'Switch to light mode'}
          >
            {theme === 'light' ? <Moon size={20} /> : <Sun size={20} />}
          </button>
        </div>
      </div>
    </div>
  );
}
