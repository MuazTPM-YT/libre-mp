# Epson EasyMP Streamer

Stream your desktop to any Epson projector. Cross-platform: Linux (Arch/Ubuntu/Fedora), macOS, and Windows.

## Quick Start

```bash
cd Rust
cargo build --release
cargo run --release
```

1. Select the projector's Wi-Fi network (marked with ★)
2. Enter the projector's Wi-Fi password and connect
3. Streaming starts at 24fps

Passwords auto-save to `projectors.txt` — instant connect next time!

> **No password set?** Set one on the projector via:
> **Menu → Network → Security → Web Control Password**

---

## Build Instructions

### Dependencies by Platform

| Platform | Rustup | Build Tools | JPEG Assembler | Capture Tool |
|---|---|---|---|---|
| **Arch Linux** | `pacman -S rust` | built-in | `pacman -S nasm` | `pacman -S grim` (Wayland) |
| **Ubuntu/Debian** | [rustup.rs](https://rustup.rs) | `apt install build-essential cmake pkg-config` | `apt install nasm` | `apt install imagemagick` (X11) |
| **Fedora** | `dnf install rust cargo` | `dnf install cmake` | `dnf install nasm` | `dnf install ImageMagick` (X11) |
| **macOS** | [rustup.rs](https://rustup.rs) | Xcode CLI: `xcode-select --install` | `brew install nasm` | Built-in (`screencapture`) |
| **Windows** | [rustup.rs](https://rustup.rs) | Visual Studio Build Tools | [nasm.us](https://nasm.us) + [cmake.org](https://cmake.org) | Built-in (PowerShell) |

### Full Install Commands

**Arch Linux (Wayland)**
```bash
sudo pacman -S rust nasm grim
cd Rust && cargo build --release
```

**Ubuntu / Debian (X11)**
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Build deps + capture tool
sudo apt install build-essential cmake nasm pkg-config imagemagick

cd Rust && cargo build --release
```

**Fedora (X11 or Wayland)**
```bash
sudo dnf install rust cargo cmake nasm ImageMagick
# For Wayland: sudo dnf install grim

cd Rust && cargo build --release
```

**macOS**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
brew install cmake nasm

cd Rust && cargo build --release
```

**Windows (PowerShell as Admin)**
```powershell
# 1. Install Rust from https://rustup.rs
# 2. Install Visual Studio Build Tools (C++ workload)
# 3. Install CMake from https://cmake.org
# 4. Install NASM from https://nasm.us (add to PATH)

cd Rust
cargo build --release
```

---

## How It Works

1. **Wi-Fi**: Scans for networks, detects Epson projectors by SSID pattern
2. **Connect**: Connects to projector's Wi-Fi with your password
3. **Stream**: Captures screen → JPEG tiles → EasyMP protocol → projector display

### Screen Capture Methods

| Platform | Method | Notes |
|---|---|---|
| Linux (Wayland) | `grim` → PPM pipe | Zero-copy, fastest |
| Linux (X11) | `gnome-screenshot` / `import` / `scrot` | Auto-detected fallback |
| macOS | `screencapture` → BMP | Built-in, reliable |
| Windows | PowerShell System.Drawing → BMP | No extra deps |

### Projector Compatibility

Tested with Epson projectors using the EasyMP (RFBPlus) protocol.
The projector's IP is `192.168.88.1` (standard Epson Quick Connect).

---

## Files

| File | Purpose |
|---|---|
| `src/main.rs` | Entry point, frame loop, auto-reconnect |
| `src/wifi.rs` | Cross-platform Wi-Fi scan/connect/restore |
| `src/capture.rs` | Screen capture (Wayland/X11/macOS/Windows) |
| `src/protocol.rs` | EasyMP protocol handshake + streaming |
| `src/template.rs` | JPEG tile template system |
| `projectors.txt` | Saved Wi-Fi passwords (auto-populated) |
| `windows_perfect_stream.bin` | Protocol template (required, in parent dir) |
