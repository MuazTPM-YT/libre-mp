import { Search } from 'lucide-react';

interface Props {
  searchQuery: string;
  setSearchQuery: (query: string) => void;
}

export function SearchFilterBar({ searchQuery, setSearchQuery }: Props) {
  return (
    <div className="search-container glass-panel">
      <Search className="search-icon" size={20} />
      <input 
        type="text" 
        className="search-input" 
        placeholder="Search projectors by name or room..." 
        value={searchQuery}
        onChange={(e) => setSearchQuery(e.target.value)}
      />
      <div className="filter-pills">
        <button className="pill active">All</button>
        <button className="pill">Strongest Signal</button>
      </div>
    </div>
  );
}
