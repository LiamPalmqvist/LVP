//! What each library page shows, independent of how it is drawn: the
//! breadcrumbs, sections of tiles, and which header controls apply.
//!
//! Models are cached per page/search/sort and rebuilt every couple of
//! seconds, so large folders are listed once rather than every frame.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::app::App;
use crate::app::ports::ServiceCategory;
use crate::domain::browse::{SortOption, sort_folder, sort_items};
use crate::domain::library::{Entry, PlayRecord, SortOrder, matches_search};
use crate::domain::media::{MediaItem, MediaKind, MediaSource};
use crate::ui::icons::Icon;
use crate::ui::state::UiState;

use super::Page;

/// How tiles of a section are drawn in grid view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// 16:9 thumbnails (videos, video folders).
    Wide,
    /// Square covers and folder cards (music, playlists, services).
    Square,
}

/// Something a tile or the header "+" button does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    AddFolder { music: bool },
    CreatePlaylist,
    OpenStream,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TileKind {
    /// Opens a page (folders, playlists, albums, services).
    Nav(Page),
    /// A playable item: index into the section's `playable`.
    Media(usize),
    Action(Act),
}

#[derive(Debug, Clone)]
pub struct Tile {
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Icon,
    pub kind: TileKind,
    pub record: PlayRecord,
    /// An item whose thumbnail or cover stands for a non-media tile.
    pub cover: Option<MediaItem>,
    /// Watched or partly watched: drawn with dimmer text.
    pub dim: bool,
}

#[derive(Debug, Clone)]
pub struct SectionModel {
    pub title: Option<String>,
    /// Where the section header leads (Home rows).
    pub link: Option<Page>,
    pub shape: Shape,
    pub tiles: Vec<Tile>,
    /// The queue for media tiles, in display order.
    pub playable: Vec<MediaItem>,
    pub origin: Option<String>,
}

/// What kind of page a model is, for page-specific menus and controls.
#[derive(Debug, Clone, PartialEq)]
pub enum PageKind {
    Home,
    /// Videos or Music > Songs with its folders.
    Root {
        music: bool,
    },
    Folder {
        path: PathBuf,
        music: bool,
    },
    /// Music: Songs, Albums, Artists, Genres.
    MusicHome,
    /// Albums, Artists or Genres.
    Groups,
    /// Songs of one album, artist or genre.
    Group,
    Network,
    Service,
    NetFolder,
    Playlists,
    Playlist(u64),
    Recent,
    /// Continue Watching, Listen Again... in full.
    HomeList,
}

#[derive(Debug, Clone)]
pub struct PageModel {
    pub kind: PageKind,
    pub crumbs: Vec<(String, Option<Page>)>,
    pub sections: Vec<SectionModel>,
    pub add: Option<(&'static str, Act)>,
    pub sortable: bool,
    pub viewable: bool,
    pub loading: bool,
    pub error: Option<String>,
    pub empty: (&'static str, &'static str),
}

impl PageModel {
    fn new(kind: PageKind, crumbs: Vec<(String, Option<Page>)>) -> Self {
        Self {
            kind,
            crumbs,
            sections: Vec::new(),
            add: None,
            sortable: true,
            viewable: true,
            loading: false,
            error: None,
            empty: ("Nothing here", "This folder has no media files."),
        }
    }

    /// Items shown, not counting action tiles such as "Create playlist".
    pub fn count(&self) -> usize {
        self.sections.iter().flat_map(|s| &s.tiles).filter(|t| !matches!(t.kind, TileKind::Action(_))).count()
    }

    /// The single section of a flat page (folders, lists).
    pub fn main(&self) -> Option<&SectionModel> {
        (self.sections.len() == 1).then(|| &self.sections[0])
    }
}

// ---- cache ------------------------------------------------------------------

#[derive(Clone, PartialEq)]
struct Key {
    page: Page,
    search: String,
    sort: Option<SortOption>,
    compact: bool,
}

#[derive(Default)]
pub struct Cache {
    key: Option<Key>,
    model: Option<Arc<PageModel>>,
    built: Option<Instant>,
    /// Raw folder listings: (dir, music) -> when listed, entries.
    listings: HashMap<(PathBuf, bool), (Instant, Vec<Entry>)>,
    /// Items whose metadata was already requested this session.
    pub requested: std::collections::HashSet<String>,
}

impl Cache {
    pub fn invalidate(&mut self) {
        self.model = None;
        self.listings.clear();
    }

