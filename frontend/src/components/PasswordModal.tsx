import { useState } from 'react';
import { Lock, X, ArrowRight } from 'lucide-react';

interface Props {
  isOpen: boolean;
  networkName: string;
  onCancel: () => void;
  onSubmit: (password: string) => void;
}

export function PasswordModal({ isOpen, networkName, onCancel, onSubmit }: Props) {
  const [pwd, setPwd] = useState('');

  if (!isOpen) return null;

  const handleSubmit = () => {
    if (pwd.trim()) {
      onSubmit(pwd);
      setPwd('');
    }
  };

  const handleCancel = () => {
    setPwd('');
    onCancel();
  };

  return (
    <div className="modal-overlay" onClick={handleCancel}>
      <div className="modal-content connection-modal" onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <div className="modal-title-row">
            <Lock size={16} />
            <h2>Secure Network</h2>
          </div>
          <button className="icon-btn" onClick={handleCancel}><X size={18} /></button>
        </div>
        <div className="modal-body-pad" style={{ paddingBottom: '16px' }}>
          <p className="modal-instruction" style={{ marginBottom: '8px' }}>
            Enter the security key for <strong>{networkName}</strong>
          </p>
          <input 
            type="password" 
            className="text-input" 
            placeholder="Security key"
            autoFocus
            value={pwd}
            onChange={e => setPwd(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && handleSubmit()}
            style={{ width: '100%', padding: '10px 14px' }}
          />
        </div>
        <div className="modal-footer">
          <button className="action-btn secondary" onClick={handleCancel}>Cancel</button>
          <button 
            className="action-btn primary" 
            onClick={handleSubmit}
            disabled={!pwd.trim()}
          >
            Connect <ArrowRight size={14} />
          </button>
        </div>
      </div>
    </div>
  );
}
