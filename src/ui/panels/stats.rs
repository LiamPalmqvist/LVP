//! The "Show Statistics" overlay: VLC's input statistics over the video.

use egui::{Align2, Color32, Rect, pos2, vec2};

use crate::app::App;
use crate::ui::theme::{font, with_alpha};
use crate::ui::widgets::pal;

fn bytes(n: i64) -> String {
    let n = n.max(0) as f64;
    if n >= 1e9 {
        format!("{:.2} GB", n / 1e9)
    } else if n >= 1e6 {
        format!("{:.1} MB", n / 1e6)
    } else {
        format!("{:.0} KB", n / 1e3)
    }
}

pub fn show(ctx: &egui::Context, video: Rect, app: &App) {
    let p = pal(ctx);
    let s = &app.status.stats;
    let st = &app.status;
    let mut lines: Vec<(&str, String)> = vec![
        ("Input bitrate", format!("{:.0} kb/s", s.input_bitrate_kbps)),
        ("Demux bitrate", format!("{:.0} kb/s", s.demux_bitrate_kbps)),
        ("Read", bytes(s.read_bytes)),
        ("Decoded video blocks", s.decoded_video.to_string()),
        ("Displayed frames", s.displayed_pictures.to_string()),
        ("Lost frames", s.lost_pictures.to_string()),
        ("Decoded audio blocks", s.decoded_audio.to_string()),
        ("Played audio buffers", s.played_abuffers.to_string()),
        ("Lost audio buffers", s.lost_abuffers.to_string()),
        ("Corrupted packets", s.demux_corrupted.to_string()),
        ("Discontinuities", s.demux_discontinuity.to_string()),
    ];
    if let Some((w, h)) = st.video_size {
        lines.insert(
            0,
            ("Video", format!("{w}x{h}{}", if st.fps > 0.0 { format!(" @ {:.2} fps", st.fps) } else { String::new() })),
        );
    }
    let row_h = 20.0;
    let size = vec2(300.0, 16.0 + row_h * lines.len() as f32);
    let rect = Rect::from_min_size(video.min + vec2(24.0, 72.0), size);
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::new("player-stats")));
    painter.rect_filled(rect, 8, with_alpha(Color32::BLACK, 0.7));
    for (i, (k, v)) in lines.iter().enumerate() {
        let y = rect.min.y + 8.0 + row_h * (i as f32 + 0.5);
        painter.text(pos2(rect.min.x + 12.0, y), Align2::LEFT_CENTER, *k, font::regular(13.0), p.fg_a1);
        painter.text(pos2(rect.max.x - 12.0, y), Align2::RIGHT_CENTER, v, font::mono(13.0), p.fg1);
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(500));
}
