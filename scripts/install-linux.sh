#!/usr/bin/env bash
# install build deps for this distro, build libremp, install it for this user, add it to app launcher. --uninstall removes it
set -euo pipefail

# app id = binary name = wayland window class; desktop file, icon and wm rules all key on it
APP_ID="libremp-app"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
APPS_DIR="$DATA_DIR/applications"
ICON_DIR="$DATA_DIR/icons/hicolor"
BIN="$BIN_DIR/$APP_ID"
DESKTOP="$APPS_DIR/$APP_ID.desktop"
SRC_ICONS="$ROOT/frontend/src-tauri/icons"

# say what happen
say() { printf '\033[1m==>\033[0m %s\n' "$*"; }

# stop with message
die() {
    printf '\033[31merror:\033[0m %s\n' "$*" >&2
    exit 1
}

# png icon sizes we ship: "source-file size"
ICONS=("32x32.png 32" "64x64.png 64" "128x128.png 128" "128x128@2x.png 256" "icon.png 512")

# tell launcher + icon theme about changes; missing tools are fine
refresh_caches() {
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database -q "$APPS_DIR" || true
    fi
    # only refresh icon cache if one already exists; a stale cache hides new icons
    if [[ -f "$ICON_DIR/icon-theme.cache" ]] && command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -q -f -t "$ICON_DIR" || true
    fi
}

# remove everything install put down
uninstall() {
    say "Removing LibreMP"
    rm -f "$BIN" "$DESKTOP" "$ICON_DIR/scalable/apps/$APP_ID.svg"
    for entry in "${ICONS[@]}"; do
        read -r _ size <<<"$entry"
        rm -f "$ICON_DIR/${size}x${size}/apps/$APP_ID.png"
    done
    refresh_caches
    say "Done. Saved projectors and keychain passwords were kept."
}

# quote path for desktop Exec= (spec: double quotes, escape \ " ` $)
exec_quote() {
    local s="$1"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    s="${s//\`/\\\`}"
    s="${s//\$/\\\$}"
    printf '"%s"' "$s"
}

# oldest rust + node the build accepts (deps need rust 1.89, vite 7 needs node 20.19)
MIN_RUST="1.89.0"
MIN_NODE="20.19.0"
NODE_MAJOR="22"
TOOLS_DIR="$DATA_DIR/libremp/tools"

# true if version $1 >= $2
version_ok() { [[ "$(printf '%s\n%s\n' "$2" "$1" | sort -V | head -n1)" == "$2" ]]; }

# run as root: direct when root, else sudo
as_root() {
    if [[ $EUID -eq 0 ]]; then
        "$@"
    else
        command -v sudo >/dev/null 2>&1 || die "need root for system packages, and sudo is missing. Run: su -c '$*'"
        sudo "$@"
    fi
}

# package family from /etc/os-release: apt, dnf, pacman, zypper or unknown
pkg_family() {
    local ids=""
    [[ -r /etc/os-release ]] && ids="$(. /etc/os-release && echo "${ID:-} ${ID_LIKE:-}")"
    for id in $ids; do
        case "$id" in
            debian | ubuntu | linuxmint | kali | pop | elementary | zorin | raspbian | neon | parrot | deepin) echo apt; return ;;
            fedora | rhel | centos | rocky | almalinux | nobara | ultramarine) echo dnf; return ;;
            arch | archarm | manjaro | endeavouros | garuda | cachyos | artix) echo pacman; return ;;
            opensuse* | suse | sles) echo zypper; return ;;
        esac
    done
    for tool in apt-get:apt dnf:dnf pacman:pacman zypper:zypper; do
        command -v "${tool%%:*}" >/dev/null 2>&1 && { echo "${tool##*:}"; return; }
    done
    echo unknown
}

# screen-share portal backend for this desktop (wayland needs one); gtk has none, so it is only a last resort
portal_backend() {
    case "${XDG_CURRENT_DESKTOP,,}" in
        *gnome* | *ubuntu* | *pop* | *unity* | *budgie*) echo gnome ;;
        *kde* | *plasma* | *lxqt*) echo kde ;;
        *hyprland*) echo hyprland ;;
        *sway* | *river* | *wayfire* | *labwc* | *niri* | *wlroots*) echo wlr ;;
        *) echo gtk ;;
    esac
}

