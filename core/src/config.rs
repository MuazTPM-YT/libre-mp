//! Saved projectors: name + ssid + ip in a 0600 json file, wi-fi password in os keychain.

use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

// keychain service name for saved wi-fi passwords
const KEYRING_SERVICE: &str = "LibreMP";

// one projector joined before. password not here, it sit in keychain
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedProjector {
    pub name: String,
    #[serde(default)]
    pub ssid: String,
    #[serde(default)]
    pub last_ip: String,
    // old files kept password in plain text; moved to keychain on load
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub psk: String,
}

impl SavedProjector {
    // lookup key: ssid, or name when no ssid (lan projector)
    pub fn key(&self) -> &str {
        if self.ssid.is_empty() { &self.name } else { &self.ssid }
    }
}

// whole saved list, newest first
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SavedProjectors {
    #[serde(default)]
    pub projectors: Vec<SavedProjector>,
}

impl SavedProjectors {
    // load from user config dir, move old plain passwords to keychain
    pub fn load() -> Self {
        let Some(path) = config_path() else { return Self::default() };
        let mut store = Self::load_from(&path).unwrap_or_default();
        if store.move_passwords_out(&mut |key, pw| store_password(key, pw).is_ok()) {
            let _ = store.save_to(&path);
        }
        store
    }

    // save to user config dir
    pub fn save(&self) -> io::Result<()> {
        let p = config_path().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no config dir"))?;
        self.save_to(&p)
    }

    // load from path. missing file = empty list
    pub fn load_from(path: &Path) -> io::Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(s) => serde_json::from_str(&s).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    // save to path, make dirs, user-only perms on unix
    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    // add or replace by key, put it first (most recent)
    pub fn upsert(&mut self, p: SavedProjector) {
        self.projectors.retain(|e| e.key() != p.key());
        self.projectors.insert(0, p);
    }

    // drop by key
    pub fn remove(&mut self, key: &str) {
        self.projectors.retain(|p| p.key() != key);
    }

    // find by exact ssid
    pub fn find_by_ssid(&self, ssid: &str) -> Option<&SavedProjector> {
        self.projectors.iter().find(|p| p.ssid == ssid)
    }

    // hand plain passwords to `keep`; clear ones it took. true = file needs rewrite
    pub fn move_passwords_out(&mut self, keep: &mut dyn FnMut(&str, &str) -> bool) -> bool {
        let mut changed = false;
        for p in self.projectors.iter_mut().filter(|p| !p.psk.is_empty()) {
            if keep(p.key(), &p.psk) {
                p.psk.clear();
                changed = true;
            }
        }
        changed
    }
}

// keychain entry for one projector
fn entry(key: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, key).map_err(|e| format!("System keychain unavailable: {e}"))
}

// put wi-fi password in os keychain
pub fn store_password(key: &str, password: &str) -> Result<(), String> {
    entry(key)?.set_password(password).map_err(|e| format!("Could not save the password in the system keychain: {e}"))
}

// read wi-fi password from os keychain, then old plain file as last resort
pub fn load_password(key: &str) -> Option<String> {
    if let Some(pw) = entry(key).ok().and_then(|e| e.get_password().ok()).filter(|p| !p.is_empty()) {
        return Some(pw);
    }
    let path = config_path()?;
    let store = SavedProjectors::load_from(&path).ok()?;
    store.projectors.into_iter().find(|p| p.key() == key).map(|p| p.psk).filter(|p| !p.is_empty())
}

// drop wi-fi password from os keychain
pub fn forget_password(key: &str) {
    if let Ok(e) = entry(key) {
        let _ = e.delete_credential();
    }
}

// <config dir>/libremp/projectors.json
pub fn config_path() -> Option<PathBuf> {
    Some(config_base_dir()?.join("libremp").join("projectors.json"))
}

// per-user config dir per os, no extra crate
fn config_base_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(PathBuf::from)
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library").join("Application Support"))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(x) = std::env::var_os("XDG_CONFIG_HOME").filter(|x| !x.is_empty()) {
            return Some(PathBuf::from(x));
        }
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))
    }
}
