//! The floating mini player in the library's bottom-right corner (component
//! "Miniplayer"): preview with hover controls, a thin progress bar, the
//! title, folder and playlist chips, and the queue it can open above itself.

use egui::epaint::Shadow;
use egui::{Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, TextureId, Ui, UiBuilder, pos2, vec2};

use crate::app::ports::PlayerState;
use crate::app::{App, VideoGeometry};
use crate::domain::media::MediaItem;
use crate::domain::time::format_short;
use crate::ui::icons::Icon;
use crate::ui::pages::Page;
use crate::ui::state::{UiState, View};
use crate::ui::theme::{Palette, font, with_alpha};
use crate::ui::widgets::{self, pal};

use super::art;

/// Card size from the library frame (391x339), 6pt from the corner.
pub const WIDTH: f32 = 391.0;
pub const HEIGHT: f32 = 339.0;
const MARGIN: f32 = 6.0;
const PAD: f32 = 8.0;
const GAP: f32 = 20.0;
const PREVIEW_H: f32 = 211.0;
/// The queue list stacked above the preview when opened.
const QUEUE_H: f32 = 331.0;
const HEADER_H: f32 = 58.0;
const ROW_H: f32 = 32.0;
const ROW_GAP: f32 = 12.0;

/// The floating player in the library's bottom-right corner.
pub fn miniplayer(ui: &mut Ui, content: Rect, app: &mut App, st: &mut UiState, frame: Option<(TextureId, [u32; 2])>) {
    let Some(item) = app.queue.current_item().cloned() else { return };
    if st.music.mini_closed.as_ref() == Some(&item.source) {
        if !app.is_playing() {
            return;
        }
        st.music.mini_closed = None;
    }
    let p = pal(ui.ctx());
    // The queue only opens when the window is tall enough for both.
    let queue_open = st.music.mini_queue && content.height() >= HEIGHT + QUEUE_H + GAP + 2.0 * MARGIN;
    let h = HEIGHT + if queue_open { QUEUE_H + GAP } else { 0.0 };
    let card = Rect::from_min_size(pos2(content.max.x - MARGIN - WIDTH, content.max.y - MARGIN - h), vec2(WIDTH, h));
    let painter = ui.painter().clone();
    let shadow = Shadow { offset: [0, 0], blur: 12, spread: 0, color: Color32::from_black_alpha(97) };
    painter.add(shadow.as_shape(card, CornerRadius::same(14)));
    painter.rect_filled(card, 14, p.bg_a3);
    // Swallow clicks and hovers so the page below does not react.
    ui.interact(card, ui.id().with("mini-card"), Sense::click());
    let hovered = ui.rect_contains_pointer(card) || st.music.mini_drag.is_some();
    let ended = matches!(app.status.state, PlayerState::Ended | PlayerState::Stopped | PlayerState::Error);

    let mut y = card.min.y + PAD;
    if queue_open {
        queue_panel(ui, card, app, st, &p, hovered);
        y += QUEUE_H + GAP;
    }
    let preview = Rect::from_min_size(pos2(card.min.x + PAD, y), vec2(WIDTH - 2.0 * PAD, PREVIEW_H));
    preview_area(ui, preview, app, st, frame, &p, &item, hovered, ended);
    let bar = Rect::from_min_size(pos2(preview.min.x, preview.max.y + 4.0), vec2(preview.width(), 16.0));
    progress(ui, bar, app, &p, hovered || ended);
    info(ui, card, bar.max.y + GAP, app, st, &p, &item);
}

