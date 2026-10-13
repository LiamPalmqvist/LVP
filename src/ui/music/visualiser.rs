//! A lightweight bar visualiser behind the cover art, drawn in the style of
//! the design's waveform seek bar (2pt rounded bars on a 5pt pitch).
//!
//! libvlc 3 hands out no audio samples without replacing its audio output,
//! so the bars follow a smooth pseudo-signal instead, scaled by the track's
//! loudness at the playhead when its waveform is known. They settle when
//! playback pauses and stop repainting once still.

use std::f64::consts::TAU;
use std::time::Duration;

use egui::{Painter, Rect, Ui, pos2, vec2};

use crate::app::App;
use crate::app::ports::PlayerState;
use crate::ui::theme::{Palette, with_alpha};

const PITCH: f32 = 5.0;
const BAR_W: f32 = 2.0;
/// Repaint interval while animating (about 30 frames a second).
const FRAME: Duration = Duration::from_millis(33);

#[derive(Default)]
pub struct Visualiser {
    levels: Vec<f32>,
    /// Signal time, advanced only while playing.
    phase: f64,
    last: Option<f64>,
}

/// Deterministic value noise in [0, 1] for bar `i` at integer step `k`.
fn hash(i: u64, k: u64) -> f32 {
    let mut x = i.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ k.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    x ^= x >> 31;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 29;
    (x >> 40) as f32 / (1u64 << 24) as f32
}

/// Smoothly interpolated noise for bar `i` at time `t` (`rate` steps a second).
fn noise(i: usize, t: f64, rate: f64) -> f32 {
    let s = t * rate;
    let k = s.floor();
    let f = (s - k) as f32;
    let f = f * f * (3.0 - 2.0 * f);
    let (a, b) = (hash(i as u64, k as u64), hash(i as u64, k as u64 + 1));
    a + (b - a) * f
}

/// The pseudo-signal for bar `i` (0 at the centre) at time `t`, in [0, 1].
/// `d` is the bar's distance from the centre as a fraction of the half-width:
/// the inner bars play the role of bass and move the most.
pub fn signal(i: usize, t: f64, d: f32) -> f32 {
    let x = i as f64;
    let beat = ((t * TAU * 2.0).sin().max(0.0) as f32).powi(6);
    let w1 = ((t * 1.9 + x * 0.23).sin() * 0.5 + 0.5) as f32;
    let w2 = ((t * 3.1 - x * 0.41 + 1.7).sin() * 0.5 + 0.5) as f32;
    let w3 = ((t * 0.7 + x * 0.07 + 0.4).sin() * 0.5 + 0.5) as f32;
    let n = noise(i, t, 6.0);
    let body = 0.3 * w1 + 0.2 * w2 + 0.15 * w3 + 0.35 * n;
    let env = (1.0 - d).max(0.0).powf(1.4) * 0.8 + 0.2;
    let kick = beat * (1.0 - d).max(0.0).powi(3) * 0.45;
    ((body * (0.7 + 0.3 * beat)) * env + kick).clamp(0.0, 1.0)
}

/// Draws the bars across `area`, mirrored about the art's centre line.
#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &Ui,
    painter: &Painter,
    area: Rect,
    art: Rect,
    app: &App,
    v: &mut Visualiser,
    energy: Option<f32>,
    p: &Palette,
) {
    let playing = app.status.state == PlayerState::Playing;
    let now = ui.input(|i| i.time);
    let dt = v.last.map(|l| (now - l).clamp(0.0, 0.1)).unwrap_or(0.0) as f32;
    v.last = Some(now);
    if playing {
        v.phase += dt as f64 * app.status.rate.clamp(0.25, 4.0) as f64;
    }
    let half = ((area.width() / 2.0) / PITCH).floor().max(1.0) as usize;
    v.levels.resize(half, 0.0);
    let energy = energy.map(|e| 0.35 + 0.65 * e.clamp(0.0, 1.0)).unwrap_or(0.85);
    let max_h = (art.height() * 0.42).max(12.0);
    let cy = art.center().y;
    let cx = area.center().x;
    // Fast attack, slower release, independent of the frame rate.
    let (attack, release) = (1.0 - (-dt * 18.0).exp(), 1.0 - (-dt * 6.0).exp());
    let hidden = art.expand(2.0);
    let mut moving = false;
    for (i, level) in v.levels.iter_mut().enumerate() {
        let d = i as f32 / half as f32;
        let target = if playing { signal(i, v.phase, d) * energy } else { 0.0 };
        let k = if target > *level { attack } else { release };
        *level += (target - *level) * k;
        if (*level - target).abs() > 0.003 {
            moving = true;
        }
        let h = 2.0 + *level * max_h;
        let color = with_alpha(p.accent, 0.5 - 0.35 * d);
        for side in [-1.0f32, 1.0] {
            let x = cx + side * (i as f32 * PITCH + PITCH / 2.0);
            let r = Rect::from_center_size(pos2(x, cy), vec2(BAR_W, h));
            if hidden.contains_rect(r) || !area.contains_rect(r) {
                continue;
            }
            painter.rect_filled(r, 1, color);
        }
    }
    if playing || moving {
        ui.ctx().request_repaint_after(FRAME);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_stays_in_range_and_moves() {
        let mut changed = false;
        for i in 0..200 {
            let d = i as f32 / 200.0;
            let a = signal(i, 1.0, d);
            let b = signal(i, 1.2, d);
            assert!((0.0..=1.0).contains(&a) && (0.0..=1.0).contains(&b));
            changed |= (a - b).abs() > 0.01;
        }
        assert!(changed);
    }

    #[test]
    fn signal_is_smooth_between_frames() {
        // One 30 fps frame apart, no bar jumps by more than a fraction of its range.
        for i in 0..100 {
            let d = i as f32 / 100.0;
            let a = signal(i, 3.0, d);
            let b = signal(i, 3.0 + 1.0 / 30.0, d);
            assert!((a - b).abs() < 0.25, "bar {i}: {a} -> {b}");
        }
    }

    #[test]
    fn noise_is_deterministic() {
        assert_eq!(noise(3, 1.5, 6.0), noise(3, 1.5, 6.0));
        assert!((0.0..=1.0).contains(&hash(7, 9)));
    }
}
