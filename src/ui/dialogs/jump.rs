//! Jump to Time: an HH:MM:SS field; Enter seeks.

use egui::{RichText, vec2};

use super::frame::{self, Btn, Part};
use crate::app::App;
use crate::domain::time::{format_hms, parse_clock};
use crate::ui::theme::font;
use crate::ui::widgets::pal;

#[derive(Default)]
pub struct JumpState {
    text: String,
    /// Whether the field was filled for this opening.
    primed: bool,
}

pub fn show(ctx: &egui::Context, app: &mut App, open: &mut bool, st: &mut JumpState) {
    if !st.primed {
        st.text = format_hms(app.status.time_ms);
        st.primed = true;
    }
    let length = app.status.length_ms;
    let parsed = parse_clock(&st.text);
    let valid = parsed.is_some_and(|ms| length <= 0 || ms <= length);
    let mut go = false;
    let mut cancel = false;
    let out = frame::modal(ctx, "jump-to-time", "Jump to Time", vec2(480.0, 0.0), true, |ui, part| match part {
        Part::Body => {
            let p = pal(ui.ctx());
            frame::heading(ui, "Go to time", "Hours, minutes and seconds (HH:MM:SS), or MM:SS, or seconds.");
            ui.horizontal(|ui| {
                let r = frame::field(ui, &mut st.text, "00:00:00", 200.0);
                if !r.has_focus() && ui.memory(|m| m.focused().is_none()) {
                    r.request_focus();
                }
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    go = true;
                }
                if length > 0 {
                    ui.label(RichText::new(format!("of {}", format_hms(length))).font(font::mono(14.0)).color(p.fg_a3));
                }
            });
            if !st.text.trim().is_empty() && !valid {
                ui.label(
                    RichText::new(if parsed.is_some() { "That is past the end." } else { "Use HH:MM:SS." })
                        .font(font::regular(13.0))
                        .color(p.danger),
                );
            }
            if !app.status.seekable && app.has_media() {
                frame::note(ui, "This media cannot be seeked.");
            }
        }
        _ => {
            if frame::cancel(ui) {
                cancel = true;
            }
            frame::right(ui, |ui| {
                if frame::btn(ui, "Go", Btn::Accent, valid).clicked() {
                    go = true;
                }
            });
        }
    });
    if go && let Some(ms) = parsed.filter(|_| valid) {
        app.seek_ms(ms);
        cancel = true;
    }
    if cancel || out.close {
        *open = false;
        st.primed = false;
    }
}
