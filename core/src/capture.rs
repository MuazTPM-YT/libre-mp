use turbojpeg::{Compressor, Image, PixelFormat, Subsamp};

use crate::{STREAM_W, STREAM_H, JPEG_QUALITY};

// ─── Capture Backend Auto-Detection ────────────────────────────────────────
//
// Replaces the manual "select your OS [1-4]" prompt. The right screen-grab API
// is fully determined by the OS and, on Linux, the session type — so we detect
// it instead of asking. Crucially, Wayland resolves to the xdg-desktop-portal
// path, which works on KDE, GNOME, and wlroots alike (unlike `grim`, which only
// works on wlroots compositors).

/// The screen-capture backend chosen for the current environment.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CaptureBackend {
    /// Windows: GDI BitBlt (with cursor).
    WindowsGdi,
    /// macOS: CoreGraphics (via `scrap`).
    MacCoreGraphics,
    /// Linux/BSD on X11: XShm (via `scrap`).
    LinuxX11,
    /// Linux/BSD on Wayland: xdg-desktop-portal ScreenCast + PipeWire.
    LinuxWaylandPortal,
}

/// Pure backend selection from environment signals. Separated from the real
/// environment lookups so it can be unit-tested deterministically.
///
/// `os` is `std::env::consts::OS`; the remaining args come from the
/// `XDG_SESSION_TYPE` and `WAYLAND_DISPLAY` environment variables.
pub fn select_backend(
    os: &str,
    session_type: Option<&str>,
    wayland_display: Option<&str>,
) -> CaptureBackend {
    match os {
        "windows" => CaptureBackend::WindowsGdi,
        "macos" => CaptureBackend::MacCoreGraphics,
        // Linux, FreeBSD, and other unixes share the same X11/Wayland split.
        _ => {
            let is_wayland = matches!(session_type, Some(s) if s.eq_ignore_ascii_case("wayland"))
                || wayland_display.map(|d| !d.is_empty()).unwrap_or(false);
            if is_wayland {
                CaptureBackend::LinuxWaylandPortal
            } else {
                CaptureBackend::LinuxX11
            }
        }
    }
}

/// Auto-detect the capture backend for the current process.
pub fn detect_backend() -> CaptureBackend {
    let session_type = std::env::var("XDG_SESSION_TYPE").ok();
    let wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
    select_backend(
        std::env::consts::OS,
        session_type.as_deref(),
        wayland_display.as_deref(),
    )
}

// ─── xcap Grabber (universal; used for Wayland: KDE / GNOME / wlroots) ──────
//
// Replaces the old `grim` path, which only worked on wlroots compositors. xcap
// captures via the xdg-desktop-portal ScreenCast + PipeWire on Wayland, so it
// works across all major desktops. The portal shows a one-time output-picker
// dialog (a security feature we neither can nor should bypass).

/// Stateful screen grabber backed by `xcap`. Holds the monitor handle so the
/// portal/PipeWire session is negotiated once and reused across frames.
pub struct XcapGrabber {
    monitor: Option<xcap::Monitor>,
}

impl Default for XcapGrabber {
    fn default() -> Self {
        Self::new()
    }
}

impl XcapGrabber {
    pub fn new() -> Self {
        XcapGrabber { monitor: None }
    }

    /// Ensure a monitor handle exists; returns false if none can be acquired.
    fn ensure_monitor(&mut self) -> bool {
        if self.monitor.is_some() {
            return true;
        }
        match xcap::Monitor::all() {
            Ok(monitors) => {
                self.monitor = monitors.into_iter().next(); // primary / first
                self.monitor.is_some()
            }
            Err(_) => false,
        }
    }

