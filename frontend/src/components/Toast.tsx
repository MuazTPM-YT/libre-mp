import { useCallback, useEffect, useRef, useState } from 'react';
import { X } from 'lucide-react';

export interface ToastData {
  message: string;
  type: 'success' | 'info' | 'error';
}

/** Must match the exit transition in index.css. */
const EXIT_MS = 160;
/** How long a toast stays before it dismisses itself. */
const LIFETIME_MS = 4000;

/**
 * A single toast. It goes away on its own — a notice that needs a click to
 * dismiss is a notice that piles up. Hovering pauses the timer, so a message
 * cannot disappear while it is being read.
 */
export function Toast({ toast, onDismiss }: { toast: ToastData | null; onDismiss: () => void }) {
  const [shown, setShown] = useState<ToastData | null>(null);
  const [open, setOpen] = useState(false);
  const timer = useRef<number | undefined>(undefined);

  const stopTimer = useCallback(() => window.clearTimeout(timer.current), []);
  const startTimer = useCallback(() => {
    stopTimer();
    timer.current = window.setTimeout(onDismiss, LIFETIME_MS);
  }, [onDismiss, stopTimer]);

  // Mount, then open a frame later so the entry transition runs; on dismiss,
  // stay mounted until the exit transition has played.
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

  return (
    <div
      className={`lm-toast ${shown.type === 'error' ? 'err' : ''}`.trim()}
      data-state={open ? 'open' : 'closed'}
      role={shown.type === 'error' ? 'alert' : 'status'}
      onMouseEnter={stopTimer}
      onMouseLeave={startTimer}
    >
      <span>{shown.message}</span>
      <button
        className="lm-iconbtn lm-toast-close"
        onClick={onDismiss}
        aria-label="Dismiss notification"
      >
        <X size={14} />
      </button>
    </div>
  );
}
