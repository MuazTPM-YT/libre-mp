import { useCallback, useEffect, useId, useRef, useState, type ReactNode } from 'react';
import { X } from 'lucide-react';

/**
 * The dialog shell every modal uses.
 *
 * It owns the things that are easy to forget and obvious when missing: Escape
 * closes, focus moves in and comes back out, Tab stays inside, the dialog is
 * announced as a dialog, and it fades out instead of vanishing mid-frame.
 */

/** Must match the exit transition in index.css. */
const EXIT_MS = 140;

const FOCUSABLE =
  'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

interface Props {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  icon?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
  /** Extra class for the panel, e.g. a wider camera modal. */
  className?: string;
  /** While false the dialog cannot be dismissed (a request is in flight). */
  closable?: boolean;
}

export function Modal({
  isOpen,
  onClose,
  title,
  icon,
  children,
  footer,
  className = '',
  closable = true,
}: Props) {
  const [mounted, setMounted] = useState(false);
  const [open, setOpen] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const returnFocusRef = useRef<HTMLElement | null>(null);
  const titleId = useId();

  // Mount first, flip to "open" a frame later, so the entry transition runs.
  // Two frames: one for React to insert the node, one for the browser to take
  // its starting style. Closing keeps the node alive until the exit finishes.
  useEffect(() => {
    if (isOpen) {
      returnFocusRef.current = document.activeElement as HTMLElement | null;
      setMounted(true);
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
    const t = window.setTimeout(() => setMounted(false), EXIT_MS);
    return () => window.clearTimeout(t);
  }, [isOpen]);

  // Focus goes into the dialog, and back to the trigger when it closes.
  useEffect(() => {
    if (!mounted) return;
    const panel = panelRef.current;
    if (panel && !panel.contains(document.activeElement)) {
      (panel.querySelector<HTMLElement>(FOCUSABLE) ?? panel).focus();
    }
    return () => returnFocusRef.current?.focus?.();
  }, [mounted]);

  const close = useCallback(() => {
    if (closable) onClose();
  }, [closable, onClose]);

  // Escape closes; Tab cycles inside the dialog instead of escaping behind it.
  useEffect(() => {
    if (!mounted) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (closable) {
          e.preventDefault();
          onClose();
        }
        return;
      }
      if (e.key !== 'Tab') return;
      const panel = panelRef.current;
      if (!panel) return;
      const items = Array.from(panel.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
        (el) => el.offsetParent !== null
      );
      if (items.length === 0) {
        e.preventDefault();
        return;
      }
      const first = items[0];
      const last = items[items.length - 1];
      const inside = panel.contains(document.activeElement);
      if (e.shiftKey && (document.activeElement === first || !inside)) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && (document.activeElement === last || !inside)) {
        e.preventDefault();
        first.focus();
      }
    };
    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
  }, [mounted, closable, onClose]);

  if (!mounted) return null;

  return (
    <div
      className="lm-modal-overlay"
      data-state={open ? 'open' : 'closed'}
      // mousedown, not click: dragging a text selection from inside the dialog
      // and releasing on the backdrop must not close it.
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) close();
      }}
    >
      <div
        ref={panelRef}
        className={`lm-modal ${className}`.trim()}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
      >
        <div className="lm-modal-head">
          <div className="lm-modal-title" id={titleId}>
            {icon}
            <span>{title}</span>
          </div>
          <button className="lm-iconbtn" onClick={close} disabled={!closable} aria-label="Close">
            <X size={18} />
          </button>
        </div>
        <div className="lm-modal-body">{children}</div>
        {footer && <div className="lm-modal-foot">{footer}</div>}
      </div>
    </div>
  );
}
