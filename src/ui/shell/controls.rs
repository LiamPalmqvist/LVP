//! The player control bar, laid out from the user's toolbar layout.

use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{Align2, Color32, CornerRadius, Pos2, Rect, Sense, Shape, Stroke, Ui, pos2, vec2};

use crate::app::ports::PlayerState;
use crate::app::{App, UiCommand};
use crate::domain::settings::RepeatMode;
use crate::domain::time::format_hms;
use crate::domain::toolbar::ToolbarItem;
use crate::ui::icons::Icon;
use crate::ui::state::{PlayerPanel, UiState, View};
use crate::ui::theme::{Palette, font, mix};
use crate::ui::widgets::{self, pal};

pub const ROW_HEIGHT: f32 = 54.0;
const PAD_X: f32 = 20.0;

pub fn height(app: &App) -> f32 {
    let rows = app.settings.interface.toolbar.rows.len().max(1) as f32;
    if rows > 1.0 { 40.0 + (rows - 1.0) * 50.0 } else { ROW_HEIGHT }
}

fn fixed_width(item: ToolbarItem, app: &App, ui: &Ui) -> Option<f32> {
    use ToolbarItem::*;
    Some(match item {
        SeekBar | Spacer => return None,
        PlayPause => 54.0,
        Previous | Next => 30.0,
        CurrentTime | Duration => 76.0,
        Volume => 120.0,
        Divider => 1.0,
        PlaybackSpeed => 48.0,
        Thumbnail => 50.0,
        FileName | FolderName | ArtistName | PlaylistName => {
            let t = item_text(item, app);
            if t.is_empty() {
                0.0
            } else {
                let w = ui.painter().layout_no_wrap(t, font::regular(14.0), Color32::WHITE).size().x;
                w.min(260.0) + if item == FolderName { 32.0 } else { 0.0 }
            }
        }
        _ => 30.0,
    })
}

