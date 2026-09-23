import { useEffect, useState } from 'react';
import { CircleAlert, LoaderCircle } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { Modal } from './Modal';

export interface QrResult {
  ssid: string;
  password: string;
  ip: string;
}

interface Props {
  isOpen: boolean;
  onClose: () => void;
  onDecoded: (result: QrResult) => void;
}

// preview budget ~15 fps; each frame cross ipc as base64 jpeg
const PREVIEW_INTERVAL_MS = 66;

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const errText = (e: unknown, fallback: string) => (typeof e === 'string' ? e : fallback);

// camera sheet: live preview, take photo, read qr at once. camera runs in rust, not webview
export function LiveScanModal({ isOpen, onClose, onDecoded }: Props) {
  const [preview, setPreview] = useState<string | null>(null);
  const [still, setStill] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // live preview loop while open and not frozen
  useEffect(() => {
    if (!isOpen || still || error) return;
    let active = true;
    (async () => {
      while (active) {
        const started = Date.now();
        try {
          const url = await invoke<string>('camera_preview_frame');
          if (!active) return;
          setPreview(url);
        } catch (e) {
          if (active) setError(errText(e, 'The camera stopped.'));
          return;
        }
        const left = PREVIEW_INTERVAL_MS - (Date.now() - started);
        if (left > 0) await sleep(left);
      }
    })();
    return () => {
      active = false;
    };
  }, [isOpen, still, error]);

  // free camera and reset on close
  useEffect(() => {
    if (isOpen) return;
    invoke('camera_stop').catch(() => {});
    setPreview(null);
    setStill(null);
    setError(null);
    setBusy(false);
  }, [isOpen]);

  // freeze photo, then read qr from it
  const takePhoto = async () => {
    setBusy(true);
    try {
      setStill(await invoke<string>('camera_capture'));
      onDecoded(await invoke<QrResult>('camera_scan'));
    } catch (e) {
      setError(errText(e, 'Could not read the QR code.'));
    } finally {
      setBusy(false);
    }
  };

  const retake = () => {
    setError(null);
    setStill(null);
  };

  const shown = still || preview;

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      onSubmit={error ? retake : takePhoto}
      wide
      title="Scan QR Code"
      text="Hold the camera so the QR code on the projector’s network screen fills most of the picture."
      footer={
        <>
          <button type="button" className="lm-btn" onClick={onClose}>
            Cancel
          </button>
          {error ? (
            <button type="submit" className="lm-btn primary">
              Try Again
            </button>
          ) : (
            <button type="submit" className="lm-btn primary" disabled={busy || !preview}>
              Take Photo
            </button>
          )}
        </>
      }
    >
      <div className="lm-camera">
        {shown ? (
          <img src={shown} alt={still ? 'Photo of the QR code' : 'Live camera preview'} />
        ) : (
          !error && <span className="lm-camera-note">Starting camera…</span>
        )}
        {busy && (
          <div className="lm-camera-busy" role="status">
            <LoaderCircle size={20} className="lm-spin" />
            <span>Reading QR code…</span>
          </div>
        )}
      </div>
      {error && (
        <div className="lm-field-error" role="alert">
          <CircleAlert size={14} />
          <span>{error}</span>
        </div>
      )}
    </Modal>
  );
}
