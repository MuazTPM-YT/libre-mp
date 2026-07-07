//! Persistent store of projectors the user has connected to, so known ones can
//! be auto-rejoined without re-entering credentials.
//!
//! Stored at `<config-dir>/libremp/projectors.json` with `0600` permissions on
//! Unix, because it holds Wi-Fi PSKs.

use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

/// A projector the user has successfully connected to before.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedProjector {
    pub name: String,
    /// Direct-mode Wi-Fi SSID (empty for LAN / infrastructure projectors).
    #[serde(default)]
    pub ssid: String,
    /// Wi-Fi PSK for Direct mode (empty for open / LAN).
    #[serde(default)]
    pub psk: String,
    /// EasyMP auth token (the projector's wired MAC in hex); often equals `psk`.
    #[serde(default)]
    pub auth_token: String,
    /// Last IP the projector was reached at.
    #[serde(default)]
    pub last_ip: String,
}

/// The on-disk store of saved projectors.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SavedProjectors {
    #[serde(default)]
    pub projectors: Vec<SavedProjector>,
}

impl SavedProjectors {
    /// Load from the default per-user config path (empty store if absent).
    pub fn load() -> Self {
        match config_path() {
            Some(p) => Self::load_from(&p).unwrap_or_default(),
            None => Self::default(),
        }
    }

    /// Save to the default per-user config path.
    pub fn save(&self) -> io::Result<()> {
        let p = config_path()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no config dir"))?;
        self.save_to(&p)
    }

    /// Load from an explicit path. Returns an empty store if the file is absent.
    pub fn load_from(path: &Path) -> io::Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(s) => {
                serde_json::from_str(&s).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    /// Save to an explicit path, creating parent dirs; `0600` perms on Unix.
    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    /// Insert or update a projector, keyed by SSID (or name when SSID is empty).
    pub fn upsert(&mut self, p: SavedProjector) {
        let key = projector_key(&p);
        if let Some(existing) = self
            .projectors
            .iter_mut()
            .find(|e| projector_key(e) == key)
        {
            *existing = p;
        } else {
            self.projectors.push(p);
        }
    }

    /// Find a saved projector by exact SSID.
    pub fn find_by_ssid(&self, ssid: &str) -> Option<&SavedProjector> {
        self.projectors.iter().find(|p| p.ssid == ssid)
    }
}

/// The de-duplication key: SSID when present, otherwise the display name.
fn projector_key(p: &SavedProjector) -> &str {
    if p.ssid.is_empty() {
        &p.name
    } else {
        &p.ssid
    }
}

/// Resolve the config file path: `<config-dir>/libremp/projectors.json`.
pub fn config_path() -> Option<PathBuf> {
    Some(config_base_dir()?.join("libremp").join("projectors.json"))
}

/// Platform per-user config base dir, resolved without extra dependencies.
fn config_base_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(PathBuf::from)
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(x) = std::env::var_os("XDG_CONFIG_HOME") {
            if !x.is_empty() {
                return Some(PathBuf::from(x));
            }
        }
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))
    }
}