    /// Capture the primary monitor as RGB at `STREAM_W` x `STREAM_H`.
    /// Returns `None` on failure (caller should retry).
    pub fn capture_rgb(&mut self) -> Option<Vec<u8>> {
        if !self.ensure_monitor() {
            return None;
        }
        let monitor = self.monitor.as_ref()?;
        let rgba = match monitor.capture_image() {
            Ok(img) => img,
            Err(_) => {
                // Drop the handle so the next call re-acquires (e.g. after a
                // monitor hotplug or portal-session drop).
                self.monitor = None;
                return None;
            }
        };
        let dynimg = image::DynamicImage::ImageRgba8(rgba);
        let resized =
            dynimg.resize_exact(STREAM_W, STREAM_H, image::imageops::FilterType::Triangle);
        Some(resized.to_rgb8().into_raw())
    }

    /// Construct only if at least one monitor can be enumerated.
    pub fn try_new() -> Option<Self> {
        let mut g = XcapGrabber::new();
        if g.ensure_monitor() {
            Some(g)
        } else {
            None
        }
    }
}

// ─── Unified capture: one trait, an ordered fallback chain per platform ─────
//
// Every OS/desktop in the support matrix reduces to a display server (X11 vs
// Wayland) plus, on Wayland, a portal backend. We don't special-case distros —
// we pick the proven grabber for the detected environment and, if it can't
// initialize, fall through to the next. `xcap` (portal/PipeWire) is the
// universal fallback that works even when the fast path is unavailable.

/// Yields RGB frames at `STREAM_W` x `STREAM_H`.
pub trait FrameGrabber {
    /// Grab one frame. `None` = transient failure; the caller should retry.
    fn grab(&mut self) -> Option<Vec<u8>>;
    /// Human-readable backend name, for diagnostics.
    fn name(&self) -> &'static str;
}

impl FrameGrabber for XcapGrabber {
    fn grab(&mut self) -> Option<Vec<u8>> {
        self.capture_rgb()
    }
    fn name(&self) -> &'static str {
        "xcap (portal / PipeWire)"
    }
}

/// Fast direct grabber for X11 (XShm) and macOS (CoreGraphics), via `scrap`.
pub struct ScrapGrabber {
    capturer: scrap::Capturer,
    w: u32,
    h: u32,
}

impl ScrapGrabber {
    pub fn try_new() -> Option<Self> {
        let display = scrap::Display::primary().ok()?;
        let capturer = scrap::Capturer::new(display).ok()?;
        let w = capturer.width() as u32;
        let h = capturer.height() as u32;
        Some(ScrapGrabber { capturer, w, h })
    }
}

impl FrameGrabber for ScrapGrabber {
    fn grab(&mut self) -> Option<Vec<u8>> {
        // scrap returns WouldBlock until the compositor delivers a frame.
        for _ in 0..100 {
            match self.capturer.frame() {
                Ok(frame) => {
                    return Some(resize_bgra_to_rgb(&frame, self.w, self.h, STREAM_W, STREAM_H));
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
                Err(_) => return None,
            }
        }
        None
    }
    fn name(&self) -> &'static str {
        "scrap (X11 XShm / CoreGraphics)"
    }
}

/// Windows GDI grabber (captures the cursor). Windows-only.
#[cfg(windows)]
pub struct GdiGrabber;

#[cfg(windows)]
impl GdiGrabber {
    pub fn try_new() -> Option<Self> {
        Some(GdiGrabber)
    }
}

#[cfg(windows)]
impl FrameGrabber for GdiGrabber {
    fn grab(&mut self) -> Option<Vec<u8>> {
        capture_windows()
    }
    fn name(&self) -> &'static str {
        "windows gdi (with cursor)"
    }
}