fn item_text(item: ToolbarItem, app: &App) -> String {
    let cur = app.queue.current_item();
    match item {
        ToolbarItem::FileName => cur.map(|i| i.title()).unwrap_or_default(),
        ToolbarItem::ArtistName => {
            app.status.meta.artist.clone().or_else(|| cur.and_then(|i| i.meta.artist.clone())).unwrap_or_default()
        }
        ToolbarItem::FolderName => cur
            .and_then(|i| {
                i.source
                    .path()
                    .and_then(|p| p.parent())
                    .and_then(|d| d.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
            })
            .unwrap_or_default(),
        ToolbarItem::PlaylistName => app.queue.origin.clone().unwrap_or_default(),
        _ => String::new(),
    }
}

/// Draws the bar in `rect`. Returns the rect of the right-hand buttons so
/// popovers can anchor above them.
pub fn show(ui: &mut Ui, rect: Rect, app: &mut App, st: &mut UiState, overlay: bool) {
    let p = pal(ui.ctx());
    let alpha = if st.fullscreen { 0.93 * app.settings.interface.fs_controls_opacity.clamp(0.1, 1.0) } else { 0.93 };
    let fill = if overlay { crate::ui::theme::with_alpha(p.bg_a4, alpha) } else { p.bg_a4 };
    ui.painter().rect_filled(rect, 0, fill);
    // Swallow clicks so they don't reach the video below.
    ui.interact(rect, ui.id().with("controls-bg"), Sense::click());
    let rows = app.settings.interface.toolbar.rows.clone();
    let n = rows.len().max(1);
    let mut y = rect.min.y;
    for (ri, row) in rows.iter().enumerate() {
        let h = if n == 1 {
            ROW_HEIGHT
        } else if ri == 0 {
            40.0
        } else {
            50.0
        };
        let row_rect = Rect::from_min_size(pos2(rect.min.x, y), vec2(rect.width(), h));
        show_row(ui, row_rect, row, app, st, &p);
        y += h;
    }
}

fn show_row(ui: &mut Ui, rect: Rect, row: &[ToolbarItem], app: &mut App, st: &mut UiState, p: &Palette) {
    let gap = 10.0;
    let inner = rect.shrink2(vec2(PAD_X, 0.0));
    let fixed: Vec<Option<f32>> = row.iter().map(|i| fixed_width(*i, app, ui)).collect();
    let used: f32 = fixed.iter().flatten().sum::<f32>() + gap * row.len().saturating_sub(1) as f32;
    let seek_n = row.iter().filter(|i| **i == ToolbarItem::SeekBar).count() as f32;
    let spacer_n = row.iter().filter(|i| **i == ToolbarItem::Spacer).count() as f32;
    let free = (inner.width() - used).max(0.0);
    let (seek_w, spacer_w) = if seek_n > 0.0 { (free / seek_n, 0.0) } else { (0.0, free / spacer_n.max(1.0)) };
    let mut x = inner.min.x;
    let cy = rect.center().y;
    for (item, fw) in row.iter().zip(fixed) {
        let w = match item {
            ToolbarItem::SeekBar => seek_w,
            ToolbarItem::Spacer => spacer_w,
            _ => fw.unwrap_or(0.0),
        };
        if w <= 0.0 && *item != ToolbarItem::Spacer {
            continue;
        }
        let r = Rect::from_min_size(pos2(x, cy - 16.0), vec2(w, 32.0));
        draw_item(ui, *item, r, app, st, p);
        x += w + gap;
    }
}

fn button(ui: &mut Ui, r: Rect, id: &str, icon: Icon, active: bool, tip: &str) -> bool {
    let p = pal(ui.ctx());
    let resp = ui.interact(r, ui.id().with(id), Sense::click()).on_hover_text(tip);
    if resp.hovered() || active {
        ui.painter().rect_filled(r, 4, if active { crate::ui::theme::with_alpha(p.accent, 0.12) } else { p.hover });
    }
    let fg = if active {
        p.accent
    } else if resp.hovered() {
        p.fg2
    } else {
        p.fg_a2
    };
    widgets::paint_icon(ui, Rect::from_center_size(r.center(), vec2(24.0, 24.0)), icon, fg);
    resp.clicked()
}

fn draw_item(ui: &mut Ui, item: ToolbarItem, r: Rect, app: &mut App, st: &mut UiState, p: &Palette) {
    use ToolbarItem::*;
    let skip = |s: u32| s as i64 * 1000;
    match item {
        Previous => {
            if button(ui, r, "prev", Icon::Previous, false, "Previous") {
                app.previous();
            }
        }
        Next => {
            if button(ui, r, "next", Icon::Next, false, "Next") {
                app.next();
            }
        }
        PlayPause => {
            let pr = Rect::from_center_size(r.center(), vec2(54.0, 28.0));
            let resp = ui.interact(pr, ui.id().with("play"), Sense::click());
            let playing = app.is_playing();
            let bg = if resp.hovered() { p.accent_bright } else { p.accent };
            ui.painter().rect_filled(pr, 6, bg);
            let icon = if playing { Icon::Pause } else { Icon::Play };
            let fg = if p.dark { p.fg_b1 } else { Color32::WHITE };
            widgets::paint_icon(ui, Rect::from_center_size(pr.center(), vec2(24.0, 24.0)), icon, fg);
            if resp.on_hover_text(if playing { "Pause" } else { "Play" }).clicked() {
                app.toggle_pause();
            }
        }
        Stop => {
            if button(ui, r, "stop", Icon::Stop, false, "Stop") {
                app.stop();
            }
        }
        CurrentTime | Duration => {
            let ms = if item == CurrentTime {
                st.seek_hover.map(|h| h.1).unwrap_or(app.status.time_ms)
            } else {
                app.status.length_ms
            };
            let mut text =
                if item == Duration && app.status.length_ms <= 0 { "--:--:--".to_string() } else { format_hms(ms) };
            if item == CurrentTime && (app.ab_loop.a.is_some()) {
                text.push('*');
            }
            let resp = ui.interact(r, ui.id().with(("time", item == CurrentTime)), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(r, 6, p.hover);
            }
            ui.painter().text(r.center(), Align2::CENTER_CENTER, text, font::regular(15.0), p.fg_a2);
            if resp.clicked() {
                st.command(UiCommand::JumpToTime);
            }
        }
        SeekBar => seek_bar(ui, r, app, st, p),
        Volume => volume(ui, r, app, p),
        AudioTracks => {
            let on = st.panel == Some(PlayerPanel::Tracks(1));
            if button(ui, r, "audio", Icon::Audio, on, "Audio") {
                st.toggle_panel(PlayerPanel::Tracks(1));
            }
        }
        SubtitleTracks => {
            let on = st.panel == Some(PlayerPanel::Tracks(2));
            if button(ui, r, "subs", Icon::Subtitles, on, "Subtitles") {
                st.toggle_panel(PlayerPanel::Tracks(2));
            }
        }
        QuickSettings => {
            let on = matches!(st.panel, Some(PlayerPanel::QuickSettings(_)));
            if button(ui, r, "quick", Icon::PlaybackSettings, on, "Settings") {
                let tab = match st.panel {
                    Some(PlayerPanel::QuickSettings(t)) => t,
                    _ => 0,
                };
                st.toggle_panel(PlayerPanel::QuickSettings(tab));
            }
        }
        Queue => {
            let on = matches!(st.panel, Some(PlayerPanel::Queue(_)));
            if button(ui, r, "queue", Icon::Lists, on, "Queue") {
                st.toggle_panel(PlayerPanel::Queue(0));
            }
        }
        Fullscreen => {
            let icon = if st.fullscreen { Icon::FullscreenActive } else { Icon::Fullscreen };
            if button(ui, r, "fs", icon, false, "Fullscreen") {
                st.command(UiCommand::ToggleFullscreen);
            }
        }
        Spacer => {}
        Divider => {
            let lr = Rect::from_center_size(r.center(), vec2(1.0, 20.0));
            ui.painter().rect_filled(lr, 0, p.divider_bright);
        }
        PlaybackSpeed => {
            let resp = ui.interact(r, ui.id().with("speed"), Sense::click()).on_hover_text("Playback speed");
            let rate = if app.status.rate > 0.0 { app.status.rate } else { 1.0 };
            let text = if (rate - rate.round()).abs() < 0.01 { format!("{rate:.0}x") } else { format!("{rate:.2}x") };
            ui.painter().rect_filled(r.shrink2(vec2(0.0, 2.0)), 6, if resp.hovered() { p.bg_a1 } else { p.bg_b2 });
            ui.painter().text(
                r.center(),
                Align2::CENTER_CENTER,
                text,
                font::regular(14.0),
                if (rate - 1.0).abs() > 0.01 { p.accent } else { p.fg_a2 },
            );
            if resp.clicked() {
                st.panel = Some(PlayerPanel::QuickSettings(0));
            }
            if resp.secondary_clicked() {
                app.set_rate(1.0);
            }
        }
        Forward10 => {
            if button(ui, r, "f10", Icon::Forward10Sec, false, "Forward 10s") {
                app.jump(10_000);
            }
        }
        Backward10 => {
            if button(ui, r, "b10", Icon::Backward10Sec, false, "Backward 10s") {
                app.jump(-10_000);
            }
        }
        Forward15 => {
            if button(ui, r, "f15", Icon::Forward15Sec, false, "Forward 15s") {
                app.jump(15_000);
            }
        }
        Backward15 => {
            if button(ui, r, "b15", Icon::Backward15Sec, false, "Backward 15s") {
                app.jump(-15_000);
            }
        }
        Repeat => {
            let on = app.queue.repeat != RepeatMode::Off;
            let tip = app.queue.repeat.label();
            if button(ui, r, "repeat", if on { Icon::RepeatRight2tone } else { Icon::RepeatRight }, on, tip) {
                let next = app.queue.repeat.cycle();
                app.set_repeat(next);
            }
            if app.queue.repeat == RepeatMode::One {
                ui.painter().text(
                    r.right_top() + vec2(-6.0, 6.0),
                    Align2::CENTER_CENTER,
                    "1",
                    font::bold(9.0),
                    p.accent,
                );
            }
        }
        Shuffle => {
            let on = app.queue.shuffle;
            if button(ui, r, "shuffle", if on { Icon::ShuffleRight2tone } else { Icon::ShuffleRight }, on, "Shuffle") {
                app.set_shuffle(!on);
            }
        }
        Record => {
            let on = app.status.recording;
            if button(ui, r, "record", if on { Icon::RecordActive } else { Icon::Record }, on, "Record") {
                app.toggle_record();
            }
        }
        Snapshot => {
            if button(ui, r, "snap", Icon::Snapshot, false, "Take a snapshot") {
                app.snapshot();
            }
        }
        PictureInPicture => {
            if button(ui, r, "pip", Icon::Pip, st.pip, "Picture in picture") {
                st.command(UiCommand::PictureInPicture);
            }
        }
        Hdr => {
            if button(ui, r, "hdr", Icon::Hdr, false, "Video settings") {
                st.panel = Some(PlayerPanel::QuickSettings(1));
            }
        }
        Zoom => {
            let on = app.geometry.zoom != 0.0;
            if button(ui, r, "zoom", Icon::Zoom, on, "Zoom") {
                app.zoom_step(!on || app.geometry.zoom < 2.0);
            }
        }
        RightSidebar => {
            let on = matches!(st.panel, Some(PlayerPanel::Queue(_)));
            if button(ui, r, "rside", Icon::SidebarRight, on, "Queue sidebar") {
                st.toggle_panel(PlayerPanel::Queue(0));
            }
        }
        LeftSidebar => {
            let icon = if st.side_panel { Icon::SidebarActive } else { Icon::SidebarLeft };
            if button(ui, r, "lside", icon, false, "Library") {
                st.side_panel = !st.side_panel;
            }
        }
        More => {
            if button(ui, r, "more", Icon::More, false, "More") {
                st.panels.context_menu_at = Some(r.left_top());
            }
        }
        QuitVlc => {
            if button(ui, r, "quit", Icon::Quit, false, "Quit") {
                st.command(UiCommand::Quit);
            }
        }
        VlcSettings => {
            if button(ui, r, "prefs", Icon::Preferences, false, "Preferences") {
                st.command(UiCommand::Preferences);
            }
        }
        GoHome => {
            if button(ui, r, "home", Icon::Home, false, "Library") {
                st.navigate(crate::ui::pages::Page::Home);
            }
        }
        Thumbnail => {
            let resp = ui.interact(r, ui.id().with("thumb"), Sense::click()).on_hover_text("Back to the player");
            let tr = r.shrink2(vec2(0.0, 3.0));
            let drawn = app
                .queue
                .current_item()
                .cloned()
                .and_then(|item| app.thumbnail(&item))
                .is_some_and(|path| crate::ui::images::paint_cover(ui, tr, &path, 4));
            if !drawn {
                ui.painter().rect_filled(tr, 4, p.bg_b2);
            }
            if resp.clicked() {
                st.view = View::Player;
            }
        }
        FileName | ArtistName | PlaylistName => {
            let t = item_text(item, app);
            widgets::text_clipped(
                ui,
                pos2(r.min.x, r.center().y),
                Align2::LEFT_CENTER,
                &t,
                font::regular(14.0),
                p.fg_a2,
                r.width(),
            );
        }
        FolderName => {
            let t = item_text(item, app);
            let resp =
                ui.interact(r, ui.id().with("folder"), Sense::click()).on_hover_text("Open the folder in the library");
            widgets::paint_icon(
                ui,
                Rect::from_min_size(pos2(r.min.x, r.center().y - 12.0), vec2(24.0, 24.0)),
                Icon::Folder,
                p.fg_a2,
            );
            widgets::text_clipped(
                ui,
                pos2(r.min.x + 32.0, r.center().y),
                Align2::LEFT_CENTER,
                &t,
                font::regular(14.0),
                p.fg_a2,
                r.width() - 32.0,
            );
            if resp.clicked()
                && let Some(dir) = app
                    .queue
                    .current_item()
                    .and_then(|i| i.source.path())
                    .and_then(|p| p.parent())
                    .map(|p| p.to_path_buf())
            {
                st.navigate(crate::ui::pages::Page::Folder(dir));
            }
        }
        Properties => {
            if button(ui, r, "props", Icon::Info, false, "Media information") {
                st.command(UiCommand::MediaInfo);
            }
        }
        Replay => {
            if button(ui, r, "replay", Icon::Replay, false, "Replay") {
                app.seek_ms(0);
                app.play();
            }
        }
        Chapters => {
            let on = st.panel == Some(PlayerPanel::Queue(2));
            if button(ui, r, "chapters", Icon::Chapters, on, "Chapters") {
                st.toggle_panel(PlayerPanel::Queue(2));
            }
        }
        Favorite => {
            let fav = app.current_source().map(|s| app.library.record(s).favorite).unwrap_or(false);
            if button(ui, r, "fav", if fav { Icon::FavoriteActive } else { Icon::Favorite }, fav, "Favorite") {
                app.toggle_favorite_current();
            }
        }
        Bookmark => {
            if button(ui, r, "bm", Icon::Bookmark, false, "Add bookmark") {
                app.add_bookmark(None);
            }
        }
        Bookmarks => {
            let on = st.panel == Some(PlayerPanel::Queue(1));
            if button(ui, r, "bms", Icon::Bookmarks, on, "Bookmarks") {
                st.toggle_panel(PlayerPanel::Queue(1));
            }
        }
        Playlists => {
            if button(ui, r, "pls", Icon::Playlist, false, "Playlists") {
                st.navigate(crate::ui::pages::Page::of(crate::ui::pages::Section::Playlists));
            }
        }
        FrameStep => {
            if button(ui, r, "frame", Icon::Next2tone, false, "Next frame") {
                app.frame_step();
            }
        }
        AbLoop => {
            let on = app.ab_loop.a.is_some();
            if button(ui, r, "ab", Icon::Refresh, on, "Loop between A and B") {
                app.cycle_ab_loop();
            }
        }
    }
    let _ = skip;
}

/// The seek bar: dark track, tinted progress, accent thumb, chapter ticks,
/// A-B markers and a time bubble on hover.
fn seek_bar(ui: &mut Ui, r: Rect, app: &mut App, st: &mut UiState, p: &Palette) {
    if crate::ui::music::draw_waveform_seek(ui, r, app, st) {
        return;
    }
    let bar = Rect::from_center_size(r.center(), vec2(r.width(), 28.0));
    let resp = ui.interact(bar, ui.id().with("seek"), Sense::click_and_drag());
    let len = app.status.length_ms;
    let seekable = app.status.seekable || len > 0;
    let painter = ui.painter();
    painter.rect_filled(bar, 6, p.bg_b2);
    let inner = bar.shrink(2.0);
    let frac_at = |x: f32| ((x - inner.min.x) / inner.width()).clamp(0.0, 1.0);
    let mut frac = if len > 0 {
        (app.status.time_ms as f32 / len as f32).clamp(0.0, 1.0)
    } else {
        app.status.position.clamp(0.0, 1.0)
    };
    st.seek_hover = None;
    if seekable && let Some(pos) = resp.hover_pos().or(resp.interact_pointer_pos()) {
        let f = frac_at(pos.x);
        let t = (f as f64 * len as f64) as i64;
        if resp.dragged() || resp.is_pointer_button_down_on() {
            frac = f;
            st.seek_hover = Some((f, t));
        } else if resp.hovered() {
            // Time bubble above the bar.
            let text = format_hms(t);
            let galley = painter.layout_no_wrap(text, font::regular(13.0), p.fg1);
            let bw = galley.size().x + 16.0;
            let bx = pos.x.clamp(bar.min.x + bw / 2.0, bar.max.x - bw / 2.0);
            let bubble = Rect::from_center_size(pos2(bx, bar.min.y - 20.0), vec2(bw, 24.0));
            let tooltip_layer = egui::LayerId::new(egui::Order::Tooltip, ui.id().with("seek-bubble"));
            let lp = ui.ctx().layer_painter(tooltip_layer);
            lp.rect_filled(bubble, 6, p.bg_a1);
            lp.galley(bubble.center() - galley.size() / 2.0, galley, p.fg1);
            if let Some(ch) = chapter_at(app, t) {
                let cg = lp.layout_no_wrap(ch, font::regular(12.0), p.fg_a2);
                let cr = Rect::from_center_size(pos2(bx, bubble.min.y - 14.0), cg.size() + vec2(12.0, 6.0));
                lp.rect_filled(cr, 4, p.bg_a1);
                lp.galley(cr.center() - cg.size() / 2.0, cg, p.fg_a2);
            }
        }
        if resp.drag_stopped() || resp.clicked() {
            app.seek_fraction(f);
        }
    }
    if resp.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta);
        if scroll.y.abs() > 0.5 && seekable {
            let s = app.settings.controls.skip_short_s as i64 * 1000;
            app.jump(if scroll.y > 0.0 { s } else { -s });
        }
    }
    let painter = ui.painter();
    // A-B loop region.
    if let (Some(a), Some(b)) = (app.ab_loop.a, app.ab_loop.b)
        && len > 0
    {
        let xa = inner.min.x + inner.width() * a as f32 / len as f32;
        let xb = inner.min.x + inner.width() * b as f32 / len as f32;
        painter.rect_filled(
            Rect::from_min_max(pos2(xa, inner.min.y), pos2(xb, inner.max.y)),
            4,
            mix(p.bg_b2, p.accent, 0.12),
        );
    }
    let x = inner.min.x + inner.width() * frac;
    if x > inner.min.x + 1.0 {
        painter.rect_filled(Rect::from_min_max(inner.min, pos2(x, inner.max.y)), CornerRadius::same(4), p.bg_accent);
    }
    // Chapter ticks.
    if len > 0 {
        for ch in app.status.chapters.iter().skip(1) {
            let cx = inner.min.x + inner.width() * (ch.start_ms as f32 / len as f32);
            painter.line_segment([pos2(cx, inner.min.y + 4.0), pos2(cx, inner.max.y - 4.0)], Stroke::new(2.0, p.fg_a4));
        }
    }
    for (m, label) in [(app.ab_loop.a, "A"), (app.ab_loop.b, "B")] {
        if let Some(t) = m
            && len > 0
        {
            let mx = inner.min.x + inner.width() * t as f32 / len as f32;
            let mr = Rect::from_center_size(pos2(mx, inner.center().y), vec2(14.0, 20.0));
            painter.rect_filled(mr, 3, p.accent);
            painter.text(mr.center(), Align2::CENTER_CENTER, label, font::semibold(11.0), p.fg_b1);
        }
    }
    if seekable && (len > 0 || app.status.state == PlayerState::Playing) {
        let thumb = Rect::from_center_size(pos2(x.max(inner.min.x + 4.0), inner.center().y), vec2(8.0, 24.0));
        painter.rect_filled(thumb, 4, if resp.hovered() || resp.dragged() { p.accent_bright } else { p.accent });
    }
    if app.status.state == PlayerState::Buffering && app.status.buffering < 100.0 {
        let bw = inner.width() * app.status.buffering / 100.0;
        painter.rect_filled(Rect::from_min_size(pos2(inner.min.x, inner.max.y - 2.0), vec2(bw, 2.0)), 1, p.fg_a3);
    }
}

