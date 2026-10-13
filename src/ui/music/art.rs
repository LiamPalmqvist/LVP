//! Cover art helpers shared by the music view and the mini player: textured
//! rounded rects, cover-fit UVs, a blurred copy of the art for backdrops and
//! the placeholder drawn when a track has no art.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui::epaint::{Brush, RectShape};
use egui::{Color32, Context, CornerRadius, Id, Painter, Rect, TextureHandle, TextureId, TextureOptions, Ui, vec2};
use parking_lot::Mutex;

use crate::app::App;
use crate::domain::media::MediaItem;
use crate::ui::icons::Icon;
use crate::ui::theme::{Palette, mix};
use crate::ui::widgets;

/// Paints `tex` (the `uv` part of it) into `rect` with rounded corners.
pub fn paint_texture(painter: &Painter, rect: Rect, tex: TextureId, uv: Rect, radius: CornerRadius, tint: Color32) {
    let mut shape = RectShape::filled(rect, radius, tint);
    shape.brush = Some(Arc::new(Brush { fill_texture_id: tex, uv }));
    painter.add(shape);
}

/// The current item and its cover art path, if any.
pub fn current_art(app: &mut App) -> (Option<MediaItem>, Option<PathBuf>) {
    let item = app.queue.current_item().cloned();
    let art = item.as_ref().and_then(|i| app.thumbnail(i));
    (item, art)
}

/// "Title - Artist" for music, the file name otherwise.
pub fn display_title(app: &App, item: &MediaItem) -> String {
    let title = app.status.meta.title.clone().filter(|t| !t.trim().is_empty()).unwrap_or_else(|| item.title());
    let artist = app.status.meta.artist.clone().or_else(|| item.meta.artist.clone()).filter(|a| !a.trim().is_empty());
    match artist {
        Some(a) if item.is_audio() && !title.contains(&a) => format!("{title} - {a}"),
        _ => title,
    }
}

/// Paints cover art filling `rect`. Returns false when there is none (yet).
pub fn paint_art(ui: &Ui, rect: Rect, art: Option<&Path>, radius: u8) -> bool {
    art.is_some_and(|a| crate::ui::images::paint_cover(ui, rect, a, radius))
}

/// The design's stand-in for missing cover art: a soft tile with a note.
pub fn placeholder(ui: &Ui, rect: Rect, radius: u8, p: &Palette) {
    let base = if p.dark { mix(p.bg_a1, p.bg_b2, 0.35) } else { p.bg_a2 };
    ui.painter().rect_filled(rect, radius, base);
    // A faint inner ring echoes the record-like artwork in the design.
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.28;
    let ring = if p.dark { mix(base, p.fg_a4, 0.6) } else { mix(base, p.fg_a4, 0.35) };
    ui.painter().circle_stroke(c, r, egui::Stroke::new((r * 0.035).clamp(1.0, 2.0), ring));
    ui.painter().circle_stroke(c, r * 0.72, egui::Stroke::new(1.0, mix(base, ring, 0.6)));
    let side = (r * 0.7).clamp(16.0, 96.0);
    widgets::paint_icon(ui, Rect::from_center_size(c, vec2(side, side)), Icon::Music, p.fg_a3);
}

// ---- blurred backdrop ----------------------------------------------------------

enum Slot {
    Loading,
    Ready(TextureHandle),
    Failed,
}

type BlurCache = Arc<Mutex<HashMap<PathBuf, Slot>>>;

fn blur_cache(ctx: &Context) -> BlurCache {
    ctx.data_mut(|d| d.get_temp_mut_or_default::<BlurCache>(Id::new("music-blur-cache")).clone())
}

/// Side of the small blurred copy. Upscaled with linear filtering it reads as
/// the design's 20pt layer blur on a window-sized image.
const BLUR_SIDE: u32 = 160;

fn blurred_image(path: &Path) -> Option<egui::ColorImage> {
    let img = image::open(path).ok()?;
    let small = img.resize_to_fill(BLUR_SIDE, BLUR_SIDE, image::imageops::FilterType::Triangle).to_rgba8();
    let soft = image::imageops::blur(&small, 1.6);
    let size = [soft.width() as usize, soft.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(size, soft.as_raw()))
}

/// A blurred copy of the art at `path`, made on a background thread.
pub fn blurred(ctx: &Context, path: &Path) -> Option<TextureHandle> {
    let cache = blur_cache(ctx);
    let mut map = cache.lock();
    match map.get(path) {
        Some(Slot::Ready(t)) => return Some(t.clone()),
        Some(_) => return None,
        None => {}
    }
    if map.len() > 16 {
        map.clear();
    }
    map.insert(path.to_path_buf(), Slot::Loading);
    drop(map);
    let (ctx2, path2, cache2) = (ctx.clone(), path.to_path_buf(), cache.clone());
    let spawned = std::thread::Builder::new().name("music-blur".into()).spawn(move || {
        let slot = match blurred_image(&path2) {
            Some(img) => {
                Slot::Ready(ctx2.load_texture(format!("blur:{}", path2.display()), img, TextureOptions::LINEAR))
            }
            None => Slot::Failed,
        };
        cache2.lock().insert(path2, slot);
        ctx2.request_repaint();
    });
    if spawned.is_err() {
        cache.lock().insert(path.to_path_buf(), Slot::Failed);
    }
    None
}
