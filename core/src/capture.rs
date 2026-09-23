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
        let dynimg = image::DynamicImage::ImageRgba8(rgba);
        let resized =
            dynimg.resize_exact(STREAM_W, STREAM_H, image::imageops::FilterType::Triangle);
        Some(resized.to_rgb8().into_raw())
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
        if let Some(f) = self.stream.take_latest() {
            self.last = Some(resize_4ch_to_rgb(&f.rgba, f.width, f.height, STREAM_W, STREAM_H, [0, 1, 2]));
        } else if self.last.is_none() {
            // first frame: give compositor a moment
            std::thread::sleep(std::time::Duration::from_millis(50));
            if let Some(f) = self.stream.take_latest() {
                self.last = Some(resize_4ch_to_rgb(&f.rgba, f.width, f.height, STREAM_W, STREAM_H, [0, 1, 2]));
            }
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
                    let mut rgb = resize_bgra_to_rgb(&frame, self.w, self.h, STREAM_W, STREAM_H);
                    #[cfg(target_os = "linux")]
                    if let Some(c) = &self.cursor {
                        c.draw_into_rgb(&mut rgb, STREAM_W, STREAM_H, self.w, self.h);
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
                // then xwayland; screenshot-per-frame last (kde/gnome may ask every frame)
                first_of!(ScrapGrabber::try_new(), XcapGrabber::try_new());
            }
            // x11: direct grabber first, portal fallback
            _ => {
                first_of!(ScrapGrabber::try_new(), XcapGrabber::try_new());
            }
        }
    }

    eprintln!("[-] No capture backend initialized; retrying via lazy xcap.");
    Ok(Box::new(XcapGrabber::new()))
}

// ─── bgra resize ────────────────────────────────────────────────────────────

// resize bgra to rgb in one pass
pub fn resize_bgra_to_rgb(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    resize_4ch_to_rgb(src, sw, sh, dw, dh, [2, 1, 0])
}

// nearest-neighbour resize 4-channel to rgb; `rgb` = r, g, b byte offsets
fn resize_4ch_to_rgb(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32, rgb: [usize; 3]) -> Vec<u8> {
    let mut dst = vec![0u8; (dw * dh * 3) as usize];
    let sw_usize = sw as usize;
    for y in 0..dh {
        let sy = ((y as u64 * sh as u64) / dh as u64) as usize;
        let dst_row = (y as usize) * (dw as usize) * 3;
        let src_row = sy * sw_usize * 4; // 4 bytes per pixel
        for x in 0..dw {
            let sx = ((x as u64 * sw as u64) / dw as u64) as usize;
            let si = src_row + sx * 4;
            let di = dst_row + (x as usize) * 3;
            dst[di] = src[si + rgb[0]];
            dst[di + 1] = src[si + rgb[1]];
            dst[di + 2] = src[si + rgb[2]];
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

// encode tile, drop quality till jpeg fits max_size
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

        let jpeg = comp.compress_to_vec(image).unwrap_or_default();

        if jpeg.len() <= max_size || quality <= 5 {
            return jpeg;
        }

        quality -= 5;
        if quality < 5 {
            quality = 5;
        }
    }
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
// non-windows stub
pub fn capture_windows() -> Option<Vec<u8>> {
    None
}

