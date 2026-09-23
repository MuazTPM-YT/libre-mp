//! LibreMP desktop backend: tauri commands over libremp-core. cast runs in-process.

use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use libremp_core::config::{self, SavedProjector, SavedProjectors};
use libremp_core::session::{self, CastError, CastOptions, FailKind};
use libremp_core::wifi::{self, WifiNetwork};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

// jobs for camera thread. it owns the camera (not Send), so handle never lives in state
enum CamCmd {
    Preview(tokio::sync::oneshot::Sender<Result<String, String>>),
    Capture(tokio::sync::oneshot::Sender<Result<String, String>>),
    Scan(tokio::sync::oneshot::Sender<Result<QrProjector, String>>),
    Stop,
}

// running cast: stop flag + its thread
struct Cast {
    running: Arc<AtomicBool>,
    thread: std::thread::JoinHandle<()>,
}

// app state. locks never held across await
#[derive(Default)]
pub struct AppState {
    cast: Mutex<Option<Cast>>,
    // wi-fi we were on before joining projector: nm uuid, windows profile, or macos ssid
    prev_wifi: Mutex<Option<String>>,
    cam_tx: Mutex<Option<mpsc::Sender<CamCmd>>>,
}

// lock that shrugs off poison, state stays usable after a panic
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

// run blocking work off async threads
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())
}

// projector seen on lan
#[derive(Debug, Serialize, Clone)]
pub struct ProjectorInfo {
    pub name: String,
    pub ip: String,
}

// wi-fi networks around us
#[tauri::command]
async fn scan_wifi_networks() -> Result<Vec<WifiNetwork>, String> {
    blocking(wifi::scan).await?
}

// what epson qr hold
#[derive(Debug, Serialize)]
pub struct QrProjector {
    pub ssid: String,
    pub password: String,
    pub ip: String,
}

impl From<libremp_core::qr::EpsonQr> for QrProjector {
    fn from(qr: libremp_core::qr::EpsonQr) -> Self {
        QrProjector {
            ssid: qr.ssid().unwrap_or_default().to_string(),
            password: qr.wifi_password().unwrap_or_default().to_string(),
            ip: qr.ip.to_string(),
        }
    }
}

// read epson qr from uploaded photo bytes
#[tauri::command]
async fn decode_projector_qr(image_bytes: Vec<u8>) -> Result<QrProjector, String> {
    blocking(move || libremp_core::qr::parse_from_image_bytes(&image_bytes))
        .await?
        .map(QrProjector::from)
        .ok_or_else(|| "No projector QR code found. Use a sharp photo where the QR fills most of the picture.".into())
}

// open camera if needed, grab one rgb frame
fn grab_rgb(guard: &mut Option<nokhwa::Camera>) -> Result<(u32, u32, Vec<u8>), String> {
    use nokhwa::pixel_format::RgbFormat;
    use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};

    if guard.is_none() {
        let format = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestResolution);
        let mut camera =
            nokhwa::Camera::new(CameraIndex::Index(0), format).map_err(|e| format!("Could not open the camera: {e}"))?;
        camera.open_stream().map_err(|e| format!("Could not start the camera: {e}"))?;
        *guard = Some(camera);
    }
    let camera = guard.as_mut().ok_or("Camera is not open.")?;
    let frame = camera.frame().map_err(|e| e.to_string())?;
    if frame.source_frame_format() == nokhwa::utils::FrameFormat::MJPEG {
        return libremp_core::capture::decode_jpeg_rgb(frame.buffer()).ok_or_else(|| "Could not read the camera frame.".into());
    }
    let img = frame.decode_image::<RgbFormat>().map_err(|e| e.to_string())?;
    Ok((img.width(), img.height(), img.into_raw()))
}

// rgb frame to jpeg data: url for <img>
fn jpeg_data_url(rgb: &[u8], w: u32, h: u32, quality: i32) -> Result<String, String> {
    use base64::Engine as _;
    let jpeg = libremp_core::capture::encode_jpeg(rgb, w, h, quality).ok_or("Could not encode the camera frame.")?;
    Ok(format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(&jpeg)))
}

