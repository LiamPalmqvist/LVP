//! The design's icon set. SVGs are recoloured for the active palette and
//! rasterised at the display's pixel density, then cached as textures.

use std::collections::HashMap;
use std::sync::Arc;

use egui::{Color32, Context, Id, Rect, Response, Sense, TextureHandle, TextureOptions, Ui, Vec2};
use parking_lot::Mutex;

use crate::domain::settings::Accent;

include!(concat!(env!("OUT_DIR"), "/icons.rs"));

impl Icon {
    /// Speaker icon for a volume level (the design files these under "close").
    pub fn volume(level: i32, muted: bool) -> Icon {
        if muted || level == 0 {
            Icon::CloseNone
        } else if level < 40 {
            Icon::CloseLow
        } else if level < 80 {
            Icon::CloseMedium
        } else {
            Icon::VolumeHigh
        }
    }
}

/// Colours substituted into an icon: the default grey becomes `fg`, the
/// design's orange becomes the accent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IconColors {
    pub fg: Color32,
    pub accent: Color32,
    pub accent_dim: Color32,
}

/// The app icon for the window, taskbar and Dock: the logo in the accent's
/// colour, `px` pixels square, as straight (not premultiplied) RGBA. Orange
/// takes the brighter orange of the packaged icon.
pub fn app_icon(accent: Accent, px: u32) -> Option<egui::IconData> {
    let rgb = if accent == Accent::Orange { 0xFF8800 } else { accent.hex() };
    let colour = format!("rgb({},{},{})", rgb >> 16, (rgb >> 8) & 0xFF, rgb & 0xFF);
    let svg = Icon::Logo.svg().replace("rgb(211,121,24)", &colour);
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).ok()?;
    let mut pm = resvg::tiny_skia::Pixmap::new(px, px)?;
    let k = px as f32 / tree.size().width();
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(k, k), &mut pm.as_mut());
    let rgba = pm
        .data()
        .chunks_exact(4)
        .flat_map(|p| {
            let a = p[3] as f32 / 255.0;
            let f = |c: u8| if a > 0.0 { ((c as f32 / a).round().min(255.0)) as u8 } else { 0 };
            [f(p[0]), f(p[1]), f(p[2]), p[3]]
        })
        .collect();
    Some(egui::IconData { rgba, width: px, height: px })
}

type Key = (Icon, u32, IconColors);

#[derive(Default)]
struct Cache {
    textures: HashMap<Key, TextureHandle>,
}

fn cache(ctx: &Context) -> Arc<Mutex<Cache>> {
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Arc<Mutex<Cache>>>(Id::new("lvp-icon-cache")).clone())
}

fn css(c: Color32) -> String {
    format!("rgb({},{},{})", c.r(), c.g(), c.b())
}

fn rasterise(icon: Icon, px: u32, colors: IconColors) -> Option<egui::ColorImage> {
    let mut svg = icon.svg().replace("rgb(143,143,143)", &css(colors.fg));
    svg = svg.replace("rgb(211,121,24)", &css(colors.accent));
    svg = svg.replace("rgb(171,99,20)", &css(colors.accent_dim));
    // Black in the source marks parts the design recolours per instance
    // (the third dot of "more", the strokes of "add" and "edit"); they take
    // the foreground colour.
    svg = svg.replace("fill=\"rgb(0,0,0)\"", &format!("fill=\"{}\"", css(colors.fg)));
    if colors.fg.a() != 255 {
        // Fold the colour's alpha into the opacity of the whole icon.
        svg = svg.replacen("<svg ", &format!("<svg opacity=\"{:.3}\" ", colors.fg.a() as f32 / 255.0), 1);
    }
    let opt = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(&svg, &opt).ok()?;
    let size = tree.size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(px, px)?;
    let k = px as f32 / size.width().max(size.height());
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(k, k), &mut pixmap.as_mut());
    // tiny-skia stores premultiplied RGBA, which is what egui expects.
    let pixels =
        pixmap.data().chunks_exact(4).map(|p| Color32::from_rgba_premultiplied(p[0], p[1], p[2], p[3])).collect();
    Some(egui::ColorImage { size: [px as usize, px as usize], source_size: Vec2::splat(px as f32), pixels })
}

/// Texture for an icon at `size` points.
pub fn texture(ctx: &Context, icon: Icon, size: f32, colors: IconColors) -> Option<TextureHandle> {
    let px = (size * ctx.pixels_per_point()).round().max(1.0) as u32;
    let key = (icon, px, colors);
    let c = cache(ctx);
    let mut c = c.lock();
    if let Some(t) = c.textures.get(&key) {
        return Some(t.clone());
    }
    let img = rasterise(icon, px, colors)?;
    let tex = ctx.load_texture(format!("icon-{}-{px}", icon.name()), img, TextureOptions::LINEAR);
    if c.textures.len() > 2000 {
        c.textures.clear();
    }
    c.textures.insert(key, tex.clone());
    Some(tex)
}

pub fn paint(ui: &Ui, rect: Rect, icon: Icon, colors: IconColors) {
    if let Some(t) = texture(ui.ctx(), icon, rect.width().max(rect.height()), colors) {
        ui.painter().image(
            t.id(),
            rect,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
}

/// Allocates and paints an icon; returns a clickable response.
pub fn show(ui: &mut Ui, icon: Icon, size: f32, colors: IconColors) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    paint(ui, rect, icon, colors);
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_icon_takes_the_accent_colour() {
        let px = |icon: &egui::IconData, x: u32, y: u32| {
            let i = ((y * icon.width + x) * 4) as usize;
            [icon.rgba[i], icon.rgba[i + 1], icon.rgba[i + 2], icon.rgba[i + 3]]
        };
        let blue = app_icon(Accent::Blue, 96).unwrap();
        // A point on the tile, clear of the play symbol, and the symbol's centre.
        assert_eq!(px(&blue, 20, 20), [0x18, 0x5D, 0xD3, 255]);
        assert_eq!(px(&blue, 48, 48)[3], 0);
        let orange = app_icon(Accent::Orange, 96).unwrap();
        assert_eq!(px(&orange, 20, 20), [0xFF, 0x88, 0x00, 255]);
    }
}
