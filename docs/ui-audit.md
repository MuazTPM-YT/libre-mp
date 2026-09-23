# LibreMP UI/UX Audit

Date: 2026-09-23. Scope: the whole desktop UI (`frontend/src`). Goal: an Apple-style app. That means calm, consistent themes, clear hierarchy, and restrained motion. It does not mean Liquid Glass or showy animation.

Method: screenshots of every state (dark, light, empty, casting, error, each modal) from a mocked backend. The review used Apple's WWDC design guidance and Emil Kowalski's design-engineering rules.

## 1. Big problems

1. **No single place tells you what is happening.** Status is spread over four places: a "IDLE / CONNECTED / CASTING" chip in the toolbar, a banner under the toolbar, a floating cast bar at the bottom, and toasts. "Connected" means "on the projector's Wi-Fi", which users cannot tell apart from casting. The banner and cast bar also cover content.
2. **The UI said "Casting" before anything was casting.** The old flow showed "Casting to X" the moment the helper process started. That was before the handshake, and before screen sharing was allowed. A failure then showed up as a vague message 3 seconds later.
3. **Too many primary buttons.** Every projector row and every saved row had a filled teal button. Three large hero cards all looked equally important. Nothing told a first-time user where to start.
4. **Visual language is "terminal", not Apple.** The logo, section labels, IPs, pills and status chip use monospace, UPPERCASE and wide letter spacing. It uses two competing accent colours (teal and amber), plus red and orange for signal bars.
5. **The Projector Keyword could not be entered in the app.** A refused login ended in an error, with no way to type the 4-digit keyword.
6. **The window was fixed at 1000×800 and could not be resized.** It could not grow or shrink, and it was cramped on small laptop screens.

## 2. Findings and fixes

| Before | After | Why |
| --- | --- | --- |
| Status spread over a toolbar chip, a banner, a bottom cast bar and toasts | One **status card** at the top: *Connect* (idle), *Connecting…* (with the real phase), *Casting to X* (with Stop), or an error with a clear next step | One place to look. Apple: "Where am I? What is happening? How do I get out?" |
| "Casting to X" shown as soon as the process started | The UI follows real backend events: *Waiting for screen sharing* → *Connecting* → *Casting* → *Reconnecting* | Feedback must be true. Status, completion and error are separate states |
| Error banner vanished after 8 s | Errors stay in the status card until you dismiss them or start again | A person cannot always read an error in 8 s |
| Refused login: dead end | The error offers **Enter Keyword…**, and a 4-digit sheet retries | No dead ends. Every error has a next action |
| Three equal hero cards with paragraphs of text | One primary button **Scan QR Code**, with secondary **Choose Photo…** and **Enter Manually…** | One obvious first step. Simplicity is not the same as minimalism |
| Every row a separate bordered card, with a coloured left border for projectors | **Grouped inset lists**: one rounded container, hairline separators, icon tiles | Apple's list idiom. Less noise, better scanning |
| Home Wi-Fi networks mixed with projectors | **Projectors** first. **Other Networks** is collapsed by default | The app is for projectors. Other networks are the rare case |
| Filled teal "Connect" on every row | Quiet gray push buttons in rows. Only the hero action is blue | One primary action per screen |
| Monospace, UPPERCASE, `letter-spacing: .2em` labels | System font (SF Pro / Segoe / Cantarell via `system-ui`), sentence case, size-specific tracking | Familiarity. The platform font already has optical sizing |
| Teal and amber accents, red/orange/teal signal bars | One accent (Apple blue), semantic green only for "live", red only for destructive | Colour means something again |
| Signal shown as coloured bars **and** a "71%" pill | One monochrome Wi-Fi glyph (lucide `Wifi*`), plus a lock for secured networks | Say it once |
| Theme toggle button in the toolbar, not following the system | **Appearance: System / Light / Dark** in Settings, System by default | Apple apps follow the system |
| Radial colour gradients on the window background | Flat, neutral window background | Calm. Content first |
| Toolbar `backdrop-filter: blur`, modal scrim blur | Solid surfaces, plain dim scrim | You asked for no glass. Blur is also costly in WebKitGTK |
| Rows and hero cards animate in on every paint (staggered keyframes) | No entrance animation on lists | Lists are seen dozens of times a day. Emil: frequent actions do not animate |
| Hero cards lift 3 px with a big shadow on hover | Rows and buttons change background on hover only (pointer devices) | Desktop Apple lists do not jump |
| Infinite "ping" pulse on the casting lamp | Static green dot. The spinner is the only thing that repeats, and only while work is running | Endless motion distracts, and it runs next to a 24 fps encoder |
| Modals: icon + title + × in a header bar | Sheets: title, text, buttons bottom-right (Cancel, then default). Esc cancels, Enter confirms | macOS sheet idiom |
| Scan: Capture, then press Scan | **Take Photo** reads the QR at once. Retake appears only on failure | One step less |
| Placeholders used as labels in Manual entry | Real labels above the fields. Placeholders show examples | Placeholders vanish when you type |
| Teal focus outline | Accent focus ring (3 px, soft) on `:focus-visible` only | Visible for keyboard users, invisible for mouse users |
| Two Connect clicks could start two joins at once | While a connection is running, all other Connect actions are disabled | Stops races in the Wi-Fi code |
| Wi-Fi and LAN scans ran every 12 s during a cast | Scans pause while connecting and casting | Less radio churn during a stream |
| Window fixed at 1000×800 | Resizable, min 720×560, content column max 760 px | Flexibility |
| Page title "Tauri + React + Typescript" | "LibreMP" | Craft |
| No `prefers-contrast` / `prefers-reduced-transparency` handling | Stronger separators and borders under `prefers-contrast: more` | Accessibility |

