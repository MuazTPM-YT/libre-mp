# LibreMP Cross-Platform Overhaul — Design Spec

**Date:** 2026-07-07
**Status:** Approved (design), pending implementation plan
**Author:** Muaz + Claude

## 1. Problem & Goals

LibreMP is a FOSS wireless-display client that casts a computer screen to networked
projectors (currently Epson EasyMP/iProjection) without any proprietary SDK. It works
today against one specific projector ("RESEARCHLAB") on Arch/Hyprland, but:

1. Connecting requires the user to manually join the projector's Wi-Fi and type a
   password that is really the projector's wired MAC address.
2. The handshake is hardcoded from a single packet capture (baked-in IP `192.168.88.2`,
   magic byte blobs) and does not generalize across Epson models.
3. Screen capture uses a manual OS picker (1–4) and `grim`, which only works on wlroots
   compositors — it fails on KDE/GNOME Wayland.
4. The repo is 11 GB because `Rust/target/` (49,156 files) is committed to git and the
   `.gitignore` is UTF-16-encoded, so its rules never matched.

### Goals (from user decisions)

- **Protocol:** Robust across Epson EasyMP/iProjection models (not multi-vendor).
- **Connection:** LAN/Infrastructure-mode discovery first (no Wi-Fi switching when the
  projector is reachable on the existing network, wired or wireless); save credentials
  and auto-rejoin known Direct-mode projectors.
- **Capture:** One auto-detecting backend across Windows, macOS, and all Linux
  desktops (X11 + Wayland/KDE/GNOME/wlroots). Remove the manual OS picker.
- **Size:** Stop tracking build artifacts, fix `.gitignore`, and purge `Rust/target/`
  from all git history.

### Non-Goals

- Non-Epson vendors (BenQ, Panasonic, Miracast) — different protocols, out of scope.
- Bypassing WPA encryption — cryptographically impossible; not attempted.
- A UI rewrite — the React frontend is structurally sound; polish only.

## 2. Architecture (Option 1: shared core library)

Today there are two Rust crates plus a dead third implementation:

- `Rust/` — the **proven** CLI streamer (protocol + template + capture). Works.
- `frontend/src-tauri/` — the Tauri GUI, which **spawns** the CLI binary, and *also*
  contains an **unused** second protocol implementation (`src/protocol/`).

### Target structure

```
libremp/
├── core/                 # NEW: libremp-core library crate
│   └── src/
│       ├── lib.rs
│       ├── discovery.rs   # LAN + Direct-mode projector discovery
│       ├── protocol.rs    # Epson EasyMP handshake + streaming (from Rust/src/protocol.rs)
│       ├── capture/       # auto-detecting capture backends
│       │   ├── mod.rs     # runtime backend selection
│       │   ├── windows.rs
│       │   ├── macos.rs
│       │   ├── x11.rs
│       │   └── wayland.rs # xdg-desktop-portal + PipeWire
│       ├── wifi.rs        # OS-native wifi scan/connect/restore
│       ├── config.rs      # NEW: saved projectors (~/.config/libremp/)
│       └── template.rs / hex.rs
├── cli/                  # thin CLI binary depending on core
│   └── src/main.rs
└── frontend/
    └── src-tauri/        # Tauri GUI depending on core (dead protocol/ deleted)
```

