//! Backend auto-detection replaces the manual "select your OS [1-4]" picker.
//! These lock the selection logic so KDE/GNOME/wlroots Wayland all resolve to
//! the portal path and X11 stays on the direct grabber.

use libremp_core::capture::{select_backend, CaptureBackend};

#[test]
fn windows_uses_gdi() {
    assert_eq!(select_backend("windows", None, None), CaptureBackend::WindowsGdi);
}

#[test]
fn macos_uses_coregraphics() {
    assert_eq!(select_backend("macos", None, None), CaptureBackend::MacCoreGraphics);
}

#[test]
fn linux_x11_session_uses_x11() {
    assert_eq!(
        select_backend("linux", Some("x11"), None),
        CaptureBackend::LinuxX11
    );
}

#[test]
fn linux_wayland_session_uses_portal() {
    // Covers KDE, GNOME, and wlroots uniformly — all report session type wayland.
    assert_eq!(
        select_backend("linux", Some("wayland"), Some("wayland-0")),
        CaptureBackend::LinuxWaylandPortal
    );
}

#[test]
fn wayland_display_alone_implies_portal() {
    // Some setups leave XDG_SESSION_TYPE unset but export WAYLAND_DISPLAY.
    assert_eq!(
        select_backend("linux", None, Some("wayland-1")),
        CaptureBackend::LinuxWaylandPortal
    );
}

#[test]
fn no_signals_falls_back_to_x11() {
    assert_eq!(select_backend("linux", None, None), CaptureBackend::LinuxX11);
    // Empty WAYLAND_DISPLAY must not be treated as a live Wayland session.
    assert_eq!(select_backend("linux", Some("tty"), Some("")), CaptureBackend::LinuxX11);
}

#[test]
fn freebsd_shares_the_linux_split() {
    assert_eq!(select_backend("freebsd", Some("wayland"), None), CaptureBackend::LinuxWaylandPortal);
    assert_eq!(select_backend("freebsd", Some("x11"), None), CaptureBackend::LinuxX11);
}
