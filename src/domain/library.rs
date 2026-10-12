//! Media library records: what has been seen, where playback stopped, and how
//! lists are sorted and filtered.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use super::media::{MediaItem, MediaKind};

/// A named position inside a file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bookmark {
    pub name: String,
    pub time_ms: i64,
}

/// Per-item history kept by the library.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayRecord {
    /// Last position in milliseconds, cleared when the item finishes.
    pub position_ms: i64,
    pub duration_ms: i64,
    pub last_played: i64,
    pub play_count: u32,
    pub favorite: bool,
    pub bookmarks: Vec<Bookmark>,
    /// Remembered per-file track choices and delays.
    pub audio_track: Option<i32>,
    pub subtitle_track: Option<i32>,
    pub audio_delay_us: i64,
    pub subtitle_delay_us: i64,
}

impl PlayRecord {
    /// Fraction watched, for the progress line under thumbnails.
    pub fn progress(&self) -> Option<f32> {
        (self.duration_ms > 0 && self.position_ms > 0)
            .then(|| (self.position_ms as f32 / self.duration_ms as f32).clamp(0.0, 1.0))
    }

    /// True once the item has been played to (nearly) the end.
    pub fn finished(&self) -> bool {
        self.play_count > 0 && self.position_ms == 0
    }

    /// Whether resuming makes sense: not at the very start or the very end.
    pub fn resumable(&self) -> bool {
        self.position_ms > 5_000 && (self.duration_ms <= 0 || self.position_ms < self.duration_ms - 10_000)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SortKey {
    #[default]
    Name,
    Duration,
    DateModified,
    DateAdded,
    LastPlayed,
    Artist,
    Album,
    FileSize,
}

impl SortKey {
    pub const ALL: &'static [SortKey] = &[
        SortKey::Name,
        SortKey::Duration,
        SortKey::DateModified,
        SortKey::DateAdded,
        SortKey::LastPlayed,
        SortKey::Artist,
        SortKey::Album,
        SortKey::FileSize,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SortKey::Name => "Name",
            SortKey::Duration => "Duration",
            SortKey::DateModified => "Date modified",
            SortKey::DateAdded => "Date added",
            SortKey::LastPlayed => "Last played",
            SortKey::Artist => "Artist",
            SortKey::Album => "Album",
            SortKey::FileSize => "File size",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SortOrder {
    pub key: SortKey,
    pub descending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ViewMode {
    #[default]
    Grid,
    List,
}

/// A file or folder shown on a library page.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub item: MediaItem,
    pub is_dir: bool,
    pub size: u64,
    pub modified: i64,
    pub record: PlayRecord,
}

impl Entry {
    pub fn kind(&self) -> MediaKind {
        self.item.kind()
    }
}

/// Natural ordering: "Episode 2" before "Episode 10", case-insensitive.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut ai, mut bi) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let mut na = String::new();
                while let Some(c) = ai.peek().copied().filter(char::is_ascii_digit) {
                    na.push(c);
                    ai.next();
                }
                let mut nb = String::new();
                while let Some(c) = bi.peek().copied().filter(char::is_ascii_digit) {
                    nb.push(c);
                    bi.next();
                }
                let (ta, tb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
                let ord = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb));
                if ord != Ordering::Equal {
                    return ord;
                }
            }
            (Some(x), Some(y)) => {
                let ord = x.to_lowercase().cmp(y.to_lowercase());
                if ord != Ordering::Equal {
                    return ord;
                }
                ai.next();
                bi.next();
            }
        }
    }
}

/// Sorts entries with folders first, then by the chosen key.
pub fn sort_entries(entries: &mut [Entry], order: SortOrder) {
    entries.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then_with(|| {
            let ord = match order.key {
                SortKey::Name => natural_cmp(&a.item.title(), &b.item.title()),
                SortKey::Duration => a.item.meta.duration_ms.unwrap_or(0).cmp(&b.item.meta.duration_ms.unwrap_or(0)),
                SortKey::DateModified => a.modified.cmp(&b.modified),
                SortKey::DateAdded => a.modified.cmp(&b.modified),
                SortKey::LastPlayed => a.record.last_played.cmp(&b.record.last_played),
                SortKey::Artist => natural_cmp(
                    a.item.meta.artist.as_deref().unwrap_or(""),
                    b.item.meta.artist.as_deref().unwrap_or(""),
                ),
                SortKey::Album => {
                    natural_cmp(a.item.meta.album.as_deref().unwrap_or(""), b.item.meta.album.as_deref().unwrap_or(""))
                }
                SortKey::FileSize => a.size.cmp(&b.size),
            }
            .then_with(|| natural_cmp(&a.item.title(), &b.item.title()));
            if order.descending { ord.reverse() } else { ord }
        })
    });
}

/// Case-insensitive match on title, artist and album; every word must match.
pub fn matches_search(item: &MediaItem, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    let hay = format!(
        "{} {} {} {}",
        item.title(),
        item.meta.artist.as_deref().unwrap_or(""),
        item.meta.album.as_deref().unwrap_or(""),
        item.source.display_name()
    )
    .to_lowercase();
    q.split_whitespace().all(|w| hay.contains(w))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_order() {
        let mut v = vec!["Ep 10", "ep 2", "Ep 1", "Ep 02b"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, vec!["Ep 1", "ep 2", "Ep 02b", "Ep 10"]);
    }

    #[test]
    fn resume_rules() {
        let r = PlayRecord { position_ms: 60_000, duration_ms: 600_000, ..Default::default() };
        assert!(r.resumable());
        assert_eq!(r.progress(), Some(0.1));
        let end = PlayRecord { position_ms: 595_000, duration_ms: 600_000, ..Default::default() };
        assert!(!end.resumable());
    }

    #[test]
    fn search_all_words() {
        let mut item = MediaItem::file("/x/The_Wand_S01E05.mp4");
        item.meta.artist = Some("juxtopposed".into());
        assert!(matches_search(&item, "wand s01e05"));
        assert!(matches_search(&item, "JUXT"));
        assert!(!matches_search(&item, "wand e06"));
    }

    #[test]
    fn folders_first() {
        let mk = |name: &str, dir: bool| Entry {
            item: MediaItem::file(format!("/x/{name}")),
            is_dir: dir,
            size: 0,
            modified: 0,
            record: PlayRecord::default(),
        };
        let mut v = vec![mk("b.mp4", false), mk("z", true), mk("a.mp4", false)];
        sort_entries(&mut v, SortOrder::default());
        let names: Vec<_> = v.iter().map(|e| e.item.title()).collect();
        assert_eq!(names, vec!["z", "a.mp4", "b.mp4"]);
    }
}
