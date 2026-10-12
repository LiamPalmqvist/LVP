//! UI-only state: what is shown where. Each feature module owns its own
//! sub-state type; this struct just holds them together.

use std::time::Instant;

use crate::app::UiCommand;

use super::{dialogs, music, pages, panels, settings};

/// Library browsing or the player taking the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Library,
    Player,
}

/// Popovers anchored to the control bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerPanel {
    /// Queue / Bookmarks / Chapters, with the tab index.
    Queue(usize),
    /// Track lists: 0 video, 1 audio, 2 subtitles.
    Tracks(usize),
    /// Playback / Video / Audio / Subtitles quick settings.
    QuickSettings(usize),
}

pub struct UiState {
    pub view: View,
    pub page: pages::Page,
    pub back: Vec<pages::Page>,
    pub forward: Vec<pages::Page>,
    /// Labels in the library sidebar (expanded) or icons only.
    pub sidebar_expanded: bool,
    /// The library column to the left of the video in player view.
    pub side_panel: bool,
    pub fullscreen: bool,
    /// "Hide UI": video only, controls reappear on demand.
    pub hide_ui: bool,
    pub always_on_top: bool,
    /// Compact window with just the picture (picture in picture).
    pub pip: bool,
    pub panel: Option<PlayerPanel>,
    /// Controls stay visible until this instant when auto-hiding.
    pub controls_until: Instant,
    /// Commands raised anywhere in the UI, handled at the end of the frame.
    pub pending: Vec<UiCommand>,
    pub library: pages::LibraryState,
    pub panels: panels::PanelsState,
    pub settings: settings::SettingsState,
    pub dialogs: dialogs::DialogsState,
    pub music: music::MusicState,
    /// Hover time over the seek bar, for the preview bubble.
    pub seek_hover: Option<(f32, i64)>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            view: View::Library,
            page: pages::Page::default(),
            back: Vec::new(),
            forward: Vec::new(),
            sidebar_expanded: true,
            side_panel: false,
            fullscreen: false,
            hide_ui: false,
            always_on_top: false,
            pip: false,
            panel: None,
            controls_until: Instant::now(),
            pending: Vec::new(),
            library: Default::default(),
            panels: Default::default(),
            settings: Default::default(),
            dialogs: Default::default(),
            music: Default::default(),
            seek_hover: None,
        }
    }
}

impl UiState {
    /// Goes to a library page, recording history for back/forward.
    pub fn navigate(&mut self, page: pages::Page) {
        if self.page != page {
            let old = std::mem::replace(&mut self.page, page);
            self.back.push(old);
            self.forward.clear();
        }
        self.view = View::Library;
    }

    pub fn go_back(&mut self) {
        if let Some(p) = self.back.pop() {
            let cur = std::mem::replace(&mut self.page, p);
            self.forward.push(cur);
        }
    }

    pub fn go_forward(&mut self) {
        if let Some(p) = self.forward.pop() {
            let cur = std::mem::replace(&mut self.page, p);
            self.back.push(cur);
        }
    }

    pub fn toggle_panel(&mut self, p: PlayerPanel) {
        self.panel = if self.panel == Some(p) { None } else { Some(p) };
    }

    pub fn command(&mut self, c: UiCommand) {
        self.pending.push(c);
    }

    pub fn bump_controls(&mut self, secs: f32) {
        self.controls_until = Instant::now() + std::time::Duration::from_secs_f32(secs.max(0.5));
    }
}