fn chapter_at(app: &App, t: i64) -> Option<String> {
    app.status.chapters.iter().rev().find(|c| c.start_ms <= t).map(|c| c.name.clone())
}

/// Speaker icon plus the wedge-shaped level meter.
fn volume(ui: &mut Ui, r: Rect, app: &mut App, p: &Palette) {
    let icon_r = Rect::from_min_size(pos2(r.min.x, r.center().y - 12.0), vec2(24.0, 24.0));
    let level = app.status.volume;
    let muted = app.status.muted;
    let max = app.settings.audio.max_volume.clamp(100, 200);
    let ir =
        ui.interact(icon_r, ui.id().with("mute"), Sense::click()).on_hover_text(if muted { "Unmute" } else { "Mute" });
    widgets::paint_icon(ui, icon_r, Icon::volume(level, muted), if ir.hovered() { p.fg2 } else { p.fg_a2 });
    if ir.clicked() {
        app.toggle_mute();
    }
    let w = Rect::from_min_size(pos2(r.min.x + 32.0, r.center().y - 14.0), vec2(88.0, 28.0));
    let resp = ui.interact(w, ui.id().with("vol"), Sense::click_and_drag()).on_hover_text(format!("Volume {level}%"));
    if (resp.dragged() || resp.clicked())
        && let Some(pos) = resp.interact_pointer_pos()
    {
        let f = ((pos.x - w.min.x) / w.width()).clamp(0.0, 1.0);
        app.set_volume((f * max as f32).round() as i32);
    }
    if resp.hovered() {
        let s = ui.input(|i| i.smooth_scroll_delta.y);
        if s.abs() > 0.5 {
            app.change_volume(if s > 0.0 { 5 } else { -5 });
        }
    }
    let frac = if muted { 0.0 } else { (level as f32 / max as f32).clamp(0.0, 1.0) };
    paint_wedge(ui, w, frac, p);
}

