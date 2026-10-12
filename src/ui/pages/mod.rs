//! Library pages: Home, Videos, Music, Network, Playlists, Recent, folders.

use std::path::PathBuf;

use egui::Ui;

use crate::app::App;
use crate::ui::state::UiState;
use crate::ui::widgets::{self, pal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Home,
    Videos,
    Music,
    Network,
    Playlists,
    Recent,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Page {
    #[default]
    Home,
    Videos,
    Music,
    Network,
    Playlists,
    Recent,
    /// A folder on disk, reached from Videos, Music or the toolbar.
    Folder(PathBuf),
    Playlist(u64),
}

impl Page {
    pub fn of(s: Section) -> Page {
        match s {
            Section::Home => Page::Home,
            Section::Videos => Page::Videos,
            Section::Music => Page::Music,
            Section::Network => Page::Network,
            Section::Playlists => Page::Playlists,
            Section::Recent => Page::Recent,
        }
    }

    /// Which sidebar entry is highlighted for this page.
    pub fn section(&self) -> Option<Section> {
        Some(match self {
            Page::Home => Section::Home,
            Page::Videos | Page::Folder(_) => Section::Videos,
            Page::Music => Section::Music,
            Page::Network => Section::Network,
            Page::Playlists | Page::Playlist(_) => Section::Playlists,
            Page::Recent => Section::Recent,
        })
    }
}

#[derive(Default)]
pub struct LibraryState {
    pub search: String,
}

/// Draws the current page in library view.
pub fn show(ui: &mut Ui, _app: &mut App, st: &mut UiState) {
    let p = pal(ui.ctx());
    ui.label(egui::RichText::new(format!("{:?}", st.page)).color(p.fg_a2));
    widgets::empty_state(ui, crate::ui::icons::Icon::Folder, "Library", "This page is being built.");
}

/// The narrow library column shown beside the video.
pub fn side_panel(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    show(ui, app, st);
}