// camera thread: own camera + last still, serve jobs till stop. drop frees device
fn camera_worker(rx: mpsc::Receiver<CamCmd>) {
    let mut cam: Option<nokhwa::Camera> = None;
    let mut still: Option<(u32, u32, Vec<u8>)> = None;
    while let Ok(cmd) = rx.recv() {
        match cmd {
            CamCmd::Preview(reply) => {
                let _ = reply.send(grab_rgb(&mut cam).and_then(|(w, h, rgb)| jpeg_data_url(&rgb, w, h, 55)));
            }
            CamCmd::Capture(reply) => {
                let r = grab_rgb(&mut cam).and_then(|(w, h, rgb)| {
                    let url = jpeg_data_url(&rgb, w, h, 88)?;
                    still = Some((w, h, rgb));
                    Ok(url)
                });
                let _ = reply.send(r);
            }
            CamCmd::Scan(reply) => {
                let r = match &still {
                    None => Err("Take a photo first.".to_string()),
                    Some((w, h, rgb)) => libremp_core::qr::parse_from_rgb(*w, *h, rgb).map(QrProjector::from).ok_or_else(|| {
                        "No projector QR code found. Retake the photo with the QR filling more of the frame.".to_string()
                    }),
                };
                let _ = reply.send(r);
            }
            CamCmd::Stop => break,
        }
    }
}

// camera thread sender, start thread on first use
fn camera_sender(state: &AppState) -> mpsc::Sender<CamCmd> {
    let mut guard = lock(&state.cam_tx);
    if let Some(tx) = guard.as_ref() {
        return tx.clone();
    }
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || camera_worker(rx));
    *guard = Some(tx.clone());
    tx
}

// send one job to camera thread, wait reply
async fn camera_ask<T>(state: &AppState, make: impl FnOnce(tokio::sync::oneshot::Sender<Result<T, String>>) -> CamCmd) -> Result<T, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    camera_sender(state).send(make(reply_tx)).map_err(|_| "Camera stopped.".to_string())?;
    reply_rx.await.map_err(|_| "Camera stopped.".to_string())?
}

// one live preview frame as data: url
#[tauri::command]
async fn camera_preview_frame(state: State<'_, AppState>) -> Result<String, String> {
    camera_ask(&state, CamCmd::Preview).await
}

// freeze full-size still for scan, return it as data: url
#[tauri::command]
async fn camera_capture(state: State<'_, AppState>) -> Result<String, String> {
    camera_ask(&state, CamCmd::Capture).await
}

// read projector qr from frozen still
#[tauri::command]
async fn camera_scan(state: State<'_, AppState>) -> Result<QrProjector, String> {
    camera_ask(&state, CamCmd::Scan).await
}

// let camera go
#[tauri::command]
fn camera_stop(state: State<'_, AppState>) {
    if let Some(tx) = lock(&state.cam_tx).take() {
        let _ = tx.send(CamCmd::Stop);
    }
}

// saved projector as ui sees it. password stays in keychain
#[derive(Debug, Serialize)]
pub struct SavedProjectorDto {
    pub name: String,
    pub ssid: String,
    pub ip: String,
}

// saved projectors, most recent first
#[tauri::command]
async fn list_saved_projectors() -> Result<Vec<SavedProjectorDto>, String> {
    let store = blocking(SavedProjectors::load).await?;
    Ok(store.projectors.into_iter().map(|p| SavedProjectorDto { name: p.name, ssid: p.ssid, ip: p.last_ip }).collect())
}

// remember projector; password into keychain. err = saved, but password not kept
#[tauri::command]
async fn save_projector(name: String, ssid: String, password: String, ip: String) -> Result<(), String> {
    blocking(move || {
        let mut store = SavedProjectors::load();
        let p = SavedProjector { name, ssid, last_ip: ip, psk: String::new() };
        let key = p.key().to_string();
        store.upsert(p);
        store.save().map_err(|e| format!("Could not save the projector: {e}"))?;
        if password.is_empty() { Ok(()) } else { config::store_password(&key, &password) }
    })
    .await?
}

// forget projector and its keychain password
#[tauri::command]
async fn forget_projector(key: String) -> Result<(), String> {
    blocking(move || {
        let mut store = SavedProjectors::load();
        store.remove(&key);
        config::forget_password(&key);
        store.save().map_err(|e| format!("Could not update saved projectors: {e}"))
    })
    .await?
}

