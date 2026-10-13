//! The music player view (frames "Player - Music"): the cover art blurred
//! across the background, the art centred with a soft glow, the title under
//! it and a bar visualiser behind it.

use egui::epaint::Shadow;
use egui::{Align2, Color32, CornerRadius, Rect, Ui, pos2, vec2};

use crate::app::App;
use crate::ui::state::UiState;
use crate::ui::theme::{font, with_alpha};
use crate::ui::widgets::{self, pal};

use super::{art, visualiser};

/// Height of the design's player area (1080 minus title and control bars);
/// the layout scales from it.
const DESIGN_AREA_H: f32 = 986.0;

/// Where the art, title and visualiser go inside `rect`.
pub struct Layout {
    pub art: Rect,
    pub title_top: f32,
    pub font_size: f32,
    pub radius: u8,
    pub scale: f32,
}

/// Design: 73pt margin, 738pt art, 74pt gap, 29pt title, 73pt margin.
/// The art is centred on `centre_x` (the window's centre in the design, also
/// with the library column open) as far as the area allows.
pub fn layout(rect: Rect, centre_x: f32) -> Layout {
    let k = (rect.height() / DESIGN_AREA_H).min(1.4);
    let margin = (73.0 * k).max(12.0);
    let gap = (74.0 * k).clamp(10.0, 74.0);
    let font_size = (24.0 * k).clamp(13.0, 24.0);
    let title_h = font_size * 1.2;
    let side_margin = margin.max(24.0);
    let side = (rect.height() - 2.0 * margin - gap - title_h).min(rect.width() - 2.0 * side_margin).max(32.0);
    let block = side + gap + title_h;
    let top = rect.center().y - block / 2.0;
    let lo = rect.min.x + side_margin + side / 2.0;
    let hi = rect.max.x - side_margin - side / 2.0;
    let cx = if lo <= hi { centre_x.clamp(lo, hi) } else { rect.center().x };
    let art = Rect::from_min_size(pos2(cx - side / 2.0, top), vec2(side, side));
    Layout { art, title_top: art.max.y + gap, font_size, radius: (20.0 * k).clamp(8.0, 20.0) as u8, scale: k }
}

/// Shown in the player when the current item has no picture.
pub fn show_player(ui: &mut Ui, rect: Rect, app: &mut App, st: &mut UiState) {
    let p = pal(ui.ctx());
    let (item, cover) = art::current_art(app);
    let Some(item) = item else { return };
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0, p.bg_a3);

    // In picture-in-picture the control bar overlays the bottom of a small
    // window; keep the art and title clear of it so they never jump.
    let lay = if st.pip {
        let bar = crate::ui::shell::controls::height(app);
        Rect::from_min_max(rect.min, pos2(rect.max.x, (rect.max.y - bar).max(rect.min.y + 120.0)))
    } else {
        rect
    };
    let l = layout(lay, ui.ctx().content_rect().center().x);

    // Backdrop: the art again, centred behind it and covering the area,
    // blurred and dimmed.
    if let Some(bd) = cover.as_deref().and_then(|c| art::blurred(ui.ctx(), c)) {
        let cx = l.art.center().x;
        let half_w = (cx - rect.min.x).max(rect.max.x - cx);
        let side = (2.0 * half_w).max(rect.height()) + 32.0;
        let bg = Rect::from_center_size(pos2(cx, rect.center().y), vec2(side, side));
        let tint = if p.dark { Color32::from_gray((bd.tint_for(0.16) * 255.0) as u8) } else { Color32::WHITE };
        let full = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        art::paint_texture(&painter, bg, bd.texture.id(), full, CornerRadius::ZERO, tint);
        if !p.dark {
            painter.rect_filled(rect, 0, with_alpha(p.bg_b1, 0.74));
        }
    }

    let waveform = app.waveform(&item.source);
    let energy = waveform.as_deref().map(|w| energy_at(w, progress(app)));
    visualiser::show(ui, &painter, rect, l.art, app, &mut st.music.visualiser, energy, &p);

    // The art with the design's faint glow around it.
    let glow = if p.dark { with_alpha(Color32::WHITE, 0.03) } else { with_alpha(Color32::BLACK, 0.06) };
    let shadow = Shadow {
        offset: [0, 0],
        blur: (90.0 * l.scale).clamp(12.0, 90.0) as u8,
        spread: (64.0 * l.scale).clamp(6.0, 64.0) as u8,
        color: glow,
    };
    painter.add(shadow.as_shape(l.art, CornerRadius::same(l.radius)));
    if !art::paint_art(ui, l.art, cover.as_deref(), l.radius) {
        art::placeholder(ui, l.art, l.radius, &p);
    }

    let title = art::display_title(app, &item);
    let color = if p.dark { Color32::WHITE } else { p.fg1 };
    widgets::text_clipped(
        ui,
        pos2(l.art.center().x, l.title_top),
        Align2::CENTER_TOP,
        &title,
        font::medium(l.font_size),
        color,
        (rect.width() - 48.0).max(40.0),
    );
}

fn progress(app: &App) -> f32 {
    let len = app.status.length_ms;
    if len > 0 { (app.status.time_ms as f32 / len as f32).clamp(0.0, 1.0) } else { app.status.position.clamp(0.0, 1.0) }
}

/// Loudness of the track around `frac`, from the waveform peaks.
fn energy_at(peaks: &[f32], frac: f32) -> f32 {
    if peaks.is_empty() {
        return 0.8;
    }
    let x = frac * (peaks.len() - 1) as f32;
    let i = x.floor() as usize;
    let j = (i + 1).min(peaks.len() - 1);
    let t = x - i as f32;
    peaks[i] * (1.0 - t) + peaks[j] * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_matches_the_design_at_1080p() {
        // The player area of the design frame: below the title bar, above the controls.
        let area = Rect::from_min_size(pos2(0.0, 40.0), vec2(1920.0, 986.0));
        let l = layout(area, 960.0);
        assert!((l.art.width() - 738.0).abs() < 2.0, "{:?}", l.art);
        assert!((l.art.min.x - 591.0).abs() < 2.0, "{:?}", l.art);
        assert!((l.art.min.y - 113.0).abs() < 2.0, "{:?}", l.art);
        assert!((l.title_top - 924.0).abs() < 2.0, "{}", l.title_top);
        assert_eq!(l.radius, 20);
    }

    #[test]
    fn layout_fits_picture_in_picture() {
        let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(480.0, 300.0));
        let l = layout(area, 240.0);
        assert!(area.contains_rect(l.art));
        assert!(l.title_top + l.font_size < area.max.y);
        assert!(l.font_size >= 13.0);
    }

    #[test]
    fn layout_stays_centred_on_the_window_beside_the_library_column() {
        // Rail (50) and library column (377) on the left, as in 4778:25123.
        let area = Rect::from_min_max(pos2(427.0, 40.0), pos2(1920.0, 1026.0));
        let l = layout(area, 960.0);
        assert!((l.art.min.x - 591.0).abs() < 2.0, "{:?}", l.art);
        // A narrow area pulls the art inside it.
        let narrow = Rect::from_min_max(pos2(1000.0, 40.0), pos2(1920.0, 1026.0));
        assert!(narrow.contains_rect(layout(narrow, 960.0).art));
    }

    #[test]
    fn energy_interpolates_between_peaks() {
        assert_eq!(energy_at(&[], 0.5), 0.8);
        assert!((energy_at(&[0.0, 1.0], 0.5) - 0.5).abs() < 1e-6);
        assert_eq!(energy_at(&[0.2, 0.4, 0.6], 1.0), 0.6);
    }
}
