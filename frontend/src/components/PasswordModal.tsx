import { useState, useEffect } from 'react';
import { Lock, ArrowRight, AlertCircle, Eye, EyeOff, RotateCcw } from 'lucide-react';
import { Modal } from './Modal';

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

  const submit = () => {
    if (pwd.trim() && !isLoading) onSubmit(pwd);
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onCancel}
      closable={!isLoading}
      title="Enter passphrase"
      icon={<Lock size={16} />}
      footer={
        <>
          <button className="lm-btn ghost" onClick={onCancel} disabled={isLoading}>
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
        </>
      }
    >
      <p className="lm-field-label" id="lm-pwd-label">
        Security key for <strong>{networkName}</strong>
      </p>
      <div className={`lm-input-wrap ${error ? 'err' : ''}`}>
        <input
          type={show ? 'text' : 'password'}
          placeholder="Passphrase"
          autoFocus
          value={pwd}
          disabled={isLoading}
          aria-labelledby="lm-pwd-label"
          aria-invalid={!!error}
          aria-describedby={error ? 'lm-pwd-error' : undefined}
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
        <div className="lm-inline-err" id="lm-pwd-error" role="alert">
          <AlertCircle size={14} />
          <span>{error}</span>
        </div>
      )}
    </Modal>
  );
}
