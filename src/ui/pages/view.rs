//! Draws a page model in library view: a virtualised grid or list for flat
//! pages, titled rows for Home, plus the playlist action bar, tile
//! interaction (select, open, play, drag to reorder) and keyboard control.

use egui::{Align2, Rect, Sense, Stroke, Ui, pos2, vec2};

use crate::app::App;
use crate::domain::library::ViewMode;
use crate::domain::media::{IMAGE_EXTENSIONS, MediaItem};
use crate::ui::icons::Icon;
use crate::ui::state::UiState;
use crate::ui::theme::font;
use crate::ui::widgets::{self, ButtonKind, pal};

use super::cards::{self, card_h};
use super::model::{Act, PageKind, PageModel, SectionModel, Shape, TileKind};
use super::{Page, menus, play};

pub fn run_act(app: &mut App, st: &mut UiState, act: Act) {
    match act {
        Act::AddFolder { music } => {
            let title = if music { "Add Music Folder" } else { "Add Video Folder" };
            if let Some(dir) = rfd::FileDialog::new().set_title(title).pick_folder() {
                app.add_library_folder(dir, music);
                st.library.invalidate();
            }
        }
        Act::CreatePlaylist => st.dialogs.new_playlist = true,
        Act::OpenStream => st.dialogs.open_network = true,
    }
}

/// Opens a folder or page, plays an item, or runs an action.
pub fn activate(app: &mut App, st: &mut UiState, sec: &SectionModel, ti: usize) {
    let Some(tile) = sec.tiles.get(ti) else { return };
    match &tile.kind {
        TileKind::Nav(page) => st.navigate(page.clone()),
        TileKind::Media(i) => play(app, st, sec.playable.clone(), *i, sec.origin.clone()),
        TileKind::Action(a) => run_act(app, st, *a),
    }
}

/// Asks for metadata once per item, only for tiles that are on screen.
pub fn want_meta(app: &mut App, st: &mut UiState, item: &MediaItem) {
    if st.library.cache.requested.len() > 50_000 {
        st.library.cache.requested.clear();
    }
    if st.library.cache.requested.insert(item.source.mrl()) {
        app.request_meta(item);
    }
}

/// Right-click menus for a tile.
pub fn tile_menu(resp: &egui::Response, app: &mut App, st: &mut UiState, m: &PageModel, sec: &SectionModel, ti: usize) {
    let Some(tile) = sec.tiles.get(ti) else { return };
    match &tile.kind {
        TileKind::Media(i) => menus::item_menu(resp, app, st, &m.kind, sec, *i),
        TileKind::Nav(Page::Folder(d)) => {
            let root = app.settings.library.video_folders.contains(d);
            menus::folder_menu(resp, app, st, d.clone(), false, root);
        }
        TileKind::Nav(Page::MusicFolder(d)) => {
            let root = app.settings.library.music_folders.contains(d);
            menus::folder_menu(resp, app, st, d.clone(), true, root);
        }
        TileKind::Nav(Page::Playlist(id)) => menus::playlist_menu(resp, app, st, *id),
        _ => {}
    }
}

pub fn body(ui: &mut Ui, app: &mut App, st: &mut UiState, m: &PageModel) {
    let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(ui.max_rect()));
    let ui = &mut ui;
    if let PageKind::Playlist(id) = m.kind {
        playlist_bar(ui, app, st, id, m);
    }
    if let Some(err) = &m.error {
        widgets::empty_state(ui, Icon::Info, "Not available", err);
        return;
    }
    let has_action = m.sections.iter().flat_map(|s| &s.tiles).any(|t| matches!(t.kind, TileKind::Action(_)));
    if m.count() == 0 && !has_action {
        if m.loading {
            ui.add_space(80.0);
            ui.vertical_centered(|ui| ui.add(egui::Spinner::new().size(28.0)));
        } else {
            widgets::empty_state(ui, Icon::Folder, m.empty.0, m.empty.1);
            if let Some((label, act)) = m.add {
                ui.add_space(16.0);
                ui.vertical_centered(|ui| {
                    if widgets::button_with(ui, Some(Icon::Add), label, ButtonKind::Primary, vec2(0.0, 36.0)).clicked()
                    {
                        run_act(app, st, act);
                    }
                });
            }
        }
        return;
    }
    match m.main().filter(|s| s.title.is_none()) {
        Some(sec) => flat(ui, app, st, m, sec),
        None => sections(ui, app, st, m),
    }
}

