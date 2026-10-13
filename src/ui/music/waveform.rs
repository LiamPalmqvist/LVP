//! The music seek bar (control bar variants "waveform" and "music"): thin
//! vertical bars across the track showing its loudness, the played part in
//! the accent, and a slim thumb.

use egui::{Rect, Sense, Ui, pos2, vec2};

use crate::app::App;
use crate::app::ports::PlayerState;
use crate::domain::time::format_hms;
use crate::ui::state::UiState;
use crate::ui::theme::{Palette, font, mix};
use crate::ui::widgets::pal;

const PITCH: f32 = 5.0;
const BAR_W: f32 = 2.0;
const INSET: f32 = 6.0;
const MAX_H: f32 = 14.0;
const MIN_H: f32 = 2.0;

/// Peak of `peaks` over bar `j` of `n` (max over the slice, or linear
/// interpolation when there are more bars than peaks).
pub fn bar_level(peaks: &[f32], j: usize, n: usize) -> f32 {
    if peaks.is_empty() || n == 0 {
        return 0.0;
    }
    let len = peaks.len();
    if n >= len {
        let x = if n > 1 { j as f32 * (len - 1) as f32 / (n - 1) as f32 } else { 0.0 };
        let i = x.floor() as usize;
        let k = (i + 1).min(len - 1);
        let t = x - i as f32;
        return peaks[i.min(len - 1)] * (1.0 - t) + peaks[k] * t;
    }
    let a = j * len / n;
    let b = ((j + 1) * len / n).max(a + 1).min(len);
    peaks[a..b].iter().cloned().fold(0.0, f32::max)
}

/// Draws the waveform seek bar in `rect` when the current item is audio and
/// its waveform is known. Returns false (drawing nothing) otherwise, so the
/// caller can fall back to the plain bar.
pub fn draw_waveform_seek(ui: &mut Ui, rect: Rect, app: &mut App, st: &mut UiState) -> bool {
    let Some(item) = app.queue.current_item() else { return false };
    if !item.is_audio() {
        return false;
    }
    let source = item.source.clone();
    let Some(peaks) = app.waveform(&source) else { return false };
    let p = pal(ui.ctx());
    let bar = Rect::from_center_size(rect.center(), vec2(rect.width(), 28.0));
    let resp = ui.interact(bar, ui.id().with("seek"), Sense::click_and_drag());
    let len = app.status.length_ms;
    let seekable = app.status.seekable || len > 0;
    let inner = bar.shrink2(vec2(INSET, 0.0));
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
            time_bubble(ui, bar, pos.x, t, &p);
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
    painter.rect_filled(bar, 6, p.bg_b2);
    // A-B loop region, as on the plain bar.
    if let (Some(a), Some(b)) = (app.ab_loop.a, app.ab_loop.b)
        && len > 0
    {
        let xa = inner.min.x + inner.width() * a as f32 / len as f32;
        let xb = inner.min.x + inner.width() * b as f32 / len as f32;
        let r = Rect::from_min_max(pos2(xa, bar.min.y + 2.0), pos2(xb, bar.max.y - 2.0));
        painter.rect_filled(r, 4, mix(p.bg_b2, p.accent, 0.12));
    }
    let n = ((inner.width() - BAR_W) / PITCH).floor().max(0.0) as usize + 1;
    let played_x = inner.min.x + inner.width() * frac;
    let unplayed = if p.dark { p.bg_a1 } else { p.bg_a4 };
    let cy = bar.center().y;
    for j in 0..n {
        let x = inner.min.x + j as f32 * PITCH;
        let h = (MIN_H + bar_level(&peaks, j, n) * (MAX_H - MIN_H)).round();
        let r = Rect::from_min_size(pos2(x, cy - h / 2.0), vec2(BAR_W, h));
        painter.rect_filled(r, 4, if x + BAR_W <= played_x { p.accent } else { unplayed });
    }
    if seekable && (len > 0 || app.status.state == PlayerState::Playing) {
        let active = resp.hovered() || resp.dragged();
        let thumb = Rect::from_center_size(pos2(played_x.max(inner.min.x + 2.0), cy), vec2(4.0, 24.0));
        painter.rect_filled(thumb, 4, if active { p.accent_bright } else { p.accent });
    }
    true
}

/// The hover time above the bar, matching the plain seek bar's bubble.
fn time_bubble(ui: &Ui, bar: Rect, x: f32, t: i64, p: &Palette) {
    let layer = egui::LayerId::new(egui::Order::Tooltip, ui.id().with("seek-bubble"));
    let lp = ui.ctx().layer_painter(layer);
    let galley = lp.layout_no_wrap(format_hms(t), font::regular(13.0), p.fg1);
    let bw = galley.size().x + 16.0;
    let bx = x.clamp(bar.min.x + bw / 2.0, bar.max.x - bw / 2.0);
    let bubble = Rect::from_center_size(pos2(bx, bar.min.y - 20.0), vec2(bw, 24.0));
    lp.rect_filled(bubble, 6, p.bg_a1);
    lp.galley(bubble.center() - galley.size() / 2.0, galley, p.fg1);
}

#[cfg(test)]
mod tests {
    use super::bar_level;

    #[test]
    fn bars_take_the_loudest_peak_of_their_slice() {
        let peaks = [0.1, 0.9, 0.2, 0.3];
        assert_eq!(bar_level(&peaks, 0, 2), 0.9);
        assert_eq!(bar_level(&peaks, 1, 2), 0.3);
    }

    #[test]
    fn more_bars_than_peaks_interpolate() {
        let peaks = [0.0, 1.0];
        assert_eq!(bar_level(&peaks, 0, 3), 0.0);
        assert!((bar_level(&peaks, 1, 3) - 0.5).abs() < 1e-6);
        assert_eq!(bar_level(&peaks, 2, 3), 1.0);
        assert_eq!(bar_level(&[], 0, 3), 0.0);
    }
}
