import { useEffect, useState } from 'react';
import { X, Camera, RotateCcw, AlertCircle, ScanLine, ArrowRight } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

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

/**
 * Live camera QR: preview → Capture → Scan. The camera is driven natively in
 * Rust (a worker thread); frames arrive as JPEG data: URLs, so there's a real
 * preview without the webview's getUserMedia (which segfaults WebKitGTK).
 */
export function LiveScanModal({ isOpen, onClose, onDecoded }: Props) {
  const [preview, setPreview] = useState<string | null>(null);
  const [captured, setCaptured] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // Live preview loop — runs while open and not frozen on a captured still.
  useEffect(() => {
    if (!isOpen || captured || error) return;
    let active = true;
    const loop = async () => {
      while (active) {
        try {
          const url = await invoke<string>('camera_preview_frame');
          if (!active) return;
          setPreview(url);
        } catch (e) {
          if (active) setError(typeof e === 'string' ? e : 'Camera error.');
          return;
        }
      }
    };
    loop();
    return () => {
      active = false;
    };
  }, [isOpen, captured, error]);

  // Release the camera and reset when the modal closes.
  useEffect(() => {
    if (isOpen) return;
    invoke('camera_stop').catch(() => {});
    setPreview(null);
    setCaptured(null);
    setError(null);
    setBusy(false);
  }, [isOpen]);

  const capture = async () => {
    setBusy(true);
    try {
      setCaptured(await invoke<string>('camera_capture'));
    } catch (e) {
      setError(typeof e === 'string' ? e : 'Could not take the photo.');
    } finally {
      setBusy(false);
    }
  };

  const scan = async () => {
    setBusy(true);
    try {
      onDecoded(await invoke<QrResult>('camera_scan'));
    } catch (e) {
      setError(typeof e === 'string' ? e : 'Scan failed.');
    } finally {
      setBusy(false);
    }
  };

  const retake = () => {
    setError(null);
    setCaptured(null);
  };

  if (!isOpen) return null;

  const shown = captured || preview;

  return (
    <div className="lm-modal-overlay" onClick={onClose}>
      <div className="lm-modal lm-scan-modal" onClick={(e) => e.stopPropagation()}>
        <div className="lm-modal-head">
          <div className="lm-modal-title">
            <Camera size={16} />
            <span>Live camera scan</span>
          </div>
          <button className="lm-iconbtn" onClick={onClose} aria-label="Close">
            <X size={18} />
          </button>
        </div>

        <div className="lm-modal-body">
          {error ? (
            <div className="lm-scan-state">
              <AlertCircle size={30} className="lm-scan-glyph err" />
              <p className="lm-scan-copy">{error}</p>
            </div>
          ) : (
            <>
              <div className="lm-cam-view">
                {shown ? (
                  <img className="lm-cam-img" src={shown} alt="camera preview" />
                ) : (
                  <span className="lm-scan-hint">Starting camera…</span>
                )}
              </div>
              <p className="lm-scan-copy">
                {captured
                  ? 'Photo taken. Press Scan to read the QR, or retake it.'
                  : 'Line up the projector’s QR in the frame, then press Capture.'}
              </p>
            </>
          )}
        </div>

        <div className="lm-modal-foot">
          <button className="lm-btn ghost" onClick={onClose}>
            Cancel
          </button>
          {error ? (
            <button className="lm-btn signal" onClick={retake}>
              Try again <RotateCcw size={14} />
            </button>
          ) : captured ? (
            <>
              <button className="lm-btn ghost" onClick={retake} disabled={busy}>
                Retake
              </button>
              <button className="lm-btn signal" onClick={scan} disabled={busy}>
                Scan {busy ? <RotateCcw size={14} className="lm-spin" /> : <ArrowRight size={14} />}
              </button>
            </>
          ) : (
            <button className="lm-btn signal" onClick={capture} disabled={busy || !preview}>
              Capture <ScanLine size={14} />
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
