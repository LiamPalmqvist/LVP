//! The customisable player control bar ("Customize Player" in the design).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolbarItem {
    // Fixed parts of the default bar.
    Previous,
    PlayPause,
    Next,
    CurrentTime,
    SeekBar,
    Duration,
    Volume,
    AudioTracks,
    SubtitleTracks,
    QuickSettings,
    Queue,
    Fullscreen,
    // Optional items offered in the customise dialog.
    Spacer,
    Divider,
    PlaybackSpeed,
    Forward10,
    Backward10,
    Forward15,
    Backward15,
    Stop,
    Repeat,
    Shuffle,
    Record,
    Snapshot,
    PictureInPicture,
    Hdr,
    Zoom,
    RightSidebar,
    LeftSidebar,
    More,
    QuitVlc,
    VlcSettings,
    GoHome,
    Thumbnail,
    FileName,
    ArtistName,
    FolderName,
    PlaylistName,
    Properties,
    Replay,
    Chapters,
    Favorite,
    Bookmark,
    Bookmarks,
    Playlists,
    FrameStep,
    AbLoop,
}

impl ToolbarItem {
    /// Items offered in the customise dialog's palette, in the design's order.
    pub const PALETTE: &'static [ToolbarItem] = &[
        ToolbarItem::Spacer,
        ToolbarItem::Divider,
        ToolbarItem::PlaybackSpeed,
        ToolbarItem::Forward10,
        ToolbarItem::Backward10,
        ToolbarItem::Forward15,
        ToolbarItem::Backward15,
        ToolbarItem::Stop,
        ToolbarItem::Repeat,
        ToolbarItem::Shuffle,
        ToolbarItem::Record,
        ToolbarItem::Snapshot,
        ToolbarItem::PictureInPicture,
        ToolbarItem::Hdr,
        ToolbarItem::Zoom,
        ToolbarItem::RightSidebar,
        ToolbarItem::QuitVlc,
        ToolbarItem::VlcSettings,
        ToolbarItem::GoHome,
        ToolbarItem::Thumbnail,
        ToolbarItem::FileName,
        ToolbarItem::ArtistName,
        ToolbarItem::FolderName,
        ToolbarItem::PlaylistName,
        ToolbarItem::Properties,
        ToolbarItem::Replay,
        ToolbarItem::Chapters,
        ToolbarItem::Favorite,
        ToolbarItem::Bookmark,
        ToolbarItem::Bookmarks,
        ToolbarItem::Playlists,
        ToolbarItem::FrameStep,
        ToolbarItem::AbLoop,
        ToolbarItem::Fullscreen,
        ToolbarItem::LeftSidebar,
        ToolbarItem::More,
    ];

    pub fn label(self) -> &'static str {
        use ToolbarItem::*;
        match self {
            Previous => "Previous",
            PlayPause => "Play / Pause",
            Next => "Next",
            CurrentTime => "Current Time",
            SeekBar => "Seek Bar",
            Duration => "Duration",
            Volume => "Volume",
            AudioTracks => "Audio",
            SubtitleTracks => "Subtitles",
            QuickSettings => "Settings",
            Queue => "Queue",
            Fullscreen => "Fullscreen",
            Spacer => "Spacer",
            Divider => "Divider",
            PlaybackSpeed => "Playback Speed",
            Forward10 => "Forward 10s",
            Backward10 => "Backward 10s",
            Forward15 => "Forward 15s",
            Backward15 => "Backward 15s",
            Stop => "Stop",
            Repeat => "Repeat",
            Shuffle => "Shuffle",
            Record => "Record",
            Snapshot => "Snapshot",
            PictureInPicture => "Picture in Picture",
            Hdr => "HDR",
            Zoom => "Zoom",
            RightSidebar => "Right Sidebar",
            LeftSidebar => "Left Sidebar",
            More => "More",
            QuitVlc => "Quit VLC",
            VlcSettings => "VLC Settings",
            GoHome => "Go Home",
            Thumbnail => "Thumbnail",
            FileName => "File Name",
            ArtistName => "Artist Name",
            FolderName => "Folder Name",
            PlaylistName => "Playlist Name",
            Properties => "Properties",
            Replay => "Replay",
            Chapters => "Chapters",
            Favorite => "Favorite",
            Bookmark => "Bookmark",
            Bookmarks => "Bookmarks",
            Playlists => "Playlists",
            FrameStep => "Next Frame",
            AbLoop => "A-B Loop",
        }
    }

    /// Items that may appear more than once in a layout.
    pub fn repeatable(self) -> bool {
        matches!(self, ToolbarItem::Spacer | ToolbarItem::Divider)
    }
}

