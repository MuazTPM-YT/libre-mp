//! The saved-projector store persists credentials for auto-rejoin. These verify
//! round-trip fidelity, upsert (no duplicates), missing-file tolerance, and that
//! the file is written with restrictive permissions (it holds Wi-Fi PSKs).

use libremp_core::config::{SavedProjector, SavedProjectors};

fn sample() -> SavedProjector {
    SavedProjector {
        name: "RESEARCHLAB".into(),
        ssid: "RESEARCHLAB-abcdef".into(),
        psk: "b0f8ef531400".into(),
        auth_token: "b0f8ef531400".into(),
        last_ip: "192.168.88.1".into(),
    }
}

#[test]
fn roundtrips_through_disk_and_upsert_is_idempotent() {
    let dir = std::env::temp_dir().join(format!("libremp-cfg-{}", std::process::id()));
    let path = dir.join("projectors.json");
    let _ = std::fs::remove_dir_all(&dir);

    let mut store = SavedProjectors::default();
    store.upsert(sample());
    store.save_to(&path).unwrap();

    let loaded = SavedProjectors::load_from(&path).unwrap();
    assert_eq!(loaded.projectors.len(), 1);
    let p = loaded.find_by_ssid("RESEARCHLAB-abcdef").unwrap();
    assert_eq!(p.psk, "b0f8ef531400");
    assert_eq!(p.name, "RESEARCHLAB");

    // Re-inserting the same SSID updates in place instead of duplicating.
    let mut store2 = loaded.clone();
    let mut updated = sample();
    updated.psk = "NEWPSK".into();
    store2.upsert(updated);
    assert_eq!(store2.projectors.len(), 1);
    assert_eq!(store2.find_by_ssid("RESEARCHLAB-abcdef").unwrap().psk, "NEWPSK");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_file_loads_empty() {
    let path = std::env::temp_dir().join("libremp-nonexistent-xyz-123/projectors.json");
    let store = SavedProjectors::load_from(&path).unwrap();
    assert!(store.projectors.is_empty());
}

#[cfg(unix)]
#[test]
fn saved_file_is_0600() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("libremp-perm-{}", std::process::id()));
    let path = dir.join("projectors.json");
    let _ = std::fs::remove_dir_all(&dir);

    SavedProjectors::default().save_to(&path).unwrap();
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "config with PSKs must be user-only readable");

    let _ = std::fs::remove_dir_all(&dir);
}