    /// Lists a folder, reusing a recent listing.
    fn listing(&mut self, app: &App, dir: &Path, music: bool) -> Vec<Entry> {
        let key = (dir.to_path_buf(), music);
        if let Some((at, v)) = self.listings.get(&key)
            && at.elapsed() < LISTING_TTL
        {
            return v.clone();
        }
        let kinds: &[MediaKind] = if music { &[MediaKind::Audio] } else { &[MediaKind::Video] };
        let v = app.library.list_dir(dir, kinds, app.settings.library.show_hidden, "", SortOrder::default());
        if self.listings.len() > 64 {
            self.listings.clear();
        }
        self.listings.insert(key, (Instant::now(), v.clone()));
        v
    }
}

/// How long a model is reused: discovery changes quickly, folders slowly.
fn model_ttl(page: &Page) -> Duration {
    match page {
        Page::Service(_) => Duration::from_secs(1),
        Page::Folder(_) | Page::MusicFolder(_) | Page::Videos | Page::Songs => Duration::from_secs(5),
        _ => Duration::from_secs(2),
    }
}
const LISTING_TTL: Duration = Duration::from_secs(10);

/// The model for the current page, rebuilt when stale.
pub fn build(app: &mut App, st: &mut UiState, compact: bool) -> Arc<PageModel> {
    let key = Key {
        page: st.page.clone(),
        search: st.library.search.trim().to_string(),
        sort: st.library.sort_choice(),
        compact,
    };
    let lib = &mut st.library;
    let fresh = lib.cache.built.is_some_and(|t| t.elapsed() < model_ttl(&key.page))
        && lib.cache.key.as_ref() == Some(&key)
        && !super::network::changed(&mut lib.net);
    if fresh && let Some(m) = &lib.cache.model {
        return m.clone();
    }
    let m = Arc::new(make(app, st, &key));
    let lib = &mut st.library;
    lib.cache.key = Some(key);
    lib.cache.built = Some(Instant::now());
    lib.cache.model = Some(m.clone());
    m
}

// ---- builders -----------------------------------------------------------------

fn make(app: &mut App, st: &mut UiState, key: &Key) -> PageModel {
    let q = key.search.as_str();
    let sort = key.sort;
    match &key.page {
        Page::Home => home(app, q, sort, key.compact),
        Page::Videos => root(app, st, q, sort, false),
        Page::Songs => root(app, st, q, sort, true),
        Page::Folder(p) => folder(app, st, p, q, sort, false),
        Page::MusicFolder(p) => folder(app, st, p, q, sort, true),
        Page::Music => music_home(q),
        Page::Albums | Page::Artists | Page::Genres => super::music::groups(app, st, &key.page, q),
        Page::Album(_) | Page::Artist(_) | Page::Genre(_) => super::music::group(app, st, &key.page, q, sort),
        Page::Network => network(app, q, key.compact),
        Page::Service(name) => super::network::service(app, st, name, q),
        Page::NetFolder { mrl, name } => super::network::folder(app, st, mrl, name, q),
        Page::Playlists => playlists(app, q, sort),
        Page::Playlist(id) => playlist(app, *id, q),
        Page::Recent => recent(app, q, sort),
        Page::ContinueWatching => home_list(app, &key.page, q, sort),
        Page::ListenAgain => home_list(app, &key.page, q, sort),
        Page::Favorites => home_list(app, &key.page, q, sort),
        Page::FrequentFolders => frequent(app, q),
    }
}

fn crumb(label: &str, page: Option<Page>) -> (String, Option<Page>) {
    (label.to_string(), page)
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string())
}

fn keep(item: &MediaItem, q: &str) -> bool {
    q.is_empty() || matches_search(item, q)
}

