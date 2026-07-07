import type { ReactNode } from 'react';
import { X, HelpCircle } from 'lucide-react';

interface Props {
  isOpen: boolean;
  onClose: () => void;
}

/** Quick guide: how to get from launch to casting. */
export function HelpModal({ isOpen, onClose }: Props) {
  if (!isOpen) return null;

  const steps: [string, ReactNode][] = [
    [
      'Show the QR',
      <>
        On the projector, open <strong>LAN / Network</strong> — it displays a QR code
        with the network details.
      </>,
    ],
    [
      'Scan or upload it',
      <>
        Use <strong>Scan with camera</strong> to point at the QR, or{' '}
        <strong>Upload QR photo</strong> to pick a picture of it. LibreMP reads the SSID
        and passphrase automatically.
      </>,
    ],
    [
      'It connects and casts',
      <>
        LibreMP joins the projector’s network and starts mirroring your screen. The lamp
        in the top bar turns amber while casting.
      </>,
    ],
    [
      'Next time is one tap',
      <>
        Connected projectors are saved under <strong>Saved</strong> — reconnect instantly
        without scanning again.
      </>,
    ],
  ];

  return (
    <div className="lm-modal-overlay" onClick={onClose}>
      <div className="lm-modal" onClick={(e) => e.stopPropagation()}>
        <div className="lm-modal-head">
          <div className="lm-modal-title">
            <HelpCircle size={16} />
            <span>How to connect</span>
          </div>
          <button className="lm-iconbtn" onClick={onClose} aria-label="Close">
            <X size={18} />
          </button>
        </div>
        <div className="lm-modal-body">
          <ol className="lm-help-list">
            {steps.map(([title, body], i) => (
              <li key={i}>
                <span className="lm-help-num">{String(i + 1).padStart(2, '0')}</span>
                <p>
                  <strong>{title}.</strong> {body}
                </p>
              </li>
            ))}
          </ol>
        </div>
        <div className="lm-modal-foot">
          <button className="lm-btn signal" onClick={onClose}>
            Got it
          </button>
        </div>
      </div>
    </div>
  );
}
