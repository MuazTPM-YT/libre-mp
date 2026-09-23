# LibreMP (Epson EasyMP Cross-Platform Streamer)

> **Disclaimer**: The code for this project was written with the assistance of AI. However, **the entire logic, network protocol reverse-engineering, architecture design, and problem-solving** were accomplished entirely by us.

## Problem Statement
Many projectors available today rely on proprietary software (like Epson EasyMP) that is strictly designed and supported only for the Windows operating system. This software limitation leaves users of Linux and macOS without a native, reliable way to connect to and cast their screens onto these devices. As teams and environments grow more diverse in the operating systems they use daily, this "Windows-only" restriction creates a significant barrier to communication, collaboration, and productivity.

## Solution
Our team, **LibreMP**, built a lightweight, highly compatible cross-platform desktop application designed to interact seamlessly with Epson projectors across all major operating systems. We reverse-engineered the EasyMP protocol from raw packet captures and built a solution capable of discovering available projectors on the network, bypassing the vendor's restrictive single-OS software. Our application allows Linux, macOS, and Windows users to easily manage, connect, and stream to projectors at 24fps.

### How It Works
1. **Scan the QR code.** The projector's LAN screen shows a QR code. Choose **Scan QR Code** (take a photo with your webcam), **Choose Photo…** (use a picture you already have), or **Enter Manually…**. The Epson QR code is not a normal Wi-Fi QR code: it is an XOR-obfuscated binary record. LibreMP decodes it and reads the network name and password. You type nothing.
2. **Connect.** If the projector is already on your network, LibreMP casts without switching Wi-Fi. If not, it joins the projector's Wi-Fi, and switches back to your previous Wi-Fi when you stop casting or quit.
3. **Share the screen.** The capture method is chosen automatically:
   - Windows: GDI, with the mouse pointer.
   - macOS: CoreGraphics.
   - Linux X11: XShm. The pointer is added through XFixes, because an X11 screen grab never contains it.
   - Linux Wayland: LibreMP drives the xdg-desktop-portal ScreenCast + PipeWire session itself. It works on KDE, GNOME and wlroots desktops, includes the pointer, and asks the desktop to remember your choice.
4. **Stream.** Frames are encoded as JPEG tiles with libjpeg-turbo (SIMD) and sent with the native EasyMP video protocol at up to 24 fps. Like Epson's own client, LibreMP sends only the parts of the screen that changed. It sends the whole picture once a second so any lost part heals.
5. **Remember.** Projectors you cast to appear under **Saved** for one-click reconnect (optional: reconnect on launch). Their Wi-Fi passwords are kept in your **system keychain** (Secret Service / KWallet on Linux, Keychain on macOS, Credential Manager on Windows), never in plain files.

> **Note on projector credentials:** On Epson Quick Connect / Simple AP projectors the
> default Wi-Fi password is the projector's **MAC address** in hex (no separators).
> The EasyMP login itself does not need a password: like Epson's Windows client,
> LibreMP reads the projector's name and MAC from its registration reply. LibreMP
> cannot bypass a network's Wi-Fi encryption — that is cryptography, not a software limitation.

## Architecture
LibreMP is a Cargo **workspace** with a shared core library:

- **`core/`** — `libremp-core`: all real logic. EasyMP protocol (`protocol.rs`), the cast loop with changed-parts detection (`session.rs`), screen capture (`capture.rs`, `screencast.rs`, `x11_cursor.rs`), Wi-Fi on every OS (`wifi.rs`), Epson QR decoding (`qr.rs`), and saved projectors + keychain (`config.rs`).
- **`cli/`** — `epson-streamer`: an optional command-line tool over the core.
- **`frontend/`** — the Tauri + React desktop app. Its Rust backend links `libremp-core` and casts **in-process** on its own thread. No helper program is needed.

Background notes on the protocol and design decisions are in [`docs/NOTES.md`](docs/NOTES.md). The UI audit is in [`docs/ui-audit.md`](docs/ui-audit.md).

## Tech Stack
- **Tauri** + **React** — the cross-platform desktop UI.
- **Rust** — `libremp-core` and the `epson-streamer` CLI.
- **xdg-desktop-portal + PipeWire** (Wayland), **scrap** (X11 / macOS), **GDI** (Windows) — screen capture. **xcap** is only a last-resort fallback.
- **libjpeg-turbo** (via the `turbojpeg` crate) — JPEG encoding for the stream, and decoding for the webcam.
- **keyring** — the OS keychain for saved passwords.

## Required Dependencies
- **Node.js & npm** — frontend dependencies and dev scripts.
- **Rust & Cargo** — compiles the core, the CLI and the Tauri backend.
- **NASM + CMake** — required by `turbojpeg-sys` for SIMD JPEG code.
- **OS build tools** — a C/C++ toolchain. On Linux also: WebKitGTK dev libraries, `libclang` (bindings generation), and the PipeWire, XCB, EGL, GBM and Wayland dev packages (see your distribution below).
- **Wayland capture (Linux only)** — **PipeWire** and **xdg-desktop-portal** with a backend for your desktop (`xdg-desktop-portal-kde`, `-gnome`/`-gtk`, or `-wlr`/`-hyprland`). These ship with most modern desktops.
- **Keychain (Linux only)** — a Secret Service provider (GNOME Keyring or KWallet). Without one, projectors are still saved, but their passwords are not.