/// Build the best available grabber for the current environment, trying proven
/// backends in priority order and falling through to `xcap` if needed. Always
/// returns a grabber (a lazy `xcap` one as the last resort).
pub fn detect_grabber() -> Box<dyn FrameGrabber> {
    let backend = detect_backend();
    eprintln!("[*] Capture: {:?} session detected", backend);

    macro_rules! first_of {
        ($($ctor:expr),+ $(,)?) => {{
            $(
                if let Some(g) = $ctor {
                    eprintln!("[+] Capture backend: {}", g.name());
                    return Box::new(g);
                }
            )+
        }};
    }

    #[cfg(windows)]
    {
        let _ = backend;
        first_of!(GdiGrabber::try_new(), ScrapGrabber::try_new(), XcapGrabber::try_new());
    }
    #[cfg(target_os = "macos")]
    {
        let _ = backend;
        first_of!(ScrapGrabber::try_new(), XcapGrabber::try_new());
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        match backend {
            // Wayland: the portal is the only correct primary; scrap can still
            // work under XWayland as a fallback.
            CaptureBackend::LinuxWaylandPortal => {
                first_of!(XcapGrabber::try_new(), ScrapGrabber::try_new());
            }
            // X11: fast direct grabber first, portal as fallback.
            _ => {
                first_of!(ScrapGrabber::try_new(), XcapGrabber::try_new());
            }
        }
    }

    eprintln!("[-] No capture backend initialized; retrying via lazy xcap.");
    Box::new(XcapGrabber::new())
}

// ─── High-Performance BGRA Resizer ─────────────────────────────────────────

/// Resizes a BGRA image and converts it to RGB simultaneously.
pub fn resize_bgra_to_rgb(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    let mut dst = vec![0u8; (dw * dh * 3) as usize];
    let sw_usize = sw as usize;
    for y in 0..dh {
        let sy = ((y as u64 * sh as u64) / dh as u64) as usize;
        let dst_row = (y as usize) * (dw as usize) * 3;
        let src_row = sy * sw_usize * 4; // 4 bytes per pixel for BGRA
        for x in 0..dw {
            let sx = ((x as u64 * sw as u64) / dw as u64) as usize;
            let si = src_row + sx * 4;
            let di = dst_row + (x as usize) * 3;
            // BGRA -> RGB
            dst[di]     = src[si + 2]; // R
            dst[di + 1] = src[si + 1]; // G
            dst[di + 2] = src[si];     // B
        }
    }
    dst
}

/// Extracts a bounded rectangular tile from the main RGB screen buffer.
fn extract_tile(screen: &[u8], x: u16, y: u16, w: u16, h: u16) -> Vec<u8> {
    let sw = STREAM_W;
    let sh = STREAM_H;
    let cx = (x as u32).min(sw.saturating_sub(1));
    let cy = (y as u32).min(sh.saturating_sub(1));
    let cw = (w as u32).min(sw - cx);
    let ch = (h as u32).min(sh - cy);

    let mut rgb_buf = vec![0u8; (cw * ch * 3) as usize];
    let mut idx = 0;
    for row in cy..cy + ch {
        let src_row = (row as usize) * (sw as usize) * 3;
        for col in cx..cx + cw {
            let si = src_row + (col as usize) * 3;
            rgb_buf[idx] = screen[si];
            rgb_buf[idx + 1] = screen[si + 1];
            rgb_buf[idx + 2] = screen[si + 2];
            idx += 3;
        }
    }
    rgb_buf
}

/// Encode a tile, adaptively reducing quality until JPEG fits in max_size.
pub fn encode_tile_adaptive(
    screen: &[u8],
    x: u16,
    y: u16,
    w: u16,
    h: u16,
    max_size: usize,
) -> Vec<u8> {
    let cw = (w as u32).min(STREAM_W - (x as u32).min(STREAM_W.saturating_sub(1)));
    let ch = (h as u32).min(STREAM_H - (y as u32).min(STREAM_H.saturating_sub(1)));

    let rgb_buf = extract_tile(screen, x, y, w, h);

    let image = Image {
        pixels: rgb_buf.as_slice(),
        width: cw as usize,
        pitch: (cw * 3) as usize,
        height: ch as usize,
        format: PixelFormat::RGB,
    };

    let mut quality = JPEG_QUALITY;
    loop {
        let mut comp = Compressor::new().expect("turbojpeg");
        let _ = comp.set_quality(quality);
        let _ = comp.set_subsamp(Subsamp::Sub2x2); // 4:2:0 required

        let jpeg = comp.compress_to_vec(image.clone()).unwrap_or_default();

        if jpeg.len() <= max_size || quality <= 5 {
            return jpeg;
        }

        quality -= 5;
        if quality < 5 {
            quality = 5;
        }
    }
}

