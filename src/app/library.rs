//! The media library: play history, saved playlists, known metadata, and
//! folder listings for the library pages.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::app::ports::Store;
use crate::domain::library::{Entry, PlayRecord, SortOrder, matches_search, sort_entries};
use crate::domain::media::{MediaItem, MediaKind, MediaMeta, MediaSource, kind_for_path};
use crate::domain::playlist::Playlist;

pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FolderVisit {
    pub count: u32,
    pub last: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct LibraryData {
    records: HashMap<String, PlayRecord>,
    meta: HashMap<String, MediaMeta>,
    recent: Vec<String>,
    playlists: Vec<Playlist>,
    folders: HashMap<PathBuf, FolderVisit>,
    next_playlist_id: u64,
}

pub struct Library {
    store: Arc<dyn Store>,
    data: LibraryData,
    dirty: bool,
}

const RECENT_MAX: usize = 200;

impl Library {
    pub fn load(store: Arc<dyn Store>) -> Self {
        let data = store.load("library").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Self { store, data, dirty: false }
    }

    pub fn save(&mut self) {
        if !self.dirty {
            return;
        }
        match serde_json::to_string(&self.data) {
            Ok(json) => {
                if let Err(e) = self.store.save("library", &json) {
                    log::error!("saving library: {e:#}");
                } else {
                    self.dirty = false;
                }
            }
            Err(e) => log::error!("serialising library: {e}"),
        }
    }

    pub fn thumbnail_dir(&self) -> PathBuf {
        self.store.cache_dir().join("thumbnails")
    }

    /// Where the thumbnail for a source lives (whether or not it exists yet).
    pub fn thumbnail_path(&self, source: &MediaSource) -> PathBuf {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        source.mrl().hash(&mut h);
        if let Some(p) = source.path()
            && let Ok(md) = std::fs::metadata(p)
        {
            md.len().hash(&mut h);
            md.modified().ok().hash(&mut h);
        }
        self.thumbnail_dir().join(format!("{:016x}.png", h.finish()))
    }

    // ---- records ---------------------------------------------------------

    pub fn record(&self, source: &MediaSource) -> PlayRecord {
        self.data.records.get(&source.mrl()).cloned().unwrap_or_default()
    }

    pub fn update_record(&mut self, source: &MediaSource, f: impl FnOnce(&mut PlayRecord)) {
        f(self.data.records.entry(source.mrl()).or_default());
        self.dirty = true;
    }

    pub fn meta(&self, source: &MediaSource) -> Option<&MediaMeta> {
        self.data.meta.get(&source.mrl())
    }

    pub fn set_meta(&mut self, source: &MediaSource, meta: MediaMeta) {
        let e = self.data.meta.entry(source.mrl()).or_default();
        e.merge(meta);
        self.dirty = true;
    }

    /// Fills an item's metadata from what the library knows.
    pub fn enrich(&self, item: &mut MediaItem) {
        if let Some(m) = self.meta(&item.source) {
            let mut merged = m.clone();
            merged.merge(item.meta.clone());
            item.meta = merged;
        }
    }

    pub fn mark_played(&mut self, item: &MediaItem, save_recent: bool) {
        let mrl = item.source.mrl();
        {
            let r = self.data.records.entry(mrl.clone()).or_default();
            r.last_played = now();
            r.play_count += 1;
        }
        if save_recent {
            self.data.recent.retain(|m| *m != mrl);
            self.data.recent.insert(0, mrl);
            self.data.recent.truncate(RECENT_MAX);
        }
        if !item.meta.is_empty_for_cache() {
            self.set_meta(&item.source, item.meta.clone());
        }
        if let Some(dir) = item.source.path().and_then(Path::parent) {
            let v = self.data.folders.entry(dir.to_path_buf()).or_default();
            v.count += 1;
            v.last = now();
        }
        self.dirty = true;
    }

    pub fn save_position(&mut self, source: &MediaSource, position_ms: i64, duration_ms: i64) {
        let r = self.data.records.entry(source.mrl()).or_default();
        r.position_ms = position_ms;
        if duration_ms > 0 {
            r.duration_ms = duration_ms;
        }
        self.dirty = true;
    }

    pub fn toggle_favorite(&mut self, source: &MediaSource) -> bool {
        let r = self.data.records.entry(source.mrl()).or_default();
        r.favorite = !r.favorite;
        self.dirty = true;
        r.favorite
    }

    pub fn clear_history(&mut self) {
        self.data.recent.clear();
        for r in self.data.records.values_mut() {
            r.position_ms = 0;
            r.last_played = 0;
        }
        self.data.folders.clear();
        self.dirty = true;
    }

    pub fn remove_from_recent(&mut self, source: &MediaSource) {
        let mrl = source.mrl();
        self.data.recent.retain(|m| *m != mrl);
        self.dirty = true;
    }

    // ---- views for the pages ---------------------------------------------

    fn item_for(&self, mrl: &str) -> MediaItem {
        let mut item = MediaItem::new(MediaSource::from_user_input(mrl));
        self.enrich(&mut item);
        item
    }

    pub fn recent(&self) -> Vec<(MediaItem, PlayRecord)> {
        self.data
            .recent
            .iter()
            .map(|m| (self.item_for(m), self.data.records.get(m).cloned().unwrap_or_default()))
            .collect()
    }

    /// Partly watched videos, most recent first.
    pub fn continue_watching(&self, limit: usize) -> Vec<(MediaItem, PlayRecord)> {
        self.recent()
            .into_iter()
            .filter(|(i, r)| i.kind() != MediaKind::Audio && r.resumable())
            .filter(|(i, _)| i.source.path().is_none_or(Path::exists))
            .take(limit)
            .collect()
    }

    /// Recently played audio.
    pub fn listen_again(&self, limit: usize) -> Vec<(MediaItem, PlayRecord)> {
        self.recent()
            .into_iter()
            .filter(|(i, _)| i.kind() == MediaKind::Audio)
            .filter(|(i, _)| i.source.path().is_none_or(Path::exists))
            .take(limit)
            .collect()
    }

    pub fn frequent_folders(&self, limit: usize) -> Vec<(PathBuf, FolderVisit)> {
        let mut v: Vec<_> =
            self.data.folders.iter().filter(|(p, _)| p.is_dir()).map(|(p, f)| (p.clone(), f.clone())).collect();
        v.sort_by(|a, b| b.1.count.cmp(&a.1.count).then(b.1.last.cmp(&a.1.last)));
        v.truncate(limit);
        v
    }

    pub fn favorites(&self) -> Vec<MediaItem> {
        self.data.records.iter().filter(|(_, r)| r.favorite).map(|(m, _)| self.item_for(m)).collect()
    }

    /// Lists a folder for the library pages: subfolders plus media files of
    /// the wanted kinds, filtered and sorted.
    pub fn list_dir(
        &self,
        dir: &Path,
        kinds: &[MediaKind],
        show_hidden: bool,
        search: &str,
        order: SortOrder,
    ) -> Vec<Entry> {
        let mut out = Vec::new();
        let Ok(rd) = std::fs::read_dir(dir) else { return out };
        for e in rd.flatten() {
            let path = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if !show_hidden && name.starts_with('.') {
                continue;
            }
            let Ok(md) = e.metadata() else { continue };
            let modified = md
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let is_dir = md.is_dir() && !is_bundle(&path);
            if !is_dir && !kinds.contains(&kind_for_path(&path)) {
                continue;
            }
            let mut item = MediaItem::file(&path);
            self.enrich(&mut item);
            if !search.is_empty() && !matches_search(&item, search) {
                continue;
            }
            let record = self.record(&item.source);
            out.push(Entry { item, is_dir, size: md.len(), modified, record });
        }
        sort_entries(&mut out, order);
        out
    }

    /// Every media file of the wanted kinds below `dir`, for "Play all".
    pub fn collect_media(&self, dir: &Path, kinds: &[MediaKind], recursive: bool, show_hidden: bool) -> Vec<MediaItem> {
        let mut out: Vec<MediaItem> = walkdir::WalkDir::new(dir)
            .max_depth(if recursive { 16 } else { 1 })
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| show_hidden || e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.'))
            .flatten()
            .filter(|e| e.file_type().is_file() && kinds.contains(&kind_for_path(e.path())))
            .map(|e| {
                let mut item = MediaItem::file(e.path());
                self.enrich(&mut item);
                item
            })
            .collect();
        out.sort_by(|a, b| crate::domain::library::natural_cmp(&a.source.mrl(), &b.source.mrl()));
        out
    }

    // ---- playlists -------------------------------------------------------

    pub fn playlists(&self) -> &[Playlist] {
        &self.data.playlists
    }

    pub fn playlist(&self, id: u64) -> Option<&Playlist> {
        self.data.playlists.iter().find(|p| p.id == id)
    }

    pub fn create_playlist(&mut self, name: &str, items: Vec<MediaItem>) -> u64 {
        self.data.next_playlist_id += 1;
        let id = self.data.next_playlist_id;
        self.data.playlists.push(Playlist { id, name: name.trim().to_string(), items, created: now() });
        self.dirty = true;
        id
    }

    pub fn edit_playlist(&mut self, id: u64, f: impl FnOnce(&mut Playlist)) {
        if let Some(p) = self.data.playlists.iter_mut().find(|p| p.id == id) {
            f(p);
            self.dirty = true;
        }
    }

    pub fn delete_playlist(&mut self, id: u64) {
        self.data.playlists.retain(|p| p.id != id);
        self.dirty = true;
    }
}

/// macOS bundles (.app, .bundle, .photoslibrary...) look like folders but are not.
fn is_bundle(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        matches!(
            e.to_ascii_lowercase().as_str(),
            "app" | "bundle" | "framework" | "photoslibrary" | "musiclibrary" | "tvlibrary" | "plugin"
        )
    })
}

