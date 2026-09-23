import { Modal } from './Modal';

interface Props {
  isOpen: boolean;
  onClose: () => void;
}

// four steps from launch to casting
export function HelpModal({ isOpen, onClose }: Props) {
  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      onSubmit={onClose}
      title="How to Connect"
      footer={
        <button type="submit" className="lm-btn primary">
          Done
        </button>
      }
    >
      <ol className="lm-steps">
        <li>
          <p>
            <strong>Show the QR code.</strong> On the projector, open the <strong>LAN</strong> or{' '}
            <strong>Network</strong> screen.
          </p>
        </li>
        <li>
          <p>
            <strong>Scan it.</strong> Choose <strong>Scan QR Code</strong> and take a photo, or{' '}
            <strong>Choose Photo…</strong> if you already have a picture.
          </p>
        </li>
        <li>
          <p>
            <strong>Allow screen sharing.</strong> Your desktop may ask which screen to share. LibreMP then
            joins the projector and casts.
          </p>
        </li>
        <li>
          <p>
            <strong>Next time, one click.</strong> The projector appears under <strong>Saved</strong>. Its
            password is kept in your system keychain.
          </p>
        </li>
      </ol>
    </Modal>
  );
}
