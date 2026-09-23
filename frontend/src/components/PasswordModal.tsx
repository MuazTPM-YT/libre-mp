import { useEffect, useState } from 'react';
import { CircleAlert, Eye, EyeOff, LoaderCircle } from 'lucide-react';
import { Modal } from './Modal';

interface Props {
  isOpen: boolean;
  networkName: string;
  isProjector: boolean;
  error?: string | null;
  isLoading?: boolean;
  onCancel: () => void;
  onSubmit: (password: string) => void;
}

// ask wi-fi password for secured network
export function PasswordModal({ isOpen, networkName, isProjector, error, isLoading, onCancel, onSubmit }: Props) {
  const [pwd, setPwd] = useState('');
  const [show, setShow] = useState(false);

  useEffect(() => {
    if (!isOpen) {
      setPwd('');
      setShow(false);
    }
  }, [isOpen]);

  const submit = () => {
    if (pwd && !isLoading) onSubmit(pwd);
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onCancel}
      closable={!isLoading}
      onSubmit={submit}
      title={`Enter the password for “${networkName}”`}
      text={
        isProjector
          ? 'It is on the projector’s network screen. On Epson projectors it is often the projector’s MAC address.'
          : 'LibreMP joins this network, then looks for a projector on it.'
      }
      footer={
        <>
          <button type="button" className="lm-btn" onClick={onCancel} disabled={isLoading}>
            Cancel
          </button>
          <button type="submit" className="lm-btn primary" disabled={!pwd || isLoading}>
            {isLoading && <LoaderCircle size={14} className="lm-spin" />}
            {isLoading ? 'Joining…' : 'Join'}
          </button>
        </>
      }
    >
      <label className="lm-field">
        <span className="lm-field-label">Password</span>
        <span className={`lm-input ${error ? 'invalid' : ''}`.trim()}>
          <input
            type={show ? 'text' : 'password'}
            autoFocus
            autoComplete="off"
            spellCheck={false}
            value={pwd}
            disabled={isLoading}
            aria-invalid={!!error}
            aria-describedby={error ? 'lm-pwd-error' : undefined}
            onChange={(e) => setPwd(e.target.value)}
          />
          <button
            type="button"
            className="lm-icon-btn"
            onClick={() => setShow(!show)}
            aria-label={show ? 'Hide password' : 'Show password'}
          >
            {show ? <EyeOff size={16} /> : <Eye size={16} />}
          </button>
        </span>
      </label>
      {error && (
        <div className="lm-field-error" id="lm-pwd-error" role="alert">
          <CircleAlert size={14} />
          <span>{error}</span>
        </div>
      )}
    </Modal>
  );
}
