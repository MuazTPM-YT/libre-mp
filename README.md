# LibreMP (Epson EasyMP Cross-Platform Streamer)

> **Disclaimer**: The code for this project was written with the assistance of AI. However, **the entire logic, network protocol reverse-engineering, architecture design, and problem-solving** were accomplished entirely by us.

## Problem Statement
Many projectors available today rely on proprietary software (like Epson EasyMP) that is strictly designed and supported only for the Windows operating system. This software limitation leaves users of Linux and macOS without a native, reliable way to connect to and cast their screens onto these devices. As teams and environments grow more diverse in the operating systems they use daily, this "Windows-only" restriction creates a significant barrier to communication, collaboration, and productivity.

## Solution
Our team, **LibreMP**, built a lightweight, highly compatible cross-platform desktop application designed to interact seamlessly with Epson projectors across all major operating systems. We reverse-engineered the EasyMP protocol from raw packet captures and built a solution capable of discovering available projectors on the network, bypassing the vendor's restrictive single-OS software. Our application allows Linux, macOS, and Windows users to easily manage, connect, and stream to projectors at 24fps.

### How It Works
1. **Scan the QR**: On the projector's LAN screen there's a QR code. Use **Live scan** (take a photo with your webcam), **Upload QR photo**, or **Enter details** — LibreMP decodes the Epson QR (it's an XOR-obfuscated binary record, not a standard Wi-Fi QR), extracts the SSID + passphrase, and connects with **no typing**. Projectors you've used are **saved** for one-tap reconnect (and optional auto-reconnect on launch). You can also connect to discovered LAN projectors or Wi-Fi networks directly from the list.
2. **Auto-detected capture**: The screen-capture backend is chosen automatically from your OS and session — there is **no manual OS picker**. Windows uses GDI (with cursor), macOS uses CoreGraphics, Linux X11 uses XShm, and **Linux Wayland uses xdg-desktop-portal ScreenCast + PipeWire**, so it works on KDE, GNOME and wlroots compositors alike. On Wayland the mouse pointer is included in the picture, and LibreMP asks the desktop to remember your choice so it need not ask again (where the desktop supports that). On X11 the pointer is fetched through XFixes and drawn into the picture, because an X11 screen grab never contains it.
3. **Stream**: The frame is encoded into JPEG tiles with TurboJPEG (SIMD) and sent to the projector using the native EasyMP video protocol.

> **Note on projector credentials:** On Epson Quick Connect / Simple AP projectors the
> default Wi-Fi password is the projector's **MAC address** in hex (no separators).
> The EasyMP login itself does not need a password: like Epson's Windows client,
> LibreMP reads the projector's name and MAC from its registration reply. If the
> projector is already reachable on your current network, LibreMP casts without
> switching Wi-Fi. LibreMP cannot bypass a network's Wi-Fi encryption — that is
> cryptography, not a software limitation.
>
> **Not supported yet:** the projector setting **Projector Keyword** (a 4-digit code
> shown on screen). Turn it off in the projector's network menu, or send us a packet
> capture of Epson iProjection connecting with a keyword so it can be added.

## Architecture
LibreMP is a Cargo **workspace** with a shared core library:

- **`core/`** — `libremp-core`: all real logic (EasyMP protocol, universal screen capture, Wi-Fi helpers, saved-projector config, frame framing).
- **`cli/`** — `epson-streamer`: a thin command-line binary over the core. This is the binary the GUI runs to actually stream.
- **`frontend/`** — the Tauri + React desktop app. Its Rust backend depends on `libremp-core` and **spawns the prebuilt `epson-streamer` binary** to cast.

Because the GUI spawns the streamer binary, you must build it (`cargo build --release`
at the repo root) **before** the "Cast" button will work.

## Tech Stack
- **Tauri** + **React** — the cross-platform desktop UI.
- **Rust** — the `libremp-core` library and `epson-streamer` binary (network discovery, EasyMP protocol, capture, encoding).
- **xcap** — universal screen capture, including the Wayland xdg-desktop-portal + PipeWire path.
- **TurboJPEG** — high-speed SIMD JPEG encoding of screen frames.

## Required Dependencies
- **Node.js & npm** — frontend dependencies and dev scripts.
- **Rust & Cargo** — compiles both the `epson-streamer` binary and the Tauri backend.
- **NASM + CMake** — required by `turbojpeg-sys` for SIMD JPEG encoding.
- **OS build tools** — C/C++ toolchain and, on Linux, WebKit/WebView dev libraries (e.g. `libwebkit2gtk-4.1-dev`).
- **Wayland capture (Linux only)** — **PipeWire** and **xdg-desktop-portal** with a backend for your desktop (`xdg-desktop-portal-kde`, `-gnome`/`-gtk`, or `-wlr`/`-hyprland`). These ship with most modern desktops. `grim` is **no longer required**, and neither is XWayland.

### Projector Keyword
Some projectors show a 4-digit **Projector Keyword** on screen and demand it. Pass it
with `--keyword 1234`. If the projector still refuses, LibreMP says so clearly rather
than hanging.

The field it goes in is **inferred, not captured** — the packet capture we reverse
engineered came from a projector with the setting switched off. Everything else in the
handshake is byte-verified against Epson's own Windows client. If `--keyword` does not
work on your projector, capture Epson iProjection connecting to it and compare its
`0x0101` packet with `core/tests/handshake_snapshot.rs`; the fix is then a one-line
offset change. Until then, switching the keyword off in the projector's network menu
always works.

### Known limitations
- Windows and macOS builds have not been re-tested since the Linux-focused rewrite.

### Checking screen capture without a projector
```bash
cargo run --release -p libremp-core --example portal_probe out.png
```
This captures one frame exactly as the projector would receive it and writes `out.png`.

---

## Installation

The build is the same two steps on every platform once prerequisites are installed:

```bash
git clone https://github.com/MuazTPM-YT/libre-mp.git
cd libre-mp

# 1. Build the streamer binary (from the repo root — this is the workspace).
cargo build --release            # produces target/release/epson-streamer

# 2. Run the GUI.
cd frontend
npm install
npm run tauri dev                # or: npm run tauri build   (for a release bundle)
```

Platform-specific prerequisites follow.

### 1. Arch Linux (Wayland / X11)
```bash
sudo pacman -S base-devel curl wget nodejs npm rustup nasm cmake webkit2gtk-4.1
# Wayland capture (portal + PipeWire + a backend for your compositor):
sudo pacman -S pipewire xdg-desktop-portal
#   KDE:      sudo pacman -S xdg-desktop-portal-kde
#   GNOME:    sudo pacman -S xdg-desktop-portal-gnome
#   Hyprland/wlroots: sudo pacman -S xdg-desktop-portal-hyprland   # or -wlr
rustup default stable
```

### 2. Ubuntu / Debian
```bash
sudo apt update
sudo apt install build-essential cmake curl wget file libssl-dev \
  libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev nasm nodejs npm \
  pipewire xdg-desktop-portal xdg-desktop-portal-gtk
# Install Rust:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 3. Fedora
```bash
sudo dnf install rust cargo cmake nasm nodejs npm webkit2gtk4.1-devel \
  pipewire xdg-desktop-portal
# Desktop portal backend (KDE example): sudo dnf install xdg-desktop-portal-kde
```

### 4. macOS
```bash
# Xcode Command Line Tools
xcode-select --install
# Dependencies via Homebrew
brew install node nasm cmake
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 5. Windows
1. Install **Node.js** from the official website.
2. Install **Rust** via `rustup-init.exe` from [rustup.rs](https://rustup.rs/).
3. Install the **Microsoft C++ Build Tools** ("Desktop development with C++").
4. Install **CMake** (`cmake.org`) and **NASM** (`nasm.us`); add both to your `PATH`.
5. Install **WebView2** (pre-installed on Windows 11).

Then run the two build steps from the **Installation** section in PowerShell.

---

## Command-line usage (optional)

The streamer can run standalone without the GUI — useful for testing against a
projector. Build it (`cargo build --release`) and run it:

```bash
./target/release/epson-streamer --skip-wifi --ssid <PROJECTOR_SSID> --password <MAC_HEX>
```

Flags:
- `--skip-wifi` — assume you are already on the projector's network.
- `--ssid <name>` — the projector SSID. Only a fallback name source; the projector reports its own name.
- `--password <hex>` — the projector MAC (hex). Only a fallback; the projector reports its own MAC.
- `--projector-ip <ip>` — try this address first (otherwise every default gateway, then `192.168.88.1`).
- `--keyword <digits>` — the projector's 4-digit Projector Keyword, if it shows one (see above).
- `--give-up-after <n>` — exit with status 2 after `n` failed connection attempts in a row.
- `--stop-on-stdin-eof` — disconnect cleanly when stdin closes (used by the GUI).

The capture backend is auto-detected; there is no `--os` selection.
