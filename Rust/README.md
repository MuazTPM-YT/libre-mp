# Epson EasyMP Streamer

Cross-platform desktop streaming to Epson projectors via the EasyMP (RFBPlus) protocol.

## Supported Platforms

| OS | Display Server | Capture Tool | Wi-Fi Tool |
|---|---|---|---|
| **Arch Linux** | Wayland | `grim` | `nmcli` |
| **Ubuntu/Fedora** | X11 | `gnome-screenshot` / `import` / `scrot` | `nmcli` |
| **macOS** | Aqua | `screencapture` | `networksetup` |
| **Windows** | Desktop | PowerShell (.NET) | `netsh wlan` |

## Build Instructions

### Arch Linux
```bash
sudo pacman -S rust grim   # grim for Wayland capture
cd Rust && cargo build --release
```

### Ubuntu / Debian
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install build dependencies
sudo apt install build-essential cmake nasm pkg-config libturbojpeg0-dev

# Install a capture tool (at least one):
sudo apt install imagemagick   # provides 'import' command
# or: sudo apt install scrot
# or: gnome-screenshot comes pre-installed on GNOME

cd Rust && cargo build --release
```

### Fedora
```bash
sudo dnf install rust cargo cmake nasm turbojpeg-devel
sudo dnf install ImageMagick   # for 'import' command
cd Rust && cargo build --release
```

### macOS
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
brew install cmake nasm
cd Rust && cargo build --release
```

### Windows
```powershell
# Install Rust from https://rustup.rs
# Install CMake from https://cmake.org
# Install NASM from https://nasm.us
cd Rust
cargo build --release
```

## Usage

```bash
cd Rust && cargo run --release
```

1. Select the projector's Wi-Fi network (marked with ★)
2. If first time: enter the Wired LAN MAC address from the projector's menu
   - On projector: **Menu → Network → Net. Info → Wired LAN → MAC Address**
   - Type it in (dots/colons are OK, e.g., `A4.D7.3C.CD.AF.45`)
   - It auto-saves to `projectors.txt` for instant connect next time
3. Streaming starts automatically at 24fps

## Projector Password

The Wi-Fi password for Epson projectors is the **Wired LAN MAC address**
(NOT the Wireless MAC / BSSID visible in Wi-Fi scans).

Find it on the projector sticker or via:
**Menu → Network → Net. Info → Wired LAN → MAC Address**

Saved passwords are stored in `projectors.txt`:
```
RESEARCHLAB-fE8DSypQz51AR2Q = A4D73CCDAF45
A325-fC8DSypQye1AKdd = A4D73CCDAF28
```
