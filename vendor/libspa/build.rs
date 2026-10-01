fn main() {
    // FIXME: It would be nice to run this only when tests are run.
    println!("cargo:rerun-if-changed=tests/pod.c");

    let libs = system_deps::Config::new()
        .probe()
        .expect("Cannot find libspa");
    let libspa = libs.get_by_name("libspa").unwrap();

    // libremp patch: pipewire < 0.3.65 (ubuntu 22.04, mint 21) has no video `flags` and a signed `modifier`
    println!("cargo:rustc-check-cfg=cfg(spa_old_video_raw)");
    let raw_h = libspa
        .include_paths
        .iter()
        .find_map(|p| std::fs::read_to_string(p.join("spa/param/video/raw.h")).ok())
        .unwrap_or_default();
    let info = raw_h.split("struct spa_video_info_raw {").nth(1).and_then(|s| s.split("};").next()).unwrap_or("");
    if !info.is_empty() && !info.contains("uint32_t flags;") {
        println!("cargo:rustc-cfg=spa_old_video_raw");
    }

    cc::Build::new()
        .file("tests/pod.c")
        .flag("-Wno-missing-field-initializers")
        .includes(&libspa.include_paths)
        .compile("pod");
}
