//! The shared modal look: dimmed backdrop, a centred BG/B1 95% card with
//! radius 14 and the design's shadow, a 50pt header with the title and a
//! close button, the body, and a 72pt footer with Cancel on the left and
//! the primary action on the right (Open Network Stream, `6216:13614`).

use egui::{Align, Align2, Color32, CornerRadius, Id, Layout, Rect, Response, RichText, Sense, Stroke, Ui, Vec2, vec2};

use crate::ui::icons::Icon;
use crate::ui::theme::{font, with_alpha};
use crate::ui::widgets::{self, pal};

pub const HEADER_H: f32 = 50.0;
pub const FOOTER_H: f32 = 72.0;
pub const PAD_X: f32 = 20.0;
pub const PAD_TOP: f32 = 22.0;

/// Which part of the dialog the content closure is asked to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Body,
    Footer,
}

/// What happened to the dialog this frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct ModalOut {
    /// The close button or Esc.
    pub close: bool,
}

/// Draws a modal dialog. `size.y == 0` lets the body decide the height.
/// The closure is called once for the body and once for the footer (when
/// `footer` is true), so a single closure can borrow the dialog's state.
pub fn modal(
    ctx: &egui::Context,
    id: &str,
    title: &str,
    size: Vec2,
    footer: bool,
    mut add: impl FnMut(&mut Ui, Part),
) -> ModalOut {
    let p = pal(ctx);
    let screen = ctx.content_rect();
    let w = size.x.min(screen.width() - 32.0).max(280.0);
    let h = if size.y > 0.0 { size.y.min(screen.height() - 32.0).max(200.0) } else { 0.0 };
    let frame = egui::Frame::new()
        .fill(p.bg_b1_t)
        .corner_radius(14)
        .stroke(Stroke::new(1.0, with_alpha(p.divider_dim, 0.6)))
        .shadow(egui::epaint::Shadow { offset: [0, 8], blur: 30, spread: 0, color: Color32::from_black_alpha(118) });
    let mut out = ModalOut::default();
    let resp = egui::Modal::new(Id::new(("notvlc-dialog", id)))
        .backdrop_color(Color32::from_black_alpha(if p.dark { 150 } else { 90 }))
        .frame(frame)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
            ui.set_width(w);
            if header(ui, title, w) {
                out.close = true;
            }
            let body_h = if h > 0.0 { h - HEADER_H - if footer { FOOTER_H } else { 0.0 } } else { 0.0 };
            let body_rect_min = ui.cursor().min;
            ui.allocate_ui_with_layout(
                vec2(w, if body_h > 0.0 { body_h } else { 0.0 }),
                Layout::top_down(Align::Min),
                |ui| {
                    egui::Frame::new()
                        .inner_margin(egui::Margin {
                            left: PAD_X as i8,
                            right: PAD_X as i8,
                            top: PAD_TOP as i8,
                            bottom: if footer { 0 } else { PAD_TOP as i8 },
                        })
                        .show(ui, |ui| {
                            ui.set_width(w - 2.0 * PAD_X);
                            if body_h > 0.0 {
                                ui.set_max_height(body_h - PAD_TOP);
                            }
                            add(ui, Part::Body);
                        });
                },
            );
            if body_h > 0.0 {
                // Keep the footer pinned to the bottom of a fixed-size dialog.
                let used = ui.cursor().min.y - body_rect_min.y;
                if used < body_h {
                    ui.add_space(body_h - used - ui.spacing().item_spacing.y);
                }
            }
            if footer {
                let (rect, _) = ui.allocate_exact_size(vec2(w, FOOTER_H), Sense::hover());
                let inner = Rect::from_min_size(rect.min + vec2(PAD_X, 20.0), vec2(w - 2.0 * PAD_X, 32.0));
                let mut f =
                    ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(Layout::left_to_right(Align::Center)));
                f.spacing_mut().item_spacing.x = 10.0;
                add(&mut f, Part::Footer);
            }
        });
    if resp.is_top_modal
        && !resp.any_popup_open
        && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        out.close = true;
    }
    out
}