// ─── Windows GDI Capture (with cursor) ────────────────────────────────────

#[cfg(windows)]
/// Captures the screen on Windows using GDI, including the mouse cursor.
pub fn capture_windows() -> Option<Vec<u8>> {
    use std::ptr::null_mut;
    use winapi::um::wingdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDeviceCaps,
        GetDIBits, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
    };
    use winapi::um::winuser::{
        DrawIconEx, GetCursorInfo, GetDC, GetIconInfo, ReleaseDC, CURSORINFO, CURSOR_SHOWING,
        ICONINFO,
    };
    use winapi::shared::minwindef::TRUE;
    
    unsafe {
        let hdc_screen = GetDC(null_mut());
        if hdc_screen.is_null() {
            return None;
        }

        // 118 = DESKTOPHORZRES, 117 = DESKTOPVERTRES
        let width = GetDeviceCaps(hdc_screen, 118);
        let height = GetDeviceCaps(hdc_screen, 117);
        
        let width = if width == 0 { GetDeviceCaps(hdc_screen, 8) } else { width }; // fallback to HORZRES
        let height = if height == 0 { GetDeviceCaps(hdc_screen, 10) } else { height }; // fallback to VERTRES

        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hbm_screen = CreateCompatibleBitmap(hdc_screen, width, height);

        let hbm_old = SelectObject(hdc_mem, hbm_screen as *mut _);

        // Copy screen
        BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, 0, 0, SRCCOPY);

        // Draw cursor
        let mut ci: CURSORINFO = std::mem::zeroed();
        ci.cbSize = std::mem::size_of::<CURSORINFO>() as u32;
        if GetCursorInfo(&mut ci) == TRUE {
            if ci.flags == CURSOR_SHOWING {
                let mut ii: ICONINFO = std::mem::zeroed();
                if GetIconInfo(ci.hCursor, &mut ii) == TRUE {
                    // Offset by hotspot
                    let draw_x = ci.ptScreenPos.x - ii.xHotspot as i32;
                    let draw_y = ci.ptScreenPos.y - ii.yHotspot as i32;
                    DrawIconEx(
                        hdc_mem,
                        draw_x,
                        draw_y,
                        ci.hCursor,
                        0,
                        0,
                        0,
                        null_mut(),
                        3, // DI_NORMAL
                    );
                    
                    if !ii.hbmColor.is_null() { DeleteObject(ii.hbmColor as *mut _); }
                    if !ii.hbmMask.is_null() { DeleteObject(ii.hbmMask as *mut _); }
                }
            }
        }

        // Extract DIB bits
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = width;
        bmi.bmiHeader.biHeight = -height; // Top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let mut bgra_buf = vec![0u8; (width * height * 4) as usize];
        let res = GetDIBits(
            hdc_screen,
            hbm_screen,
            0,
            height as u32,
            bgra_buf.as_mut_ptr() as *mut _,
            &mut bmi,
            DIB_RGB_COLORS,
        );

        SelectObject(hdc_mem, hbm_old);
        DeleteObject(hbm_screen as *mut _);
        DeleteDC(hdc_mem);
        ReleaseDC(null_mut(), hdc_screen);

        if res == 0 {
            return None;
        }

        Some(crate::capture::resize_bgra_to_rgb(
            &bgra_buf,
            width as u32,
            height as u32,
            crate::STREAM_W,
            crate::STREAM_H,
        ))
    }
}
#[cfg(not(windows))]
/// Dummy implementation of Windows screen capture for non-Windows platforms.
pub fn capture_windows() -> Option<Vec<u8>> {
    None
}

