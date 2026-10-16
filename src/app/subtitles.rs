//! Batch subtitles: listing subtitle and video files, attaching subtitles
//! to queue items, and copying them next to videos so VLC autoloads them.

use std::path::{Path, PathBuf};

use crate::app::App;
use crate::domain::media::MediaKind;
use crate::domain::subtitles::copy_target;

impl App {
    /// Subtitle files in a folder and its subfolders ("Subs", "Subtitles").
    pub fn subtitle_files(&self, dir: &Path) -> Vec<PathBuf> {
        self.library
            .collect_media(dir, &[MediaKind::Subtitle], true, false)
            .into_iter()
            .filter_map(|i| i.source.path().map(Path::to_path_buf))
            .collect()
    }

    /// Video files directly in a folder.
    pub fn video_files(&self, dir: &Path) -> Vec<PathBuf> {
        self.library
            .collect_media(dir, &[MediaKind::Video], false, self.settings.library.show_hidden)
            .into_iter()
            .filter_map(|i| i.source.path().map(Path::to_path_buf))
            .collect()
    }

    /// Gives queue items a subtitle file (`:sub-file=`), replacing any set
    /// before; the playing item also gets it right away. Returns how many
    /// items changed.
    pub fn attach_subtitles(&mut self, pairs: &[(usize, PathBuf)]) -> usize {
        let current = self.queue.current;
        let mut n = 0;
        let mut now = None;
        for (index, sub) in pairs {
            let Some(item) = self.queue.items.get_mut(*index) else { continue };
            item.options.retain(|o| !o.starts_with(":sub-file="));
            item.options.push(format!(":sub-file={}", sub.display()));
            n += 1;
            if current == Some(*index) {
                now = Some(sub.clone());
            }
        }
        if let Some(sub) = now {
            self.add_subtitle(sub);
        }
        self.save_queue();
        if n > 0 {
            self.toast(format!("Added subtitles to {n} item{}", if n == 1 { "" } else { "s" }));
        }
        n
    }

    /// Where each pair's subtitle would be copied, and whether that would
    /// replace an existing file.
    pub fn subtitle_copy_plan(&self, pairs: &[(PathBuf, PathBuf)]) -> Vec<(PathBuf, PathBuf, bool)> {
        pairs
            .iter()
            .map(|(video, sub)| {
                let to = copy_target(video, sub);
                let exists = to.exists() && to != *sub;
                (sub.clone(), to, exists)
            })
            .collect()
    }

    /// Copies each subtitle next to its video under the video's name.
    /// Returns how many were copied.
    pub fn copy_subtitles(&mut self, pairs: &[(PathBuf, PathBuf)]) -> usize {
        let mut ok = 0;
        let mut failed = Vec::new();
        for (sub, to, _) in self.subtitle_copy_plan(pairs) {
            if sub == to {
                ok += 1;
                continue;
            }
            match std::fs::copy(&sub, &to) {
                Ok(_) => ok += 1,
                Err(e) => failed.push(format!("{}: {e}", to.display())),
            }
        }
        if failed.is_empty() {
            self.toast(format!("Copied {ok} subtitle file{}", if ok == 1 { "" } else { "s" }));
        } else {
            self.toast_error(format!("Could not copy {} subtitle files: {}", failed.len(), failed.join("; ")));
        }
        ok
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use crate::app::App;
    use crate::app::fake::{FakeEngine, FakeProbe};
    use crate::app::ports::{EngineCommand, MediaEngine};
    use crate::app::testing::MemStore;
    use crate::domain::media::MediaItem;

    #[test]
    fn attaches_and_copies() {
        let dir = tempfile::tempdir().unwrap();
        let fake = Arc::new(FakeEngine::new());
        let engine: Arc<dyn MediaEngine> = fake.clone();
        let mut app = App::new(engine, Arc::new(FakeProbe), Arc::new(MemStore::new(dir.path())));
        let v = dir.path().join("videos");
        let s = dir.path().join("subs");
        std::fs::create_dir_all(&v).unwrap();
        std::fs::create_dir_all(s.join("Subs")).unwrap();
        for n in ["a.mkv", "b.mkv"] {
            std::fs::write(v.join(n), b"").unwrap();
        }
        std::fs::write(s.join("a.srt"), b"1").unwrap();
        std::fs::write(s.join("Subs/b.en.srt"), b"2").unwrap();
        assert_eq!(app.video_files(&v).len(), 2);
        assert_eq!(app.subtitle_files(&s).len(), 2);

        app.play_items(vec![MediaItem::file(v.join("a.mkv")), MediaItem::file(v.join("b.mkv"))], 0, None);
        let n = app.attach_subtitles(&[(0, s.join("a.srt")), (1, s.join("Subs/b.en.srt"))]);
        assert_eq!(n, 2);
        assert_eq!(app.queue.items[1].options, vec![format!(":sub-file={}", s.join("Subs/b.en.srt").display())]);
        assert!(fake.sent.lock().iter().any(|c| matches!(c, EngineCommand::AddSubtitle(p) if p.ends_with("a.srt"))));

        let pairs: Vec<(PathBuf, PathBuf)> = vec![(v.join("b.mkv"), s.join("Subs/b.en.srt"))];
        assert!(!app.subtitle_copy_plan(&pairs)[0].2);
        assert_eq!(app.copy_subtitles(&pairs), 1);
        assert_eq!(std::fs::read(v.join("b.en.srt")).unwrap(), b"2");
        assert!(app.subtitle_copy_plan(&pairs)[0].2);
    }
}
