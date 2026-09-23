// saved projector list: disk roundtrip, upsert, old plain passwords moved out, 0600 file

use libremp_core::config::{SavedProjector, SavedProjectors};

fn sample(ssid: &str) -> SavedProjector {
    SavedProjector { name: "LAB".into(), ssid: ssid.into(), last_ip: "192.168.88.1".into(), psk: String::new() }
}

// fresh temp dir per test
fn temp(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("libremp-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

// roundtrip keeps data; same key replaces in place and moves to front
#[test]
fn roundtrip_and_upsert() {
    let dir = temp("cfg");
    let path = dir.join("projectors.json");
    let mut store = SavedProjectors::default();
    store.upsert(sample("A-1"));
    store.upsert(sample("B-2"));
    store.save_to(&path).unwrap();

    let mut loaded = SavedProjectors::load_from(&path).unwrap();
    assert_eq!(loaded.projectors.iter().map(|p| p.ssid.as_str()).collect::<Vec<_>>(), ["B-2", "A-1"]);
    let mut again = sample("A-1");
    again.last_ip = "10.0.0.9".into();
    loaded.upsert(again);
    assert_eq!(loaded.projectors.len(), 2);
    assert_eq!(loaded.projectors[0].last_ip, "10.0.0.9");
    loaded.remove("B-2");
    assert_eq!(loaded.projectors.len(), 1);
    assert!(!std::fs::read_to_string(&path).unwrap().contains("psk"), "no password field written");
    let _ = std::fs::remove_dir_all(&dir);
}

// lan projector with no ssid keyed by name
#[test]
fn key_falls_back_to_name() {
    let mut p = sample("");
    p.name = "Hall".into();
    assert_eq!(p.key(), "Hall");
}

// old file with plain psk: moved when keychain takes it, kept when it cannot
#[test]
fn old_plain_passwords_move_out() {
    let old = r#"{"projectors":[{"name":"LAB","ssid":"LAB-x","psk":"SECRET1","auth_token":"SECRET1","last_ip":"1.2.3.4"}]}"#;
    let dir = temp("mig");
    let path = dir.join("projectors.json");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&path, old).unwrap();

    let mut store = SavedProjectors::load_from(&path).unwrap();
    assert!(!store.move_passwords_out(&mut |_, _| false), "keychain down: nothing changes");
    assert_eq!(store.projectors[0].psk, "SECRET1");

    let mut taken = Vec::new();
    assert!(store.move_passwords_out(&mut |k, pw| {
        taken.push((k.to_string(), pw.to_string()));
        true
    }));
    assert_eq!(taken, [("LAB-x".to_string(), "SECRET1".to_string())]);
    store.save_to(&path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains("SECRET1"), "plain password gone from disk: {text}");
    let _ = std::fs::remove_dir_all(&dir);
}

// missing file is empty list
#[test]
fn missing_file_loads_empty() {
    let path = std::env::temp_dir().join("libremp-nonexistent-xyz-123/projectors.json");
    assert!(SavedProjectors::load_from(&path).unwrap().projectors.is_empty());
}

// file readable by user only
#[cfg(unix)]
#[test]
fn saved_file_is_0600() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp("perm");
    let path = dir.join("projectors.json");
    SavedProjectors::default().save_to(&path).unwrap();
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let _ = std::fs::remove_dir_all(&dir);
}

// real os keychain store, read, forget. needs a keychain, so run by hand: --ignored
#[test]
#[ignore]
fn keychain_roundtrip() {
    use libremp_core::config::{forget_password, load_password, store_password};
    let key = "libremp-selftest-entry";
    store_password(key, "pw-123").unwrap();
    assert_eq!(load_password(key).as_deref(), Some("pw-123"));
    forget_password(key);
    assert_eq!(load_password(key), None);
}
