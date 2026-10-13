//! Sorting for the library pages, following the design's "Sorting Options"
//! menu (watched state, date, title, artist, album, track, URI, disc).

use std::cmp::Ordering;

use super::library::{Entry, PlayRecord, natural_cmp};
use super::media::MediaItem;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SortOption {
    #[default]
    UnwatchedFirst,
    WatchedFirst,
    NewestFirst,
    OldestFirst,
    TitleAsc,
    TitleDesc,
    ArtistAsc,
    ArtistDesc,
    AlbumAsc,
    AlbumDesc,
    TrackAsc,
    TrackDesc,
    UriAsc,
    UriDesc,
    DiscAsc,
    DiscDesc,
}

impl SortOption {
    pub const ALL: &'static [SortOption] = &[
        SortOption::UnwatchedFirst,
        SortOption::WatchedFirst,
        SortOption::NewestFirst,
        SortOption::OldestFirst,
        SortOption::TitleAsc,
        SortOption::TitleDesc,
        SortOption::ArtistAsc,
        SortOption::ArtistDesc,
        SortOption::AlbumAsc,
        SortOption::AlbumDesc,
        SortOption::TrackAsc,
        SortOption::TrackDesc,
        SortOption::UriAsc,
        SortOption::UriDesc,
        SortOption::DiscAsc,
        SortOption::DiscDesc,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SortOption::UnwatchedFirst => "Unwatched First",
            SortOption::WatchedFirst => "Watched First",
            SortOption::NewestFirst => "Newest First",
            SortOption::OldestFirst => "Oldest First",
            SortOption::TitleAsc => "Title A-Z",
            SortOption::TitleDesc => "Title Z-A",
            SortOption::ArtistAsc => "Artist A-Z",
            SortOption::ArtistDesc => "Artist Z-A",
            SortOption::AlbumAsc => "Album A-Z",
            SortOption::AlbumDesc => "Album Z-A",
            SortOption::TrackAsc => "Track Number Ascending",
            SortOption::TrackDesc => "Track Number Descending",
            SortOption::UriAsc => "URI Ascending",
            SortOption::UriDesc => "URI Descending",
            SortOption::DiscAsc => "Disc Number Ascending",
            SortOption::DiscDesc => "Disc Number Descending",
        }
    }

    /// Orders two media items; `modified` is the file time when known.
    pub fn compare(self, a: &Sortable, b: &Sortable) -> Ordering {
        use SortOption::*;
        let title = natural_cmp(&a.item.title(), &b.item.title());
        let meta = |f: fn(&MediaItem) -> Option<&str>| {
            natural_cmp(f(a.item).unwrap_or(""), f(b.item).unwrap_or("")).then(title)
        };
        let number =
            |f: fn(&MediaItem) -> Option<&str>| leading_number(f(a.item)).cmp(&leading_number(f(b.item))).then(title);
        match self {
            UnwatchedFirst => watch_rank(a.record).cmp(&watch_rank(b.record)).then(title),
            WatchedFirst => watch_rank(b.record).cmp(&watch_rank(a.record)).then(title),
            NewestFirst => b.modified.cmp(&a.modified).then(title),
            OldestFirst => a.modified.cmp(&b.modified).then(title),
            TitleAsc => title,
            TitleDesc => title.reverse(),
            ArtistAsc => meta(|i| i.meta.artist.as_deref()),
            ArtistDesc => meta(|i| i.meta.artist.as_deref()).reverse(),
            AlbumAsc => meta(|i| i.meta.album.as_deref()),
            AlbumDesc => meta(|i| i.meta.album.as_deref()).reverse(),
            TrackAsc => number(|i| i.meta.track_number.as_deref()),
            TrackDesc => number(|i| i.meta.track_number.as_deref()).reverse(),
            UriAsc => natural_cmp(&a.item.source.mrl(), &b.item.source.mrl()),
            UriDesc => natural_cmp(&a.item.source.mrl(), &b.item.source.mrl()).reverse(),
            DiscAsc => number(|i| i.meta.disc_number.as_deref()),
            DiscDesc => number(|i| i.meta.disc_number.as_deref()).reverse(),
        }
    }
}

/// What a sort needs to know about one row.
pub struct Sortable<'a> {
    pub item: &'a MediaItem,
    pub record: &'a PlayRecord,
    pub modified: i64,
}

