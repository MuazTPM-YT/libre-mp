/* ═══════════════════════════════════════════════════════════
   Libre MP — SVG Icon Module
   Minimal line-style icons, 20x20 default
   ═══════════════════════════════════════════════════════════ */

const svgAttrs = `xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"`;

export const icons = {
  search: `<svg ${svgAttrs}><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>`,

  filter: `<svg ${svgAttrs}><polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"/></svg>`,

  settings: `<svg ${svgAttrs}><circle cx="12" cy="12" r="3"/><path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/></svg>`,

  close: `<svg ${svgAttrs}><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>`,

  projector: `<svg ${svgAttrs}><rect x="2" y="7" width="20" height="12" rx="2"/><circle cx="12" cy="13" r="3"/><line x1="6" y1="7" x2="6" y2="5"/><line x1="18" y1="7" x2="18" y2="5"/></svg>`,

  refresh: `<svg ${svgAttrs}><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/></svg>`,

  clock: `<svg ${svgAttrs}><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>`,

  help: `<svg ${svgAttrs}><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>`,

  display: `<svg ${svgAttrs}><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>`,

  brightness: `<svg ${svgAttrs}><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>`,

  resolution: `<svg ${svgAttrs}><polyline points="15 3 21 3 21 9"/><polyline points="9 21 3 21 3 15"/><line x1="21" y1="3" x2="14" y2="10"/><line x1="3" y1="21" x2="10" y2="14"/></svg>`,

  bandwidth: `<svg ${svgAttrs}><path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/></svg>`,

  audio: `<svg ${svgAttrs}><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/><path d="M19.07 4.93a10 10 0 0 1 0 14.14"/><path d="M15.54 8.46a5 5 0 0 1 0 7.07"/></svg>`,

  reconnect: `<svg ${svgAttrs}><polyline points="17 1 21 5 17 9"/><path d="M3 11V9a4 4 0 0 1 4-4h14"/><polyline points="7 23 3 19 7 15"/><path d="M21 13v2a4 4 0 0 1-4 4H3"/></svg>`,

  link: `<svg ${svgAttrs}><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/></svg>`,

  wifi: `<svg ${svgAttrs}><path d="M5 12.55a11 11 0 0 1 14.08 0"/><path d="M1.42 9a16 16 0 0 1 21.16 0"/><path d="M8.53 16.11a6 6 0 0 1 6.95 0"/><line x1="12" y1="20" x2="12.01" y2="20"/></svg>`,

  disconnect: `<svg ${svgAttrs}><path d="M16 16v1a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2h2m5.66 0H14a2 2 0 0 1 2 2v3.34"/><line x1="1" y1="1" x2="23" y2="23"/></svg>`,
};

/**
 * Inject icon SVG into a DOM element by id.
 * @param {string} elementId
 * @param {string} iconName
 */
export function injectIcon(elementId, iconName) {
  const el = document.getElementById(elementId);
  if (el && icons[iconName]) {
    el.innerHTML = icons[iconName];
  }
}

/**
 * Return icon SVG string.
 * @param {string} name
 * @param {number} size
 */
export function icon(name, size = 20) {
  const svg = icons[name] || '';
  return svg.replace(/width="\d+"/, `width="${size}"`).replace(/height="\d+"/, `height="${size}"`);
}