/// An icon button positioned absolutely; returns true when clicked.
fn button(ui: &Ui, r: Rect, id: &str, icon: Icon, tip: &str, active: bool, p: &Palette) -> bool {
    let resp = ui.interact(r, ui.id().with(id), Sense::click()).on_hover_text(tip);
    if resp.hovered() || active {
        ui.painter().rect_filled(r, 4, if active { with_alpha(p.accent, 0.12) } else { p.hover });
    }
    let fg = if active {
        p.accent
    } else if resp.hovered() {
        p.fg2
    } else {
        p.fg_a2
    };
    if icon == Icon::Add {
        // The design's plus is drawn in black strokes, which the icon renderer
        // treats as cut-outs; draw it directly.
        let c = r.center();
        let stroke = Stroke::new(1.5, fg);
        ui.painter().line_segment([c - vec2(7.0, 0.0), c + vec2(7.0, 0.0)], stroke);
        ui.painter().line_segment([c - vec2(0.0, 7.0), c + vec2(0.0, 7.0)], stroke);
    } else {
        widgets::paint_icon(ui, Rect::from_center_size(r.center(), vec2(24.0, 24.0)), icon, fg);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

#[allow(clippy::too_many_arguments)]
fn preview_area(
    ui: &Ui,
    preview: Rect,
    app: &mut App,
    st: &mut UiState,
    frame: Option<(TextureId, [u32; 2])>,
    p: &Palette,
    item: &MediaItem,
    hovered: bool,
    ended: bool,
) {
    let resp = ui.interact(preview, ui.id().with("mini-preview"), Sense::click()).on_hover_text("Back to the player");
    let painter = ui.painter();
    if ended {
        painter.rect_filled(preview, 6, p.bg_a2);
    } else {
        painter.rect_filled(preview, 6, p.stage);
        if let Some((tex, [w, h])) = frame {
            let geo = VideoGeometry {
                aspect: app.geometry.aspect.clone(),
                crop: app.geometry.crop.clone(),
                ..Default::default()
            };
            let (dest, uv) =
                crate::ui::video::fit(w, h, app.status.video_sar, preview, &geo, &app.settings.video.effects.crop);
            art::paint_texture(ui.painter(), dest.intersect(preview), tex, uv, CornerRadius::same(6), Color32::WHITE);
        } else {
            let cover = app.thumbnail(item);
            if !art::paint_art(ui, preview, cover.as_deref(), 6) {
                art::placeholder(ui, preview, 6, p);
            }
        }
        if hovered {
            painter.rect_filled(preview, 6, Color32::from_black_alpha(153));
        }
    }
    let edge = if p.dark { Color32::BLACK } else { p.bg_a4 };
    painter.rect_stroke(preview, 6, Stroke::new(2.0, edge), StrokeKind::Inside);

    if hovered || ended {
        let corner = vec2(24.0, 24.0);
        let close = Rect::from_min_size(preview.min + vec2(16.0, 16.0), corner);
        if button(ui, close.expand(3.0), "mini-close", Icon::Close, "Close the mini player", false, p) {
            app.stop();
            st.music.mini_closed = Some(item.source.clone());
        }
        let open = Rect::from_min_size(pos2(preview.max.x - 16.0 - 24.0, preview.min.y + 16.0), corner);
        if button(ui, open.expand(3.0), "mini-open", Icon::Open, "Open the player", false, p) {
            st.view = View::Player;
        }
        let c = preview.center();
        for (dx, id, icon, tip) in
            [(-75.0, "mini-prev", Icon::Previous, "Previous"), (75.0, "mini-next", Icon::Next, "Next")]
        {
            let r = Rect::from_center_size(c + vec2(dx, 0.0), vec2(40.0, 40.0));
            let resp = ui.interact(r, ui.id().with(id), Sense::click()).on_hover_text(tip);
            let bg = if resp.hovered() { with_alpha(Color32::WHITE, 0.08) } else { Color32::from_black_alpha(64) };
            ui.painter().rect_filled(r, 7, bg);
            let fg = if resp.hovered() { p.fg2 } else { p.fg_a2 };
            widgets::paint_icon(ui, Rect::from_center_size(r.center(), corner), icon, fg);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                if dx < 0.0 {
                    app.previous();
                } else {
                    app.next();
                }
            }
        }
        let mid = Rect::from_center_size(c, vec2(43.0, 43.0));
        let (icon, tip) = if ended {
            (Icon::Replay, "Play again")
        } else if app.is_playing() {
            (Icon::Pause, "Pause")
        } else {
            (Icon::Play, "Play")
        };
        let resp = ui.interact(mid, ui.id().with("mini-play"), Sense::click()).on_hover_text(tip);
        let fg = if resp.hovered() { p.fg1 } else { p.fg_a2 };
        widgets::paint_icon(ui, Rect::from_center_size(c, corner), icon, fg);
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            if ended {
                if let Some(i) = app.queue.current {
                    app.play_index_from(i, Some(0));
                }
            } else {
                app.toggle_pause();
            }
        }
    }
    if resp.clicked() {
        st.view = View::Player;
    }
}