fn keep_text(text: &str, q: &str) -> bool {
    let t = text.to_lowercase();
    q.is_empty() || q.to_lowercase().split_whitespace().all(|w| t.contains(w))
}

/// A tile for a playable item.
pub fn media_tile(item: &MediaItem, record: PlayRecord, index: usize) -> Tile {
    let audio = item.is_audio();
    let title = if audio {
        item.meta.title.clone().filter(|t| !t.trim().is_empty()).unwrap_or_else(|| {
            item.source
                .path()
                .and_then(Path::file_stem)
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| item.source.display_name())
        })
    } else {
        item.title()
    };
    let dim = record.progress().is_some() || record.finished();
    Tile {
        title,
        subtitle: if audio { item.meta.artist.clone() } else { None },
        icon: match item.kind() {
            MediaKind::Audio => Icon::Music,
            MediaKind::Stream => Icon::Streaming,
            MediaKind::Disc => Icon::Disc,
            _ => Icon::Video,
        },
        kind: TileKind::Media(index),
        record,
        cover: None,
        dim,
    }
}

fn nav_tile(title: String, icon: Icon, page: Page) -> Tile {
    Tile {
        title,
        subtitle: None,
        icon,
        kind: TileKind::Nav(page),
        record: PlayRecord::default(),
        cover: None,
        dim: false,
    }
}

fn action_tile(title: &str, icon: Icon, act: Act) -> Tile {
    Tile {
        title: title.to_string(),
        subtitle: None,
        icon,
        kind: TileKind::Action(act),
        record: PlayRecord::default(),
        cover: None,
        dim: false,
    }
}

fn section(shape: Shape) -> SectionModel {
    SectionModel { title: None, link: None, shape, tiles: Vec::new(), playable: Vec::new(), origin: None }
}

/// A section of media items with their records.
fn media_section(items: Vec<(MediaItem, PlayRecord)>, shape: Shape, origin: Option<String>) -> SectionModel {
    let mut s = section(shape);
    for (i, (item, rec)) in items.into_iter().enumerate() {
        s.tiles.push(media_tile(&item, rec, i));
        s.playable.push(item);
    }
    s.origin = origin;
    s
}

fn shape_for(items: &[(MediaItem, PlayRecord)]) -> Shape {
    if !items.is_empty() && items.iter().all(|(i, _)| i.is_audio()) { Shape::Square } else { Shape::Wide }
}

// ---- Home -----------------------------------------------------------------------

fn home(app: &mut App, q: &str, sort: Option<SortOption>, compact: bool) -> PageModel {
    let sorted = |mut v: Vec<(MediaItem, PlayRecord)>| {
        if let Some(o) = sort {
            sort_items(&mut v, o);
        }
        v
    };
    let mut m = PageModel::new(PageKind::Home, vec![crumb("Home", Some(Page::Home))]);
    m.empty = ("Welcome to NotVLC", "Things you play show up here. Open a file or browse Videos and Music.");
    let n = if compact { 3 } else { 24 };

    let cw = sorted(app.library.continue_watching(n).into_iter().filter(|(i, _)| keep(i, q)).collect());
    if !cw.is_empty() {
        let mut s = media_section(cw, Shape::Wide, Some("Continue Watching".into()));
        s.title = Some("Continue Watching".into());
        s.link = Some(Page::ContinueWatching);
        m.sections.push(s);
    }
    let la = sorted(app.library.listen_again(n).into_iter().filter(|(i, _)| keep(i, q)).collect());
    if !la.is_empty() {
        let mut s = media_section(la, Shape::Square, Some("Listen Again".into()));
        s.title = Some("Listen Again".into());
        s.link = Some(Page::ListenAgain);
        m.sections.push(s);
    }
    let folders = folder_tiles(app, n, q);
    if !folders.is_empty() {
        let mut s = section(Shape::Square);
        s.title = Some("Frequently Visited Folders".into());
        s.link = Some(Page::FrequentFolders);
        s.tiles = folders;
        m.sections.push(s);
    }
    let lists: Vec<Tile> = app
        .library
        .playlists()
        .iter()
        .filter(|pl| keep_text(&pl.name, q))
        .take(n)
        .map(|pl| {
            let mut t = nav_tile(pl.name.clone(), Icon::Playlist, Page::Playlist(pl.id));
            t.subtitle = Some(items_label(pl.items.len()));
            t.cover = pl.items.first().cloned();
            t
        })
        .collect();
    if !lists.is_empty() {
        let mut s = section(Shape::Square);
        s.title = Some("Frequently Visited Playlists".into());
        s.link = Some(Page::Playlists);
        s.tiles = lists;
        m.sections.push(s);
    }
    let favs = sorted(favorites(app).into_iter().filter(|(i, _)| keep(i, q)).take(n).collect());
    if !favs.is_empty() {
        let shape = shape_for(&favs);
        let mut s = media_section(favs, shape, Some("Favorites".into()));
        s.title = Some("Favorites".into());
        s.link = Some(Page::Favorites);
        m.sections.push(s);
    }
    let streams: Vec<_> = app
        .settings
        .network
        .recent_urls
        .iter()
        .filter(|u| keep_text(u, q))
        .take(n)
        .map(|u| (MediaItem::new(MediaSource::from_user_input(u)), PlayRecord::default()))
        .collect();
    if !streams.is_empty() {
        let mut s = media_section(streams, Shape::Square, None);
        s.title = Some("Latest Streams".into());
        s.link = Some(Page::Network);
        m.sections.push(s);
    }
    m
}