## 3. Motion rules (kept deliberately small)

| Element | Motion | Value |
| --- | --- | --- |
| Button press | `scale(0.97)` | 100 ms, ease-out |
| Sheet open / close | opacity + `scale(0.97 → 1)`, centred | 200 ms in / 150 ms out, `cubic-bezier(0.23, 1, 0.32, 1)` |
| Scrim | opacity | 200 ms / 150 ms |
| Toast | opacity + 8 px rise | 200 ms / 150 ms |
| Switch knob | `translateX` | 180 ms, ease-out |
| Disclosure chevron | `rotate(90deg)` | 150 ms, ease-out |
| Hover colour | background-color | 120 ms, ease |
| Spinner | rotate | 0.8 s linear, only while working |

Only `transform` and `opacity` animate. Reduced motion removes every scale and translate, and keeps short fades.

## 4. Colour tokens

Both themes use the same token names. Values are Apple system colours, adjusted where needed for WCAG AA text contrast.

| Token | Light | Dark |
| --- | --- | --- |
| `--bg` window | `#f5f5f7` | `#161618` |
| `--surface` group / card | `#ffffff` | `#232326` |
| `--raised` sheet / toast | `#ffffff` | `#2c2c2f` |
| `--label` | `#1d1d1f` | `#f5f5f7` |
| `--secondary` | `#6e6e73` | `#a1a1a6` |
| `--tertiary` | `#aeaeb2` | `#6e6e73` |
| `--separator` | `rgb(60 60 67 / .16)` | `rgb(255 255 255 / .09)` |
| `--fill` (quiet buttons) | `rgb(120 120 128 / .12)` | `rgb(120 120 128 / .24)` |
| `--accent` (fills, white text 4.6:1) | `#0071e3` | `#0071e3` |
| `--accent-text` (links, icons) | `#0066cc` | `#2997ff` |
| `--green` (live) | `#248a3d` | `#30d158` |
| `--red` (destructive) | `#d70015` | `#ff453a` |

## 5. Not changed, on purpose

- **Toasts** stay, but only for things that happen out of view: "Casting stopped" and "saved without password". The status card covers the rest.
- **Camera through Rust**, not `getUserMedia`. The webview camera crashes WebKitGTK on Linux.
- **No animation library.** CSS transitions cover every case here. Nothing is gesture-driven, so there is no need for springs.