/// The black title strip with the close button. Returns true when closed.
fn header(ui: &mut Ui, title: &str, w: f32) -> bool {
    let p = pal(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(w, HEADER_H), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius { nw: 14, ne: 14, sw: 0, se: 0 }, p.bg_b2);
    ui.painter().text(rect.left_center() + vec2(19.0, 0.0), Align2::LEFT_CENTER, title, font::regular(14.0), p.fg_a2);
    let close = Rect::from_min_size(rect.right_top() + vec2(-41.0, 9.0), vec2(32.0, 32.0));
    let mut c = ui.new_child(egui::UiBuilder::new().max_rect(close));
    widgets::icon_button_sized(&mut c, Icon::Close, vec2(32.0, 32.0), 24.0, false, "Close").clicked()
}

/// Footer and inline buttons in the dialog style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Btn {
    /// BG/A3 fill, FG/A2 text (Cancel, Browse).
    Plain,
    /// BG/Accent fill with accent text (Save, Play).
    Accent,
    /// Accent fill with dark text (the onboarding's Next).
    Solid,
}

pub fn btn(ui: &mut Ui, label: &str, kind: Btn, enabled: bool) -> Response {
    btn_sized(ui, label, kind, enabled, vec2(100.0, 32.0), font::semibold(14.0))
}

pub fn btn_sized(ui: &mut Ui, label: &str, kind: Btn, enabled: bool, min: Vec2, f: egui::FontId) -> Response {
    let p = pal(ui.ctx());
    let galley = ui.painter().layout_no_wrap(label.to_string(), f, Color32::PLACEHOLDER);
    let size = vec2((galley.size().x + 32.0).max(min.x), min.y);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    let hovered = enabled && resp.hovered();
    let (bg, fg) = match kind {
        Btn::Plain => (if hovered { p.bg_a1 } else { p.bg_a3 }, if hovered { p.fg1 } else { p.fg_a2 }),
        Btn::Accent => (if hovered { with_alpha(p.accent, 0.35) } else { p.bg_accent }, p.accent),
        Btn::Solid => (if hovered { p.accent_bright } else { p.accent }, if p.dark { p.fg_b1 } else { Color32::WHITE }),
    };
    let a = if enabled { 1.0 } else { 0.5 };
    let fade = |c: Color32| with_alpha(c, a * c.a() as f32 / 255.0);
    ui.painter().rect_filled(rect, CornerRadius::same(6), fade(bg));
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, fade(fg));
    if enabled { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp }
}

/// Footer helper: Cancel on the left. Returns true when clicked.
pub fn cancel(ui: &mut Ui) -> bool {
    btn(ui, "Cancel", Btn::Plain, true).clicked()
}

/// Lays out the rest of the footer from the right edge.
pub fn right<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.with_layout(Layout::right_to_left(Align::Center), add).inner
}

/// "Network URL" style heading (Regular 16, FG/2) with an optional hint
/// line below it (Regular 14).
pub fn heading(ui: &mut Ui, title: &str, hint: &str) {
    let p = pal(ui.ctx());
    ui.spacing_mut().item_spacing.y = 5.0;
    ui.label(RichText::new(title).font(font::regular(16.0)).color(p.fg2));
    if !hint.is_empty() {
        ui.label(RichText::new(hint).font(font::regular(14.0)).color(p.fg_a3));
    }
    ui.spacing_mut().item_spacing.y = 10.0;
}

/// Small label above a field (Regular 14, FG/A2).
pub fn label(ui: &mut Ui, text: &str) {
    let p = pal(ui.ctx());
    ui.label(RichText::new(text).font(font::regular(14.0)).color(p.fg_a2));
}

/// Muted explanatory text (Regular 13, FG/A3), wrapping.
pub fn note(ui: &mut Ui, text: &str) {
    let p = pal(ui.ctx());
    ui.add(egui::Label::new(RichText::new(text).font(font::regular(13.0)).color(p.fg_a3)).wrap());
}

fn field_frame(ui: &Ui) -> egui::Frame {
    let p = pal(ui.ctx());
    egui::Frame::new()
        .fill(p.bg_b2)
        .stroke(Stroke::new(2.0, p.bg_a4))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(12, 8))
}

/// The design's input: BG/B2 with a 2pt BG/A4 border, radius 6.
pub fn field(ui: &mut Ui, text: &mut String, hint: &str, width: f32) -> Response {
    let p = pal(ui.ctx());
    field_frame(ui)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(text)
                    .hint_text(RichText::new(hint).color(p.fg_a4))
                    .frame(false)
                    .text_color(p.fg1)
                    .font(font::regular(14.0))
                    .desired_width(width - 28.0),
            )
        })
        .inner
}

