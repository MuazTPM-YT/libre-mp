import { useState, useEffect } from 'react';
import { Lock, X, ArrowRight, AlertCircle, Eye, EyeOff, RotateCcw } from 'lucide-react';

interface Props {
  isOpen: boolean;
  networkName: string;
  error?: string | null;
  isLoading?: boolean;
  onCancel: () => void;
  onSubmit: (password: string) => void;
}

/** Prompt for a Wi-Fi passphrase when connecting to a secured, non-projector network. */
export function PasswordModal({ isOpen, networkName, error, isLoading, onCancel, onSubmit }: Props) {
  const [pwd, setPwd] = useState('');
  const [show, setShow] = useState(false);

  useEffect(() => {
    if (!isOpen) {
      setPwd('');
      setShow(false);
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const submit = () => {
    if (pwd.trim() && !isLoading) onSubmit(pwd);
  };
  const cancel = () => {
    if (!isLoading) onCancel();
  };

  return (
    <div className="lm-modal-overlay" onClick={cancel}>
      <div className="lm-modal" onClick={(e) => e.stopPropagation()}>
        <div className="lm-modal-head">
          <div className="lm-modal-title">
            <Lock size={16} />
            <span>Enter passphrase</span>
          </div>
          <button className="lm-iconbtn" onClick={cancel} disabled={isLoading} aria-label="Close">
            <X size={18} />
          </button>
        </div>

        <div className="lm-modal-body">
          <p className="lm-field-label">
            Security key for <strong>{networkName}</strong>
          </p>
          <div className={`lm-input-wrap ${error ? 'err' : ''}`}>
            <input
              type={show ? 'text' : 'password'}
              placeholder="Passphrase"
              autoFocus
              value={pwd}
              disabled={isLoading}
              onChange={(e) => setPwd(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && submit()}
            />
            <button
              className="lm-iconbtn"
              onClick={() => setShow(!show)}
              tabIndex={-1}
              aria-label={show ? 'Hide passphrase' : 'Show passphrase'}
            >
              {show ? <EyeOff size={17} /> : <Eye size={17} />}
            </button>
          </div>
          {error && (
            <div className="lm-inline-err">
              <AlertCircle size={14} />
              <span>{error}</span>
            </div>
          )}
        </div>

        <div className="lm-modal-foot">
          <button className="lm-btn ghost" onClick={cancel} disabled={isLoading}>
            Cancel
          </button>
          <button className="lm-btn signal" onClick={submit} disabled={!pwd.trim() || isLoading}>
            {isLoading ? (
              <>
                Connecting <RotateCcw size={14} className="lm-spin" />
              </>
            ) : (
              <>
                Connect <ArrowRight size={14} />
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