### Projector Keyword
Some projectors show a 4-digit **Projector Keyword** on screen. When the projector refuses the login, the app shows **Enter Keyword…**. In the CLI, pass `--keyword 1234`.

The keyword goes as 4 ASCII digits into the 16-byte field right after the MAC in the `0x0101` login packet. This matches Rhino Security Labs' working EasyMP PIN tool, which puts the PIN in the same place. Our own packet capture came from a projector with the keyword switched off, so this path has not yet run against real keyword-protected hardware. If it fails on your projector, capture Epson iProjection connecting to it and compare the `0x0101` packet with `core/tests/handshake_snapshot.rs`. Switching the keyword off in the projector's network menu always works.

### Known limitations
- Windows and macOS are built and unit-tested in CI ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)). Casting to real hardware from Windows and macOS still needs a person with a projector to try it.
- If a projector shows broken or stale areas, force whole frames: start the app with `LIBREMP_FULL_FRAMES=1`, or pass `--full-frames` to the CLI.

### Checking screen capture without a projector
```bash
cargo run --release -p libremp-core --example portal_probe out.png
```
This captures one frame exactly as the projector would receive it and writes `out.png`.

---

## Installation

Once the prerequisites for your platform are installed, the build is the same everywhere:

```bash
git clone https://github.com/MuazTPM-YT/libre-mp.git
cd libre-mp/frontend
npm install
npm run tauri dev                # or: npm run tauri build   (release bundle)
```

The CLI is optional: `cargo build --release` at the repository root produces `target/release/epson-streamer`.

### Install as an app (Linux)

One command builds LibreMP and adds it to your app launcher (rofi, fuzzel, GNOME, KDE, quickshell, …), with its icon:

```bash
./scripts/install-linux.sh              # build + install for this user (no sudo)
./scripts/install-linux.sh --uninstall  # remove it again (saved projectors are kept)
```

It installs `~/.local/bin/libremp-app`, `~/.local/share/applications/libremp-app.desktop` and the icons under `~/.local/share/icons/hicolor/`. Run it again after `git pull` to update.

The window has a fixed size (920×760) and its own title bar. Tiling window managers float it automatically, because its size cannot change. On Hyprland you can also centre it:

```lua
-- ~/.config/hypr/custom/rules.lua (Lua config)
hl.window_rule({match = {class = "^(libremp-app)$" }, float = true})
hl.window_rule({match = {class = "^(libremp-app)$" }, center = true})
```

With the classic `hyprland.conf`, use `windowrule = float, class:^(libremp-app)$` and `windowrule = center, class:^(libremp-app)$`.

Platform-specific prerequisites follow.

### 1. Arch Linux (Wayland / X11)
```bash
sudo pacman -S base-devel curl wget nodejs npm rustup nasm cmake clang webkit2gtk-4.1 \
  pipewire libxcb mesa wayland
sudo pacman -S xdg-desktop-portal
#   KDE:      sudo pacman -S xdg-desktop-portal-kde
#   GNOME:    sudo pacman -S xdg-desktop-portal-gnome
#   Hyprland/wlroots: sudo pacman -S xdg-desktop-portal-hyprland   # or -wlr
rustup default stable
```

### 2. Ubuntu / Debian
```bash
sudo apt update
sudo apt install build-essential cmake curl wget file nasm pkg-config libclang-dev nodejs npm \
  libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libssl-dev \
  libpipewire-0.3-dev libxcb1-dev libxcb-shm0-dev libxcb-randr0-dev libxcb-xfixes0-dev \
  libegl-dev libgbm-dev libwayland-dev \
  pipewire xdg-desktop-portal xdg-desktop-portal-gtk
# Install Rust:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 3. Fedora
```bash
sudo dnf install rust cargo cmake nasm clang-devel nodejs npm webkit2gtk4.1-devel \
  pipewire-devel libxcb-devel mesa-libEGL-devel mesa-libgbm-devel wayland-devel \
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

Then run the build steps from the **Installation** section in PowerShell.

---

## Command-line usage (optional)

The CLI casts without the GUI. It is useful for testing against a projector:

```bash
./target/release/epson-streamer --skip-wifi --ssid <PROJECTOR_SSID> --password <MAC_HEX>
```

Flags:
- `--skip-wifi` — assume you are already on the projector's network. Without it, the CLI lists Wi-Fi networks and asks which one to join.
- `--ssid <name>` — the projector SSID. Only a fallback name source; the projector reports its own name.
- `--password <hex>` — the projector MAC (hex). Only a fallback; the projector reports its own MAC.
- `--projector-ip <ip>` — try this address first (otherwise every default gateway, then `192.168.88.1`).
- `--keyword <digits>` — the projector's 4-digit Projector Keyword, if it shows one.
- `--give-up-after <n>` — exit with status 2 after `n` failed connection attempts in a row.
- `--full-frames` — send the whole picture every frame instead of only the changed parts.

Exit status: `0` stopped by you (Ctrl+C), `2` projector unreachable or refused, `3` screen capture failed.

## Tests

```bash
cargo test --workspace
```

The tests include a fake projector on `127.0.0.1` (ports 3620/3621). It runs the full handshake, the stream and the goodbye. Every captured Windows frame is rebuilt byte for byte.

Tests against real projector QR codes read their data from `.env` (copy `.env.example`). Without it they are skipped. Real Wi-Fi passwords never go into the code.