/// Extra room at the bottom so the last row can scroll clear of the
/// floating mini player.
fn bottom_pad(app: &App) -> f32 {
    if app.has_media() { 360.0 } else { 20.0 }
}

struct Layout {
    cols: usize,
    cell: egui::Vec2,
    gap: egui::Vec2,
    top: f32,
}

impl Layout {
    fn new(width: f32, list: bool, shape: Shape, subtitle: bool) -> Self {
        let (min_w, gap) = if list {
            (406.0, vec2(10.0, 10.0))
        } else {
            match shape {
                Shape::Wide => (316.0, vec2(20.0, 40.0)),
                Shape::Square => (200.0, vec2(20.0, 40.0)),
            }
        };
        let cols = (((width + gap.x) / (min_w + gap.x)).floor() as usize).max(1);
        let w = ((width - gap.x * (cols - 1) as f32) / cols as f32).floor();
        let h = if list { 32.0 } else { card_h(shape, w, subtitle) };
        Self { cols, cell: vec2(w, h), gap, top: if list { 30.0 } else { 20.0 } }
    }

    fn rows(&self, n: usize) -> usize {
        n.div_ceil(self.cols)
    }

    fn height(&self, n: usize) -> f32 {
        let rows = self.rows(n) as f32;
        self.top + rows * self.cell.y + (rows - 1.0).max(0.0) * self.gap.y
    }

    fn cell_rect(&self, origin: egui::Pos2, i: usize) -> Rect {
        let (r, c) = (i / self.cols, i % self.cols);
        let x = origin.x + c as f32 * (self.cell.x + self.gap.x);
        let y = origin.y + self.top + r as f32 * (self.cell.y + self.gap.y);
        Rect::from_min_size(pos2(x, y), self.cell)
    }
}

fn flat(ui: &mut Ui, app: &mut App, st: &mut UiState, m: &PageModel, sec: &SectionModel) {
    let list = st.library.view == ViewMode::List && m.viewable;
    let subtitle = sec.tiles.iter().any(|t| t.subtitle.is_some());
    let n = sec.tiles.len();
    let reorder = match m.kind {
        PageKind::Playlist(id) if st.library.search.trim().is_empty() => Some(id),
        _ => None,
    };
    let pad = bottom_pad(app);
    egui::ScrollArea::vertical()
        .id_salt(("lib-flat", st.library.view == ViewMode::List))
        .auto_shrink(false)
        .show_viewport(ui, |ui, vp| {
            let width = ui.available_width();
            let lay = Layout::new(width, list, sec.shape, subtitle);
            st.library.cols = lay.cols;
            let total = lay.height(n) + pad;
            let (rect, _) = ui.allocate_exact_size(vec2(width, total), Sense::hover());
            let origin = rect.min;
            let stride = lay.cell.y + lay.gap.y;
            let first = (((vp.min.y - lay.top) / stride).floor().max(0.0) as usize) * lay.cols;
            let last = ((((vp.max.y - lay.top) / stride).ceil().max(0.0) as usize) + 1) * lay.cols;
            for ti in first..last.min(n) {
                let r = lay.cell_rect(origin, ti);
                tile(ui, app, st, m, sec, ti, r, list, reorder);
            }
            if let (Some(id), Some((from, to))) = (reorder, st.library.drag) {
                drag_feedback(ui, app, st, &lay, origin, n, id, from, to);
            }
            if st.library.scroll_to_selected
                && let Some(sel) = st.library.selected.filter(|s| *s < n)
            {
                ui.scroll_to_rect(lay.cell_rect(origin, sel).expand(8.0), None);
                st.library.scroll_to_selected = false;
            }
        });
}

