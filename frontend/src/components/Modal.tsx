import { useEffect, useId, useRef, useState, type ReactNode } from 'react';

// must match sheet exit transition in index.css
const EXIT_MS = 150;

const FOCUSABLE =
  'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

interface Props {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  text?: ReactNode;
  children?: ReactNode;
  footer: ReactNode;
  wide?: boolean;
  // false while work runs: esc + backdrop do nothing
  closable?: boolean;
  // enter key / submit button
  onSubmit?: () => void;
}

// centred sheet: esc closes, focus in and back out, tab stays inside, fade + small scale
export function Modal({ isOpen, onClose, title, text, children, footer, wide, closable = true, onSubmit }: Props) {
  const [mounted, setMounted] = useState(false);
  const [open, setOpen] = useState(false);
  const panelRef = useRef<HTMLFormElement>(null);
  const returnFocusRef = useRef<HTMLElement | null>(null);
  const titleId = useId();

  // mount, then open two frames later so entry transition runs; keep node till exit ends
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

  // focus goes in on open, back to trigger on close
  useEffect(() => {
    if (!mounted) return;
    const panel = panelRef.current;
    if (panel && !panel.contains(document.activeElement)) {
      const auto = panel.querySelector<HTMLElement>('[autofocus]');
      (auto ?? panel.querySelector<HTMLElement>('.primary') ?? panel.querySelector<HTMLElement>(FOCUSABLE) ?? panel).focus();
    }
    return () => returnFocusRef.current?.focus?.();
  }, [mounted]);

  // esc closes; tab cycles inside
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
      const panel = panelRef.current;
      if (e.key !== 'Tab' || !panel) return;
      const items = Array.from(panel.querySelectorAll<HTMLElement>(FOCUSABLE)).filter((el) => el.offsetParent !== null);
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
      className="lm-scrim"
      data-state={open ? 'open' : 'closed'}
      // mousedown, so a text drag ending on backdrop does not close
      onMouseDown={(e) => {
        if (e.target === e.currentTarget && closable) onClose();
      }}
    >
      <form
        ref={panelRef}
        className={`lm-sheet ${wide ? 'wide' : ''}`.trim()}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        onSubmit={(e) => {
          e.preventDefault();
          onSubmit?.();
        }}
      >
        <h2 className="lm-sheet-title" id={titleId}>
          {title}
        </h2>
        {text && <p className="lm-sheet-text">{text}</p>}
        {children}
        <div className="lm-sheet-foot">{footer}</div>
      </form>
    </div>
  );
}
