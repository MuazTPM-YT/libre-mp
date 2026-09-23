#!/usr/bin/env bash
# build libremp, install it for this user, add it to app launcher. --uninstall removes it
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

# build release app without installer bundles
build() {
    command -v cargo >/dev/null 2>&1 || die "cargo not found. Install Rust: https://rustup.rs"
    command -v npm >/dev/null 2>&1 || die "npm not found. Install Node.js and npm."
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

case "${1:-}" in
    --uninstall)
        uninstall
        ;;
    --no-build)
        install_files
        say "Installed. Search for LibreMP in your app launcher."
        ;;
    "")
        build
        install_files
        say "Installed. Search for LibreMP in your app launcher."
        ;;
    -h | --help)
        echo "Usage: scripts/install-linux.sh [--no-build | --uninstall]"
        echo "  (none)       build the release app, then install it for this user"
        echo "  --no-build   install the release app that is already built"
        echo "  --uninstall  remove LibreMP (saved projectors are kept)"
        ;;
    *)
        die "unknown option: $1 (try --help)"
        ;;
esac