/// The thin progress bar under the preview; click or drag to seek.
fn progress(ui: &Ui, bar: Rect, app: &mut App, p: &Palette, show_thumb: bool) {
    let resp = ui.interact(bar, ui.id().with("mini-seek"), Sense::click_and_drag());
    let painter = ui.painter();
    painter.rect_filled(bar, 6, p.bg_b2);
    let inner = bar.shrink(2.0);
    let len = app.status.length_ms;
    let mut frac = if len > 0 {
        (app.status.time_ms as f32 / len as f32).clamp(0.0, 1.0)
    } else {
        app.status.position.clamp(0.0, 1.0)
    };
    if let Some(pos) = resp.interact_pointer_pos() {
        let f = ((pos.x - inner.min.x) / inner.width()).clamp(0.0, 1.0);
        if resp.dragged() || resp.is_pointer_button_down_on() {
            frac = f;
        }
        if resp.drag_stopped() || resp.clicked() {
            app.seek_fraction(f);
        }
    }
    let x = inner.min.x + inner.width() * frac;
    if x > inner.min.x + 1.0 {
        painter.rect_filled(Rect::from_min_max(inner.min, pos2(x, inner.max.y)), 4, p.bg_accent);
    }
    let active = resp.hovered() || resp.dragged();
    if show_thumb || active {
        let tx = x.clamp(inner.min.x + 7.0, inner.max.x);
        let thumb = Rect::from_min_max(pos2(tx - 7.0, inner.min.y), pos2(tx + 1.0, inner.max.y));
        painter.rect_filled(thumb, 4, if active { p.accent_bright } else { p.accent });
    }
    if active {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
}

/// Title, folder and playlist chips, and the queue toggle.
fn info(ui: &Ui, card: Rect, top: f32, app: &mut App, st: &mut UiState, p: &Palette, item: &MediaItem) {
    let x0 = card.min.x + 16.0;
    let title = art::display_title(app, item);
    widgets::text_clipped(
        ui,
        pos2(x0, top + 4.0),
        Align2::LEFT_TOP,
        &title,
        font::regular(16.0),
        p.fg_a1,
        WIDTH - 32.0,
    );
    let row_y = top + 4.0 + 19.0 + 14.0;
    let list_btn = Rect::from_min_size(pos2(card.max.x - 16.0 - 30.0, row_y), vec2(30.0, 32.0));
    let max_x = list_btn.min.x - 12.0;
    let mut x = x0;
    let dir = item.source.path().and_then(|f| f.parent()).map(|d| d.to_path_buf());
    if let Some(dir) = dir.clone()
        && let Some(name) = dir.file_name().map(|n| n.to_string_lossy().into_owned())
    {
        let (r, clicked) = chip(ui, pos2(x, row_y + 4.0), Icon::Folder, &name, max_x, "mini-folder", p);
        if clicked {
            st.navigate(Page::Folder(dir));
        }
        x = r.max.x + GAP;
    }
    if let Some(origin) = app.queue.origin.clone()
        && x < max_x - 40.0
    {
        let (_, clicked) = chip(ui, pos2(x, row_y + 4.0), Icon::Artist, &origin, max_x, "mini-playlist", p);
        if clicked {
            match app.library.playlists().iter().find(|pl| pl.name == origin) {
                Some(pl) => st.navigate(Page::Playlist(pl.id)),
                None => {
                    if let Some(d) = dir {
                        st.navigate(Page::Folder(d));
                    }
                }
            }
        }
    }
    let open = st.music.mini_queue;
    if button(
        ui,
        list_btn,
        "mini-queue-toggle",
        Icon::Lists,
        if open { "Hide the queue" } else { "Show the queue" },
        open,
        p,
    ) {
        st.music.mini_queue = !open;
        st.music.mini_reveal = !open;
    }
}

/// Icon plus label, clickable. Returns its rect and whether it was clicked.
fn chip(ui: &Ui, at: egui::Pos2, icon: Icon, text: &str, max_x: f32, id: &str, p: &Palette) -> (Rect, bool) {
    let galley = ui.painter().layout_no_wrap(text.to_string(), font::regular(16.0), p.fg_a1);
    let w = (24.0 + 8.0 + galley.size().x).min(max_x - at.x).max(24.0);
    let r = Rect::from_min_size(at, vec2(w, 24.0));
    let resp = ui.interact(r, ui.id().with(id), Sense::click()).on_hover_text(text);
    let (fg_icon, fg_text) = if resp.hovered() { (p.fg2, p.fg2) } else { (p.fg_a2, p.fg_a1) };
    let ir = Rect::from_min_size(at, vec2(24.0, 24.0));
    widgets::paint_icon(ui, ir, icon, fg_icon);
    if icon == Icon::Artist {
        artist_arc(ui, ir, fg_icon);
    }
    widgets::text_clipped(
        ui,
        pos2(at.x + 32.0, r.center().y),
        Align2::LEFT_CENTER,
        text,
        font::regular(16.0),
        fg_text,
        (w - 32.0).max(0.0),
    );
    (r, resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked())
}

/// The "artist" icon's shoulders are a black cut-out in the design file, which
/// the icon renderer drops; draw them in the icon's colour.
fn artist_arc(ui: &Ui, r: Rect, color: Color32) {
    let k = r.width() / 24.0;
    let at = |x: f32, y: f32| pos2(r.min.x + x * k, r.min.y + y * k);
    let stroke = Stroke::new(1.5 * k, color);
    for pts in [
        [at(2.0, 19.0), at(2.4, 16.0), at(4.6, 13.0), at(9.0, 13.0)],
        [at(9.0, 13.0), at(13.4, 13.0), at(15.7, 16.0), at(16.0, 19.0)],
    ] {
        ui.painter().add(egui::epaint::CubicBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, stroke));
    }
}