# build + runtime packages per distro family
install_deps() {
    local family portal
    family="$(pkg_family)"
    portal="xdg-desktop-portal-$(portal_backend)"
    say "Installing system packages ($family). This needs your password."
    case "$family" in
        apt)
            as_root apt-get update
            as_root env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
                build-essential curl ca-certificates xz-utils file pkg-config cmake nasm clang libclang-dev \
                libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libssl-dev \
                libpipewire-0.3-dev libxcb1-dev libxcb-shm0-dev libxcb-randr0-dev libxcb-xfixes0-dev \
                libegl-dev libgbm-dev libwayland-dev libv4l-dev pipewire xdg-desktop-portal
            as_root env DEBIAN_FRONTEND=noninteractive apt-get install -y "$portal" || say "Could not install $portal; screen sharing on Wayland may not work."
            ;;
        dnf)
            as_root dnf install -y gcc gcc-c++ make curl xz file pkgconf-pkg-config cmake nasm clang clang-devel \
                webkit2gtk4.1-devel gtk3-devel librsvg2-devel openssl-devel \
                pipewire-devel libxcb-devel mesa-libEGL-devel mesa-libgbm-devel wayland-devel libv4l-devel \
                pipewire xdg-desktop-portal
            as_root dnf install -y "$portal" || say "Could not install $portal; screen sharing on Wayland may not work."
            ;;
        pacman)
            # no -y: syncing without full upgrade breaks arch. stale package list? run: sudo pacman -Syu
            as_root pacman -S --needed --noconfirm base-devel curl xz file pkgconf cmake nasm clang \
                webkit2gtk-4.1 gtk3 librsvg openssl pipewire libxcb mesa wayland v4l-utils xdg-desktop-portal
            as_root pacman -S --needed --noconfirm "$portal" || say "Could not install $portal; screen sharing on Wayland may not work."
            ;;
        zypper)
            # pkgconfig names survive opensuse package renames (webkit2gtk3-devel became webkitgtk3-devel)
            as_root zypper --non-interactive install gcc gcc-c++ make curl xz file pkgconf cmake nasm clang clang-devel \
                'pkgconfig(webkit2gtk-4.1)' 'pkgconfig(gtk+-3.0)' 'pkgconfig(librsvg-2.0)' 'pkgconfig(openssl)' \
                'pkgconfig(libpipewire-0.3)' 'pkgconfig(xcb)' 'pkgconfig(xcb-shm)' 'pkgconfig(xcb-randr)' 'pkgconfig(xcb-xfixes)' \
                'pkgconfig(egl)' 'pkgconfig(gbm)' 'pkgconfig(wayland-client)' 'pkgconfig(libv4l2)' pipewire xdg-desktop-portal
            as_root zypper --non-interactive install "$portal" || say "Could not install $portal; screen sharing on Wayland may not work."
            ;;
        *)
            say "Unknown distro. Install these yourself, then run again with --no-deps:"
            say "  C/C++ compiler, cmake, nasm, clang/libclang, pkg-config, curl, xz,"
            say "  WebKitGTK 4.1, GTK 3, librsvg, OpenSSL, PipeWire, libxcb, EGL, GBM, Wayland, libv4l (dev packages),"
            say "  xdg-desktop-portal + the portal for your desktop."
            die "cannot install packages on this distro automatically"
            ;;
    esac
    command -v nmcli >/dev/null 2>&1 || say "NetworkManager (nmcli) not found: join the projector Wi-Fi yourself before casting."
}

# rust >= MIN_RUST; distro rust is often older, so add rustup toolchain for this user
ensure_rust() {
    export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
    local have=""
    command -v rustc >/dev/null 2>&1 && have="$(rustc --version | awk '{print $2}')"
    if [[ -n "$have" ]] && version_ok "$have" "$MIN_RUST" && command -v cargo >/dev/null 2>&1; then
        return
    fi
    if command -v rustup >/dev/null 2>&1; then
        say "Updating Rust (have ${have:-none}, need $MIN_RUST)"
        rustup toolchain install stable --profile minimal
        rustup default stable
    else
        say "Installing Rust for this user with rustup (have ${have:-none}, need $MIN_RUST)"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal --default-toolchain stable
    fi
    hash -r
    version_ok "$(rustc --version | awk '{print $2}')" "$MIN_RUST" || die "Rust is still older than $MIN_RUST"
}