/// The volume wedge: thin at the left, full height at the right.
fn paint_wedge(ui: &Ui, w: Rect, frac: f32, p: &Palette) {
    let edge = |t: f32, inset: f32| -> (f32, f32) {
        // Top and bottom of the wedge at fraction t (from the design path).
        let top = egui::lerp(8.6..=0.3, t) + inset;
        let bottom = egui::lerp(19.4..=27.7, t) - inset;
        (w.min.y + top, w.min.y + bottom)
    };
    let mut bg: Vec<Pos2> = Vec::new();
    for i in 0..=8 {
        let t = i as f32 / 8.0;
        bg.push(pos2(w.min.x + w.width() * t, edge(t, 0.0).0));
    }
    for i in (0..=8).rev() {
        let t = i as f32 / 8.0;
        bg.push(pos2(w.min.x + w.width() * t, edge(t, 0.0).1));
    }
    ui.painter().add(Shape::convex_polygon(bg, p.bg_b2, Stroke::NONE));
    if frac <= 0.0 {
        return;
    }
    let stops = p.volume_stops();
    let color_at = |t: f32| -> Color32 {
        for win in stops.windows(2) {
            let (a, b) = (win[0], win[1]);
            if t <= b.0 {
                return mix(a.1, b.1, ((t - a.0) / (b.0 - a.0)).clamp(0.0, 1.0));
            }
        }
        stops[3].1
    };
    let mut mesh = Mesh::default();
    let steps = 24;
    let x0 = w.min.x + 1.5;
    let span = w.width() - 3.0;
    for i in 0..=steps {
        let t = frac * i as f32 / steps as f32;
        let (top, bottom) = edge(t, 1.5);
        let c = color_at(t);
        let x = x0 + span * t;
        mesh.vertices.push(Vertex { pos: pos2(x, top), uv: WHITE_UV, color: c });
        mesh.vertices.push(Vertex { pos: pos2(x, bottom), uv: WHITE_UV, color: c });
        if i > 0 {
            let b = (i as u32 - 1) * 2;
            mesh.indices.extend_from_slice(&[b, b + 1, b + 2, b + 1, b + 3, b + 2]);
        }
    }
    ui.painter().add(Shape::mesh(mesh));
}
