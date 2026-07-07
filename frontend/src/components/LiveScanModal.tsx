import { useEffect, useState, useCallback } from 'react';
import { X, Camera, RotateCcw, AlertCircle } from 'lucide-react';
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
 * Live QR scan using the native camera (Rust `scan_qr_camera` command, V4L2 /
 * AVFoundation / Media Foundation). There is no in-window video preview because
 * the webview's getUserMedia path segfaults WebKitGTK — the camera is driven
 * entirely in Rust, which decodes frames and returns the first Epson QR.
 */
export function LiveScanModal({ isOpen, onClose, onDecoded }: Props) {
  const [error, setError] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);

  const runScan = useCallback(() => {
    setError(null);
    setScanning(true);
    invoke<QrResult>('scan_qr_camera')
      .then((r) => onDecoded(r))
      .catch((e) => setError(typeof e === 'string' ? e : 'Camera scan failed.'))
      .finally(() => setScanning(false));
  }, [onDecoded]);

  useEffect(() => {
    if (!isOpen) return;
    runScan();
    return () => {
      invoke('cancel_camera_scan').catch(() => {});
    };
  }, [isOpen, runScan]);

  const close = () => {
    invoke('cancel_camera_scan').catch(() => {});
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="lm-modal-overlay" onClick={close}>
      <div className="lm-modal" onClick={(e) => e.stopPropagation()}>
        <div className="lm-modal-head">
          <div className="lm-modal-title">
            <Camera size={16} />
            <span>Live camera scan</span>
          </div>
          <button className="lm-iconbtn" onClick={close} aria-label="Close">
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
            <div className="lm-scan-state">
              <span className="lm-scan-pulse">
                <Camera size={28} />
              </span>
              <p className="lm-scan-copy">
                Hold the projector’s QR code in front of your camera. LibreMP connects the
                moment it reads it.
              </p>
              <span className="lm-scan-hint">{scanning ? 'Scanning…' : 'Starting camera…'}</span>
            </div>
          )}
        </div>

        <div className="lm-modal-foot" style={{ justifyContent: 'center' }}>
          <button className="lm-btn ghost" onClick={close}>
            Cancel
          </button>
          {error && (
            <button className="lm-btn signal" onClick={runScan}>
              Try again <RotateCcw size={14} />
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