/// "2h 3m" or "14m" for the queue summary.
pub fn format_total(ms: i64) -> String {
    let mins = (ms.max(0) + 30_000) / 60_000;
    let (h, m) = (mins / 60, mins % 60);
    if h > 0 { format!("{h}h {m}m") } else { format!("{m}m") }
}

fn queue_summary(app: &App) -> String {
    let n = app.queue.len();
    let total: i64 = app
        .queue
        .items
        .iter()
        .filter_map(|i| i.meta.duration_ms.or_else(|| app.meta_for(&i.source).and_then(|m| m.duration_ms)))
        .sum();
    let items = if n == 1 { "1 item".to_string() } else { format!("{n} items") };
    if total > 0 { format!("{items} - {}", format_total(total)) } else { items }
}

/// The queue list above the preview (variants 2 and 4 of the component).
fn queue_panel(ui: &mut Ui, card: Rect, app: &mut App, st: &mut UiState, p: &Palette, hovered: bool) {
    let (body_fill, head_fill) = if hovered { (p.bg_a3, p.bg_a1) } else { (p.bg_b1, p.bg_b2) };
    let body = Rect::from_min_size(card.min + vec2(PAD, PAD), vec2(WIDTH - 2.0 * PAD, QUEUE_H));
    let painter = ui.painter().clone();
    painter.rect_filled(body, 10, body_fill);
    let head = Rect::from_min_size(card.min, vec2(WIDTH, HEADER_H));
    painter.rect_filled(head, CornerRadius { nw: 14, ne: 14, sw: 2, se: 2 }, head_fill);
    painter.text(
        pos2(card.center().x, card.min.y + 20.0 + 9.5),
        Align2::CENTER_CENTER,
        "Queue",
        font::regular(16.0),
        p.fg_a2,
    );

    let summary_y = card.min.y + HEADER_H + 1.0 + 32.0;
    painter.text(
        pos2(card.min.x + 36.0, summary_y),
        Align2::LEFT_CENTER,
        queue_summary(app),
        font::regular(14.0),
        p.fg_a3,
    );
    let bx = |dx: f32| Rect::from_center_size(pos2(card.min.x + dx + 12.0, summary_y), vec2(30.0, 32.0));
    let desc = st.music.mini_sort_desc;
    if button(ui, bx(251.0), "mini-q-sort", Icon::Sort, if desc { "Sort Z to A" } else { "Sort A to Z" }, false, p) {
        if desc {
            app.queue.sort_by_key(|i| std::cmp::Reverse(i.title().to_lowercase()));
        } else {
            app.queue.sort_by_key(|i| i.title().to_lowercase());
        }
        st.music.mini_sort_desc = !desc;
        st.music.mini_selected = None;
        app.save_queue();
    }
    let selected = st.music.mini_selected.filter(|i| *i < app.queue.len());
    if button(ui, bx(295.0), "mini-q-remove", Icon::Remove, "Remove the selected item", false, p)
        && let Some(i) = selected
    {
        app.queue.remove(i);
        st.music.mini_selected = None;
        app.save_queue();
    }
    if button(ui, bx(339.0), "mini-q-add", Icon::Add, "Add files to the queue", false, p)
        && let Some(files) = rfd::FileDialog::new().set_title("Add to Queue").pick_files()
    {
        let items = app.expand_paths(&files);
        app.enqueue(items);
    }

    let rows =
        Rect::from_min_max(pos2(card.min.x + 28.0, card.min.y + 124.0), pos2(card.max.x - 28.0, body.max.y - 4.0));
    ui.scope_builder(UiBuilder::new().max_rect(rows), |ui| {
        ui.set_clip_rect(rows);
        ui.spacing_mut().item_spacing.y = ROW_GAP;
        egui::ScrollArea::vertical().id_salt("mini-queue-rows").auto_shrink([false, false]).show(ui, |ui| {
            queue_rows(ui, rows.width(), app, st, p);
        });
    });
}

