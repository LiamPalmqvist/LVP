//! Library pages: Home, Videos, Music, Network, Playlists, Recent, folders.
//!
//! Every page is first turned into a `model::PageModel` (breadcrumbs,
//! sections of tiles, what the header offers), then drawn by `view` as a
//! grid or list in library view, or by `side` as the compact column beside
//! the video.

mod cards;
mod dialog;
mod header;
mod menus;
mod model;
mod music;
mod network;
mod side;
mod view;

use egui::{Rect, Ui, pos2, vec2};
use std::path::PathBuf;

use crate::app::App;
use crate::domain::browse::SortOption;
use crate::domain::library::ViewMode;
use crate::domain::media::{MediaItem, MediaKind};
use crate::ui::state::{UiState, View};
use crate::ui::widgets::pal;

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
    /// A folder below the music folders (lists audio files).
    MusicFolder(PathBuf),
    /// Music > Songs: the music folders themselves.
    Songs,
    Albums,
    Artists,
    Genres,
    Album(String),
    Artist(String),
    Genre(String),
    /// Home section pages.
    ContinueWatching,
    ListenAgain,
    FrequentFolders,
    Favorites,
    /// A network discovery service (by its libvlc name) and what it found.
    Service(String),
    /// A container found on the network (an SMB share, a UPnP folder...).
    NetFolder {
        mrl: String,
        name: String,
    },
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
            Page::Home | Page::ContinueWatching | Page::ListenAgain | Page::FrequentFolders | Page::Favorites => {
                Section::Home
            }
            Page::Videos | Page::Folder(_) => Section::Videos,
            Page::Music
            | Page::MusicFolder(_)
            | Page::Songs
            | Page::Albums
            | Page::Artists
            | Page::Genres
            | Page::Album(_)
            | Page::Artist(_)
            | Page::Genre(_) => Section::Music,
            Page::Network | Page::Service(_) | Page::NetFolder { .. } => Section::Network,
            Page::Playlists | Page::Playlist(_) => Section::Playlists,
            Page::Recent => Section::Recent,
        })
    }
}

/// Library browsing state: view mode, sorting, search, selection, caches.
#[derive(Default)]
pub struct LibraryState {
    pub search: String,
    pub view: ViewMode,
    /// Chosen sort; `None` keeps each page's natural order.
    pub sort: Option<SortOption>,
    /// Selected tile on the current page (keyboard and single click).
    pub selected: Option<usize>,
    /// Columns in the last grid or list layout, for arrow keys.
    cols: usize,
    /// Set when the selection moved by keyboard so the view scrolls to it.
    scroll_to_selected: bool,
    /// The page the selection and search belong to.
    seen_page: Option<Page>,
    /// When the current page was entered: the second click of a
    /// double-click that opened it must not activate a tile on it.
    entered: Option<std::time::Instant>,
    /// The item a context-menu action applies to (media information,
    /// convert, stream), for the dialogs to read.
    pub action_item: Option<MediaItem>,
    /// Renaming a playlist: its id and the edited name.
    pub rename: Option<(u64, String)>,
    /// Asking before deleting a playlist.
    pub confirm_delete: Option<u64>,
    /// The "New Playlist" dialog (frame 6259:14954).
    pub new_playlist: dialog::NewPlaylist,
    /// Dragging a playlist row: (from index, current hover index).
    drag: Option<(usize, usize)>,
    cache: model::Cache,
    music: music::MusicIndex,
    net: network::NetState,
}

impl LibraryState {
    /// Forces the current page to be rebuilt on the next frame.
    pub fn invalidate(&mut self) {
        self.cache.invalidate();
    }

    fn sort_choice(&self) -> Option<SortOption> {
        self.sort
    }

    /// False just after navigating, while a double-click may still land.
    fn settled(&self) -> bool {
        self.entered.is_none_or(|t| t.elapsed() > std::time::Duration::from_millis(400))
    }
}

/// Plays `items` starting at `index`; video switches to the player.
pub(crate) fn play(app: &mut App, st: &mut UiState, items: Vec<MediaItem>, index: usize, origin: Option<String>) {
    if items.is_empty() {
        return;
    }
    let video = items.get(index).is_some_and(|i| i.kind() != MediaKind::Audio);
    app.play_items(items, index, origin);
    if video {
        st.view = View::Player;
    }
}

/// Called once per frame before drawing: resets per-page state on navigation.
fn track_page(st: &mut UiState) {
    if st.library.seen_page.as_ref() != Some(&st.page) {
        st.library.seen_page = Some(st.page.clone());
        st.library.selected = None;
        st.library.search.clear();
        st.library.drag = None;
        st.library.rename = None;
        st.library.entered = Some(std::time::Instant::now());
    }
}

/// Draws the current page in library view.
pub fn show(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    track_page(st);
    let p = pal(ui.ctx());
    let rect = ui.max_rect();
    let model = model::build(app, st, false);
    let header = Rect::from_min_size(rect.min, vec2(rect.width(), header::HEIGHT));
    header::show(ui, app, st, header, &model);
    let line_y = rect.min.y + header::HEIGHT + 17.0;
    ui.painter().hline(rect.x_range(), line_y + 0.5, egui::Stroke::new(1.0, p.divider_dim));
    let body = Rect::from_min_max(pos2(rect.min.x, line_y + 1.0), rect.max);
    ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| view::body(ui, app, st, &model));
    view::keyboard(ui, app, st, &model);
    dialog::show(ui.ctx(), app, st);
    menus::confirm_delete(ui.ctx(), app, st);
}

/// The narrow library column shown beside the video.
pub fn side_panel(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    track_page(st);
    let model = model::build(app, st, true);
    side::show(ui, app, st, &model);
    dialog::show(ui.ctx(), app, st);
    menus::confirm_delete(ui.ctx(), app, st);
}