pub fn items_label(n: usize) -> String {
    if n == 1 { "1 item".into() } else { format!("{n} items") }
}

fn favorites(app: &App) -> Vec<(MediaItem, PlayRecord)> {
    let mut v: Vec<_> = app
        .library
        .favorites()
        .into_iter()
        .map(|i| {
            let r = app.library.record(&i.source);
            (i, r)
        })
        .collect();
    v.sort_by(|a, b| b.1.last_played.cmp(&a.1.last_played));
    v
}

/// Whether a folder lies below one of the music folders.
pub fn is_music_dir(app: &App, dir: &Path) -> bool {
    let music = app.settings.library.music_folders.iter().any(|r| dir.starts_with(r));
    let video = app.settings.library.video_folders.iter().any(|r| dir.starts_with(r));
    music && !video
}

pub fn folder_page(app: &App, dir: &Path) -> Page {
    if is_music_dir(app, dir) { Page::MusicFolder(dir.to_path_buf()) } else { Page::Folder(dir.to_path_buf()) }
}

fn folder_tiles(app: &App, n: usize, q: &str) -> Vec<Tile> {
    app.library
        .frequent_folders(n)
        .into_iter()
        .filter(|(p, _)| keep_text(&file_name(p), q))
        .map(|(p, _)| nav_tile(file_name(&p), Icon::Folder, folder_page(app, &p)))
        .collect()
}

fn frequent(app: &App, q: &str) -> PageModel {
    let mut m = PageModel::new(
        PageKind::HomeList,
        vec![crumb("Home", Some(Page::Home)), crumb("Frequently Visited Folders", None)],
    );
    m.sortable = false;
    m.empty = ("No folders yet", "Folders you play from show up here.");
    let mut s = section(Shape::Square);
    s.tiles = folder_tiles(app, 200, q);
    m.sections.push(s);
    m
}

fn home_list(app: &App, page: &Page, q: &str, sort: Option<SortOption>) -> PageModel {
    let (title, mut items, shape) = match page {
        Page::ContinueWatching => ("Continue Watching", app.library.continue_watching(500), Shape::Wide),
        Page::ListenAgain => ("Listen Again", app.library.listen_again(500), Shape::Square),
        _ => {
            let f = favorites(app);
            let shape = shape_for(&f);
            ("Favorites", f, shape)
        }
    };
    items.retain(|(i, _)| keep(i, q));
    if let Some(o) = sort {
        sort_items(&mut items, o);
    }
    let mut m = PageModel::new(PageKind::HomeList, vec![crumb("Home", Some(Page::Home)), crumb(title, None)]);
    m.empty = ("Nothing here yet", "Play something and it will show up here.");
    m.sections.push(media_section(items, shape, Some(title.to_string())));
    m
}