// broadcast epson discovery probes, collect who answers in 2s
#[tauri::command]
async fn discover_projectors() -> Result<Vec<ProjectorInfo>, String> {
    use tokio::net::UdpSocket;

    let socket = UdpSocket::bind("0.0.0.0:0").await.map_err(|e| e.to_string())?;
    socket.set_broadcast(true).map_err(|e| e.to_string())?;

    // esc/vp.net hello (0x01) + connect (0x03, older firmware) on 3629; eemp probe on 3620
    let hello = b"ESC/VP.net\x10\x01\x00\x00\x00\x00";
    let connect = b"ESC/VP.net\x10\x03\x00\x00\x00\x00";
    let eemp_probe = b"EEMP0100\x00\x00\x00\x00\x02\x00\x00\x00\x30\x00\x00\x00";
    let mut targets: Vec<String> = vec!["255.255.255.255".into()];
    targets.extend(local_broadcasts().iter().map(|b| b.to_string()));
    targets.extend(["192.168.88.255", "192.168.1.255", "192.168.0.255", "10.255.255.255"].map(String::from));
    targets.dedup();
    for b in &targets {
        let _ = socket.send_to(hello, format!("{b}:3629")).await;
        let _ = socket.send_to(connect, format!("{b}:3629")).await;
    }
    let _ = socket.send_to(eemp_probe, "255.255.255.255:3620").await;

    let mut found: Vec<ProjectorInfo> = Vec::new();
    let mut buf = [0u8; 2048];
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        if let Ok(Ok((len, addr))) = tokio::time::timeout(Duration::from_millis(300), socket.recv_from(&mut buf)).await {
            let ip = addr.ip().to_string();
            if !found.iter().any(|p| p.ip == ip) {
                found.push(ProjectorInfo { name: projector_name(&buf[..len], &ip), ip });
            }
        }
    }
    Ok(found)
}

// broadcast address of each directly attached ipv4 net (linux route table)
fn local_broadcasts() -> Vec<std::net::Ipv4Addr> {
    let mut out = Vec::new();
    #[cfg(target_os = "linux")]
    if let Ok(table) = std::fs::read_to_string("/proc/net/route") {
        for line in table.lines().skip(1) {
            let c: Vec<&str> = line.split_whitespace().collect();
            if c.len() < 8 || c[1] == "00000000" || c[0] == "lo" {
                continue;
            }
            if let (Ok(dst), Ok(mask)) = (u32::from_str_radix(c[1], 16), u32::from_str_radix(c[7], 16)) {
                // kernel store both little-endian; or-ing inverted mask works same
                let b = std::net::Ipv4Addr::from((dst | !mask).to_le_bytes());
                if mask != 0 && mask != u32::MAX && !out.contains(&b) {
                    out.push(b);
                }
            }
        }
    }
    out
}

// best readable name in discovery reply, else "Projector (ip)"
fn projector_name(data: &[u8], ip: &str) -> String {
    let body = if data.starts_with(b"ESC/VP.net") {
        &data[10.min(data.len())..]
    } else if data.starts_with(b"EEMP0100") {
        &data[20.min(data.len())..]
    } else {
        data
    };
    longest_word(body).or_else(|| longest_word(data)).unwrap_or_else(|| format!("Projector ({ip})"))
}

// longest run of letters, digits, space, - or _ (3+ chars)
fn longest_word(data: &[u8]) -> Option<String> {
    data.split(|&b| !(b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b' '))
        .map(|w| String::from_utf8_lossy(w).trim().to_string())
        .filter(|w| w.len() >= 3)
        .max_by_key(|w| w.len())
}

// join wi-fi (password or keychain). on fail, go back to old wi-fi
#[tauri::command]
async fn connect_to_wifi(state: State<'_, AppState>, ssid: String, password: Option<String>) -> Result<(), String> {
    let current = blocking(wifi::current_id).await?;
    {
        let mut prev = lock(&state.prev_wifi);
        if prev.is_none() {
            *prev = current;
        }
    }
    let result = blocking(move || {
        let pw = password.filter(|p| !p.is_empty()).or_else(|| config::load_password(&ssid));
        wifi::connect(&ssid, pw.as_deref())
    })
    .await?;
    if result.is_err() {
        if let Some(id) = lock(&state.prev_wifi).take() {
            wifi::restore(&id);
        }
    }
    result
}

