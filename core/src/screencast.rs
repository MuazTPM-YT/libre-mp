//! Wayland screen capture: xdg-desktop-portal ScreenCast + PipeWire.
//!
//! Why this exists instead of `xcap`'s recorder:
//!
//! 1. **The mouse pointer.** The portal hides it unless the session asks for
//!    `cursor_mode = EMBEDDED`. A presentation without a pointer is useless, so
//!    we ask for it. (The Windows GDI path draws the cursor for the same reason.)
//! 2. **No X11.** `xcap::Monitor::all()` talks XCB, so it fails on a Wayland
//!    session with XWayland switched off. The portal picks the output itself, so
//!    nothing here needs an X server.
//! 3. **Remembering the choice.** With `persist_mode` the compositor hands back a
//!    restore token, so the second run casts without asking again.
//!
//! This works on any desktop with a portal backend: KDE, GNOME, wlroots
//! (Hyprland/Sway), and anything else implementing `org.freedesktop.portal.ScreenCast`.

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex};

use pipewire::{
    context::ContextRc,
    keys::{MEDIA_CATEGORY, MEDIA_ROLE, MEDIA_TYPE},
    main_loop::MainLoopRc,
    properties,
    spa::{
        param::{
            format::{FormatProperties, MediaSubtype, MediaType},
            format_utils,
            video::{VideoFormat, VideoInfoRaw},
            ParamType,
        },
        pod::{self, serialize::PodSerializer, Pod},
        utils::{Direction, Fraction, Rectangle, SpaTypes},
    },
    stream::{StreamFlags, StreamRc},
};
use zbus::{
    blocking::{Connection, Proxy},
    zvariant::{DeserializeDict, OwnedObjectPath, Type, Value},
};

/// Why a screen share could not start.
pub enum PortalError {
    /// The user said no (or closed the dialog). Falling back to another capture
    /// method would project a black screen, so callers must give up instead.
    Cancelled,
    /// No portal, no PipeWire, or the desktop failed — another method may work.
    Unavailable(String),
}

impl std::fmt::Display for PortalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortalError::Cancelled => write!(f, "screen sharing was refused"),
            PortalError::Unavailable(e) => write!(f, "{e}"),
        }
    }
}

impl From<String> for PortalError {
    fn from(e: String) -> Self {
        PortalError::Unavailable(e)
    }
}

/// One captured frame, tightly packed RGBA.
pub struct PortalFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(DeserializeDict, Type, Debug)]
#[zvariant(signature = "dict")]
struct CreateSessionResponse {
    session_handle: String,
}

#[allow(dead_code)]
#[derive(DeserializeDict, Type, Debug)]
#[zvariant(signature = "dict")]
struct StartStream {
    id: Option<String>,
    position: Option<(i32, i32)>,
    size: Option<(i32, i32)>,
    source_type: Option<u32>,
    mapping_id: Option<String>,
}

#[derive(DeserializeDict, Type, Debug)]
#[zvariant(signature = "dict")]
struct StartResponse {
    streams: Option<Vec<(u32, StartStream)>>,
    restore_token: Option<String>,
}

#[derive(DeserializeDict, Type, Debug)]
#[zvariant(signature = "dict")]
struct EmptyResponse {}

/// A running screen-share session. Dropping it stops the PipeWire loop.
pub struct PortalStream {
    latest: Arc<Mutex<Option<PortalFrame>>>,
    quit: pipewire::channel::Sender<()>,
    /// The portal ties the session to this D-Bus connection: drop it and the
    /// compositor closes the session, so it must outlive the stream.
    _conn: Connection,
}

impl Drop for PortalStream {
    fn drop(&mut self) {
        let _ = self.quit.send(());
    }
}

impl PortalStream {
    /// Asks the desktop to share a screen, then starts receiving frames.
    /// Blocks while the user answers the portal dialog.
    pub fn start() -> Result<Self, PortalError> {
        let conn = Connection::session().map_err(|e| format!("no session D-Bus: {e}"))?;
        let proxy = Proxy::new(
            &conn,
            "org.freedesktop.portal.Desktop",
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.ScreenCast",
        )
        .map_err(|e| format!("no ScreenCast portal: {e}"))?;

        eprintln!("[*] Waiting for you to allow screen sharing...");
        let session = create_session(&conn, &proxy).map_err(classify)?;
        let token = load_restore_token();
        select_sources(&conn, &proxy, &session, token.as_deref()).map_err(classify)?;
        let response = start_session(&conn, &proxy, &session).map_err(classify)?;

        if let Some(t) = &response.restore_token {
            save_restore_token(t);
        }
        let node_id = response
            .streams
            .and_then(|s| s.first().map(|(id, _)| *id))
            .ok_or_else(|| "the desktop shared no screen".to_string())?;

        // Portal-created nodes live on the PipeWire connection the portal hands
        // us, not on the session's default one.
        let fd = open_pipewire_remote(&proxy, &session)?;

        let latest = Arc::new(Mutex::new(None));
        let quit = spawn_pipewire(fd, node_id, latest.clone())?;
        Ok(PortalStream { latest, quit, _conn: conn })
    }