- All real logic lives in **`libremp-core`**, unit-testable in isolation.
- The CLI and the Tauri app are thin front-ends over the same core.
- The Tauri app calls core **in-process** (no more spawning a sibling binary and
  guessing its path via `find_streamer_binary`'s four fallback candidates).
- `frontend/src-tauri/src/protocol/` (client.rs, streamer.rs, payloads.rs, config.rs)
  is **deleted**.

> Note: the existing `Rust/` directory is refactored into `core/` + `cli/`. A Cargo
> workspace at the repo root ties `core`, `cli`, and `frontend/src-tauri` together.

## 3. Component Design

### 3.1 Discovery-first connection (`discovery.rs`)

Flow on "connect":

1. **LAN probe first.** Broadcast Epson discovery probes (ESC/VP.net on 3629, EEMP on
   3620) across the *current* network's broadcast address(es), computed from the active
   interface rather than hardcoded `192.168.88.255`/`192.168.1.255`. Any responder is a
   candidate projector reachable **without touching Wi-Fi** — this covers wired LAN and
   infrastructure-mode Wi-Fi uniformly (TCP is transport-agnostic).
2. **If found on current LAN → connect directly.** No Wi-Fi change.
3. **If not found → Direct-mode fallback.** Offer projector-looking SSIDs
   (`DIRECT-*EPSON*`, etc.), join via OS-native wifi, then discover on that subnet.
4. **Save & rejoin.** On success, persist `{ssid, psk, last_ip, auth_token}` to
   `~/.config/libremp/projectors.json` (0600). On next launch, known projectors are
   auto-probed and, if a saved Direct SSID is visible, auto-joined without prompting.

Interfaces:
- `discover(timeout) -> Vec<Projector>` — LAN broadcast discovery.
- `Projector { name, ip, model, reachable_now: bool }`.
- `SavedProjectors::load()/save()/find(ssid)`.

### 3.2 Epson-generic protocol (`protocol.rs`)

Refactor the proven handshake to remove model-specific assumptions:

- **No baked-in IPs.** `response_0x0108` currently hardcodes `192.168.88.2` and rewrites
  it; instead build payloads from the detected `my_ip`/`proj_ip` from the start.
- **Auth token as a real parameter.** Today `auth_payload` assumes `password == MAC`.
  Make the auth token explicit: derive it from (a) an explicit user-provided token,
  (b) the projector's advertised MAC from discovery, or (c) the Wi-Fi PSK as a
  last-resort compatibility path. Document which projectors need which.
- **Tolerant response parsing.** Don't require exactly 296 bytes; parse the status field
  and post-auth command stream defensively, log unknowns, continue when safe.
- **Keep** gateway auto-detection for projector IP (already works across subnets) and the
  template-based framing (`windows_perfect_stream.bin`) that is proven to render.

Preserve the existing keepalive / `drain_auth` / warmup behavior — it is battle-tested.

### 3.3 Universal capture (`capture/`)

`capture::Grabber::detect()` returns a backend chosen at runtime:

| Platform / session          | Backend                                   |
|-----------------------------|-------------------------------------------|
| Windows                     | GDI/DXGI (existing winapi path)           |
| macOS                       | CoreGraphics (via `scrap`)                |
| Linux + X11                 | XShm (via `scrap`)                        |
| Linux + Wayland (any DE)    | xdg-desktop-portal ScreenCast + PipeWire  |

- Session detection: `$XDG_SESSION_TYPE`, presence of `$WAYLAND_DISPLAY`.
- The Wayland path uses the **portal**, so it works on KDE, GNOME, and wlroots — unlike
  `grim`, which is dropped. The portal shows a one-time OS picker for the shared output
  (a security feature we cannot and should not bypass).
- Backend exposes a uniform `fn frame(&mut self) -> Option<Rgb frame at STREAM_WxH>`,
  so the streaming loop is backend-agnostic. The manual `--os` picker and the
  `OSSelectModal` are removed.
- Black-frame heuristic is retained as a diagnostic but no longer tied to `os_mode == 3`.

### 3.4 Saved-projector config (`config.rs`)

- Location: `~/.config/libremp/projectors.json` (XDG on Linux, platform equivalents via
  the `directories` crate). File mode `0600` since it stores PSKs.
- Schema: list of `{ name, ssid, psk, auth_token, last_ip, last_seen }`.

### 3.5 Repo size cleanup

1. Rewrite `.gitignore` as **UTF-8** with correct rules for all `target/`, `node_modules/`,
   `dist/`, saved-config, and OS cruft.
2. `git rm -r --cached Rust/target` (and any other tracked artifacts).
3. Purge from history: `git filter-repo --path Rust/target --invert-paths` (or
   `git-filter-branch`/BFG fallback). This rewrites history — a force-push and fresh
   clones are required. **Back up the repo first** (tag + bundle) before rewriting.
4. Verify `.git` shrinks from ~935 MB to single-digit MB.

### 3.6 UI/UX polish (React frontend)

- Remove `OSSelectModal` and its flow (capture auto-detects).
- Discovered LAN projectors render as first-class rows with a one-click "Cast" (no
  Wi-Fi step). Saved projectors show a "known" badge and auto-connect affordance.
- Keep the existing component structure, theming, and scan loop. No framework changes.

## 4. Error Handling

- Discovery timeout → clear "no projector found on this network" state with the Direct-Wi-Fi
  fallback offered explicitly.
- Auth failure → distinguish "wrong token/PSK" from "unreachable" from "unsupported model."
- Capture backend init failure (e.g., portal denied) → actionable message, not a black frame.
- Wi-Fi restore on exit remains best-effort (existing `wifi_restore`).

## 5. Testing Strategy

- **Unit (core):** payload builders (byte-exact against known-good captures), IP/broadcast
  math, PPM/JPEG framing, config load/save round-trip, session detection logic.
- **Integration:** discovery against a mock UDP responder; capture backend `detect()`
  returns the right backend for faked env vars.
- **Manual/hardware:** verify against the RESEARCHLAB projector (regression) and, where
  possible, a second Epson model; verify capture on X11, KDE Wayland, GNOME Wayland,
  Hyprland, Windows, macOS.
- Regression guard: the existing working path (template framing, keepalives) must remain
  byte-compatible; snapshot its emitted handshake bytes in a test.

## 6. Rollout / Order of Work

1. Repo size fix (isolated, immediate value, no logic risk) — but **after** a full backup.
2. Cargo workspace + extract `libremp-core` (move, don't rewrite, the proven code).
3. Delete dead `src-tauri/src/protocol/`; point Tauri at core.
4. Discovery-first connection + saved config.
5. Protocol de-hardcoding.
6. Universal capture backend + remove OS picker.
7. UI polish.

Each step keeps the app building and the proven path working.
