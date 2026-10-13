//! Use cases behind the library pages: library folders, watched state,
//! playlist editing and revealing files in the system file manager.

use std::path::{Path, PathBuf};

use crate::domain::media::{MediaItem, MediaSource};

use super::App;

impl App {
    /// Adds a folder to Videos or Music (ignored if already there).
    pub fn add_library_folder(&mut self, path: PathBuf, music: bool) {
        self.update_settings(|s| {
            let list = if music { &mut s.library.music_folders } else { &mut s.library.video_folders };
            if !list.contains(&path) {
                list.push(path);
            }
        });
    }

    pub fn remove_library_folder(&mut self, path: &Path, music: bool) {
        self.update_settings(|s| {
            let list = if music { &mut s.library.music_folders } else { &mut s.library.video_folders };
            list.retain(|p| p != path);
        });
    }

    /// Marks an item watched (played to the end) or unwatched.
    pub fn set_watched(&mut self, source: &MediaSource, watched: bool) {
        self.library.update_record(source, |r| {
            r.position_ms = 0;
            r.play_count = if watched { r.play_count.max(1) } else { 0 };
        });
    }

    /// Where a playlist's custom cover image is kept.
    pub fn playlist_cover_path(&self, id: u64) -> PathBuf {
        self.library.thumbnail_path(&MediaSource::Location(format!("notvlc-playlist://{id}")))
    }

    /// Copies an image to be a playlist's cover.
    pub fn set_playlist_cover(&mut self, id: u64, image: &Path) {
        let dst = self.playlist_cover_path(id);
        if let Some(dir) = dst.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        // Thumbnails are PNG; re-encode so any image format works.
        let result = image::open(image).map_err(|e| e.to_string()).and_then(|img| {
            img.thumbnail(512, 512).save_with_format(&dst, image::ImageFormat::Png).map_err(|e| e.to_string())
        });
        if let Err(e) = result {
            self.toast_error(format!("Could not use that image: {e}"));
        }
    }

    pub fn add_to_playlist(&mut self, id: u64, items: Vec<MediaItem>) {
        let n = items.len();
        let mut name = String::new();
        self.library.edit_playlist(id, |p| {
            p.items.extend(items);
            name = p.name.clone();
        });
        if !name.is_empty() {
            self.toast(format!("Added {n} item{} to {name}", if n == 1 { "" } else { "s" }));
        }
    }

    pub fn rename_playlist(&mut self, id: u64, name: &str) {
        let name = name.trim();
        if !name.is_empty() {
            self.library.edit_playlist(id, |p| p.name = name.to_string());
        }
    }

    pub fn remove_playlist_item(&mut self, id: u64, index: usize) {
        self.library.edit_playlist(id, |p| {
            if index < p.items.len() {
                p.items.remove(index);
            }
        });
    }

    /// Moves a playlist entry from one position to another.
    pub fn move_playlist_item(&mut self, id: u64, from: usize, to: usize) {
        self.library.edit_playlist(id, |p| {
            if from < p.items.len() && to < p.items.len() && from != to {
                let item = p.items.remove(from);
                p.items.insert(to, item);
            }
        });
    }

    pub fn delete_playlist(&mut self, id: u64) {
        let _ = std::fs::remove_file(self.playlist_cover_path(id));
        self.library.delete_playlist(id);
    }

    /// Shows a file or folder in Finder (or the platform's file manager).
    pub fn reveal_in_file_manager(&mut self, path: &Path) {
        let result = if cfg!(target_os = "macos") {
            std::process::Command::new("open").arg("-R").arg(path).spawn()
        } else if cfg!(target_os = "windows") {
            std::process::Command::new("explorer").arg(format!("/select,{}", path.display())).spawn()
        } else {
            std::process::Command::new("xdg-open").arg(path.parent().unwrap_or(path)).spawn()
        };
        if let Err(e) = result {
            self.toast_error(format!("Could not open the file manager: {e}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::app::App;
    use crate::app::fake::{FakeEngine, FakeProbe};
    use crate::app::testing::MemStore;
    use crate::domain::media::MediaItem;

    fn app() -> (tempfile::TempDir, App) {
        let dir = tempfile::tempdir().unwrap();
        let a = App::new(Arc::new(FakeEngine::new()), Arc::new(FakeProbe), Arc::new(MemStore::new(dir.path())));
        (dir, a)
    }

    #[test]
    fn playlist_editing() {
        let (_d, mut a) = app();
        let id = a.library.create_playlist("Mix", vec![MediaItem::file("/m/a.mp3"), MediaItem::file("/m/b.mp3")]);
        a.add_to_playlist(id, vec![MediaItem::file("/m/c.mp3")]);
        a.move_playlist_item(id, 2, 0);
        let titles: Vec<_> = a.library.playlist(id).unwrap().items.iter().map(|i| i.title()).collect();
        assert_eq!(titles, vec!["c.mp3", "a.mp3", "b.mp3"]);
        a.remove_playlist_item(id, 1);
        a.rename_playlist(id, "  New name ");
        let pl = a.library.playlist(id).unwrap();
        assert_eq!(pl.name, "New name");
        assert_eq!(pl.items.len(), 2);
        a.delete_playlist(id);
        assert!(a.library.playlist(id).is_none());
    }

    #[test]
    fn watched_state_and_folders() {
        let (dir, mut a) = app();
        let item = MediaItem::file(dir.path().join("x.mp4"));
        a.set_watched(&item.source, true);
        assert!(a.library.record(&item.source).finished());
        a.set_watched(&item.source, false);
        assert!(!a.library.record(&item.source).finished());
        let before = a.settings.library.video_folders.len();
        a.add_library_folder(dir.path().to_path_buf(), false);
        a.add_library_folder(dir.path().to_path_buf(), false);
        assert_eq!(a.settings.library.video_folders.len(), before + 1);
        a.remove_library_folder(dir.path(), false);
        assert_eq!(a.settings.library.video_folders.len(), before);
    }
}