// ---- Videos, Music > Songs, folders -------------------------------------------

fn roots(app: &App, music: bool) -> Vec<PathBuf> {
    let l = &app.settings.library;
    if music { l.music_folders.clone() } else { l.video_folders.clone() }
}

fn root_crumbs(music: bool) -> Vec<(String, Option<Page>)> {
    if music {
        vec![crumb("Music", Some(Page::Music)), crumb("Songs", Some(Page::Songs))]
    } else {
        vec![crumb("Videos", Some(Page::Videos))]
    }
}

fn root(app: &mut App, st: &mut UiState, q: &str, sort: Option<SortOption>, music: bool) -> PageModel {
    let roots = roots(app, music);
    if roots.len() == 1 {
        let mut m = folder(app, st, &roots[0], q, sort, music);
        m.kind = PageKind::Root { music };
        m.crumbs = root_crumbs(music);
        m.add = Some(("Add a folder", Act::AddFolder { music }));
        return m;
    }
    let mut m = PageModel::new(PageKind::Root { music }, root_crumbs(music));
    m.add = Some(("Add a folder", Act::AddFolder { music }));
    m.sortable = false;
    m.empty = if music {
        ("No music folders", "Add a folder with music to see it here.")
    } else {
        ("No video folders", "Add a folder with videos to see it here.")
    };
    let mut s = section(if music { Shape::Square } else { Shape::Wide });
    for r in roots.iter().filter(|r| keep_text(&file_name(r), q)) {
        let page = if music { Page::MusicFolder(r.clone()) } else { Page::Folder(r.clone()) };
        s.tiles.push(nav_tile(file_name(r), Icon::Folder, page));
    }
    m.sections.push(s);
    m
}

/// Breadcrumbs for a folder: the section, then each folder below its root.
fn folder_crumbs(app: &App, dir: &Path, music: bool) -> Vec<(String, Option<Page>)> {
    let roots = roots(app, music);
    let mk = |p: &Path| if music { Page::MusicFolder(p.to_path_buf()) } else { Page::Folder(p.to_path_buf()) };
    let mut out = root_crumbs(music);
    let root = roots.iter().filter(|r| dir.starts_with(r)).max_by_key(|r| r.components().count());
    let (base, mut acc) = match root {
        Some(r) => {
            if roots.len() > 1 {
                out.push((file_name(r), Some(mk(r))));
            }
            (r.clone(), r.clone())
        }
        None => (PathBuf::from("/"), PathBuf::from("/")),
    };
    if let Ok(rest) = dir.strip_prefix(&base) {
        for c in rest.components() {
            acc.push(c);
            out.push((c.as_os_str().to_string_lossy().into_owned(), Some(mk(&acc))));
        }
    }
    if let Some(last) = out.last_mut() {
        last.1 = None;
    }
    out
}

fn folder(app: &mut App, st: &mut UiState, dir: &Path, q: &str, sort: Option<SortOption>, music: bool) -> PageModel {
    let mut entries = st.library.cache.listing(app, dir, music);
    entries.retain(|e| keep(&e.item, q));
    // Video folders put what is left to watch first, as in the design (970:2703).
    let natural = if music { SortOption::TitleAsc } else { SortOption::UnwatchedFirst };
    sort_folder(&mut entries, sort.unwrap_or(natural));
    let mut m = PageModel::new(PageKind::Folder { path: dir.to_path_buf(), music }, folder_crumbs(app, dir, music));
    if !dir.is_dir() {
        m.error = Some(format!("{} is not available.", dir.display()));
    }
    let mut s = section(if music { Shape::Square } else { Shape::Wide });
    s.origin = Some(file_name(dir));
    for e in entries {
        if e.is_dir {
            let page = if music {
                Page::MusicFolder(e.item.source.path().unwrap_or(dir).to_path_buf())
            } else {
                Page::Folder(e.item.source.path().unwrap_or(dir).to_path_buf())
            };
            s.tiles.push(nav_tile(e.item.title(), Icon::Folder, page));
        } else {
            let i = s.playable.len();
            s.tiles.push(media_tile(&e.item, e.record.clone(), i));
            s.playable.push(e.item);
        }
    }
    m.sections.push(s);
    m
}

