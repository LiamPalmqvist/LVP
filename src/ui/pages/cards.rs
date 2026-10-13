//! Painters for library tiles: grid cards (16:9 thumbnails, square covers,
//! folder cards), compact list rows, progress lines and the play badge.

use std::path::PathBuf;

use egui::{Align2, Color32, Pos2, Rect, Shape as EShape, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::app::App;
use crate::domain::media::MediaItem;
use crate::domain::time::format_short;
use crate::ui::icons::Icon;
use crate::ui::images;
use crate::ui::theme::{Palette, font, with_alpha};
use crate::ui::widgets::{self, pal};

use super::model::{Shape, Tile, TileKind};

/// Paints an icon; the design's "grid" icon has no paths in the export, so
/// its four rounded squares are drawn here.
pub fn icon(ui: &Ui, rect: Rect, icon: Icon, color: Color32) {
    let k = rect.width() / 24.0;
    if icon == Icon::Add {
        // The exported "add" strokes are black (an override), so draw the plus.
        let c = rect.center();
        let s = Stroke::new(1.5 * k, color);
        ui.painter().line_segment([c - vec2(7.0 * k, 0.0), c + vec2(7.0 * k, 0.0)], s);
        ui.painter().line_segment([c - vec2(0.0, 7.0 * k), c + vec2(0.0, 7.0 * k)], s);
        return;
    }
    if icon != Icon::Grid {
        widgets::paint_icon(ui, rect, icon, color);
        return;
    }
    for (x, y) in [(1.0, 3.0), (13.0, 3.0), (1.0, 13.0), (13.0, 13.0)] {
        let r = Rect::from_min_size(rect.min + vec2(x, y) * k, vec2(10.0, 8.0) * k).shrink(0.75 * k);
        ui.painter().rect_stroke(r, 2.0 * k, Stroke::new(1.5 * k, color), StrokeKind::Middle);
    }
}

/// Thumbnail height for a 16:9 card of width `w` (316x178 in the design).
pub fn wide_h(w: f32) -> f32 {
    (w * 178.0 / 316.0).round()
}

/// Gap between a card's picture and its title.
pub const TITLE_GAP: f32 = 20.0;
/// Title line height (Regular 16).
pub const LINE: f32 = 19.0;
/// Gap between title and subtitle.
pub const SUB_GAP: f32 = 10.0;

/// Card height for a grid cell of width `w`.
pub fn card_h(shape: Shape, w: f32, subtitle: bool) -> f32 {
    let img = match shape {
        Shape::Wide => wide_h(w),
        Shape::Square => w,
    };
    img + TITLE_GAP + LINE + if subtitle { SUB_GAP + LINE } else { 0.0 }
}

/// The convex polygon where `inner` overlaps a rounded rectangle.
fn rounded_clip(outer: Rect, r: f32, inner: Rect) -> Vec<Pos2> {
    let r = r.min(outer.width() / 2.0).min(outer.height() / 2.0);
    let mut pts = Vec::with_capacity(40);
    let corners = [
        (pos2(outer.max.x - r, outer.max.y - r), 0.0f32),
        (pos2(outer.min.x + r, outer.max.y - r), 90.0),
        (pos2(outer.min.x + r, outer.min.y + r), 180.0),
        (pos2(outer.max.x - r, outer.min.y + r), 270.0),
    ];
    for (c, start) in corners {
        for k in 0..=8 {
            let a = (start + k as f32 * 90.0 / 8.0).to_radians();
            pts.push(pos2(c.x + r * a.cos(), c.y + r * a.sin()));
        }
    }
    // Sutherland-Hodgman against the four edges of `inner`.
    let clip = |pts: Vec<Pos2>, inside: &dyn Fn(Pos2) -> bool, cut: &dyn Fn(Pos2, Pos2) -> Pos2| {
        let mut out = Vec::with_capacity(pts.len() + 4);
        for i in 0..pts.len() {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            match (inside(a), inside(b)) {
                (true, true) => out.push(b),
                (true, false) => out.push(cut(a, b)),
                (false, true) => {
                    out.push(cut(a, b));
                    out.push(b);
                }
                (false, false) => {}
            }
        }
        out
    };
    let lerp_x = |a: Pos2, b: Pos2, x: f32| {
        let t = if (b.x - a.x).abs() < f32::EPSILON { 0.0 } else { (x - a.x) / (b.x - a.x) };
        pos2(x, a.y + (b.y - a.y) * t)
    };
    let lerp_y = |a: Pos2, b: Pos2, y: f32| {
        let t = if (b.y - a.y).abs() < f32::EPSILON { 0.0 } else { (y - a.y) / (b.y - a.y) };
        pos2(a.x + (b.x - a.x) * t, y)
    };
    let pts = clip(pts, &|p| p.x >= inner.min.x, &|a, b| lerp_x(a, b, inner.min.x));
    let pts = clip(pts, &|p| p.x <= inner.max.x, &|a, b| lerp_x(a, b, inner.max.x));
    let pts = clip(pts, &|p| p.y >= inner.min.y, &|a, b| lerp_y(a, b, inner.min.y));
    clip(pts, &|p| p.y <= inner.max.y, &|a, b| lerp_y(a, b, inner.max.y))
}

/// The accent line along the bottom of a thumbnail, clipped to its corners.
pub fn progress_line(ui: &Ui, thumb: Rect, radius: f32, frac: f32, height: f32) {
    let p = pal(ui.ctx());
    let line = Rect::from_min_max(
        pos2(thumb.min.x, thumb.max.y - height),
        pos2(thumb.min.x + thumb.width() * frac, thumb.max.y),
    );
    if line.width() < 0.5 {
        return;
    }
    let pts = rounded_clip(thumb, radius, line);
    if pts.len() >= 3 {
        ui.painter().add(EShape::convex_polygon(pts, p.accent, Stroke::NONE));
    }
}

/// Paints a picture (thumbnail or cover) or, failing that, an icon on a
/// plain card. Returns whether a picture was drawn.
pub fn picture(
    ui: &Ui,
    rect: Rect,
    path: Option<&PathBuf>,
    icon: Icon,
    icon_size: f32,
    radius: u8,
    fill: Color32,
) -> bool {
    let p = pal(ui.ctx());
    ui.painter().rect_filled(rect, radius, fill);
    if let Some(path) = path
        && images::paint_cover(ui, rect, path, radius)
    {
        return true;
    }
    self::icon(ui, Rect::from_center_size(rect.center(), vec2(icon_size, icon_size)), icon, p.fg_a2);
    false
}

/// The accent "play" pill shown over a hovered thumbnail.
pub fn play_badge(ui: &Ui, center: Pos2, hovered: bool) -> Rect {
    let p = pal(ui.ctx());
    let r = Rect::from_center_size(center, vec2(54.0, 36.0));
    ui.painter().rect_filled(r, 8, if hovered { p.accent_bright } else { p.accent });
    let fg = if p.dark { Color32::from_rgb(0x16, 0x16, 0x16) } else { Color32::WHITE };
    widgets::paint_icon(ui, Rect::from_center_size(r.center(), vec2(24.0, 24.0)), Icon::Play, fg);
    r
}

/// What picture stands for a tile: a media thumbnail, a playlist cover, or
/// the cover of a representative item.
pub fn tile_picture(app: &mut App, tile: &Tile, item: Option<&MediaItem>) -> Option<PathBuf> {
    if let Some(i) = item {
        return app.thumbnail(i);
    }
    if let TileKind::Nav(super::Page::Playlist(id)) = tile.kind {
        let custom = app.playlist_cover_path(id);
        if custom.exists() {
            return Some(custom);
        }
    }
    let cover = tile.cover.clone()?;
    app.thumbnail(&cover)
}

fn text_color(p: &Palette, tile: &Tile, hovered: bool) -> Color32 {
    if hovered {
        p.fg2
    } else if tile.dim || matches!(tile.kind, TileKind::Action(_)) {
        p.fg_a3
    } else {
        p.fg_a1
    }
}

#[allow(clippy::too_many_arguments)]
/// Draws one grid card in `cell` (picture on top, title and subtitle below).
/// Returns the picture rect and, for media, the play badge rect.
pub fn grid_card(
    ui: &Ui,
    app: &mut App,
    cell: Rect,
    shape: Shape,
    tile: &Tile,
    item: Option<&MediaItem>,
    hovered: bool,
    selected: bool,
) -> (Rect, Option<Rect>) {
    let p = pal(ui.ctx());
    let w = cell.width();
    let img = Rect::from_min_size(cell.min, vec2(w, if shape == Shape::Wide { wide_h(w) } else { w }));
    let pic = tile_picture(app, tile, item);
    let is_media = matches!(tile.kind, TileKind::Media(_));
    let fill = if is_media || pic.is_some() { p.bg_a5 } else { p.bg_a2 };
    let icon_size = if is_media { 40.0 } else { 48.0 };
    let drawn = picture(ui, img, pic.as_ref(), tile.icon, icon_size, 12, fill);
    if drawn && !hovered {
        ui.painter().rect_filled(img, 12, with_alpha(Color32::BLACK, 0.2));
    }
    if !is_media && drawn {
        // A playlist or album with a cover: keep its kind recognisable.
        let badge = Rect::from_min_size(img.min + vec2(10.0, img.height() - 42.0), vec2(32.0, 32.0));
        ui.painter().rect_filled(badge, 6, with_alpha(Color32::BLACK, 0.55));
        widgets::paint_icon(ui, badge.shrink(4.0), tile.icon, p.fg2);
    }
    if !is_media && hovered {
        ui.painter().rect_filled(img, 12, p.hover);
    }
    if let Some(f) = tile.record.progress() {
        progress_line(ui, img, 12.0, f, 4.0);
    }
    if selected {
        ui.painter().rect_stroke(img.expand(3.0), 14, Stroke::new(2.0, p.accent), StrokeKind::Inside);
    }
    let badge = (is_media && hovered).then(|| {
        let hover_badge = ui.rect_contains_pointer(Rect::from_center_size(img.center(), vec2(54.0, 36.0)));
        play_badge(ui, img.center(), hover_badge)
    });
    let ty = img.max.y + TITLE_GAP;
    widgets::text_clipped(
        ui,
        pos2(cell.min.x, ty),
        Align2::LEFT_TOP,
        &tile.title,
        font::regular(16.0),
        text_color(&p, tile, hovered),
        w,
    );
    if let Some(sub) = &tile.subtitle {
        widgets::text_clipped(
            ui,
            pos2(cell.min.x, ty + LINE + SUB_GAP),
            Align2::LEFT_TOP,
            sub,
            font::regular(16.0),
            p.fg_a3,
            w,
        );
    }
    (img, badge)
}

#[allow(clippy::too_many_arguments)]
/// Draws a compact row: a 24pt thumbnail or icon in a 32pt box, the title,
/// and the duration on the right when known.
pub fn list_row(
    ui: &Ui,
    app: &mut App,
    row: Rect,
    tile: &Tile,
    item: Option<&MediaItem>,
    hovered: bool,
    selected: bool,
    show_duration: bool,
) {
    let p = pal(ui.ctx());
    if selected {
        ui.painter().rect_filled(row.expand2(vec2(4.0, 0.0)), 6, p.bg_a3);
    }
    let boxr = Rect::from_min_size(row.min + vec2(0.0, (row.height() - 32.0) / 2.0), vec2(32.0, 32.0));
    let is_media = matches!(tile.kind, TileKind::Media(_));
    if is_media {
        let th = Rect::from_center_size(boxr.center(), vec2(24.0, 24.0));
        let pic = tile_picture(app, tile, item);
        let drawn = picture(ui, th, pic.as_ref(), tile.icon, 16.0, 2, p.bg_a1);
        if let Some(f) = tile.record.progress() {
            progress_line(ui, th, 2.0, f, 2.0);
        }
        if hovered && is_media {
            if drawn {
                ui.painter().rect_filled(th, 2, with_alpha(Color32::BLACK, 0.5));
            }
            widgets::paint_icon(ui, Rect::from_center_size(th.center(), vec2(16.0, 16.0)), Icon::Play, p.fg1);
        }
    } else {
        icon(ui, Rect::from_center_size(boxr.center(), vec2(24.0, 24.0)), tile.icon, p.fg_a2);
    }
    let mut right = row.max.x;
    if show_duration
        && let Some(d) =
            item.and_then(|i| app.library.meta(&i.source).and_then(|m| m.duration_ms).or(i.meta.duration_ms))
        && d > 0
    {
        let r = ui.painter().text(
            pos2(row.max.x - 4.0, row.center().y),
            Align2::RIGHT_CENTER,
            format_short(d),
            font::regular(13.0),
            p.fg_a3,
        );
        right = r.min.x - 12.0;
    }
    let col = if hovered && is_media { p.accent } else { text_color(&p, tile, hovered) };
    let label = match (&tile.subtitle, is_media && item.is_some_and(MediaItem::is_audio)) {
        (Some(s), true) => format!("{} - {}", tile.title, s),
        _ => tile.title.clone(),
    };
    widgets::text_clipped(
        ui,
        pos2(boxr.max.x + 10.0, row.center().y),
        Align2::LEFT_CENTER,
        &label,
        font::regular(14.0),
        col,
        (right - boxr.max.x - 10.0).max(10.0),
    );
}