trait MetaExt {
    fn is_empty_for_cache(&self) -> bool;
}

impl MetaExt for MediaMeta {
    fn is_empty_for_cache(&self) -> bool {
        self.title.is_none() && self.artist.is_none() && self.duration_ms.is_none() && self.artwork_url.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::MemStore;

    fn lib() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let l = Library::load(Arc::new(MemStore::new(dir.path())));
        (dir, l)
    }

    #[test]
    fn history_and_resume() {
        let (dir, mut l) = lib();
        let f = dir.path().join("a.mp4");
        std::fs::write(&f, b"x").unwrap();
        let item = MediaItem::file(&f);
        l.mark_played(&item, true);
        l.save_position(&item.source, 60_000, 600_000);
        assert_eq!(l.continue_watching(10).len(), 1);
        assert_eq!(l.frequent_folders(5)[0].0, dir.path());
        let store = l.store.clone();
        l.save();
        let l2 = Library::load(store);
        assert_eq!(l2.record(&item.source).position_ms, 60_000);
    }

    #[test]
    fn playlists_crud() {
        let (_d, mut l) = lib();
        let id = l.create_playlist(" Cool Songs ", vec![MediaItem::file("/m/a.mp3")]);
        assert_eq!(l.playlist(id).unwrap().name, "Cool Songs");
        l.edit_playlist(id, |p| p.items.push(MediaItem::file("/m/b.mp3")));
        assert_eq!(l.playlist(id).unwrap().items.len(), 2);
        l.delete_playlist(id);
        assert!(l.playlists().is_empty());
    }

    #[test]
    fn list_dir_filters_kinds() {
        let (dir, l) = lib();
        std::fs::write(dir.path().join("v.mkv"), b"").unwrap();
        std::fs::write(dir.path().join("s.mp3"), b"").unwrap();
        std::fs::write(dir.path().join("n.txt"), b"").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let e = l.list_dir(dir.path(), &[MediaKind::Video], false, "", SortOrder::default());
        let names: Vec<_> = e.iter().map(|e| e.item.title()).collect();
        assert!(names.contains(&"v.mkv".to_string()) && names.contains(&"sub".to_string()));
        assert!(!names.contains(&"s.mp3".to_string()));
    }
}