    /// The newest frame since the last call, or `None` if nothing changed.
    pub fn take_latest(&self) -> Option<PortalFrame> {
        self.latest.lock().ok()?.take()
    }
}

/// Turns the internal cancellation marker into the typed error.
fn classify(e: String) -> PortalError {
    if e == CANCELLED {
        PortalError::Cancelled
    } else {
        PortalError::Unavailable(e)
    }
}

// ─── Portal (D-Bus) ─────────────────────────────────────────────────────────

/// Portal request/session handle tokens only have to be unique per connection.
fn token(kind: &str) -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    format!("libremp_{kind}_{}_{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed))
}

/// Proxy for the `Request` object a portal call will answer on. Created (and its
/// signal match registered) *before* the call, so the reply cannot be missed.
fn request_proxy(conn: &Connection, handle_token: &str) -> Result<Proxy<'static>, String> {
    let unique = conn
        .unique_name()
        .ok_or_else(|| "D-Bus connection has no name".to_string())?
        .trim_start_matches(':')
        .replace('.', "_");
    Proxy::new_owned(
        conn.clone(),
        "org.freedesktop.portal.Desktop".to_string(),
        format!("/org/freedesktop/portal/desktop/request/{unique}/{handle_token}"),
        "org.freedesktop.portal.Request".to_string(),
    )
    .map_err(|e| format!("portal request proxy: {e}"))
}

/// Marker the portal helpers use to report "the user said no" through `String`.
const CANCELLED: &str = "@cancelled";

/// Blocks until the portal answers, then decodes the reply.
fn wait_response<T>(signals: &mut zbus::blocking::proxy::SignalIterator<'_>) -> Result<T, String>
where
    T: for<'de> serde::Deserialize<'de> + Type,
{
    let message = signals.next().ok_or_else(|| "the portal closed".to_string())?;
    let body = message.body();
    let (code, value): (u32, T) = body
        .deserialize()
        .map_err(|e| format!("portal reply: {e}"))?;
    match code {
        0 => Ok(value),
        1 => Err(CANCELLED.to_string()),
        n => Err(format!("the desktop refused screen sharing (code {n})")),
    }
}

fn create_session(conn: &Connection, proxy: &Proxy<'_>) -> Result<OwnedObjectPath, String> {
    let handle = token("req");
    let session_token = token("session");
    let request = request_proxy(conn, &handle)?;
    let mut signals = request
        .receive_signal("Response")
        .map_err(|e| format!("portal signal: {e}"))?;

    let mut options = HashMap::new();
    options.insert("handle_token", Value::from(&handle));
    options.insert("session_handle_token", Value::from(&session_token));
    proxy
        .call_method("CreateSession", &(options,))
        .map_err(|e| format!("CreateSession: {e}"))?;

    let response: CreateSessionResponse = wait_response(&mut signals)?;
    OwnedObjectPath::try_from(response.session_handle)
        .map_err(|e| format!("bad session handle: {e}"))
}

fn select_sources(
    conn: &Connection,
    proxy: &Proxy<'_>,
    session: &OwnedObjectPath,
    restore_token: Option<&str>,
) -> Result<(), String> {
    let handle = token("req");
    let request = request_proxy(conn, &handle)?;
    let mut signals = request
        .receive_signal("Response")
        .map_err(|e| format!("portal signal: {e}"))?;

    let mut options = HashMap::new();
    options.insert("handle_token", Value::from(&handle));
    options.insert("types", Value::from(1_u32)); // monitors
    options.insert("multiple", Value::from(false));
    // 2 = EMBEDDED: draw the mouse pointer into the frames.
    options.insert("cursor_mode", Value::from(2_u32));
    // 2 = PERSISTENT: remember this choice so the next run does not ask again.
    options.insert("persist_mode", Value::from(2_u32));
    if let Some(t) = restore_token {
        options.insert("restore_token", Value::from(t));
    }

    if let Err(e) = proxy.call_method("SelectSources", &(session, options)) {
        return Err(format!("SelectSources: {e}"));
    }
    let _: EmptyResponse = wait_response(&mut signals)?;
    Ok(())
}

fn start_session(
    conn: &Connection,
    proxy: &Proxy<'_>,
    session: &OwnedObjectPath,
) -> Result<StartResponse, String> {
    let handle = token("req");
    let request = request_proxy(conn, &handle)?;
    let mut signals = request
        .receive_signal("Response")
        .map_err(|e| format!("portal signal: {e}"))?;

    let mut options = HashMap::new();
    options.insert("handle_token", Value::from(&handle));
    proxy
        .call_method("Start", &(session, "", options))
        .map_err(|e| format!("Start: {e}"))?;

    wait_response(&mut signals)
}

/// The portal's own PipeWire connection.
fn open_pipewire_remote(
    proxy: &Proxy<'_>,
    session: &OwnedObjectPath,
) -> Result<std::os::fd::OwnedFd, String> {
    let options: HashMap<&str, Value<'_>> = HashMap::new();
    let fd: zbus::zvariant::OwnedFd = proxy
        .call("OpenPipeWireRemote", &(session, options))
        .map_err(|e| format!("OpenPipeWireRemote: {e}"))?;
    Ok(fd.into())
}

/// Where the compositor's "share this screen again" token is kept.
fn restore_token_path() -> Option<std::path::PathBuf> {
    Some(crate::config::config_path()?.with_file_name("screencast_token"))
}

fn load_restore_token() -> Option<String> {
    let t = std::fs::read_to_string(restore_token_path()?).ok()?;
    let t = t.trim().to_string();
    (!t.is_empty()).then_some(t)
}

fn save_restore_token(token: &str) {
    if let Some(p) = restore_token_path() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, token);
    }
}

