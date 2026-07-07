import { useState, useEffect } from 'react';
import { X, KeyRound, ArrowRight } from 'lucide-react';

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

  if (!isOpen) return null;

  const submit = () => {
    if (ssid.trim()) onConnect(ssid.trim(), pwd.trim());
  };

  return (
    <div className="lm-modal-overlay" onClick={onClose}>
      <div className="lm-modal" onClick={(e) => e.stopPropagation()}>
        <div className="lm-modal-head">
          <div className="lm-modal-title">
            <KeyRound size={16} />
            <span>Enter projector details</span>
          </div>
          <button className="lm-iconbtn" onClick={onClose} aria-label="Close">
            <X size={18} />
          </button>
        </div>

        <div className="lm-modal-body">
          <p className="lm-field-label">
            Type the <strong>SSID</strong> and passphrase shown on the projector’s LAN screen.
          </p>
          <div className="lm-input-wrap" style={{ marginBottom: 10 }}>
            <input
              placeholder="SSID — e.g. RESEARCHLAB-fE8D…"
              value={ssid}
              autoFocus
              onChange={(e) => setSsid(e.target.value)}
            />
          </div>
          <div className="lm-input-wrap">
            <input
              placeholder="Passphrase — often the projector’s MAC"
              value={pwd}
              onChange={(e) => setPwd(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && submit()}
            />
          </div>
        </div>

        <div className="lm-modal-foot">
          <button className="lm-btn ghost" onClick={onClose}>
            Cancel
          </button>
          <button className="lm-btn signal" onClick={submit} disabled={!ssid.trim()}>
            Connect <ArrowRight size={14} />
          </button>
        </div>
      </div>
    </div>
  );
}
