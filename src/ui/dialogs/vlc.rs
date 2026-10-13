//! VLC's own prompts (libvlc dialog callbacks): login, question, progress
//! and error. The front of `App::dialogs` is shown; replies go back to VLC.

use egui::{RichText, vec2};

use super::frame::{self, Btn, Part};
use crate::app::App;
use crate::app::ports::{DialogReply, DialogRequest};
use crate::ui::theme::font;
use crate::ui::widgets::{self, pal};

/// Fields of the login prompt being answered.
#[derive(Default)]
pub struct LoginState {
    id: Option<u64>,
    user: String,
    pass: String,
    store: bool,
}

pub fn show(ctx: &egui::Context, app: &mut App, st: &mut LoginState) {
    let Some(req) = app.dialogs.front().cloned() else { return };
    let size = vec2(560.0, 0.0);
    match req {
        DialogRequest::Error { title, text } => {
            let mut ok = false;
            let out = frame::modal(ctx, "vlc-error", title_or(&title, "Error"), size, true, |ui, part| match part {
                Part::Body => {
                    let p = pal(ui.ctx());
                    ui.horizontal_top(|ui| {
                        widgets::icon(ui, crate::ui::icons::Icon::Info, 24.0, p.danger);
                        ui.add(egui::Label::new(RichText::new(&text).font(font::regular(14.0)).color(p.fg2)).wrap());
                    });
                }
                _ => {
                    frame::right(ui, |ui| {
                        if frame::btn(ui, "OK", Btn::Accent, true).clicked() {
                            ok = true;
                        }
                    });
                }
            });
            if ok || out.close {
                app.dismiss_error();
            }
        }
        DialogRequest::Login { id, title, text, default_username, ask_store } => {
            if st.id != Some(id) {
                *st = LoginState { id: Some(id), user: default_username, pass: String::new(), store: false };
            }
            let mut reply = None;
            let out = frame::modal(ctx, "vlc-login", title_or(&title, "Login"), size, true, |ui, part| match part {
                Part::Body => {
                    let p = pal(ui.ctx());
                    if !text.is_empty() {
                        ui.add(egui::Label::new(RichText::new(&text).font(font::regular(14.0)).color(p.fg2)).wrap());
                        ui.add_space(4.0);
                    }
                    let w = ui.available_width();
                    frame::label(ui, "Username");
                    let r = frame::field(ui, &mut st.user, "Username", w);
                    if st.user.is_empty() && !r.has_focus() && ui.memory(|m| m.focused().is_none()) {
                        r.request_focus();
                    }
                    frame::label(ui, "Password");
                    let pr = frame::secret(ui, &mut st.pass, "Password", w);
                    if pr.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        reply = Some(true);
                    }
                    if ask_store {
                        ui.horizontal(|ui| {
                            widgets::toggle(ui, &mut st.store);
                            frame::label(ui, "Remember");
                        });
                    }
                }
                _ => {
                    if frame::cancel(ui) {
                        reply = Some(false);
                    }
                    frame::right(ui, |ui| {
                        if frame::btn(ui, "Log in", Btn::Accent, true).clicked() {
                            reply = Some(true);
                        }
                    });
                }
            });
            match reply {
                Some(true) => {
                    let r = DialogReply::Login {
                        username: st.user.clone(),
                        password: std::mem::take(&mut st.pass),
                        store: st.store,
                    };
                    app.reply_dialog(id, r);
                    st.id = None;
                }
                Some(false) => {
                    app.reply_dialog(id, DialogReply::Dismiss);
                    st.id = None;
                }
                None if out.close => {
                    app.reply_dialog(id, DialogReply::Dismiss);
                    st.id = None;
                }
                None => {}
            }
        }
        DialogRequest::Question { id, title, text, cancel, action1, action2 } => {
            let mut reply = None;
            let out =
                frame::modal(ctx, "vlc-question", title_or(&title, "Question"), size, true, |ui, part| match part {
                    Part::Body => {
                        let p = pal(ui.ctx());
                        ui.add(egui::Label::new(RichText::new(&text).font(font::regular(14.0)).color(p.fg2)).wrap());
                    }
                    _ => {
                        let cancel_label = if cancel.is_empty() { "Cancel" } else { cancel.as_str() };
                        if frame::btn(ui, cancel_label, Btn::Plain, true).clicked() {
                            reply = Some(DialogReply::Dismiss);
                        }
                        frame::right(ui, |ui| {
                            // Right-to-left: the first action ends up rightmost.
                            if let Some(a) = &action1
                                && frame::btn(ui, a, Btn::Accent, true).clicked()
                            {
                                reply = Some(DialogReply::Action(1));
                            }
                            if let Some(a) = &action2
                                && frame::btn(ui, a, Btn::Plain, true).clicked()
                            {
                                reply = Some(DialogReply::Action(2));
                            }
                        });
                    }
                });
            if out.close && reply.is_none() {
                reply = Some(DialogReply::Dismiss);
            }
            if let Some(r) = reply {
                app.reply_dialog(id, r);
            }
        }
        DialogRequest::Progress { id, title, text, position, cancel } => {
            let mut dismiss = false;
            let has_cancel = cancel.is_some();
            frame::modal(ctx, "vlc-progress", title_or(&title, "Working"), size, has_cancel, |ui, part| match part {
                Part::Body => {
                    let p = pal(ui.ctx());
                    if !text.is_empty() {
                        ui.add(egui::Label::new(RichText::new(&text).font(font::regular(14.0)).color(p.fg2)).wrap());
                    }
                    let w = ui.available_width();
                    frame::progress(ui, position, w);
                }
                _ => {
                    if let Some(c) = &cancel
                        && frame::btn(ui, if c.is_empty() { "Cancel" } else { c }, Btn::Plain, true).clicked()
                    {
                        dismiss = true;
                    }
                }
            });
            if dismiss {
                app.reply_dialog(id, DialogReply::Dismiss);
            }
        }
        // Updates and cancels are folded into the queue by `App::tick`;
        // drop any that reach the front.
        DialogRequest::UpdateProgress { .. } | DialogRequest::Cancel { .. } => {
            app.dialogs.pop_front();
        }
    }
}

fn title_or<'a>(title: &'a str, fallback: &'a str) -> &'a str {
    if title.trim().is_empty() { fallback } else { title }
}