// ---- Music -------------------------------------------------------------------------

fn music_home(q: &str) -> PageModel {
    let mut m = PageModel::new(PageKind::MusicHome, vec![crumb("Music", Some(Page::Music))]);
    m.sortable = false;
    m.add = Some(("Add a folder", Act::AddFolder { music: true }));
    let mut s = section(Shape::Square);
    for (name, icon, page) in [
        ("Songs", Icon::Music, Page::Songs),
        ("Albums", Icon::Album, Page::Albums),
        ("Artists", Icon::Artist, Page::Artists),
        ("Genres", Icon::Genre, Page::Genres),
    ] {
        if keep_text(name, q) {
            s.tiles.push(nav_tile(name.into(), icon, page));
        }
    }
    m.sections.push(s);
    m
}

pub(super) fn group_model(
    kind: PageKind,
    crumbs: Vec<(String, Option<Page>)>,
    sec: SectionModel,
    loading: bool,
) -> PageModel {
    let mut m = PageModel::new(kind, crumbs);
    m.loading = loading;
    m.empty = ("No music found", "Songs in your music folders show up here once they are scanned.");
    m.sections.push(sec);
    m
}

pub(super) fn songs_section(
    mut items: Vec<(MediaItem, PlayRecord)>,
    sort: Option<SortOption>,
    origin: &str,
) -> SectionModel {
    match sort {
        Some(o) => sort_items(&mut items, o),
        None => {
            sort_items(&mut items, SortOption::TrackAsc);
            sort_items(&mut items, SortOption::DiscAsc);
        }
    }
    media_section(items, Shape::Square, Some(origin.to_string()))
}

pub(super) fn group_tile(
    title: String,
    subtitle: Option<String>,
    icon: Icon,
    page: Page,
    cover: Option<MediaItem>,
) -> Tile {
    let mut t = nav_tile(title, icon, page);
    t.subtitle = subtitle;
    t.cover = cover;
    t
}

pub(super) fn square_section(tiles: Vec<Tile>) -> SectionModel {
    let mut s = section(Shape::Square);
    s.tiles = tiles;
    s
}

pub(super) fn keep_q(text: &str, q: &str) -> bool {
    keep_text(text, q)
}

pub(super) fn keep_item(item: &MediaItem, q: &str) -> bool {
    keep(item, q)
}

pub(super) fn crumb_of(label: &str, page: Option<Page>) -> (String, Option<Page>) {
    crumb(label, page)
}

pub(super) fn model_of(kind: PageKind, crumbs: Vec<(String, Option<Page>)>) -> PageModel {
    PageModel::new(kind, crumbs)
}

pub(super) fn media_section_of(
    items: Vec<(MediaItem, PlayRecord)>,
    shape: Shape,
    origin: Option<String>,
) -> SectionModel {
    media_section(items, shape, origin)
}

pub(super) fn nav_tile_of(title: String, icon: Icon, page: Page) -> Tile {
    nav_tile(title, icon, page)
}

// ---- Network ----------------------------------------------------------------------------

pub fn category_label(c: ServiceCategory) -> &'static str {
    match c {
        ServiceCategory::Lan => "Local Network",
        ServiceCategory::Devices => "Devices",
        ServiceCategory::Podcasts => "Podcasts",
        ServiceCategory::LocalDirs => "My Folders",
    }
}

