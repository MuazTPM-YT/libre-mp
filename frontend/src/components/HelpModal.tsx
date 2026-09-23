import type { ReactNode } from 'react';
import { HelpCircle } from 'lucide-react';
import { Modal } from './Modal';

interface Props {
  isOpen: boolean;
  onClose: () => void;
}

/** Quick guide: how to get from launch to casting. */
export function HelpModal({ isOpen, onClose }: Props) {
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
        Use <strong>Live scan</strong> to take a photo of the QR, or{' '}
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
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title="How to connect"
      icon={<HelpCircle size={16} />}
      footer={
        <button className="lm-btn signal" onClick={onClose}>
          Got it
        </button>
      }
    >
      <ol className="lm-help-list">
        {steps.map(([title, body], i) => (
          <li key={title}>
            <span className="lm-help-num">{String(i + 1).padStart(2, '0')}</span>
            <p>
              <strong>{title}.</strong> {body}
            </p>
          </li>
        ))}
      </ol>
    </Modal>
  );
}