/// 0 unwatched, 1 partly watched, 2 finished.
pub fn watch_rank(r: &PlayRecord) -> u8 {
    if r.finished() {
        2
    } else if r.progress().is_some() {
        1
    } else {
        0
    }
}

/// "3/12" -> 3, "07" -> 7; missing numbers sort last.
fn leading_number(s: Option<&str>) -> u64 {
    s.and_then(|s| {
        let digits: String = s.trim().chars().take_while(char::is_ascii_digit).collect();
        digits.parse().ok()
    })
    .unwrap_or(u64::MAX)
}

/// Sorts folder entries: folders first (by name), then files by `opt`.
pub fn sort_folder(entries: &mut [Entry], opt: SortOption) {
    entries.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then_with(|| {
            if a.is_dir {
                let ord = natural_cmp(&a.item.title(), &b.item.title());
                return if matches!(opt, SortOption::TitleDesc) { ord.reverse() } else { ord };
            }
            opt.compare(
                &Sortable { item: &a.item, record: &a.record, modified: a.modified },
                &Sortable { item: &b.item, record: &b.record, modified: b.modified },
            )
        })
    });
}

/// Sorts loose items (songs, recent) with their play records.
pub fn sort_items(items: &mut [(MediaItem, PlayRecord)], opt: SortOption) {
    items.sort_by(|a, b| {
        opt.compare(
            &Sortable { item: &a.0, record: &a.1, modified: a.1.last_played },
            &Sortable { item: &b.0, record: &b.1, modified: b.1.last_played },
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, dir: bool, record: PlayRecord, modified: i64) -> Entry {
        Entry { item: MediaItem::file(format!("/x/{name}")), is_dir: dir, size: 0, modified, record }
    }

    fn names(v: &[Entry]) -> Vec<String> {
        v.iter().map(|e| e.item.title()).collect()
    }

    #[test]
    fn unwatched_first_puts_progress_after() {
        let part = PlayRecord { position_ms: 60_000, duration_ms: 600_000, play_count: 1, ..Default::default() };
        let done = PlayRecord { position_ms: 0, play_count: 1, ..Default::default() };
        let mut v = vec![
            entry("E01.mp4", false, done.clone(), 0),
            entry("E02.mp4", false, part.clone(), 0),
            entry("E03.mp4", false, PlayRecord::default(), 0),
            entry("Extras", true, PlayRecord::default(), 0),
        ];
        sort_folder(&mut v, SortOption::UnwatchedFirst);
        assert_eq!(names(&v), vec!["Extras", "E03.mp4", "E02.mp4", "E01.mp4"]);
        sort_folder(&mut v, SortOption::WatchedFirst);
        assert_eq!(names(&v), vec!["Extras", "E01.mp4", "E02.mp4", "E03.mp4"]);
    }

    #[test]
    fn dates_and_titles() {
        let mut v = vec![
            entry("b.mp4", false, PlayRecord::default(), 5),
            entry("a10.mp4", false, PlayRecord::default(), 9),
            entry("a2.mp4", false, PlayRecord::default(), 1),
        ];
        sort_folder(&mut v, SortOption::NewestFirst);
        assert_eq!(names(&v), vec!["a10.mp4", "b.mp4", "a2.mp4"]);
        sort_folder(&mut v, SortOption::TitleAsc);
        assert_eq!(names(&v), vec!["a2.mp4", "a10.mp4", "b.mp4"]);
        sort_folder(&mut v, SortOption::TitleDesc);
        assert_eq!(names(&v), vec!["b.mp4", "a10.mp4", "a2.mp4"]);
    }

    #[test]
    fn track_numbers_parse_leading_digits() {
        let mk = |t: &str, n: Option<&str>| {
            let mut i = MediaItem::file(format!("/m/{t}.mp3"));
            i.meta.track_number = n.map(str::to_string);
            (i, PlayRecord::default())
        };
        let mut v = vec![mk("c", Some("10/12")), mk("a", None), mk("b", Some("2"))];
        sort_items(&mut v, SortOption::TrackAsc);
        let t: Vec<_> = v.iter().map(|(i, _)| i.title()).collect();
        assert_eq!(t, vec!["b.mp3", "c.mp3", "a.mp3"]);
    }
}