fn network(app: &App, q: &str, compact: bool) -> PageModel {
    let mut m = PageModel::new(PageKind::Network, vec![crumb("Network", Some(Page::Network))]);
    m.sortable = false;
    m.add = Some(("Open Network Stream", Act::OpenStream));
    m.empty = ("No network sources", "Open a network stream to play a URL.");
    let services = app.engine.services();
    let mut flat = section(Shape::Square);
    if compact {
        flat.tiles.push(action_tile("Add Network Stream", Icon::Add, Act::OpenStream));
    }
    for cat in [ServiceCategory::Lan, ServiceCategory::Devices, ServiceCategory::Podcasts, ServiceCategory::LocalDirs] {
        let tiles: Vec<Tile> = services
            .iter()
            .filter(|s| s.category == cat && keep_text(&s.long_name, q))
            .map(|s| nav_tile(s.long_name.clone(), Icon::Streaming, Page::Service(s.name.clone())))
            .collect();
        if tiles.is_empty() {
            continue;
        }
        if compact {
            flat.tiles.extend(tiles);
        } else {
            let mut s = section(Shape::Square);
            s.title = Some(category_label(cat).into());
            s.tiles = tiles;
            m.sections.push(s);
        }
    }
    let streams: Vec<_> = app
        .settings
        .network
        .recent_urls
        .iter()
        .filter(|u| keep_text(u, q))
        .map(|u| (MediaItem::new(MediaSource::from_user_input(u)), PlayRecord::default()))
        .collect();
    if compact {
        for (i, (item, rec)) in streams.into_iter().enumerate() {
            let mut t = media_tile(&item, rec, i);
            t.subtitle = Some(item.source.mrl());
            flat.tiles.push(t);
            flat.playable.push(item);
        }
        m.sections.push(flat);
    } else {
        // "Streams" first: open a URL, then the recent ones.
        let mut s = media_section(streams, Shape::Square, None);
        for (t, item) in s.tiles.iter_mut().zip(&s.playable) {
            t.subtitle = Some(item.source.mrl());
        }
        s.tiles.insert(0, action_tile("Open Network Stream", Icon::Add, Act::OpenStream));
        s.title = Some("Streams".into());
        m.sections.insert(0, s);
    }
    m
}

// ---- Playlists -------------------------------------------------------------------------

fn playlists(app: &App, q: &str, sort: Option<SortOption>) -> PageModel {
    let mut m = PageModel::new(PageKind::Playlists, vec![crumb("Playlists", Some(Page::Playlists))]);
    m.add = Some(("Create playlist", Act::CreatePlaylist));
    let mut s = section(Shape::Square);
    s.tiles.push(action_tile("Create playlist", Icon::Add, Act::CreatePlaylist));
    let mut lists: Vec<&crate::domain::playlist::Playlist> =
        app.library.playlists().iter().filter(|pl| keep_text(&pl.name, q)).collect();
    match sort {
        Some(SortOption::TitleAsc) => lists.sort_by(|a, b| crate::domain::library::natural_cmp(&a.name, &b.name)),
        Some(SortOption::TitleDesc) => lists.sort_by(|a, b| crate::domain::library::natural_cmp(&b.name, &a.name)),
        Some(SortOption::NewestFirst) => lists.sort_by(|a, b| b.created.cmp(&a.created)),
        Some(SortOption::OldestFirst) => lists.sort_by(|a, b| a.created.cmp(&b.created)),
        _ => {}
    }
    for pl in lists {
        let mut t = nav_tile(pl.name.clone(), Icon::Playlist, Page::Playlist(pl.id));
        t.subtitle = Some(items_label(pl.items.len()));
        t.cover = pl.items.first().cloned();
        s.tiles.push(t);
    }
    m.sections.push(s);
    m
}

fn playlist(app: &App, id: u64, q: &str) -> PageModel {
    let Some(pl) = app.library.playlist(id) else {
        let mut m = PageModel::new(PageKind::Playlists, vec![crumb("Playlists", Some(Page::Playlists))]);
        m.error = Some("This playlist no longer exists.".into());
        m.sections.push(section(Shape::Square));
        return m;
    };
    let mut m =
        PageModel::new(PageKind::Playlist(id), vec![crumb("Playlists", Some(Page::Playlists)), crumb(&pl.name, None)]);
    m.sortable = false;
    m.empty = ("This playlist is empty", "Add files with the button above, or from any item's menu.");
    let items: Vec<(MediaItem, PlayRecord)> = pl
        .items
        .iter()
        .filter(|i| keep(i, q))
        .map(|i| {
            let mut i = i.clone();
            app.library.enrich(&mut i);
            let r = app.library.record(&i.source);
            (i, r)
        })
        .collect();
    let shape = shape_for(&items);
    m.sections.push(media_section(items, shape, Some(pl.name.clone())));
    m
}

