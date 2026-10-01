use turbojpeg::{Compressor, Image, PixelFormat, Subsamp};

use crate::{STREAM_W, STREAM_H, JPEG_QUALITY};

// ─── backend pick ───────────────────────────────────────────────────────────

// which screen grabber this os + session need
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CaptureBackend {
    // windows: gdi bitblt, cursor in
    WindowsGdi,
    // macos: coregraphics via scrap
    MacCoreGraphics,
    // x11: xshm via scrap
    LinuxX11,
    // wayland: portal screencast + pipewire
    LinuxWaylandPortal,
}

// pure pick from os + session env, so testable
pub fn select_backend(
    os: &str,
    session_type: Option<&str>,
    wayland_display: Option<&str>,
) -> CaptureBackend {
    match os {
        "windows" => CaptureBackend::WindowsGdi,
        "macos" => CaptureBackend::MacCoreGraphics,
        // linux, bsd: same x11 / wayland split
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

// pick backend for this process
pub fn detect_backend() -> CaptureBackend {
    let session_type = std::env::var("XDG_SESSION_TYPE").ok();
    let wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
    select_backend(
        std::env::consts::OS,
        session_type.as_deref(),
        wayland_display.as_deref(),
    )
}

// ─── xcap grabber (last resort) ─────────────────────────────────────────────

// xcap grabber, keeps monitor handle between frames
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

    // get monitor handle; false if none
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

    // grab main monitor as rgb at stream size; none = retry
    pub fn capture_rgb(&mut self) -> Option<Vec<u8>> {
        if !self.ensure_monitor() {
            return None;
        }
        let monitor = self.monitor.as_ref()?;
        let rgba = match monitor.capture_image() {
            Ok(img) => img,
            Err(_) => {
                // drop handle so next call re-acquires (hotplug, portal drop)
                self.monitor = None;
                return None;
            }
        };
        Some(fit_4ch_to_rgb(&rgba, rgba.width(), rgba.height(), [0, 1, 2]))
    }

    // build only if a monitor exists
    pub fn try_new() -> Option<Self> {
        let mut g = XcapGrabber::new();
        if g.ensure_monitor() {
            Some(g)
        } else {
            None
        }
    }
}

// rgb to jpeg via turbojpeg, for camera preview
pub fn encode_jpeg(rgb: &[u8], w: u32, h: u32, quality: i32) -> Option<Vec<u8>> {
    if rgb.len() < (w as usize) * (h as usize) * 3 {
        return None;
    }
    let image = Image {
        pixels: rgb,
        width: w as usize,
        pitch: (w * 3) as usize,
        height: h as usize,
        format: PixelFormat::RGB,
    };
    let mut comp = Compressor::new().ok()?;
    comp.set_quality(quality).ok()?;
    comp.set_subsamp(Subsamp::Sub2x2).ok()?;
    comp.compress_to_vec(image).ok()
}

// jpeg bytes (camera mjpeg frame) to rgb. same libjpeg-turbo as streamer, no second jpeg lib
pub fn decode_jpeg_rgb(jpeg: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let img = turbojpeg::decompress(jpeg, PixelFormat::RGB).ok()?;
    let (w, h) = (img.width, img.height);
    let rgb = if img.pitch == w * 3 {
        img.pixels
    } else {
        img.pixels.chunks(img.pitch).flat_map(|row| &row[..w * 3]).copied().collect()
    };
    Some((w as u32, h as u32, rgb))
}

// ─── one trait, fallback chain per os ───────────────────────────────────────

// gives rgb frames at stream size
pub trait FrameGrabber {
    // one frame; none = try again
    fn grab(&mut self) -> Option<Vec<u8>>;
    // backend name for logs
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

// wayland grabber: portal screencast + pipewire, pointer drawn in
#[cfg(target_os = "linux")]
pub struct PipeWireGrabber {
    stream: crate::screencast::PortalStream,
    last: Option<Vec<u8>>,
}

#[cfg(target_os = "linux")]
impl PipeWireGrabber {
    // start share session (desktop may show share dialog)
    pub fn try_new() -> Result<Self, crate::screencast::PortalError> {
        crate::screencast::PortalStream::start().map(|stream| PipeWireGrabber { stream, last: None })
    }
}

#[cfg(target_os = "linux")]
impl FrameGrabber for PipeWireGrabber {
    fn grab(&mut self) -> Option<Vec<u8>> {
        // pipewire only sends on change, so repeat newest
        let mut f = self.stream.take_latest();
        if f.is_none() && self.last.is_none() {
            // first frame: give compositor a moment
            std::thread::sleep(std::time::Duration::from_millis(50));
            f = self.stream.take_latest();
        }
        if let Some(f) = f {
            self.last = Some(fit_4ch_to_rgb(&f.rgba, f.width, f.height, [0, 1, 2]));
        }
        self.last.clone()
    }
    fn name(&self) -> &'static str {
        "portal screencast + PipeWire (cursor shown)"
    }
}

// fast grabber for x11 (xshm) + macos (coregraphics) via scrap
pub struct ScrapGrabber {
    capturer: scrap::Capturer,
    w: u32,
    h: u32,
    // x11 frames lack pointer, so we draw it
    #[cfg(target_os = "linux")]
    cursor: Option<crate::x11_cursor::CursorSource>,
}

impl ScrapGrabber {
    pub fn try_new() -> Option<Self> {
        let display = scrap::Display::primary().ok()?;
        let capturer = scrap::Capturer::new(display).ok()?;
        let w = capturer.width() as u32;
        let h = capturer.height() as u32;
        Some(ScrapGrabber {
            capturer,
            w,
            h,
            #[cfg(target_os = "linux")]
            cursor: crate::x11_cursor::CursorSource::new(),
        })
    }
}

impl FrameGrabber for ScrapGrabber {
    fn grab(&mut self) -> Option<Vec<u8>> {
        // scrap says wouldblock till compositor has frame
        for _ in 0..100 {
            match self.capturer.frame() {
                Ok(frame) => {
                    #[allow(unused_mut)]
                    let mut rgb = fit_bgra_to_rgb(&frame, self.w, self.h);
                    #[cfg(target_os = "linux")]
                    if let Some(c) = &self.cursor {
                        c.draw_into_rgb(&mut rgb, self.w, self.h);
                    }
                    return Some(rgb);
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

// windows gdi grabber, cursor in
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

// best grabber for this env in proven order, xcap last. err only when user refuse share
pub fn detect_grabber() -> Result<Box<dyn FrameGrabber>, String> {
    let backend = detect_backend();
    eprintln!("[*] Capture: {:?} session detected", backend);

    macro_rules! first_of {
        ($($ctor:expr),+ $(,)?) => {{
            $(
                if let Some(g) = $ctor {
                    eprintln!("[+] Capture backend: {}", g.name());
                    return Ok(Box::new(g));
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
            // wayland: portal first; scrap under xwayland as fallback
            CaptureBackend::LinuxWaylandPortal => {
                #[cfg(target_os = "linux")]
                match PipeWireGrabber::try_new() {
                    Ok(g) => {
                        eprintln!("[+] Capture backend: {}", g.name());
                        return Ok(Box::new(g));
                    }
                    Err(crate::screencast::PortalError::Cancelled) => {
                        return Err("Screen sharing was refused. Allow it when your desktop asks, then cast again.".to_string());
                    }
                    Err(e) => eprintln!("[*] Screen sharing unavailable: {e}"),
                }
                // no xwayland scrap: it only sees x11 windows, so wayland screen comes out black.
                // screenshot per frame instead (wlroots screencopy; kde/gnome may ask every frame)
                first_of!(XcapGrabber::try_new());
                #[cfg(target_os = "linux")]
                return Err(
                    "Screen sharing is not available. Install xdg-desktop-portal and the portal for your desktop \
                     (xdg-desktop-portal-gnome, -kde, -wlr or -hyprland), log out and back in, then cast again."
                        .to_string(),
                );
            }
            // x11: direct grabber first, portal fallback
            _ => {
                first_of!(ScrapGrabber::try_new(), XcapGrabber::try_new());
            }
        }
    }

    #[allow(unreachable_code)]
    Err("LibreMP could not capture the screen on this system.".to_string())
}

// ─── bgra resize ────────────────────────────────────────────────────────────

// frame part projector really shows: it cuts ~5.5% off every edge (overscan, seen in grid photo of RESEARCHLAB)
pub const VISIBLE: (u32, u32, u32, u32) = (56, 42, 912, 684);
// projector shows the 4:3 frame stretched to 16:9, so one frame pixel looks 4/3 wide (circle test).
// ponytail: both measured on RESEARCHLAB only; make them a per-projector setting if another projector differs
const PIXEL_ASPECT: (u64, u64) = (4, 3);

// where sw x sh screen sits in stream frame so it looks right on the wall: inside visible part, centered, even edges.
// 16:9 screen fills all of it (stretched in frame, undone by projector)
pub fn fit_rect(sw: u32, sh: u32) -> (u32, u32, u32, u32) {
    let (vx, vy, vw, vh) = VISIBLE;
    let (pn, pd) = PIXEL_ASPECT;
    let (sw, sh, fw, fh) = (sw.max(1) as u64, sh.max(1) as u64, vw as u64, vh as u64);
    // screen wider than visible part looks on wall: fill width, else fill height
    let (w, h) = if sw * fh * pd >= sh * fw * pn { (fw, sh * fw * pn / (sw * pd)) } else { (sw * fh * pd / (sh * pn), fh) };
    let (w, h) = ((w.min(fw) as u32 & !1).max(2), (h.min(fh) as u32 & !1).max(2));
    (vx + (((vw - w) / 2) & !1), vy + (((vh - h) / 2) & !1), w, h)
}

// bgra screen into stream frame
pub fn fit_bgra_to_rgb(src: &[u8], sw: u32, sh: u32) -> Vec<u8> {
    fit_4ch_to_rgb(src, sw, sh, [2, 1, 0])
}

// 4-channel screen into black stream frame at fit_rect. area average, so thin text survives shrink; `rgb` = r, g, b byte offsets
fn fit_4ch_to_rgb(src: &[u8], sw: u32, sh: u32, rgb: [usize; 3]) -> Vec<u8> {
    let fw = STREAM_W as usize;
    let mut dst = vec![0u8; fw * STREAM_H as usize * 3];
    let (ox, oy, w, h) = fit_rect(sw, sh);
    let (sw, sh, ox, oy, w, h) = (sw as usize, sh as usize, ox as usize, oy as usize, w as usize, h as usize);
    if sw == 0 || sh == 0 || src.len() < sw * sh * 4 {
        return dst;
    }
    // source pixels [a, b) behind output pixel i of n
    let span = |i: usize, n: usize, s: usize| (i * s / n, ((i + 1) * s / n).max(i * s / n + 1));
    let cols: Vec<(usize, usize)> = (0..w).map(|x| span(x, w, sw)).collect();
    for y in 0..h {
        let (y0, y1) = span(y, h, sh);
        let out = (oy + y) * fw + ox;
        for (x, &(x0, x1)) in cols.iter().enumerate() {
            let mut acc = [0u32; 3];
            for line in src[y0 * sw * 4..y1 * sw * 4].chunks_exact(sw * 4) {
                for p in line[x0 * 4..x1 * 4].as_chunks::<4>().0 {
                    acc[0] += p[rgb[0]] as u32;
                    acc[1] += p[rgb[1]] as u32;
                    acc[2] += p[rgb[2]] as u32;
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as u32;
            for k in 0..3 {
                dst[(out + x) * 3 + k] = ((acc[k] + n / 2) / n) as u8;
            }
        }
    }
    dst
}

// copy one tile out of screen rgb
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

// one tile to jpeg at fixed quality (4:2:0, projector wants it). big frame just sends slower, tcp paces it
pub fn encode_tile(screen: &[u8], x: u16, y: u16, w: u16, h: u16) -> Vec<u8> {
    let cw = (w as u32).min(STREAM_W - (x as u32).min(STREAM_W.saturating_sub(1)));
    let ch = (h as u32).min(STREAM_H - (y as u32).min(STREAM_H.saturating_sub(1)));
    let rgb_buf = extract_tile(screen, x, y, w, h);
    encode_jpeg(&rgb_buf, cw, ch, JPEG_QUALITY).unwrap_or_default()
}

// ─── windows gdi capture ────────────────────────────────────────────────────

#[cfg(windows)]
// windows screen via gdi, cursor drawn in
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

    // SAFETY: every GDI handle is null-checked or created here and released before
    // return; `ci`/`ii`/`bmi` are zeroed plain-old-data structs with their size
    // fields set as the API requires; `bgra_buf` holds exactly width*height*4
    // bytes, matching the 32-bpp top-down DIB that GetDIBits writes into it.
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
        if GetCursorInfo(&mut ci) == TRUE && ci.flags == CURSOR_SHOWING {
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

        Some(fit_bgra_to_rgb(&bgra_buf, width as u32, height as u32))
    }
}
#[cfg(not(windows))]
// non-windows stub
pub fn capture_windows() -> Option<Vec<u8>> {
    None
}


#[cfg(test)]
mod tests {
    use super::*;

    // 16:9 fills visible part; 16:10 and 4:3 get side bars, ultrawide gets top/bottom bars
    #[test]
    fn fit_rect_looks_right_on_wall() {
        assert_eq!(fit_rect(1920, 1080), VISIBLE);
        assert_eq!(fit_rect(1920, 1200), (102, 42, 820, 684));
        assert_eq!(fit_rect(1024, 768), (170, 42, 684, 684));
        assert_eq!(fit_rect(2560, 1080), (56, 128, 912, 512));
    }

    // fine black/white stripes average to gray, not lost to dropped pixels; bars stay black
    #[test]
    fn shrink_averages_and_letterboxes() {
        let (sw, sh) = (2048u32, 1152u32);
        let mut src = vec![0u8; (sw * sh * 4) as usize];
        for (i, px) in src.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            if i % 2 == 0 {
                px[..3].copy_from_slice(&[255, 255, 255]);
            }
        }
        let out = fit_bgra_to_rgb(&src, sw, sh);
        let at = |x: usize, y: usize| out[(y * STREAM_W as usize + x) * 3];
        assert_eq!(at(500, 0), 0, "top bar black");
        assert_eq!(at(500, 767), 0, "bottom bar black");
        assert!((at(500, 400) as i32 - 128).abs() <= 1, "stripes average to gray, got {}", at(500, 400));
    }
}
