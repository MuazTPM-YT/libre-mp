import { useState, useEffect } from 'react';
import { Lock, X, ArrowRight, RotateCcw, AlertCircle, Eye, EyeOff } from 'lucide-react';

interface Props {
  isOpen: boolean;
  networkName: string;
  error?: string | null;
  isLoading?: boolean;
  onCancel: () => void;
  onSubmit: (password: string) => void;
}

/** Modal for entering Wi-Fi network passwords */
export function PasswordModal({ isOpen, networkName, error, isLoading, onCancel, onSubmit }: Props) {
  const [pwd, setPwd] = useState('');
  const [showPassword, setShowPassword] = useState(false);

  // Clear password field and reset visibility when modal opens/closes
  useEffect(() => {
    if (!isOpen) {
      setPwd('');
      setShowPassword(false);
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const handleSubmit = () => {
    if (pwd.trim() && !isLoading) {
      onSubmit(pwd);
    }
  };

  const handleCancel = () => {
    if (!isLoading) {
      onCancel();
    }
  };

  return (
    <div className="modal-overlay" onClick={handleCancel}>
      <div className="modal-content connection-modal" onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <div className="modal-title-row">
            <Lock size={16} />
            <h2>Secure Network</h2>
          </div>
          <button className="icon-btn" onClick={handleCancel} disabled={isLoading}>
            <X size={18} />
          </button>
        </div>

        <div className="modal-body-pad">
          <p className="modal-instruction">
            Enter the security key for <strong>{networkName}</strong>
          </p>

          <div className="password-input-wrapper">
            <div className="input-with-icon">
              <input
                type={showPassword ? "text" : "password"}
                className={`text-input ${error ? 'error' : ''}`}
                placeholder="Security key"
                autoFocus
                value={pwd}
                disabled={isLoading}
                onChange={e => setPwd(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && handleSubmit()}
              />
              <button
                type="button"
                className="input-icon-btn"
                onClick={() => setShowPassword(!showPassword)}
                title={showPassword ? "Hide password" : "Show password"}
                tabIndex={-1}
              >
                {showPassword ? <EyeOff size={18} /> : <Eye size={18} />}
              </button>
            </div>

            {error && (
              <div className="modal-error-message">
                <AlertCircle size={14} />
                <span>{error}</span>
              </div>
            )}
          </div>
        </div>

        <div className="modal-footer">
          <button
            className="action-btn secondary"
            onClick={handleCancel}
            disabled={isLoading}
          >
            Cancel
          </button>
          <button
            className="action-btn primary btn-min-width"
            onClick={handleSubmit}
            disabled={!pwd.trim() || isLoading}
          >
            {isLoading ? (
              <>Connecting <RotateCcw size={14} className="spinning" /></>
            ) : (
              <>Connect <ArrowRight size={14} /></>
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