# node >= MIN_NODE; ubuntu/mint/debian ship older, so fetch official node into tools dir (sha256 checked)
ensure_node() {
    local have=""
    command -v node >/dev/null 2>&1 && have="$(node -p process.versions.node 2>/dev/null || true)"
    if [[ -n "$have" ]] && version_ok "$have" "$MIN_NODE" && command -v npm >/dev/null 2>&1; then
        return
    fi
    if [[ -x "$TOOLS_DIR/node/bin/node" ]] && version_ok "$("$TOOLS_DIR/node/bin/node" -p process.versions.node)" "$MIN_NODE"; then
        export PATH="$TOOLS_DIR/node/bin:$PATH"
        return
    fi
    local arch
    case "$(uname -m)" in
        x86_64) arch=x64 ;;
        aarch64 | arm64) arch=arm64 ;;
        armv7l) arch=armv7l ;;
        *) die "no official Node.js for $(uname -m); install Node.js $MIN_NODE or newer yourself" ;;
    esac
    say "Installing Node.js $NODE_MAJOR for this build (have ${have:-none}, need $MIN_NODE)"
    local base="https://nodejs.org/dist/latest-v${NODE_MAJOR}.x" tmp file
    tmp="$(mktemp -d)"
    curl -fsSL "$base/SHASUMS256.txt" -o "$tmp/SHASUMS256.txt"
    file="$(grep -oE "node-v[0-9.]+-linux-$arch\.tar\.xz" "$tmp/SHASUMS256.txt" | head -n1)"
    [[ -n "$file" ]] || die "Node.js download list has no linux-$arch build"
    curl -fsSL "$base/$file" -o "$tmp/$file"
    (cd "$tmp" && grep " $file\$" SHASUMS256.txt | sha256sum -c --quiet -) || die "Node.js download failed its checksum"
    rm -rf "$TOOLS_DIR/node"
    mkdir -p "$TOOLS_DIR/node"
    tar -xJf "$tmp/$file" -C "$TOOLS_DIR/node" --strip-components=1
    rm -rf "$tmp"
    export PATH="$TOOLS_DIR/node/bin:$PATH"
}

# build release app without installer bundles
build() {
    ensure_rust
    ensure_node
    say "Installing frontend packages"
    (cd "$ROOT/frontend" && npm ci --no-audit --no-fund)
    say "Building LibreMP (release). The first build takes a few minutes."
    (cd "$ROOT/frontend" && npm run tauri build -- --no-bundle)
}

# copy binary, icons, desktop entry into user dirs
install_files() {
    local built="$ROOT/frontend/src-tauri/target/release/$APP_ID"
    [[ -x "$built" ]] || die "build output missing: $built"

    say "Installing to $BIN"
    install -Dm755 "$built" "$BIN"

    for entry in "${ICONS[@]}"; do
        read -r file size <<<"$entry"
        install -Dm644 "$SRC_ICONS/$file" "$ICON_DIR/${size}x${size}/apps/$APP_ID.png"
    done
    install -Dm644 "$SRC_ICONS/icon.svg" "$ICON_DIR/scalable/apps/$APP_ID.svg"

    local tmp
    tmp="$(mktemp)"
    cat >"$tmp" <<EOF
[Desktop Entry]
Type=Application
Version=1.5
Name=LibreMP
GenericName=Projector Casting
Comment=Cast your screen to Epson projectors
Exec=$(exec_quote "$BIN")
Icon=$APP_ID
Terminal=false
Categories=Network;
Keywords=projector;epson;easymp;iprojection;cast;mirror;screen;present;
StartupNotify=true
StartupWMClass=$APP_ID
EOF
    install -Dm644 "$tmp" "$DESKTOP"
    rm -f "$tmp"

    if command -v desktop-file-validate >/dev/null 2>&1; then
        desktop-file-validate "$DESKTOP" || die "desktop entry failed validation: $DESKTOP"
    fi
    refresh_caches
}

if [[ $EUID -eq 0 && "${1:-}" != "--deps-only" && "${1:-}" != "--uninstall" ]]; then
    say "Running as root: LibreMP gets installed for root only. Run as your normal user to install it for you."
fi

case "${1:-}" in
    --uninstall)
        uninstall
        ;;
    --no-build)
        install_files
        say "Installed. Search for LibreMP in your app launcher."
        ;;
    --no-deps)
        build
        install_files
        say "Installed. Search for LibreMP in your app launcher."
        ;;
    --deps-only)
        install_deps
        ;;
    "")
        install_deps
        build
        install_files
        say "Installed. Search for LibreMP in your app launcher."
        ;;
    -h | --help)
        echo "Usage: scripts/install-linux.sh [--no-deps | --deps-only | --no-build | --uninstall]"
        echo "  (none)       install system packages, build the app, install it for this user"
        echo "  --no-deps    skip system packages (you installed them yourself)"
        echo "  --deps-only  only install system packages"
        echo "  --no-build   install the release app that is already built"
        echo "  --uninstall  remove LibreMP (saved projectors are kept)"
        ;;
    *)
        die "unknown option: $1 (try --help)"
        ;;
esac
