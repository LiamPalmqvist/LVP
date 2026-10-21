//! JSON files under the platform's config and cache directories.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::app::ports::Store;

pub struct JsonStore {
    data: PathBuf,
    cache: PathBuf,
}

impl JsonStore {
    /// Uses `LVP_HOME` when set (handy for tests and portable installs),
    /// otherwise the platform's standard locations.
    pub fn new() -> Self {
        if let Some(home) = std::env::var_os("LVP_HOME") {
            let home = PathBuf::from(home);
            return Self::at(home.join("data"), home.join("cache"));
        }
        match directories::ProjectDirs::from("org", "LVP", "LVP") {
            Some(d) => {
                // LVP used to be called NotVLC: carry its settings, library and caches over.
                if let Some(old) = directories::ProjectDirs::from("org", "NotVLC", "NotVLC") {
                    adopt(old.data_dir(), d.data_dir());
                    adopt(old.cache_dir(), d.cache_dir());
                }
                Self::at(d.data_dir().to_path_buf(), d.cache_dir().to_path_buf())
            }
            None => Self::at(std::env::temp_dir().join("lvp/data"), std::env::temp_dir().join("lvp/cache")),
        }
    }

    pub fn at(data: PathBuf, cache: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&data);
        let _ = std::fs::create_dir_all(&cache);
        Self { data, cache }
    }

    fn path(&self, key: &str) -> PathBuf {
        self.data.join(format!("{key}.json"))
    }
}

/// Moves the directory `old` to `new`, unless `new` already exists.
fn adopt(old: &Path, new: &Path) {
    if !old.is_dir() || new.exists() {
        return;
    }
    if let Some(parent) = new.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::rename(old, new) {
        log::warn!("could not move {} to {}: {e}", old.display(), new.display());
    }
}

impl Default for JsonStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Store for JsonStore {
    fn load(&self, key: &str) -> Option<String> {
        std::fs::read_to_string(self.path(key)).ok()
    }

    /// Writes through a temporary file so a crash never leaves half a file.
    fn save(&self, key: &str, json: &str) -> Result<()> {
        let path = self.path(key);
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json).with_context(|| format!("writing {}", tmp.display()))?;
        std::fs::rename(&tmp, &path).with_context(|| format!("replacing {}", path.display()))?;
        Ok(())
    }

    fn data_dir(&self) -> PathBuf {
        self.data.clone()
    }

    fn cache_dir(&self) -> PathBuf {
        self.cache.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_then_load() {
        let dir = tempfile::tempdir().unwrap();
        let s = JsonStore::at(dir.path().join("d"), dir.path().join("c"));
        assert!(s.load("settings").is_none());
        s.save("settings", "{\"a\":1}").unwrap();
        assert_eq!(s.load("settings").as_deref(), Some("{\"a\":1}"));
    }

    #[test]
    fn adopt_moves_old_data_once() {
        let dir = tempfile::tempdir().unwrap();
        let (old, new) = (dir.path().join("old"), dir.path().join("parent/new"));
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("settings.json"), "{}").unwrap();
        adopt(&old, &new);
        assert!(!old.exists() && new.join("settings.json").exists());

        // An existing directory is left alone.
        std::fs::create_dir_all(&old).unwrap();
        adopt(&old, &new);
        assert!(old.exists());
    }
}
