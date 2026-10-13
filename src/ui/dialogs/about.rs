//! About: logo, versions, credits and licence.

use egui::{Align, Layout, RichText, vec2};

use super::frame::{self, Btn, Part};
use crate::app::App;
use crate::ui::icons::Icon;
use crate::ui::theme::font;
use crate::ui::widgets::{self, pal};

const CREDITS: &[(&str, &str)] = &[
    ("VLC and libvlc", "VideoLAN and the VLC authors. NotVLC plays everything through VLC's engine."),
    ("Inter", "Rasmus Andersson, SIL Open Font License 1.1."),
    ("DM Mono", "Colophon Foundry for Google, SIL Open Font License 1.1."),
    ("Design", "\"VLC Redesign (Community)\" on the Figma Community, whose screens and icons NotVLC follows."),
    ("egui", "Emil Ernerfeldt and contributors, MIT or Apache 2.0."),
];

pub fn show(ctx: &egui::Context, app: &App, open: &mut bool) {
    let vlc = app.engine.version();
    let mut close = false;
    let out = frame::modal(ctx, "about", "About NotVLC", vec2(640.0, 0.0), true, |ui, part| match part {
        Part::Body => {
            let p = pal(ui.ctx());
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                widgets::icon(ui, Icon::Vlc, 72.0, p.accent_logo);
                ui.add_space(4.0);
                ui.label(RichText::new("NotVLC").font(font::light(32.0)).color(p.fg1));
                ui.label(
                    RichText::new(format!("Version {}  ·  libvlc {}", env!("CARGO_PKG_VERSION"), vlc))
                        .font(font::mono(13.0))
                        .color(p.fg_a2),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(
                        "A media player built on VLC's engine, drawn after the VLC Redesign community design.",
                    )
                    .font(font::light(15.0))
                    .color(p.fg_a1),
                );
            });
            ui.add_space(6.0);
            frame::rule(ui);
            ui.add_space(2.0);
            for (name, text) in CREDITS {
                ui.horizontal_top(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(170.0, 18.0), egui::Sense::hover());
                    ui.painter().text(r.left_center(), egui::Align2::LEFT_CENTER, *name, font::medium(14.0), p.fg2);
                    ui.add(egui::Label::new(RichText::new(*text).font(font::regular(13.0)).color(p.fg_a2)).wrap());
                });
            }
            ui.add_space(2.0);
            frame::rule(ui);
            frame::note(
                ui,
                "NotVLC is free software under the GNU General Public License, version 2 or later. VLC and \
                 VideoLAN are trademarks of VideoLAN; NotVLC is not affiliated with VideoLAN.",
            );
        }
        Part::Footer => {
            frame::right(ui, |ui| {
                if frame::btn(ui, "Close", Btn::Accent, true).clicked() {
                    close = true;
                }
                if frame::btn(ui, "videolan.org", Btn::Plain, true).clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab("https://www.videolan.org/vlc/"));
                }
            });
        }
    });
    if close || out.close {
        *open = false;
    }
}