#[allow(clippy::too_many_arguments)]
fn tile(
    ui: &mut Ui,
    app: &mut App,
    st: &mut UiState,
    m: &PageModel,
    sec: &SectionModel,
    ti: usize,
    rect: Rect,
    list: bool,
    reorder: Option<u64>,
) {
    let t = &sec.tiles[ti];
    let sense = if reorder.is_some() { Sense::click_and_drag() } else { Sense::click() };
    let resp = ui.interact(rect, ui.id().with(("lib-tile", ti)), sense);
    let hovered = resp.hovered() && st.library.drag.is_none();
    let selected = st.library.selected == Some(ti);
    let item = match t.kind {
        TileKind::Media(i) => sec.playable.get(i),
        _ => None,
    };
    if let Some(item) = item {
        want_meta(app, st, item);
    }
    let mut play_hit = false;
    if list {
        cards::list_row(ui, app, rect, t, item, hovered, selected, true);
        let th = Rect::from_min_size(rect.min, vec2(32.0, rect.height()));
        play_hit = item.is_some() && ui.input(|i| i.pointer.interact_pos()).is_some_and(|p| th.contains(p));
    } else {
        let (_, badge) = cards::grid_card(ui, app, rect, sec.shape, t, item, hovered, selected);
        if let Some(b) = badge {
            play_hit = ui.input(|i| i.pointer.interact_pos()).is_some_and(|p| b.contains(p));
        }
    }
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if resp.clicked() {
        st.library.selected = Some(ti);
        if item.is_none() || play_hit {
            activate(app, st, sec, ti);
        }
    } else if resp.double_clicked() && item.is_some() {
        activate(app, st, sec, ti);
    }
    if reorder.is_some() {
        if resp.drag_started() {
            st.library.drag = Some((ti, ti));
        }
        if resp.drag_stopped()
            && let (Some(id), Some((from, to))) = (reorder, st.library.drag.take())
            && let (TileKind::Media(a), Some(TileKind::Media(b))) =
                (&sec.tiles[from].kind, sec.tiles.get(to).map(|t| &t.kind))
        {
            app.move_playlist_item(id, *a, *b);
            st.library.selected = Some(to);
            st.library.invalidate();
        }
    }
    tile_menu(&resp, app, st, m, sec, ti);
}

/// While dragging a playlist entry: track the target slot and draw a marker.
#[allow(clippy::too_many_arguments)]
fn drag_feedback(
    ui: &mut Ui,
    _app: &mut App,
    st: &mut UiState,
    lay: &Layout,
    origin: egui::Pos2,
    n: usize,
    _id: u64,
    from: usize,
    _to: usize,
) {
    let p = pal(ui.ctx());
    let Some(pos) = ui.ctx().pointer_interact_pos() else { return };
    let stride = lay.cell + lay.gap;
    let c = (((pos.x - origin.x) / stride.x).floor().max(0.0) as usize).min(lay.cols - 1);
    let r = ((pos.y - origin.y - lay.top) / stride.y).floor().max(0.0) as usize;
    let to = (r * lay.cols + c).min(n.saturating_sub(1));
    st.library.drag = Some((from, to));
    let target = lay.cell_rect(origin, to);
    let x = if to > from { target.max.x + lay.gap.x / 2.0 } else { target.min.x - lay.gap.x / 2.0 };
    ui.painter().vline(x, target.y_range(), Stroke::new(2.0, p.accent));
    let ghost = lay.cell_rect(origin, from);
    ui.painter().rect_stroke(ghost, 8, Stroke::new(1.0, p.divider_bright), egui::StrokeKind::Inside);
    ui.ctx().request_repaint();
}