// ─── PipeWire ───────────────────────────────────────────────────────────────

/// Runs the PipeWire loop on its own thread, storing the newest frame.
fn spawn_pipewire(
    fd: std::os::fd::OwnedFd,
    node_id: u32,
    latest: Arc<Mutex<Option<PortalFrame>>>,
) -> Result<pipewire::channel::Sender<()>, String> {
    let (quit_tx, quit_rx) = pipewire::channel::channel::<()>();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();

    std::thread::spawn(move || {
        let run = || -> Result<(MainLoopRc, StreamRc, Vec<u8>), String> {
            pipewire::init();
            let main_loop = MainLoopRc::new(None).map_err(|e| format!("pipewire loop: {e}"))?;
            let context = ContextRc::new(&main_loop, None).map_err(|e| format!("pipewire context: {e}"))?;
            let core = context
                .connect_fd_rc(fd, None)
                .map_err(|e| format!("pipewire connect: {e}"))?;
            let stream = StreamRc::new(
                core.clone(),
                "LibreMP",
                properties::properties! {
                    *MEDIA_TYPE => "Video",
                    *MEDIA_CATEGORY => "Capture",
                    *MEDIA_ROLE => "Screen",
                },
            )
            .map_err(|e| format!("pipewire stream: {e}"))?;
            Ok((main_loop, stream, Vec::new()))
        };

        let (main_loop, stream, _) = match run() {
            Ok(v) => v,
            Err(e) => {
                let _ = ready_tx.send(Err(e));
                return;
            }
        };

        let listener = stream
            .add_local_listener_with_user_data(VideoInfoRaw::default())
            .param_changed(|_, info, id, param| {
                let Some(param) = param else { return };
                if id != ParamType::Format.as_raw() {
                    return;
                }
                if let Ok((MediaType::Video, MediaSubtype::Raw)) = format_utils::parse_format(param) {
                    let _ = info.parse(param);
                    let size = info.size();
                    eprintln!("[+] Screen share: {}x{} {:?}", size.width, size.height, info.format());
                }
            })
            .process({
                let latest = latest.clone();
                move |stream, info| {
                    let Some(mut buffer) = stream.dequeue_buffer() else { return };
                    let datas = buffer.datas_mut();
                    let Some(data) = datas.first_mut() else { return };
                    let stride = data.chunk().stride().max(0) as usize;
                    let size = info.size();
                    let Some(bytes) = data.data() else { return };
                    if let Some(frame) = to_rgba(bytes, stride, size.width, size.height, info.format()) {
                        if let Ok(mut slot) = latest.lock() {
                            *slot = Some(frame);
                        }
                    }
                }
            })
            .register()
            .map_err(|e| format!("pipewire listener: {e}"));
        let _listener = match listener {
            Ok(l) => l,
            Err(e) => {
                let _ = ready_tx.send(Err(e));
                return;
            }
        };

        let values = match PodSerializer::serialize(Cursor::new(Vec::new()), &pod::Value::Object(format_pod())) {
            Ok(v) => v.0.into_inner(),
            Err(e) => {
                let _ = ready_tx.send(Err(format!("pipewire format: {e}")));
                return;
            }
        };
        let Some(pod) = Pod::from_bytes(&values) else {
            let _ = ready_tx.send(Err("pipewire format pod".to_string()));
            return;
        };
        let mut params = [pod];
        if let Err(e) = stream.connect(
            Direction::Input,
            Some(node_id),
            StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS,
            &mut params,
        ) {
            let _ = ready_tx.send(Err(format!("pipewire stream connect: {e}")));
            return;
        }

        let _quit = quit_rx.attach(main_loop.loop_(), {
            let main_loop = main_loop.clone();
            move |_| main_loop.quit()
        });

        let _ = ready_tx.send(Ok(()));
        main_loop.run();
    });

    ready_rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .map_err(|_| "PipeWire did not start".to_string())??;
    Ok(quit_tx)
}