/// A password variant of [`field`].
pub fn secret(ui: &mut Ui, text: &mut String, hint: &str, width: f32) -> Response {
    let p = pal(ui.ctx());
    field_frame(ui)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(text)
                    .password(true)
                    .hint_text(RichText::new(hint).color(p.fg_a4))
                    .frame(false)
                    .text_color(p.fg1)
                    .font(font::regular(14.0))
                    .desired_width(width - 28.0),
            )
        })
        .inner
}

/// A multi-line [`field`] of a fixed height.
pub fn area(ui: &mut Ui, text: &mut String, hint: &str, size: Vec2) -> Response {
    let p = pal(ui.ctx());
    field_frame(ui)
        .show(ui, |ui| {
            ui.set_min_size(size - vec2(28.0, 20.0));
            egui::ScrollArea::vertical()
                .id_salt("area")
                .max_height(size.y - 20.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(text)
                            .hint_text(RichText::new(hint).color(p.fg_a4))
                            .frame(false)
                            .text_color(p.fg1)
                            .font(font::regular(14.0))
                            .desired_width(size.x - 28.0)
                            .desired_rows(((size.y - 20.0) / 18.0).floor() as usize),
                    )
                })
                .inner
        })
        .inner
}

/// A thin full-width rule (Divider/Dim).
pub fn rule(ui: &mut Ui) {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 1.0), Sense::hover());
    ui.painter().rect_filled(r, 0, p.divider_dim);
}

/// A small pill that fills a field when clicked (URL examples, recents).
pub fn chip(ui: &mut Ui, label: &str, selected: bool) -> Response {
    let p = pal(ui.ctx());
    let galley = ui.painter().layout_no_wrap(label.to_string(), font::regular(13.0), Color32::PLACEHOLDER);
    let (rect, resp) = ui.allocate_exact_size(galley.size() + vec2(20.0, 12.0), Sense::click());
    let bg = if selected {
        p.bg_accent
    } else if resp.hovered() {
        p.bg_a1
    } else {
        p.bg_a5
    };
    let fg = if selected {
        p.accent
    } else if resp.hovered() {
        p.fg1
    } else {
        p.fg_a2
    };
    ui.painter().rect_filled(rect, CornerRadius::same(6), bg);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, fg);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A key/value line for information panes.
pub fn info_row(ui: &mut Ui, key: &str, value: &str, key_w: f32) {
    let p = pal(ui.ctx());
    ui.horizontal(|ui| {
        ui.set_min_height(28.0);
        let (r, _) = ui.allocate_exact_size(vec2(key_w, 20.0), Sense::hover());
        ui.painter().text(r.left_center(), Align2::LEFT_CENTER, key, font::regular(14.0), p.fg_a3);
        ui.add(
            egui::Label::new(
                RichText::new(if value.is_empty() { "-" } else { value })
                    .font(font::regular(14.0))
                    .color(if value.is_empty() { p.fg_a4 } else { p.fg2 }),
            )
            .wrap()
            .selectable(true),
        );
    });
}

/// A slim progress bar (accent on BG/B2).
pub fn progress(ui: &mut Ui, fraction: Option<f32>, width: f32) {
    let p = pal(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(width, 6.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(3), p.bg_b2);
    match fraction {
        Some(f) => {
            let mut fill = rect;
            fill.set_width(rect.width() * f.clamp(0.0, 1.0));
            ui.painter().rect_filled(fill, CornerRadius::same(3), p.accent);
        }
        None => {
            // Indeterminate: a sliding segment.
            let t = ui.ctx().input(|i| i.time) as f32;
            let seg = rect.width() * 0.25;
            let x = rect.min.x + ((t * 0.6).fract() * (rect.width() + seg)) - seg;
            let r = Rect::from_min_max(
                egui::pos2(x.max(rect.min.x), rect.min.y),
                egui::pos2((x + seg).min(rect.max.x), rect.max.y),
            );
            ui.painter().rect_filled(r, CornerRadius::same(3), p.accent);
            ui.ctx().request_repaint();
        }
    }
}