/// Titled rows of tiles (Home, Network). Home rows show as many tiles as
/// fit; other pages wrap.
fn sections(ui: &mut Ui, app: &mut App, st: &mut UiState, m: &PageModel) {
    let p = pal(ui.ctx());
    let home = m.kind == PageKind::Home;
    let pad = bottom_pad(app);
    egui::ScrollArea::vertical().id_salt("lib-sections").auto_shrink(false).show(ui, |ui| {
        let width = ui.available_width();
        for (si, sec) in m.sections.iter().enumerate() {
            ui.add_space(17.0 + 20.0);
            if let Some(title) = &sec.title {
                let (r, resp) = ui.allocate_exact_size(vec2(width, 24.0), Sense::click());
                let galley = ui.painter().layout_no_wrap(title.clone(), font::regular(20.0), p.fg_a1);
                let tw = galley.size().x;
                let hot = sec.link.is_some() && resp.hovered();
                ui.painter().galley(
                    pos2(r.min.x, r.center().y - galley.size().y / 2.0),
                    galley,
                    if hot { p.fg1 } else { p.fg_a1 },
                );
                if let Some(link) = &sec.link {
                    widgets::paint_icon(
                        ui,
                        Rect::from_min_size(pos2(r.min.x + tw + 5.0, r.min.y), vec2(24.0, 24.0)),
                        Icon::ChevronRight,
                        if hot { p.fg1 } else { p.fg_a2 },
                    );
                    let hit = Rect::from_min_size(r.min, vec2(tw + 29.0, 24.0));
                    if resp.clicked() && ui.input(|i| i.pointer.interact_pos()).is_some_and(|q| hit.contains(q)) {
                        st.navigate(link.clone());
                    }
                }
                ui.add_space(20.0);
            }
            let subtitle = sec.tiles.iter().any(|t| t.subtitle.is_some());
            let (w, gap) = match sec.shape {
                Shape::Wide => (316.0f32, 20.0),
                Shape::Square => (200.0, 20.0),
            };
            let cols = (((width + gap) / (w + gap)).floor() as usize).max(1);
            let n = if home { sec.tiles.len().min(cols) } else { sec.tiles.len() };
            let rows = n.div_ceil(cols);
            let h = card_h(sec.shape, w, subtitle);
            let total = rows as f32 * h + rows.saturating_sub(1) as f32 * 40.0;
            let (rect, _) = ui.allocate_exact_size(vec2(width, total), Sense::hover());
            for ti in 0..n {
                let (r, c) = (ti / cols, ti % cols);
                let cell = Rect::from_min_size(
                    pos2(rect.min.x + c as f32 * (w + gap), rect.min.y + r as f32 * (h + 40.0)),
                    vec2(w, h),
                );
                if !ui.is_rect_visible(cell) {
                    continue;
                }
                section_tile(ui, app, st, m, sec, si, ti, cell);
            }
            ui.add_space(20.0);
        }
        ui.add_space(pad);
    });
}

#[allow(clippy::too_many_arguments)]
fn section_tile(
    ui: &mut Ui,
    app: &mut App,
    st: &mut UiState,
    m: &PageModel,
    sec: &SectionModel,
    si: usize,
    ti: usize,
    cell: Rect,
) {
    let t = &sec.tiles[ti];
    let resp = ui.interact(cell, ui.id().with(("lib-sec", si, ti)), Sense::click());
    let item = match t.kind {
        TileKind::Media(i) => sec.playable.get(i),
        _ => None,
    };
    if let Some(item) = item {
        want_meta(app, st, item);
    }
    let (_, badge) = cards::grid_card(ui, app, cell, sec.shape, t, item, resp.hovered(), false);
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let on_badge = badge.is_some_and(|b| ui.input(|i| i.pointer.interact_pos()).is_some_and(|q| b.contains(q)));
    if (resp.clicked() && (item.is_none() || on_badge)) || (resp.double_clicked() && item.is_some()) {
        activate(app, st, sec, ti);
    }
    tile_menu(&resp, app, st, m, sec, ti);
}

/// Play, shuffle, add files, export, rename and delete for a playlist.
fn playlist_bar(ui: &mut Ui, app: &mut App, st: &mut UiState, id: u64, m: &PageModel) {
    let p = pal(ui.ctx());
    ui.add_space(20.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        if let Some((rid, name)) = st.library.rename.as_mut().filter(|(rid, _)| *rid == id) {
            let rid = *rid;
            let resp = widgets::text_field(ui, name, "Playlist name", 320.0);
            let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if !resp.has_focus() && !resp.lost_focus() {
                resp.request_focus();
            }
            let save = widgets::button_with(ui, None, "Save", ButtonKind::Primary, vec2(80.0, 32.0)).clicked();
            let cancel = widgets::button_with(ui, None, "Cancel", ButtonKind::Ghost, vec2(80.0, 32.0)).clicked();
            if save || enter {
                let n = name.clone();
                app.rename_playlist(rid, &n);
                st.library.rename = None;
                st.library.invalidate();
            } else if cancel || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                st.library.rename = None;
            }
            return;
        }
        let items: Vec<MediaItem> = app.library.playlist(id).map(|p| p.items.clone()).unwrap_or_default();
        let name = app.library.playlist(id).map(|p| p.name.clone());
        let any = !items.is_empty();
        if widgets::button_with(ui, Some(Icon::Play), "Play All", ButtonKind::Primary, vec2(0.0, 32.0)).clicked() && any
        {
            play(app, st, items.clone(), 0, name.clone());
        }
        if widgets::button_with(ui, Some(Icon::ShuffleRight), "Shuffle", ButtonKind::Secondary, vec2(0.0, 32.0))
            .clicked()
            && any
        {
            app.set_shuffle(true);
            let start = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0) as usize)
                % items.len();
            play(app, st, items.clone(), start, name.clone());
        }
        if widgets::button_with(ui, Some(Icon::Add), "Add Files", ButtonKind::Secondary, vec2(0.0, 32.0)).clicked() {
            add_files(app, st, id);
        }
        if widgets::button_with(ui, Some(Icon::Edit), "Rename", ButtonKind::Ghost, vec2(0.0, 32.0)).clicked() {
            st.library.rename = Some((id, name.clone().unwrap_or_default()));
        }
        if widgets::button_with(ui, Some(Icon::Save), "Export", ButtonKind::Ghost, vec2(0.0, 32.0)).clicked() {
            menus_export(app, id);
        }
        if widgets::button_with(ui, Some(Icon::Remove), "Delete", ButtonKind::Danger, vec2(0.0, 32.0)).clicked() {
            st.library.confirm_delete = Some(id);
        }
        let total: i64 = m.main().map(|s| s.playable.iter().filter_map(|i| i.meta.duration_ms).sum()).unwrap_or(0);
        if total > 0 {
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(crate::domain::time::format_short(total)).font(font::regular(14.0)).color(p.fg_a3),
            );
        }
    });
    let _ = Align2::LEFT_TOP;
}

