import { useCallback, useEffect, useRef, useState } from 'react';
import { CircleAlert, CircleCheck, X } from 'lucide-react';

export interface ToastData {
  message: string;
  type: 'info' | 'error';
}

// must match toast exit transition in index.css
const EXIT_MS = 150;
// how long toast stays
const LIFETIME_MS = 4000;

// one toast, bottom centre. leaves by itself; hover pauses timer
export function Toast({ toast, onDismiss }: { toast: ToastData | null; onDismiss: () => void }) {
  const [shown, setShown] = useState<ToastData | null>(null);
  const [open, setOpen] = useState(false);
  const timer = useRef<number | undefined>(undefined);

  const stopTimer = useCallback(() => window.clearTimeout(timer.current), []);
  const startTimer = useCallback(() => {
    stopTimer();
    timer.current = window.setTimeout(onDismiss, LIFETIME_MS);
  }, [onDismiss, stopTimer]);

  // mount, open a frame later; on dismiss stay till exit ends
  useEffect(() => {
    if (toast) {
      setShown(toast);
      let inner = 0;
      const outer = requestAnimationFrame(() => {
        inner = requestAnimationFrame(() => setOpen(true));
      });
      return () => {
        cancelAnimationFrame(outer);
        cancelAnimationFrame(inner);
      };
    }
    setOpen(false);
    const t = window.setTimeout(() => setShown(null), EXIT_MS);
    return () => window.clearTimeout(t);
  }, [toast]);

  useEffect(() => {
    if (!toast) return;
    startTimer();
    return stopTimer;
  }, [toast, startTimer, stopTimer]);

  if (!shown) return null;
  const isError = shown.type === 'error';

  return (
    <div
      className={`lm-toast ${isError ? 'error' : ''}`.trim()}
      data-state={open ? 'open' : 'closed'}
      role={isError ? 'alert' : 'status'}
      onMouseEnter={stopTimer}
      onMouseLeave={startTimer}
    >
      {isError ? <CircleAlert size={16} /> : <CircleCheck size={16} color="var(--green)" />}
      <span>{shown.message}</span>
      <button type="button" className="lm-icon-btn" onClick={onDismiss} aria-label="Dismiss notification">
        <X size={14} />
      </button>
    </div>
  );
}
