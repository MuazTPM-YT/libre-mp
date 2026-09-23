import { useEffect, useState } from 'react';
import { Modal } from './Modal';

interface Props {
  isOpen: boolean;
  projectorName: string;
  onClose: () => void;
  onSubmit: (keyword: string) => void;
}

// ask 4-digit projector keyword shown on projector screen
export function KeywordModal({ isOpen, projectorName, onClose, onSubmit }: Props) {
  const [code, setCode] = useState('');

  useEffect(() => {
    if (!isOpen) setCode('');
  }, [isOpen]);

  const ready = /^\d{4}$/.test(code);

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      onSubmit={() => ready && onSubmit(code)}
      title="Enter Projector Keyword"
      text={`${projectorName} shows a 4-digit keyword on its screen. Type it to connect.`}
      footer={
        <>
          <button type="button" className="lm-btn" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" className="lm-btn primary" disabled={!ready}>
            Connect
          </button>
        </>
      }
    >
      <label className="lm-field">
        <span className="lm-sr">Projector keyword</span>
        <span className="lm-input code">
          <input
            autoFocus
            inputMode="numeric"
            autoComplete="one-time-code"
            maxLength={4}
            placeholder="••••"
            value={code}
            onChange={(e) => setCode(e.target.value.replace(/\D/g, '').slice(0, 4))}
          />
        </span>
      </label>
    </Modal>
  );
}