fn menus_export(app: &mut App, id: u64) {
    let Some(pl) = app.library.playlist(id).cloned() else { return };
    if let Some(path) = rfd::FileDialog::new()
        .set_title("Export Playlist")
        .set_file_name(format!("{}.xspf", pl.name))
        .add_filter("XSPF", &["xspf"])
        .add_filter("M3U", &["m3u", "m3u8"])
        .add_filter("PLS", &["pls"])
        .save_file()
    {
        app.export_items(&pl.name, &pl.items, &path);
    }
}

fn add_files(app: &mut App, st: &mut UiState, id: u64) {
    let mut exts: Vec<&str> = crate::domain::media::VIDEO_EXTENSIONS.to_vec();
    exts.extend(crate::domain::media::AUDIO_EXTENSIONS);
    if let Some(files) = rfd::FileDialog::new().set_title("Add to Playlist").add_filter("Media", &exts).pick_files() {
        let items = app.expand_paths(&files);
        app.add_to_playlist(id, items);
        st.library.invalidate();
    }
    let _ = IMAGE_EXTENSIONS;
}

/// Arrow keys move the selection, Enter opens or plays it, Backspace goes back.
pub fn keyboard(ui: &mut Ui, app: &mut App, st: &mut UiState, m: &PageModel) {
    let ctx = ui.ctx().clone();
    if ctx.wants_keyboard_input()
        || egui::Popup::is_any_open(&ctx)
        || st.dialogs.new_playlist
        || st.library.confirm_delete.is_some()
        || st.settings.open
    {
        return;
    }
    let (left, right, up, down, enter, back) = ctx.input(|i| {
        (
            i.key_pressed(egui::Key::ArrowLeft),
            i.key_pressed(egui::Key::ArrowRight),
            i.key_pressed(egui::Key::ArrowUp),
            i.key_pressed(egui::Key::ArrowDown),
            i.key_pressed(egui::Key::Enter),
            i.key_pressed(egui::Key::Backspace),
        )
    });
    if back {
        let up = m.crumbs.iter().rev().skip(1).find_map(|c| c.1.clone()).filter(|p| *p != st.page);
        super::header::go_back_or_up(st, up);
        return;
    }
    let Some(sec) = m.main().filter(|s| s.title.is_none()) else { return };
    let n = sec.tiles.len();
    if n == 0 {
        return;
    }
    let cols = st.library.cols.max(1) as isize;
    let delta: isize = if left {
        -1
    } else if right {
        1
    } else if up {
        -cols
    } else if down {
        cols
    } else {
        0
    };
    if delta != 0 {
        let cur = st.library.selected.map(|s| s as isize).unwrap_or(if delta > 0 { -delta.min(1) } else { n as isize });
        let next = (cur + delta).clamp(0, n as isize - 1) as usize;
        st.library.selected = Some(next);
        st.library.scroll_to_selected = true;
    }
    if enter && let Some(sel) = st.library.selected.filter(|s| *s < n) {
        activate(app, st, sec, sel);
    }
}
