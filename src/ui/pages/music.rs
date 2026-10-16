//! Music by metadata: Albums, Artists and Genres built from every audio file
//! below the music folders. The folders are scanned on a background thread;
//! tags come from the library's metadata cache as files get parsed.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use crate::app::App;
use crate::domain::browse::SortOption;
use crate::domain::library::{PlayRecord, natural_cmp};
use crate::domain::media::{MediaItem, MediaKind, kind_for_path};
use crate::ui::icons::Icon;
use crate::ui::state::UiState;

use super::Page;
use super::model::{self, PageKind, PageModel};

const RESCAN: Duration = Duration::from_secs(120);

#[derive(Default)]
pub struct MusicIndex {
    roots: Vec<PathBuf>,
    paths: Vec<PathBuf>,
    scan: Option<Receiver<Vec<PathBuf>>>,
    scanned: Option<Instant>,
    /// Paths whose tags were requested, so parsing is asked for only once.
    probed: usize,
}

pub fn rescan(idx: &mut MusicIndex) {
    idx.scanned = None;
}

impl MusicIndex {
    /// Every song, with whatever tags are known. The bool is true while the
    /// first scan is still running.
    fn songs(&mut self, app: &mut App) -> (Vec<MediaItem>, bool) {
        let roots = app.settings.library.music_folders.clone();
        if let Some(rx) = &self.scan
            && let Ok(paths) = rx.try_recv()
        {
            self.paths = paths;
            self.scan = None;
            self.probed = 0;
        }
        let stale = roots != self.roots || self.scanned.is_none_or(|t| t.elapsed() > RESCAN);
        if stale && self.scan.is_none() {
            self.roots = roots.clone();
            self.scanned = Some(Instant::now());
            let hidden = app.settings.library.show_hidden;
            let (tx, rx) = channel();
            self.scan = Some(rx);
            std::thread::spawn(move || {
                let mut out = Vec::new();
                for r in roots {
                    let walk =
                        walkdir::WalkDir::new(r).max_depth(16).follow_links(true).into_iter().filter_entry(|e| {
                            hidden || e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.')
                        });
                    for e in walk.flatten() {
                        if e.file_type().is_file() && kind_for_path(e.path()) == MediaKind::Audio {
                            out.push(e.into_path());
                        }
                    }
                }
                out.sort_by(|a, b| natural_cmp(&a.to_string_lossy(), &b.to_string_lossy()));
                let _ = tx.send(out);
            });
        }
        let items: Vec<MediaItem> = self
            .paths
            .iter()
            .map(|p| {
                let mut i = MediaItem::file(p);
                app.library.enrich(&mut i);
                i
            })
            .collect();
        // Tags drive the groupings, so ask for them (a batch at a time).
        let end = (self.probed + 200).min(items.len());
        for i in &items[self.probed..end] {
            if i.meta.duration_ms.is_none() {
                app.request_meta(i);
            }
        }
        self.probed = end;
        (items, self.scan.is_some() && self.paths.is_empty())
    }
}

#[derive(Clone, Copy, PartialEq)]
enum By {
    Album,
    Artist,
    Genre,
}

fn key(item: &MediaItem, by: By) -> String {
    let m = &item.meta;
    let v = match by {
        By::Album => m.album.clone(),
        By::Artist => m.artist.clone().or_else(|| m.album_artist.clone()),
        By::Genre => m.genre.clone(),
    };
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| match by {
        By::Album => "Unknown Album".into(),
        By::Artist => "Unknown Artist".into(),
        By::Genre => "Unknown Genre".into(),
    })
}

fn crumbs(by: By, name: Option<&str>) -> Vec<(String, Option<Page>)> {
    let (label, page) = match by {
        By::Album => ("Albums", Page::Albums),
        By::Artist => ("Artists", Page::Artists),
        By::Genre => ("Genres", Page::Genres),
    };
    let mut v = vec![model::crumb_of("Music", Some(Page::Music)), model::crumb_of(label, Some(page))];
    if let Some(n) = name {
        v.push(model::crumb_of(n, None));
    }
    if let Some(last) = v.last_mut() {
        last.1 = None;
    }
    v
}

/// Albums, Artists or Genres as cards.
pub fn groups(app: &mut App, st: &mut UiState, page: &Page, q: &str) -> PageModel {
    let by = match page {
        Page::Albums => By::Album,
        Page::Artists => By::Artist,
        _ => By::Genre,
    };
    let (songs, loading) = st.library.music.songs(app);
    let mut map: BTreeMap<String, Vec<&MediaItem>> = BTreeMap::new();
    for s in &songs {
        map.entry(key(s, by)).or_default().push(s);
    }
    let mut names: Vec<&String> = map.keys().collect();
    names.sort_by(|a, b| natural_cmp(a, b));
    let tiles = names
        .into_iter()
        .filter(|n| model::keep_q(n, q))
        .map(|name| {
            let list = &map[name];
            let cover = list.iter().find(|i| i.meta.artwork_url.is_some()).or(list.first()).map(|i| (*i).clone());
            let n = list.len();
            let count = if n == 1 { "1 song".to_string() } else { format!("{n} songs") };
            let (icon, sub, target) = match by {
                By::Album => (
                    Icon::Album,
                    list.iter()
                        .find_map(|i| i.meta.album_artist.clone().or_else(|| i.meta.artist.clone()))
                        .or(Some(count)),
                    Page::Album(name.clone()),
                ),
                By::Artist => (Icon::Artist, Some(count), Page::Artist(name.clone())),
                By::Genre => (Icon::Genre, Some(count), Page::Genre(name.clone())),
            };
            model::group_tile(name.clone(), sub, icon, target, cover)
        })
        .collect();
    let mut m = model::group_model(PageKind::Groups, crumbs(by, None), model::square_section(tiles), loading);
    m.sortable = false;
    m
}

/// The songs of one album, artist or genre.
pub fn group(app: &mut App, st: &mut UiState, page: &Page, q: &str, sort: Option<SortOption>) -> PageModel {
    let (by, name) = match page {
        Page::Album(n) => (By::Album, n.clone()),
        Page::Artist(n) => (By::Artist, n.clone()),
        Page::Genre(n) => (By::Genre, n.clone()),
        _ => (By::Album, String::new()),
    };
    let (songs, loading) = st.library.music.songs(app);
    let items: Vec<(MediaItem, PlayRecord)> = songs
        .into_iter()
        .filter(|i| key(i, by) == name && model::keep_item(i, q))
        .map(|i| {
            let r = app.library.record(&i.source);
            (i, r)
        })
        .collect();
    let sec = model::songs_section(items, sort, &name);
    model::group_model(PageKind::Group, crumbs(by, Some(&name)), sec, loading)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grouping_keys_fall_back() {
        let mut i = MediaItem::file("/m/a.mp3");
        assert_eq!(key(&i, By::Album), "Unknown Album");
        i.meta.album_artist = Some("Band".into());
        assert_eq!(key(&i, By::Artist), "Band");
        i.meta.artist = Some(" Singer ".into());
        assert_eq!(key(&i, By::Artist), "Singer");
    }
}
