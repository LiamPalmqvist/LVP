//! Open Capture Device (macOS): the default camera, the default microphone,
//! or the screen, through VLC's AVFoundation and screen modules.

use egui::{RichText, vec2};

use super::frame::{self, Btn, Part};
use crate::app::App;
use crate::domain::disc::{CaptureKind, CaptureRequest};
use crate::domain::media::{MediaItem, MediaSource};
use crate::ui::state::{UiState, View};
use crate::ui::theme::font;
use crate::ui::widgets::{self, pal};

#[derive(Default)]
pub struct CaptureState {
    req: CaptureRequest,
}

pub fn show(ctx: &egui::Context, app: &mut App, st: &mut UiState) {
    let cs = &mut st.dialogs.capture_state;
    let mut action: Option<u8> = None;
    let mut cancel = false;
    let out =
        frame::modal(ctx, "open-capture", "Open Capture Device", vec2(1048.0, 560.0), true, |ui, part| match part {
            Part::Body => {
                let p = pal(ui.ctx());
                frame::heading(ui, "Capture mode", "");
                let mut i = CaptureKind::ALL.iter().position(|k| *k == cs.req.kind).unwrap_or(0);
                let labels: Vec<&str> = CaptureKind::ALL.iter().map(|k| k.label()).collect();
                if widgets::segmented(ui, &mut i, &labels, 420.0) {
                    cs.req.kind = CaptureKind::ALL[i];
                }
                ui.add_space(6.0);
                match cs.req.kind {
                    CaptureKind::Camera => {
                        frame::heading(
                            ui,
                            "Camera",
                            "VLC opens the system's default camera (avcapture). For another camera, enter its \
                         AVFoundation device ID.",
                        );
                        frame::field(ui, &mut cs.req.device, "Default camera", 420.0);
                        ui.horizontal(|ui| {
                            widgets::toggle(ui, &mut cs.req.with_audio);
                            ui.label(
                                RichText::new("Also capture the microphone").font(font::regular(14.0)).color(p.fg2),
                            );
                        });
                    }
                    CaptureKind::Microphone => {
                        frame::heading(
                            ui,
                            "Microphone",
                            "VLC records the system's default audio input (avaudiocapture), or the device whose \
                         AVFoundation ID you enter.",
                        );
                        frame::field(ui, &mut cs.req.device, "Default microphone", 420.0);
                    }
                    CaptureKind::Screen => {
                        frame::heading(
                            ui,
                            "Screen",
                            "Captures the main display. macOS asks for Screen Recording permission the first time.",
                        );
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 20.0;
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 6.0;
                                frame::label(ui, "Frame rate");
                                let mut fps = cs.req.fps as f64;
                                frame::number(ui, &mut fps, 1.0..=60.0, "fps", 150.0);
                                cs.req.fps = fps as f32;
                            });
                            let region = |ui: &mut egui::Ui, label: &str, v: &mut u32| {
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 6.0;
                                    frame::label(ui, label);
                                    let mut f = *v as f64;
                                    frame::number(ui, &mut f, 0.0..=16_384.0, "px", 130.0);
                                    *v = f.round() as u32;
                                });
                            };
                            region(ui, "Left", &mut cs.req.left);
                            region(ui, "Top", &mut cs.req.top);
                            region(ui, "Width", &mut cs.req.width);
                            region(ui, "Height", &mut cs.req.height);
                        });
                        frame::note(ui, "Leave width or height at 0 to capture the whole screen.");
                    }
                }
                ui.add_space(6.0);
                frame::rule(ui);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    frame::label(ui, "Caching").on_hover_text("live-caching: how much VLC buffers from the device.");
                    let mut c = cs.req.caching_ms as f64;
                    frame::number(ui, &mut c, 0.0..=10_000.0, "ms", 150.0);
                    cs.req.caching_ms = c.round() as u32;
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("{} {}", cs.req.mrl(), cs.req.options().join(" ")))
                        .font(font::mono(13.0))
                        .color(p.fg_a3),
                );
                frame::note(
                    ui,
                    "TV tuner (DVB) and Video4Linux capture are only available in VLC on Linux and Windows.",
                );
            }
            Part::Footer => {
                if frame::cancel(ui) {
                    cancel = true;
                }
                frame::right(ui, |ui| {
                    if frame::btn(ui, "Play", Btn::Accent, true).clicked() {
                        action = Some(0);
                    }
                    if frame::btn(ui, "Convert", Btn::Plain, true).clicked() {
                        action = Some(2);
                    }
                });
            }
        });
    if let Some(a) = action {
        let mut item = MediaItem::new(MediaSource::Location(cs.req.mrl()));
        item.options = cs.req.options();
        item.meta.title = Some(cs.req.kind.label().to_string());
        st.dialogs.capture = false;
        if a == 0 {
            app.play_items(vec![item], 0, None);
            st.view = View::Player;
        } else {
            st.dialogs.convert_items(vec![item]);
        }
        return;
    }
    if cancel || out.close {
        st.dialogs.capture = false;
    }
}