// how cast ended, sent to ui
#[derive(Debug, Serialize, Clone)]
struct CastEnd {
    error: Option<CastError>,
}

// start cast thread. progress on "cast-event", end on "cast-end"
#[tauri::command]
async fn start_cast(
    app: AppHandle,
    state: State<'_, AppState>,
    ssid: String,
    password: String,
    ip: Option<String>,
    keyword: Option<String>,
) -> Result<(), String> {
    // live cast = refuse; one still winding down = wait for it
    let old = lock(&state.cast).take();
    if let Some(c) = old {
        if !c.thread.is_finished() && c.running.load(Ordering::Relaxed) {
            *lock(&state.cast) = Some(c);
            return Err("Already casting.".into());
        }
        let _ = blocking(move || c.thread.join()).await;
    }

    let key = ssid.clone();
    let password = if password.is_empty() { blocking(move || config::load_password(&key)).await?.unwrap_or_default() } else { password };
    let opts = CastOptions {
        ssid,
        password,
        projector_ip: ip.and_then(|s| s.parse().ok()),
        keyword: keyword.map(|k| k.trim().to_string()).filter(|k| !k.is_empty()),
        give_up_after: Some(3),
        full_frames_only: false,
    };
    let running = Arc::new(AtomicBool::new(true));
    let flag = running.clone();
    let thread = std::thread::Builder::new()
        .name("cast".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
                session::run(&opts, &flag, &mut |e| {
                    let _ = app.emit("cast-event", &e);
                })
            }));
            let error = match result {
                Ok(r) => r.err(),
                Err(_) => Some(CastError { kind: FailKind::Unreachable, message: "Casting stopped because of an internal error.".into() }),
            };
            let _ = app.emit("cast-end", CastEnd { error });
        })
        .map_err(|e| format!("Could not start casting: {e}"))?;
    *lock(&state.cast) = Some(Cast { running, thread });
    Ok(())
}

// tell cast to stop, give it time to say goodbye (bounded)
fn stop_cast_blocking(state: &AppState, wait: Duration) {
    let Some(c) = lock(&state.cast).take() else { return };
    c.running.store(false, Ordering::Relaxed);
    let deadline = Instant::now() + wait;
    while !c.thread.is_finished() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    if !c.thread.is_finished() {
        *lock(&state.cast) = Some(c);
    }
}

// stop cast, go back to old wi-fi
#[tauri::command]
async fn stop_cast(app: AppHandle) -> Result<(), String> {
    blocking(move || {
        let state = app.state::<AppState>();
        stop_cast_blocking(&state, Duration::from_secs(3));
        let prev = lock(&state.prev_wifi).take();
        if let Some(id) = prev {
            wifi::restore(&id);
        }
    })
    .await
}

// true if easymp port answers at ip: cast over this network, no wi-fi switch
#[tauri::command]
async fn probe_projector(ip: String) -> Result<bool, String> {
    let Ok(ip) = ip.parse::<std::net::Ipv4Addr>() else { return Ok(false) };
    blocking(move || std::net::TcpStream::connect_timeout(&(ip, 3620).into(), Duration::from_millis(800)).is_ok()).await
}

// build app; on quit stop cast (goodbye to projector) and restore wi-fi
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // webkitgtk dmabuf renderer crash on nvidia; stable fallback, user override kept
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            scan_wifi_networks,
            discover_projectors,
            decode_projector_qr,
            camera_preview_frame,
            camera_capture,
            camera_scan,
            camera_stop,
            list_saved_projectors,
            save_projector,
            forget_projector,
            connect_to_wifi,
            start_cast,
            stop_cast,
            probe_projector
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                let state = app.state::<AppState>();
                stop_cast_blocking(&state, Duration::from_secs(2));
                let prev = lock(&state.prev_wifi).take();
                if let Some(id) = prev {
                    wifi::restore(&id);
                }
                let cam = lock(&state.cam_tx).take();
                if let Some(tx) = cam {
                    let _ = tx.send(CamCmd::Stop);
                }
            }
        });
}
