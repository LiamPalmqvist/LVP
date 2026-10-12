//! In-memory fakes for the ports, for unit tests.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use parking_lot::Mutex;

use crate::app::ports::Store;

pub struct MemStore {
    files: Mutex<HashMap<String, String>>,
    root: PathBuf,
}

impl MemStore {
    pub fn new(root: &Path) -> Self {
        Self { files: Mutex::new(HashMap::new()), root: root.to_path_buf() }
    }
}

impl Store for MemStore {
    fn load(&self, key: &str) -> Option<String> {
        self.files.lock().get(key).cloned()
    }
    fn save(&self, key: &str, json: &str) -> anyhow::Result<()> {
        self.files.lock().insert(key.to_string(), json.to_string());
        Ok(())
    }
    fn data_dir(&self) -> PathBuf {
        self.root.join("data")
    }
    fn cache_dir(&self) -> PathBuf {
        self.root.join("cache")
    }
}