// ---- Recent ------------------------------------------------------------------------------

fn recent(app: &App, q: &str, sort: Option<SortOption>) -> PageModel {
    let mut items = app.library.recent();
    items.retain(|(i, _)| keep(i, q));
    if let Some(o) = sort {
        sort_items(&mut items, o);
    }
    let mut m = PageModel::new(PageKind::Recent, vec![crumb("Recent", Some(Page::Recent))]);
    m.empty = ("No history", "Files and streams you play show up here.");
    let shape = shape_for(&items);
    m.sections.push(media_section(items, shape, Some("Recent".into())));
    m
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::app::fake::{FakeEngine, FakeProbe};
    use crate::app::testing::MemStore;

    fn setup() -> (tempfile::TempDir, App, UiState) {
        let dir = tempfile::tempdir().unwrap();
        let mut app = App::new(Arc::new(FakeEngine::new()), Arc::new(FakeProbe), Arc::new(MemStore::new(dir.path())));
        let videos = dir.path().join("Videos");
        let season = videos.join("Shows/S01");
        std::fs::create_dir_all(season.join("Extras")).unwrap();
        for n in ["E2.mp4", "E10.mp4", "E1.mp4", "notes.txt", "song.mp3"] {
            std::fs::write(season.join(n), b"").unwrap();
        }
        app.settings.library.video_folders = vec![videos];
        (dir, app, UiState::default())
    }

    #[test]
    fn folder_crumbs_follow_the_library_root() {
        let (dir, app, _) = setup();
        let season = dir.path().join("Videos/Shows/S01");
        let c = folder_crumbs(&app, &season, false);
        let labels: Vec<_> = c.iter().map(|c| c.0.as_str()).collect();
        assert_eq!(labels, vec!["Videos", "Shows", "S01"]);
        assert_eq!(c[0].1, Some(Page::Videos));
        assert_eq!(c[1].1, Some(Page::Folder(dir.path().join("Videos/Shows"))));
        assert_eq!(c[2].1, None);
    }

    #[test]
    fn folder_lists_subfolders_then_videos_in_natural_order() {
        let (dir, mut app, mut st) = setup();
        let season = dir.path().join("Videos/Shows/S01");
        let m = folder(&mut app, &mut st, &season, "", None, false);
        let sec = m.main().unwrap();
        let titles: Vec<_> = sec.tiles.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(titles, vec!["Extras", "E1.mp4", "E2.mp4", "E10.mp4"]);
        assert_eq!(sec.playable.len(), 3);
        assert_eq!(sec.tiles[2].kind, TileKind::Media(1));
        assert_eq!(m.count(), 4);
        let found = folder(&mut app, &mut st, &season, "e10", None, false);
        assert_eq!(found.count(), 1);
    }

    #[test]
    fn videos_root_with_one_folder_shows_its_contents() {
        let (_dir, mut app, mut st) = setup();
        let m = root(&mut app, &mut st, "", None, false);
        assert_eq!(m.crumbs.len(), 1);
        assert_eq!(m.main().unwrap().tiles[0].title, "Shows");
        assert!(m.add.is_some());
    }

    #[test]
    fn playlists_page_starts_with_create() {
        let (_dir, mut app, _) = setup();
        app.library.create_playlist("B list", Vec::new());
        app.library.create_playlist("A list", vec![MediaItem::file("/x/a.mp4")]);
        let m = playlists(&app, "", Some(SortOption::TitleAsc));
        let titles: Vec<_> = m.main().unwrap().tiles.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(titles, vec!["Create playlist", "A list", "B list"]);
        assert_eq!(m.count(), 2);
    }
}
