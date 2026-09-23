//! grab one frame like projector gets it, save png: cargo run --release -p libremp-core --example portal_probe out.png
fn main() {
    #[cfg(target_os = "linux")]
    {
        let mut g = match libremp_core::capture::detect_grabber() {
            Ok(g) => g,
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        };
        eprintln!("backend: {}", g.name());
        for _ in 0..40 {
            if let Some(rgb) = g.grab() {
                let out = std::env::args().nth(1).unwrap_or("frame.png".into());
                image::RgbImage::from_raw(1024, 768, rgb).unwrap().save(&out).unwrap();
                eprintln!("saved {out}");
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        eprintln!("NO FRAMES");
    }
}
