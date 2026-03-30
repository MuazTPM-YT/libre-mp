# LibreMP (Epson EasyMP Cross-Platform Streamer)

> **Disclaimer**: The code for this project was written with the assistance of AI. However, **the entire logic, network protocol reverse-engineering, architecture design, and problem-solving** were accomplished entirely by us. 

## Problem Statement
Many projectors available today rely on proprietary software (like Epson EasyMP) that is strictly designed and supported only for the Windows operating system. This software limitation leaves users of Linux and macOS without a native, reliable way to connect to and cast their screens onto these devices. As teams and environments grow more diverse in the operating systems they use daily, this "Windows-only" restriction creates a significant barrier to communication, collaboration, and productivity.

## Solution
Our team, **LibreMP**, built a lightweight, highly compatible cross-platform desktop application designed to interact seamlessly with Epson projectors across all major operating systems. We reverse-engineered the EasyMP (RFBPlus) protocol and built a solution capable of discovering available projectors on the network, bypassing the vendor's restrictive single-OS software. Our application allows Linux, macOS, and Windows users to easily manage, connect, and stream to projectors with zero friction at 24fps.

### How It Works
1. **Wi-Fi Discovery**: Automatically scans for networks and detects Epson projectors by their SSID pattern.
2. **Connect**: Connects seamlessly to the projector's Wi-Fi network (saving passwords automatically for instant reconnects).
3. **Stream**: Captures the screen, encodes it into JPEG tiles using high-performance SIMD instructions, and streams it using the native EasyMP protocol directly to the projector display.

## Tech Stack
- **Tauri**: Framework for building our tiny, blazing fast, and secure cross-platform desktop application.
- **React**: Used to create a modern, fluid, glassmorphic UI that feels "premium" and consistent on any operating system.
- **Rust**: Powers the backend, handling critical system-level operations such as network discovery, Wi-Fi management, high-performance stream encoding, and the EasyMP protocol communication with extreme safety and speed.
- **TurboJPEG**: Used for high-speed hardware-accelerated JPEG encoding of screen frames.

## Required Dependencies
- **Node.js & npm**: Required to manage frontend dependencies and development scripts.
- **Rust & Cargo**: Required to compile the Tauri backend and manage Rust crate dependencies.
- **NASM (Netwide Assembler)**: Required by our `turbojpeg-sys` dependency for high-performance JPEG encoding using SIMD instructions.
- **OS-specific Build Tools**: C++ build tools, WebKit/WebView development libraries (e.g., `libwebkit2gtk-4.1-dev`), and native screen capture utilities (like `grim` on Wayland or `ImageMagick` on X11).

---

## Installation Process

### 1. Arch Linux (Wayland / X11)
**Prerequisites:**
```bash
sudo pacman -S base-devel curl wget nodejs npm rustup nasm webkit2gtk-4.1 xdotool
# For Wayland capture:
sudo pacman -S grim
# Install Rust toolchain:
rustup default stable
```

**Clone and Build:**
```bash
git clone <repository_url> libre-mp
cd libre-mp/frontend
npm install
npm run tauri dev
```

### 2. Ubuntu / Debian
**Prerequisites:**
```bash
sudo apt update
sudo apt install build-essential curl wget file libxdo-dev libssl-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev nasm imagemagick nodejs npm
# Install Rust:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

**Clone and Build:**
```bash
git clone <repository_url> libre-mp
cd libre-mp/frontend
npm install
npm run tauri dev
```

### 3. Fedora
**Prerequisites:**
```bash
sudo dnf install rust cargo cmake nasm nodejs npm ImageMagick webkit2gtk4.1-devel
# For Wayland capture:
sudo dnf install grim
```

**Clone and Build:**
```bash
git clone <repository_url> libre-mp
cd libre-mp/frontend
npm install
npm run tauri dev
```

### 4. macOS
**Prerequisites:**
```bash
# Install Xcode Command Line Tools
xcode-select --install
# Install dependencies via Homebrew
brew install node nasm
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

**Clone and Build:**
```bash
git clone <repository_url> libre-mp
cd libre-mp/frontend
npm install
npm run tauri dev
```

### 5. Windows
**Prerequisites:**
1. Install **Node.js** from the official website.
2. Install **Rust** via `rustup-init.exe` from the [official Rust website](https://rustup.rs/).
3. Install the **Microsoft C++ Build Tools** (ensure "Desktop development with C++" is selected during installation).
4. Install **CMake** from `cmake.org` and **NASM** from `nasm.us` (ensure both executables are added to your system `PATH`).
5. Install **WebView2** (this is usually pre-installed on Windows 11).

**Clone and Build (PowerShell as Admin):**
```powershell
git clone <repository_url> libre-mp
cd libre-mp/frontend
npm install
npm run tauri dev
```
