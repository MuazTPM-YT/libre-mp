import { useEffect, useState } from 'react';
import { Modal } from './Modal';

interface Props {
  isOpen: boolean;
  onClose: () => void;
  onConnect: (ssid: string, password: string) => void;
}

// no qr: type network name + password from projector screen
export function ManualConnectModal({ isOpen, onClose, onConnect }: Props) {
  const [ssid, setSsid] = useState('');
  const [pwd, setPwd] = useState('');

  useEffect(() => {
    if (!isOpen) {
      setSsid('');
      setPwd('');
    }
  }, [isOpen]);

  const submit = () => {
    if (ssid.trim()) onConnect(ssid.trim(), pwd.trim());
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      onSubmit={submit}
      title="Enter Projector Details"
      text="Read them from the projector’s network screen."
      footer={
        <>
          <button type="button" className="lm-btn" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" className="lm-btn primary" disabled={!ssid.trim()}>
            Connect
          </button>
        </>
      }
    >
      <label className="lm-field">
        <span className="lm-field-label">Network name (SSID)</span>
        <span className="lm-input">
          <input
            autoFocus
            spellCheck={false}
            placeholder="e.g. HALL-A-xK3pQ7vRt2Lm"
            value={ssid}
            onChange={(e) => setSsid(e.target.value)}
          />
        </span>
      </label>
      <label className="lm-field">
        <span className="lm-field-label">Password</span>
        <span className="lm-input">
          <input
            spellCheck={false}
            autoComplete="off"
            placeholder="Often the projector’s MAC address"
            value={pwd}
            onChange={(e) => setPwd(e.target.value)}
          />
        </span>
      </label>
    </Modal>
  );
}
