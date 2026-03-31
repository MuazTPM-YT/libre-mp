use xcap::Monitor;
use image::{imageops::FilterType, DynamicImage};
use image::codecs::jpeg::JpegEncoder;
use crate::protocol::client::{EpsonClient, ProtocolError};
use crate::protocol::config;
use std::io::Cursor;
use tokio::time::{sleep, Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct VideoStreamer {
    client: EpsonClient,
    target_fps: u32,
    is_running: Arc<AtomicBool>,
}

impl VideoStreamer {
    /// Initializes a new video streamer instance.
    pub fn new(projector_ip: &str, fps: u32, password: &str, ssid: &str) -> Self {
        Self {
            client: EpsonClient::new(projector_ip, password, ssid),
            target_fps: fps,
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Starts the screen capture and video streaming loop.
    pub async fn start(&mut self) -> Result<(), ProtocolError> {
        self.client.connect_and_negotiate().await?;
        self.is_running.store(true, Ordering::SeqCst);
        
        let frame_duration = Duration::from_millis((1000 / self.target_fps) as u64);
        let monitors = Monitor::all().unwrap_or_else(|_| vec![]);
        let primary_monitor = monitors.into_iter().next();

        if primary_monitor.is_none() {
            println!("[-] No monitors found for screen capture.");
            self.client.disconnect().await;
            return Err(ProtocolError::NetworkError("No monitors found".into()));
        }

        let monitor = primary_monitor.unwrap();
        println!("[+] Capturing from monitor: {} ({}x{})", monitor.name().unwrap_or_else(|_| "Unknown".into()), monitor.width().unwrap_or(0), monitor.height().unwrap_or(0));
        
        let mut frame_idx = 0;
        let mut is_first = true;
        let keepalive_interval = self.target_fps;

        while self.is_running.load(Ordering::SeqCst) {
            let start_time = Instant::now();

            // Capture the screen
            if let Ok(capture) = monitor.capture_image() {
                // Resize image
                let img = DynamicImage::ImageRgba8(capture);
                let resized = img.resize_exact(config::STREAM_WIDTH, config::STREAM_HEIGHT, FilterType::Lanczos3);
                
                // Encode to JPEG
                let mut jpeg_buf = Cursor::new(Vec::new());
                let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_buf, config::JPEG_QUALITY);
                
                if encoder.encode_image(&resized).is_ok() {
                    let jpeg_bytes = jpeg_buf.into_inner();
                    // Send it
                    if let Err(e) = self.client.send_video_frame(is_first, 0, 0, config::STREAM_WIDTH as u16, config::STREAM_HEIGHT as u16, &jpeg_bytes).await {
                        println!("[-] Stream error: {:?}", e);
                        break;
                    }
                    is_first = false;
                    frame_idx += 1;
                    
                    if frame_idx % keepalive_interval == 0 {
                        let _ = self.client.send_frame_keepalive().await;
                    }
                }
            } else {
                println!("[-] Frame capture failed");
            }

            // Sleep to maintain FPS
            let elapsed = start_time.elapsed();
            if elapsed < frame_duration {
                sleep(frame_duration - elapsed).await;
            }
        }

        self.client.disconnect().await;
        Ok(())
    }

    /// Signals the streaming loop to stop.
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }
}