/// The formats we accept. The compositor picks one of them.
fn format_pod() -> pod::Object {
    pod::object!(
        SpaTypes::ObjectParamFormat,
        ParamType::EnumFormat,
        pod::property!(FormatProperties::MediaType, Id, MediaType::Video),
        pod::property!(FormatProperties::MediaSubtype, Id, MediaSubtype::Raw),
        pod::property!(
            FormatProperties::VideoFormat,
            Choice,
            Enum,
            Id,
            VideoFormat::RGBx,
            VideoFormat::BGRx,
            VideoFormat::RGBA,
            VideoFormat::BGRA,
            VideoFormat::RGB,
            VideoFormat::BGR,
        ),
        pod::property!(
            FormatProperties::VideoSize,
            Choice,
            Range,
            Rectangle,
            Rectangle { width: 1920, height: 1080 },
            Rectangle { width: 1, height: 1 },
            Rectangle { width: 8192, height: 8192 }
        ),
        pod::property!(
            FormatProperties::VideoFramerate,
            Choice,
            Range,
            Fraction,
            Fraction { num: 30, denom: 1 },
            Fraction { num: 0, denom: 1 },
            Fraction { num: 1000, denom: 1 }
        ),
    )
}

/// Converts one PipeWire buffer into tightly packed RGBA, honouring the row
/// stride (which is often wider than the image on real hardware).
fn to_rgba(bytes: &[u8], stride: usize, width: u32, height: u32, format: VideoFormat) -> Option<PortalFrame> {
    let (w, h) = (width as usize, height as usize);
    if w == 0 || h == 0 {
        return None;
    }
    let (src_px, swap_rb) = match format {
        VideoFormat::RGBx | VideoFormat::RGBA => (4, false),
        VideoFormat::BGRx | VideoFormat::BGRA => (4, true),
        VideoFormat::RGB => (3, false),
        VideoFormat::BGR => (3, true),
        _ => return None,
    };
    let row_bytes = w * src_px;
    let stride = if stride >= row_bytes { stride } else { row_bytes };
    if bytes.len() < (h - 1) * stride + row_bytes {
        return None;
    }

    let mut rgba = vec![255u8; w * h * 4];
    for y in 0..h {
        let src = &bytes[y * stride..y * stride + row_bytes];
        let dst = &mut rgba[y * w * 4..(y + 1) * w * 4];
        for (s, d) in src.chunks_exact(src_px).zip(dst.chunks_exact_mut(4)) {
            if swap_rb {
                d[0] = s[2];
                d[1] = s[1];
                d[2] = s[0];
            } else {
                d[0] = s[0];
                d[1] = s[1];
                d[2] = s[2];
            }
        }
    }
    Some(PortalFrame { width, height, rgba })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stride padding and byte order are the two things that silently produce a
    /// skewed or blue-tinted projection, so pin them down.
    #[test]
    fn converts_padded_bgrx_rows_to_rgba() {
        // 2x2 image, stride padded by 4 bytes per row. Pixels are BGRx.
        let stride = 2 * 4 + 4;
        let mut buf = vec![0u8; stride * 2];
        for y in 0..2 {
            for x in 0..2 {
                let p = y * stride + x * 4;
                buf[p] = 10; // B
                buf[p + 1] = 20; // G
                buf[p + 2] = 30; // R
            }
        }
        let f = to_rgba(&buf, stride, 2, 2, VideoFormat::BGRx).unwrap();
        assert_eq!(f.rgba.len(), 2 * 2 * 4);
        assert_eq!(&f.rgba[0..4], &[30, 20, 10, 255]);
        // Last pixel must come from the second row, not from the padding.
        assert_eq!(&f.rgba[12..16], &[30, 20, 10, 255]);
    }

    #[test]
    fn rejects_short_buffers() {
        assert!(to_rgba(&[0u8; 8], 8, 4, 4, VideoFormat::RGBx).is_none());
    }
}