fn queue_rows(ui: &mut Ui, w: f32, app: &mut App, st: &mut UiState, p: &Palette) {
    let current = app.queue.current;
    let mut rects = Vec::with_capacity(app.queue.len());
    let mut play = None;
    for i in 0..app.queue.len() {
        let (r, resp) = ui.allocate_exact_size(vec2(w, ROW_H), Sense::click());
        rects.push(r);
        if st.music.mini_reveal && current == Some(i) {
            ui.scroll_to_rect(r, Some(egui::Align::Center));
            st.music.mini_reveal = false;
        }
        if !ui.is_rect_visible(r) {
            continue;
        }
        let item = &app.queue.items[i];
        let selected = st.music.mini_selected == Some(i);
        if selected {
            ui.painter().rect_filled(r, 4, p.bg_a1);
        } else if resp.hovered() {
            ui.painter().rect_filled(r, 4, p.hover);
        }
        let handle = Rect::from_min_size(r.min, vec2(32.0, 32.0));
        let hresp = ui.interact(handle, ui.id().with(("mini-q-handle", i)), Sense::drag());
        if hresp.drag_started() {
            st.music.mini_drag = Some(i);
        }
        let hfg = if hresp.hovered() || st.music.mini_drag == Some(i) { p.fg2 } else { p.fg_a2 };
        widgets::paint_icon(ui, handle.shrink(4.0), Icon::Queue, hfg);
        if hresp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        let dur = item.meta.duration_ms.or_else(|| app.meta_for(&item.source).and_then(|m| m.duration_ms));
        let dur_text = dur.map(format_short).unwrap_or_default();
        let dg = ui.painter().layout_no_wrap(dur_text, font::regular(14.0), p.fg_a3);
        let dur_w = dg.size().x;
        ui.painter().galley(pos2(r.max.x - 4.0 - dur_w, r.center().y - dg.size().y / 2.0), dg, p.fg_a3);
        let fg = if current == Some(i) { p.accent } else { p.fg_a1 };
        widgets::text_clipped(
            ui,
            pos2(r.min.x + 42.0, r.center().y),
            Align2::LEFT_CENTER,
            &item.title(),
            font::regular(14.0),
            fg,
            (w - 42.0 - dur_w - 14.0).max(0.0),
        );
        if resp.double_clicked() {
            play = Some(i);
        } else if resp.clicked() {
            st.music.mini_selected = if selected { None } else { Some(i) };
        }
    }
    // Drag and drop by the handle: an insertion line follows the pointer.
    if let Some(from) = st.music.mini_drag {
        let y = ui.ctx().pointer_latest_pos().map(|p| p.y);
        let to = y.map(|y| rects.iter().position(|r| y < r.center().y).unwrap_or(rects.len().saturating_sub(1)));
        if let (Some(to), Some(r)) = (to, to.and_then(|t| rects.get(t))) {
            let ly = if to > from { r.max.y + ROW_GAP / 2.0 } else { r.min.y - ROW_GAP / 2.0 };
            ui.painter().line_segment([pos2(r.min.x, ly), pos2(r.max.x, ly)], Stroke::new(2.0, p.accent));
        }
        if ui.input(|i| !i.pointer.any_down()) {
            if let Some(to) = to {
                app.queue.move_item(from, to.min(app.queue.len().saturating_sub(1)));
                app.save_queue();
            }
            st.music.mini_drag = None;
            st.music.mini_selected = None;
        }
    }
    if let Some(i) = play {
        app.play_index_from(i, None);
    }
}

#[cfg(test)]
mod tests {
    use super::format_total;

    #[test]
    fn totals_read_like_the_design() {
        assert_eq!(format_total(7_380_000), "2h 3m");
        assert_eq!(format_total(14 * 60_000), "14m");
        assert_eq!(format_total(0), "0m");
    }
}
