import { useEffect, useRef, useState } from 'react';
import { X, Camera, AlertCircle } from 'lucide-react';
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
 * Live camera QR scanner. Grabs frames on an interval, hands each (as a small
 * JPEG) to the Rust `decode_projector_qr` command, and reports the first
 * successful Epson decode. Falls back gracefully when no camera is available.
 */
export function QrScannerModal({ isOpen, onClose, onDecoded }: Props) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const busyRef = useRef(false);
  const doneRef = useRef(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen) return;
    doneRef.current = false;
    setError(null);
    let cancelled = false;
    let timer: number | undefined;

    const grab = async () => {
      const v = videoRef.current;
      const c = canvasRef.current;
      if (cancelled || doneRef.current || busyRef.current || !v || !c || v.readyState < 2) return;
      const w = v.videoWidth;
      const h = v.videoHeight;
      if (!w || !h) return;
      busyRef.current = true;
      try {
        const scale = Math.min(1, 900 / Math.max(w, h));
        c.width = Math.round(w * scale);
        c.height = Math.round(h * scale);
        const ctx = c.getContext('2d');
        if (!ctx) return;
        ctx.drawImage(v, 0, 0, c.width, c.height);
        const blob: Blob | null = await new Promise((res) => c.toBlob(res, 'image/jpeg', 0.85));
        if (!blob) return;
        const bytes = Array.from(new Uint8Array(await blob.arrayBuffer()));
        const result = await invoke<QrResult>('decode_projector_qr', { imageBytes: bytes });
        if (!cancelled && !doneRef.current) {
          doneRef.current = true;
          onDecoded(result);
        }
      } catch {
        // No projector QR in this frame — keep scanning.
      } finally {
        busyRef.current = false;
      }
    };

    (async () => {
      try {
        const stream = await navigator.mediaDevices.getUserMedia({
          video: { facingMode: 'environment' },
        });
        if (cancelled) {
          stream.getTracks().forEach((t) => t.stop());
          return;
        }
        streamRef.current = stream;
        if (videoRef.current) {
          videoRef.current.srcObject = stream;
          await videoRef.current.play().catch(() => {});
        }
        timer = window.setInterval(grab, 650);
      } catch {
        setError('No camera available. Close this and use “Upload QR photo” instead.');
      }
    })();

    return () => {
      cancelled = true;
      if (timer) clearInterval(timer);
      streamRef.current?.getTracks().forEach((t) => t.stop());
      streamRef.current = null;
    };
  }, [isOpen, onDecoded]);

  if (!isOpen) return null;

  return (
    <div className="lm-modal-overlay" onClick={onClose}>
      <div className="lm-modal lm-scan-modal" onClick={(e) => e.stopPropagation()}>
        <div className="lm-modal-head">
          <div className="lm-modal-title">
            <Camera size={16} />
            <span>Scan projector QR</span>
          </div>
          <button className="lm-iconbtn" onClick={onClose} aria-label="Close">
            <X size={18} />
          </button>
        </div>
        <div className="lm-modal-body">
          {error ? (
            <div className="lm-scan-error">
              <AlertCircle size={18} />
              <p>{error}</p>
            </div>
          ) : (
            <>
              <div className="lm-scan-frame">
                <video ref={videoRef} className="lm-scan-video" playsInline muted />
                <div className="lm-scan-reticle" />
              </div>
              <p className="lm-scan-status">
                Point the camera at the projector’s QR code — it connects automatically.
              </p>
            </>
          )}
          <canvas ref={canvasRef} style={{ display: 'none' }} />
        </div>
      </div>
    </div>
  );
}
