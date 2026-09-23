import { useState, useEffect } from 'react';
import { KeyRound, ArrowRight } from 'lucide-react';
import { Modal } from './Modal';

interface Props {
  isOpen: boolean;
  onClose: () => void;
  onConnect: (ssid: string, password: string) => void;
}

/** Fallback for connecting without a QR: type the projector's SSID + passphrase. */
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
      title="Enter projector details"
      icon={<KeyRound size={16} />}
      footer={
        <>
          <button className="lm-btn ghost" onClick={onClose}>
            Cancel
          </button>
          <button className="lm-btn signal" onClick={submit} disabled={!ssid.trim()}>
            Connect <ArrowRight size={14} />
          </button>
        </>
      }
    >
      <p className="lm-field-label">
        Type the <strong>SSID</strong> and passphrase shown on the projector’s LAN screen.
      </p>
      <div className="lm-input-wrap" style={{ marginBottom: 10 }}>
        <input
          placeholder="SSID — e.g. RESEARCHLAB-fE8D…"
          value={ssid}
          autoFocus
          aria-label="Projector SSID"
          onChange={(e) => setSsid(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && submit()}
        />
      </div>
      <div className="lm-input-wrap">
        <input
          placeholder="Passphrase — often the projector’s MAC"
          value={pwd}
          aria-label="Projector passphrase"
          onChange={(e) => setPwd(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && submit()}
        />
      </div>
    </Modal>
  );
}
