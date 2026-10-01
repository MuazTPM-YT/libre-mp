//! grab one frame like projector gets it, save png: cargo run --release -p libremp-core --example portal_probe out.png
fn main() {
    #[cfg(target_os = "linux")]
    {
        let mut g = match libremp_core::capture::detect_grabber() {
            Ok(g) => g,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        };
        eprintln!("backend: {}", g.name());
        for _ in 0..40 {
            if let Some(rgb) = g.grab() {
                let out = std::env::args().nth(1).unwrap_or("frame.png".into());
                // off center: pointer starts in the middle and would hide the screen color
                let c = (200 * 1024 + 256) * 3;
                println!("sample rgb {} {} {}", rgb[c], rgb[c + 1], rgb[c + 2]);
                image::RgbImage::from_raw(1024, 768, rgb).unwrap().save(&out).unwrap();
                eprintln!("saved {out}");
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        eprintln!("NO FRAMES");
        std::process::exit(1);
    }
}
