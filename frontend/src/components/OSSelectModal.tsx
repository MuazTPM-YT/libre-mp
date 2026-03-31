import { useState, useEffect } from 'react';
import { Monitor, X, ArrowRight } from 'lucide-react';

interface Props {
    isOpen: boolean;
    onCancel: () => void;
    onSelect: (osMode: number) => void;
}

const OS_OPTIONS = [
    { id: 1, name: 'Windows', detail: 'Native DXGI', icon: '🪟' },
    { id: 2, name: 'MacOS', detail: 'Native CoreGraphics', icon: '🍎' },
    { id: 3, name: 'Ubuntu / X11', detail: 'Native XShm', icon: '🐧' },
    { id: 4, name: 'Arch Linux / Wayland', detail: 'grim wlroots (Hyprland)', icon: '🏗️' },
];

function detectOS(): number {
    const ua = navigator.userAgent.toLowerCase();
    if (ua.includes('win')) return 1;
    if (ua.includes('mac')) return 2;
    // Default to 4 for Linux (Wayland/Hyprland is what the user runs)
    return 4;
}

/** Modal for selecting the target OS mode before streaming */
export function OSSelectModal({ isOpen, onCancel, onSelect }: Props) {
    const [selected, setSelected] = useState<number>(detectOS);

    useEffect(() => {
        if (isOpen) setSelected(detectOS());
    }, [isOpen]);

    if (!isOpen) return null;

    return (
        <div className="modal-overlay" onClick={onCancel}>
            <div className="modal-content connection-modal" onClick={e => e.stopPropagation()}>
                <div className="modal-header">
                    <div className="modal-title-row">
                        <Monitor size={16} />
                        <h2>Display Environment</h2>
                    </div>
                    <button className="icon-btn" onClick={onCancel}>
                        <X size={18} />
                    </button>
                </div>

                <div className="modal-body-pad">
                    <p className="modal-instruction">
                        Select your <strong>operating system</strong> for screen capture
                    </p>

                    <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', marginTop: '12px' }}>
                        {OS_OPTIONS.map(opt => (
                            <button
                                key={opt.id}
                                onClick={() => setSelected(opt.id)}
                                style={{
                                    display: 'flex',
                                    alignItems: 'center',
                                    gap: '12px',
                                    padding: '12px 16px',
                                    border: selected === opt.id ? '2px solid var(--accent)' : '2px solid var(--border)',
                                    borderRadius: '10px',
                                    background: selected === opt.id ? 'var(--accent-bg, rgba(99, 102, 241, 0.08))' : 'var(--card-bg)',
                                    cursor: 'pointer',
                                    transition: 'all 0.15s ease',
                                    textAlign: 'left',
                                    color: 'var(--text)',
                                }}
                            >
                                <span style={{ fontSize: '20px' }}>{opt.icon}</span>
                                <div style={{ flex: 1 }}>
                                    <div style={{ fontWeight: 600, fontSize: '14px' }}>{opt.name}</div>
                                    <div style={{ fontSize: '12px', opacity: 0.6 }}>{opt.detail}</div>
                                </div>
                                {selected === opt.id && (
                                    <div style={{
                                        width: '8px', height: '8px', borderRadius: '50%',
                                        backgroundColor: 'var(--accent)',
                                    }} />
                                )}
                            </button>
                        ))}
                    </div>
                </div>

                <div className="modal-footer">
                    <button className="action-btn secondary" onClick={onCancel}>
                        Cancel
                    </button>
                    <button
                        className="action-btn primary btn-min-width"
                        onClick={() => onSelect(selected)}
                    >
                        Start Streaming <ArrowRight size={14} />
                    </button>
                </div>
            </div>
        </div>
    );
}
