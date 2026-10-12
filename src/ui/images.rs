//! Thumbnails and cover art from disk, decoded off the UI thread and cached
//! as textures.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender, unbounded};
use egui::{Color32, Context, CornerRadius, Id, Rect, TextureHandle, TextureOptions, Ui, pos2};
use parking_lot::Mutex;

enum Slot {
    Loading,
    Ready(TextureHandle, [usize; 2]),
    Failed,
}

struct Loader {
    slots: HashMap<PathBuf, Slot>,
    tx: Sender<PathBuf>,
    rx: Receiver<(PathBuf, Option<egui::ColorImage>)>,
}

const MAX_SIDE: u32 = 640;

fn loader(ctx: &Context) -> Arc<Mutex<Loader>> {
    ctx.data_mut(|d| {
        d.get_temp_mut_or_insert_with(Id::new("notvlc-image-loader"), || {
            let (req_tx, req_rx) = unbounded::<PathBuf>();
            let (res_tx, res_rx) = unbounded();
            for i in 0..2 {
                let (req_rx, res_tx, ctx) = (req_rx.clone(), res_tx.clone(), ctx.clone());
                std::thread::Builder::new()
                    .name(format!("image-loader-{i}"))
                    .spawn(move || {
                        for path in req_rx {
                            let img = decode(&path);
                            if res_tx.send((path, img)).is_err() {
                                return;
                            }
                            ctx.request_repaint();
                        }
                    })
                    .ok();
            }
            Arc::new(Mutex::new(Loader { slots: HashMap::new(), tx: req_tx, rx: res_rx }))
        })
        .clone()
    })
}

fn decode(path: &Path) -> Option<egui::ColorImage> {
    let img = image::open(path).ok()?;
    let img = if img.width() > MAX_SIDE || img.height() > MAX_SIDE { img.thumbnail(MAX_SIDE, MAX_SIDE) } else { img };
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}

/// The texture for an image file, or `None` while it loads or if it fails.
pub fn texture(ctx: &Context, path: &Path) -> Option<(TextureHandle, [usize; 2])> {
    let l = loader(ctx);
    let mut l = l.lock();
    while let Ok((p, img)) = l.rx.try_recv() {
        let slot = match img {
            Some(img) => {
                let size = img.size;
                Slot::Ready(ctx.load_texture(p.to_string_lossy(), img, TextureOptions::LINEAR), size)
            }
            None => Slot::Failed,
        };
        l.slots.insert(p, slot);
    }
    match l.slots.get(path) {
        Some(Slot::Ready(t, s)) => Some((t.clone(), *s)),
        Some(_) => None,
        None => {
            l.slots.insert(path.to_path_buf(), Slot::Loading);
            let _ = l.tx.send(path.to_path_buf());
            None
        }
    }
}

/// Forgets a cached image (after a thumbnail is regenerated).
pub fn forget(ctx: &Context, path: &Path) {
    loader(ctx).lock().slots.remove(path);
}

/// Paints an image covering `rect` (cropping to fill), with rounded corners.
/// Returns false when the image is not ready.
pub fn paint_cover(ui: &Ui, rect: Rect, path: &Path, radius: u8) -> bool {
    let Some((tex, [w, h])) = texture(ui.ctx(), path) else { return false };
    let (w, h) = (w as f32, h as f32);
    let ra = rect.width() / rect.height();
    let ia = w / h;
    let uv = if ia > ra {
        let k = ra / ia;
        Rect::from_min_max(pos2((1.0 - k) / 2.0, 0.0), pos2((1.0 + k) / 2.0, 1.0))
    } else {
        let k = ia / ra;
        Rect::from_min_max(pos2(0.0, (1.0 - k) / 2.0), pos2(1.0, (1.0 + k) / 2.0))
    };
    let mut shape = egui::epaint::RectShape::filled(rect, CornerRadius::same(radius), Color32::WHITE);
    shape.brush = Some(Arc::new(egui::epaint::Brush { fill_texture_id: tex.id(), uv }));
    ui.painter().add(shape);
    true
}

/// Paints an image fitted inside `rect` (letterboxed).
pub fn paint_contain(ui: &Ui, rect: Rect, path: &Path) -> bool {
    let Some((tex, [w, h])) = texture(ui.ctx(), path) else { return false };
    let s = (rect.width() / w as f32).min(rect.height() / h as f32);
    let r = Rect::from_center_size(rect.center(), egui::vec2(w as f32 * s, h as f32 * s));
    ui.painter().image(tex.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    true
}