/// Rows of the control bar, top to bottom. A `SeekBar` alone with its time
/// labels on the first row gives the design's two-row layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolbarLayout {
    pub rows: Vec<Vec<ToolbarItem>>,
}

impl Default for ToolbarLayout {
    fn default() -> Self {
        use ToolbarItem::*;
        Self {
            rows: vec![vec![
                Previous,
                PlayPause,
                Next,
                CurrentTime,
                SeekBar,
                Duration,
                Volume,
                AudioTracks,
                SubtitleTracks,
                QuickSettings,
                Queue,
            ]],
        }
    }
}

impl ToolbarLayout {
    /// The design's alternative: seek bar on top, file info, transport and
    /// tools below.
    pub fn two_row() -> Self {
        use ToolbarItem::*;
        Self {
            rows: vec![
                vec![CurrentTime, SeekBar, Duration],
                vec![
                    More,
                    LeftSidebar,
                    Divider,
                    FolderName,
                    FileName,
                    Spacer,
                    Previous,
                    PlayPause,
                    Next,
                    Spacer,
                    Volume,
                    AudioTracks,
                    SubtitleTracks,
                    QuickSettings,
                    Queue,
                    Fullscreen,
                ],
            ],
        }
    }

    pub fn contains(&self, item: ToolbarItem) -> bool {
        self.rows.iter().any(|r| r.contains(&item))
    }

    /// Inserts `item` at `(row, index)`, removing an earlier copy unless the
    /// item is repeatable. Out-of-range positions append.
    pub fn insert(&mut self, item: ToolbarItem, row: usize, index: usize) {
        if !item.repeatable() {
            self.remove(item);
        }
        while self.rows.len() <= row {
            self.rows.push(Vec::new());
        }
        let r = &mut self.rows[row];
        let i = index.min(r.len());
        r.insert(i, item);
    }

    pub fn remove(&mut self, item: ToolbarItem) {
        for r in &mut self.rows {
            r.retain(|i| *i != item);
        }
        self.rows.retain(|r| !r.is_empty());
    }

    pub fn remove_at(&mut self, row: usize, index: usize) -> Option<ToolbarItem> {
        let r = self.rows.get_mut(row)?;
        if index >= r.len() {
            return None;
        }
        let item = r.remove(index);
        self.rows.retain(|r| !r.is_empty());
        Some(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_moves_unique_items() {
        let mut l = ToolbarLayout::default();
        l.insert(ToolbarItem::Queue, 0, 0);
        assert_eq!(l.rows[0][0], ToolbarItem::Queue);
        assert_eq!(l.rows[0].iter().filter(|i| **i == ToolbarItem::Queue).count(), 1);
    }

    #[test]
    fn spacers_repeat_and_rows_grow() {
        let mut l = ToolbarLayout::default();
        l.insert(ToolbarItem::Spacer, 0, 1);
        l.insert(ToolbarItem::Spacer, 0, 1);
        assert_eq!(l.rows[0].iter().filter(|i| **i == ToolbarItem::Spacer).count(), 2);
        l.insert(ToolbarItem::Stop, 3, 0);
        assert_eq!(l.rows.len(), 4);
        l.remove(ToolbarItem::Stop);
        assert!(l.rows.iter().all(|r| !r.is_empty()));
    }
}
