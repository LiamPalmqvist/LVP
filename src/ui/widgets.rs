//! The design's control kit: toggles, sliders, dropdowns, segmented values,
//! fields, buttons, rows and menus. Sizes come from the Figma components.

use egui::{
    Align2, Color32, CornerRadius, Id, Pos2, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};

use super::icons::{self, Icon, IconColors};
use super::theme::{Palette, font, mix, with_alpha};

// ---- palette access ---------------------------------------------------------

pub fn set_palette(ctx: &egui::Context, p: Palette) {
    ctx.data_mut(|d| d.insert_temp(Id::new("notvlc-palette"), p));
}

pub fn pal(ctx: &egui::Context) -> Palette {
    ctx.data(|d| d.get_temp(Id::new("notvlc-palette"))).unwrap_or_else(|| Palette::dark(Default::default()))
}

pub fn colors(p: &Palette, fg: Color32) -> IconColors {
    IconColors { fg, accent: p.accent, accent_dim: p.accent_dim }
}

pub fn icon(ui: &mut Ui, icon: Icon, size: f32, fg: Color32) -> Response {
    let p = pal(ui.ctx());
    icons::show(ui, icon, size, colors(&p, fg))
}

pub fn paint_icon(ui: &Ui, rect: Rect, icon: Icon, fg: Color32) {
    let p = pal(ui.ctx());
    icons::paint(ui, rect, icon, colors(&p, fg));
}

// ---- buttons ------------------------------------------------------------------

/// A 24pt icon in a 30x32 hit area that lights up on hover ("Icon Wrapper").
pub fn icon_button(ui: &mut Ui, icon: Icon, tooltip: &str) -> Response {
    icon_button_sized(ui, icon, vec2(30.0, 32.0), 24.0, false, tooltip)
}

pub fn icon_button_sized(ui: &mut Ui, icon: Icon, size: Vec2, icon_size: f32, active: bool, tooltip: &str) -> Response {
    let p = pal(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.hovered() || active {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(4),
            if active { with_alpha(p.accent, 0.12) } else { p.hover },
        );
    }
    let fg = if active {
        p.accent
    } else if resp.hovered() {
        p.fg2
    } else {
        p.fg_a2
    };
    let ir = Rect::from_center_size(rect.center(), Vec2::splat(icon_size));
    paint_icon(ui, ir, icon, fg);
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if tooltip.is_empty() { resp } else { resp.on_hover_text(tooltip) }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    /// Accent fill (Save, Play).
    Primary,
    /// Raised dark fill (Browse, Reset).
    Secondary,
    /// Text only (Cancel).
    Ghost,
    /// Red text for destructive actions.
    Danger,
}

pub fn button(ui: &mut Ui, label: &str, kind: ButtonKind) -> Response {
    button_with(ui, None, label, kind, vec2(0.0, 32.0))
}

pub fn button_with(ui: &mut Ui, icon: Option<Icon>, label: &str, kind: ButtonKind, min: Vec2) -> Response {
    let p = pal(ui.ctx());
    let f = font::semibold(14.0);
    let galley = ui.painter().layout_no_wrap(label.to_string(), f, Color32::WHITE);
    let icon_w = if icon.is_some() { 26.0 } else { 0.0 };
    let w = (galley.size().x + icon_w + 32.0).max(min.x);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, min.y.max(32.0)), Sense::click());
    let hovered = resp.hovered();
    let (bg, fg) = match kind {
        ButtonKind::Primary => (
            if hovered { p.accent_bright } else { p.accent },
            if p.dark { Color32::from_rgb(0x16, 0x16, 0x16) } else { Color32::WHITE },
        ),
        ButtonKind::Secondary => (if hovered { p.bg_a1 } else { p.bg_a3 }, if hovered { p.fg1 } else { p.fg_a1 }),
        ButtonKind::Ghost => {
            (if hovered { p.hover } else { Color32::TRANSPARENT }, if hovered { p.fg1 } else { p.fg_a1 })
        }
        ButtonKind::Danger => (if hovered { with_alpha(p.danger, 0.15) } else { Color32::TRANSPARENT }, p.danger),
    };
    ui.painter().rect_filled(rect, CornerRadius::same(6), bg);
    let content_w = galley.size().x + icon_w;
    let mut x = rect.center().x - content_w / 2.0;
    if let Some(i) = icon {
        paint_icon(ui, Rect::from_min_size(pos2(x, rect.center().y - 10.0), Vec2::splat(20.0)), i, fg);
        x += icon_w;
    }
    ui.painter().galley(pos2(x, rect.center().y - galley.size().y / 2.0), galley, fg);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

// ---- toggle ------------------------------------------------------------------

/// The 68x32 ON/OFF switch.
pub fn toggle(ui: &mut Ui, on: &mut bool) -> Response {
    let p = pal(ui.ctx());
    let (rect, mut resp) = ui.allocate_exact_size(vec2(68.0, 32.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool_with_time(resp.id, *on, 0.12);
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(6), p.bg_b2);
    let inner = rect.shrink(3.0);
    // Label well: right side when on... the thumb slides over it.
    let label_rect_on = Rect::from_min_size(inner.min, vec2(33.0, 26.0));
    let label_rect_off = Rect::from_min_size(pos2(inner.max.x - 33.0, inner.min.y), vec2(33.0, 26.0));
    let label_rect = if *on { label_rect_on } else { label_rect_off };
    let well = mix(p.bg_a5, p.bg_accent, t);
    painter.rect_filled(label_rect, CornerRadius::same(3), well);
    let text_col = mix(p.fg_a3, p.accent, t);
    painter.text(
        label_rect.center(),
        Align2::CENTER_CENTER,
        if *on { "ON" } else { "OFF" },
        font::regular(12.0),
        text_col,
    );
    let x = egui::lerp(inner.min.x..=inner.max.x - 26.0, t);
    let thumb = Rect::from_min_size(pos2(x, inner.min.y), vec2(26.0, 26.0));
    let thumb_col = mix(p.fg_a4, p.accent_dim, t);
    painter.rect_filled(
        thumb,
        CornerRadius::same(3),
        if resp.hovered() { mix(thumb_col, Color32::WHITE, 0.08) } else { thumb_col },
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

// ---- sliders -----------------------------------------------------------------

#[derive(Clone)]
pub struct SliderStyle {
    pub height: f32,
    /// Number of tick marks drawn along the track (0 for none).
    pub ticks: usize,
    /// Value the fill starts from (for bipolar sliders such as delays).
    pub origin: Option<f64>,
    pub step: f64,
    /// Labels drawn under the track: (value, text).
    pub labels: Vec<(f64, String)>,
    pub thumb_w: f32,
    pub thumb_color: Option<Color32>,
    pub disabled: bool,
}

impl Default for SliderStyle {
    fn default() -> Self {
        Self {
            height: 28.0,
            ticks: 0,
            origin: None,
            step: 0.0,
            labels: Vec::new(),
            thumb_w: 8.0,
            thumb_color: None,
            disabled: false,
        }
    }
}

impl SliderStyle {
    pub fn ticks(mut self, n: usize) -> Self {
        self.ticks = n;
        self
    }
    pub fn origin(mut self, o: f64) -> Self {
        self.origin = Some(o);
        self
    }
    pub fn step(mut self, s: f64) -> Self {
        self.step = s;
        self
    }
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    pub fn labels(mut self, l: &[(f64, &str)]) -> Self {
        self.labels = l.iter().map(|(v, t)| (*v, t.to_string())).collect();
        self
    }
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }
}

/// Horizontal slider filling the available width: a dark track, a tinted
/// fill and a pill thumb.
pub fn slider(ui: &mut Ui, value: &mut f64, range: std::ops::RangeInclusive<f64>, style: SliderStyle) -> Response {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let label_h = if style.labels.is_empty() { 0.0 } else { 26.0 };
    let (outer, mut resp) = ui.allocate_exact_size(vec2(w, style.height + 8.0 + label_h), Sense::click_and_drag());
    let rect = Rect::from_min_size(outer.min + vec2(0.0, 4.0), vec2(w, style.height));
    let (lo, hi) = (*range.start(), *range.end());
    let track = rect.shrink2(vec2(2.0, 0.0));
    let to_x = |v: f64| {
        egui::lerp(
            track.min.x + style.thumb_w / 2.0..=track.max.x - style.thumb_w / 2.0,
            ((v - lo) / (hi - lo)).clamp(0.0, 1.0) as f32,
        )
    };
    if !style.disabled
        && (resp.dragged() || resp.clicked())
        && let Some(pos) = resp.interact_pointer_pos()
    {
        let t = ((pos.x - track.min.x - style.thumb_w / 2.0) / (track.width() - style.thumb_w)).clamp(0.0, 1.0) as f64;
        let mut v = lo + t * (hi - lo);
        if style.step > 0.0 {
            v = (v / style.step).round() * style.step;
        }
        let v = v.clamp(lo, hi);
        if v != *value {
            *value = v;
            resp.mark_changed();
        }
    }
    if resp.hovered() && !style.disabled {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y + i.smooth_scroll_delta.x);
        if scroll != 0.0 {
            let step = if style.step > 0.0 { style.step } else { (hi - lo) / 100.0 };
            let v = (*value + step * scroll.signum() as f64).clamp(lo, hi);
            if v != *value {
                *value = v;
                resp.mark_changed();
            }
        }
    }
    let painter = ui.painter();
    let r = CornerRadius::same((style.height / 4.0).min(8.0) as u8);
    painter.rect_filled(rect, r, p.bg_b2);
    let x = to_x(*value);
    let ox = to_x(style.origin.unwrap_or(lo));
    let fill = Rect::from_min_max(pos2(ox.min(x), rect.min.y + 3.0), pos2(ox.max(x), rect.max.y - 3.0));
    let fill_col = if style.disabled { p.bg_a1 } else { p.bg_accent };
    if fill.width() > 0.5 {
        painter.rect_filled(fill, CornerRadius::same(r.nw.saturating_sub(2)), fill_col);
    }
    if style.ticks > 1 {
        for i in 1..style.ticks {
            let tx = egui::lerp(track.min.x..=track.max.x, i as f32 / style.ticks as f32);
            let filled = (tx >= ox.min(x)) && (tx <= ox.max(x));
            let c = if filled { mix(p.bg_accent, p.accent, 0.25) } else { p.bg_a1 };
            let h = (style.height * 0.35).min(10.0);
            painter.line_segment(
                [pos2(tx, rect.center().y - h / 2.0), pos2(tx, rect.center().y + h / 2.0)],
                Stroke::new(2.0, c),
            );
        }
    }
    let thumb_h = style.height + 4.0;
    let thumb = Rect::from_center_size(pos2(x, rect.center().y), vec2(style.thumb_w, thumb_h));
    let tc = if style.disabled { p.fg_a3 } else { style.thumb_color.unwrap_or(p.accent_dim) };
    let tc = if resp.hovered() || resp.dragged() { mix(tc, p.accent_bright, 0.4) } else { tc };
    painter.rect_filled(thumb, CornerRadius::same((style.thumb_w / 2.0) as u8), tc);
    for (v, text) in &style.labels {
        let lx = to_x(*v);
        let align = if (*v - lo).abs() < 1e-9 {
            Align2::LEFT_TOP
        } else if (*v - hi).abs() < 1e-9 {
            Align2::RIGHT_TOP
        } else {
            Align2::CENTER_TOP
        };
        let lx = match align {
            Align2::LEFT_TOP => rect.min.x,
            Align2::RIGHT_TOP => rect.max.x,
            _ => lx,
        };
        painter.text(pos2(lx, rect.max.y + 8.0), align, text, font::regular(14.0), p.fg_a3);
    }
    resp
}

/// Convenience for f32 values.
pub fn slider_f32(ui: &mut Ui, value: &mut f32, range: std::ops::RangeInclusive<f32>, style: SliderStyle) -> Response {
    let mut v = *value as f64;
    let r = slider(ui, &mut v, *range.start() as f64..=*range.end() as f64, style);
    if r.changed() {
        *value = v as f32;
    }
    r
}

/// Vertical slider for equalizer bands.
pub fn vslider(ui: &mut Ui, value: &mut f32, range: std::ops::RangeInclusive<f32>, size: Vec2) -> Response {
    let p = pal(ui.ctx());
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let (lo, hi) = (*range.start(), *range.end());
    let track = rect.shrink2(vec2(0.0, 6.0));
    if let Some(pos) = resp.interact_pointer_pos()
        && (resp.dragged() || resp.clicked())
    {
        let t = 1.0 - ((pos.y - track.min.y) / track.height()).clamp(0.0, 1.0);
        let v = ((lo + t * (hi - lo)) * 10.0).round() / 10.0;
        if v != *value {
            *value = v;
            resp.mark_changed();
        }
    }
    if resp.double_clicked() {
        *value = 0.0f32.clamp(lo, hi);
        resp.mark_changed();
    }
    let painter = ui.painter();
    let w = size.x.min(16.0);
    let bar = Rect::from_center_size(rect.center(), vec2(w, rect.height()));
    painter.rect_filled(bar, CornerRadius::same((w / 2.0) as u8), p.bg_b2);
    let y = egui::lerp(track.max.y..=track.min.y, ((*value - lo) / (hi - lo)).clamp(0.0, 1.0));
    let fill = Rect::from_min_max(pos2(bar.min.x + 2.0, y), pos2(bar.max.x - 2.0, bar.max.y - 2.0));
    painter.rect_filled(fill, CornerRadius::same((w / 2.0 - 2.0) as u8), p.bg_accent);
    let thumb = Rect::from_center_size(pos2(bar.center().x, y), vec2(w + 8.0, 8.0));
    let tc = if resp.hovered() || resp.dragged() { p.accent_bright } else { p.accent_dim };
    painter.rect_filled(thumb, CornerRadius::same(4), tc);
    resp
}

// ---- choice controls ---------------------------------------------------------

/// The 320x36 dropdown. Returns the newly chosen index.
pub fn dropdown(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    selected: &str,
    options: &[String],
    width: f32,
) -> Option<usize> {
    let p = pal(ui.ctx());
    let id = ui.make_persistent_id(id);
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 36.0), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(6), p.bg_b2);
    let inner = rect.shrink(2.0);
    painter.rect_filled(inner, CornerRadius::same(4), if resp.hovered() { p.bg_a2 } else { p.bg_a4 });
    painter.text(
        pos2(inner.min.x + 14.0, inner.center().y),
        Align2::LEFT_CENTER,
        selected,
        font::regular(14.0),
        p.fg_a2,
    );
    paint_icon(
        ui,
        Rect::from_center_size(pos2(inner.max.x - 22.0, inner.center().y), Vec2::splat(20.0)),
        Icon::ChevronDown,
        p.fg_a2,
    );
    let mut chosen = None;
    let popup = egui::Popup::menu(&resp).id(id).width(width).close_behavior(egui::PopupCloseBehavior::CloseOnClick);
    popup.frame(menu_frame(&p)).show(|ui| {
        ui.set_min_width(width - 12.0);
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            for (i, o) in options.iter().enumerate() {
                if menu_row(ui, None, o, "", o == selected).clicked() {
                    chosen = Some(i);
                }
            }
        });
    });
    let _ = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    chosen
}

/// Dropdown over a `choice!` enum from the settings module.
pub fn choice<T: Copy + PartialEq + 'static>(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    value: &mut T,
    all: &[T],
    label: impl Fn(T) -> &'static str,
) -> bool {
    let labels: Vec<String> = all.iter().map(|v| label(*v).to_string()).collect();
    if let Some(i) = dropdown(ui, id, label(*value), &labels, 320.0)
        && all[i] != *value
    {
        *value = all[i];
        return true;
    }
    false
}

/// The 320x36 segmented control ("Values").
pub fn segmented(ui: &mut Ui, selected: &mut usize, labels: &[&str], width: f32) -> bool {
    let p = pal(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(6), p.bg_b2);
    let n = labels.len().max(1) as f32;
    let seg_w = (rect.width() - 4.0 - 2.0 * (n - 1.0)) / n;
    let mut changed = false;
    for (i, l) in labels.iter().enumerate() {
        let r =
            Rect::from_min_size(pos2(rect.min.x + 2.0 + i as f32 * (seg_w + 2.0), rect.min.y + 2.0), vec2(seg_w, 32.0));
        let resp = ui.interact(r, ui.id().with(("seg", l, i)), Sense::click());
        if resp.clicked() && *selected != i {
            *selected = i;
            changed = true;
        }
        let on = *selected == i;
        let bg = if on {
            p.bg_accent
        } else if resp.hovered() {
            p.bg_a2
        } else {
            p.bg_a4
        };
        ui.painter().rect_filled(r, CornerRadius::same(4), bg);
        ui.painter().text(
            r.center(),
            Align2::CENTER_CENTER,
            *l,
            font::regular(14.0),
            if on { p.accent } else { p.fg_a3 },
        );
    }
    changed
}

/// Accent colour picker: nine 32x32 cells.
pub fn swatches(ui: &mut Ui, selected: &mut usize, colors: &[Color32]) -> bool {
    let p = pal(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(320.0, 36.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(6), p.bg_b2);
    let mut changed = false;
    let step = (rect.width() - 4.0) / colors.len().max(1) as f32;
    for (i, c) in colors.iter().enumerate() {
        let cell = Rect::from_min_size(pos2(rect.min.x + 2.0 + i as f32 * step, rect.min.y + 2.0), vec2(32.0, 32.0));
        let resp = ui.interact(cell, ui.id().with(("swatch", i)), Sense::click());
        if resp.clicked() && *selected != i {
            *selected = i;
            changed = true;
        }
        if *selected == i {
            ui.painter().rect_filled(cell, CornerRadius::same(4), p.bg_accent);
        } else if resp.hovered() {
            ui.painter().rect_filled(cell, CornerRadius::same(4), p.hover);
        }
        let a = if *selected == i || resp.hovered() { 1.0 } else { 0.7 };
        ui.painter().rect_filled(
            Rect::from_center_size(cell.center(), Vec2::splat(20.0)),
            CornerRadius::same(1),
            with_alpha(*c, a),
        );
    }
    changed
}

// ---- text inputs ---------------------------------------------------------------

/// A dark single-line field (BG/B2, radius 6, 36 tall).
pub fn text_field(ui: &mut Ui, text: &mut String, hint: &str, width: f32) -> Response {
    let p = pal(ui.ctx());
    let frame = egui::Frame::new().fill(p.bg_b2).corner_radius(6).inner_margin(egui::Margin::symmetric(14, 9));
    frame
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(text)
                    .hint_text(RichText::new(hint).color(p.fg_a4))
                    .frame(false)
                    .text_color(p.fg_a1)
                    .font(font::regular(14.0))
                    .desired_width(width - 28.0),
            )
        })
        .inner
}

pub fn password_field(ui: &mut Ui, text: &mut String, hint: &str, width: f32) -> Response {
    let p = pal(ui.ctx());
    egui::Frame::new()
        .fill(p.bg_b2)
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(14, 9))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(text)
                    .password(true)
                    .hint_text(RichText::new(hint).color(p.fg_a4))
                    .frame(false)
                    .text_color(p.fg_a1)
                    .font(font::regular(14.0))
                    .desired_width(width - 28.0),
            )
        })
        .inner
}

/// Search box with a magnifier, as in the title bar and settings.
pub fn search_field(ui: &mut Ui, text: &mut String, hint: &str, width: f32) -> Response {
    let p = pal(ui.ctx());
    egui::Frame::new()
        .fill(p.bg_a5)
        .corner_radius(6)
        .inner_margin(egui::Margin { left: 8, right: 8, top: 5, bottom: 5 })
        .show(ui, |ui| {
            ui.set_width(width - 16.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                icon(ui, Icon::Search, 24.0, p.fg_a3);
                ui.add(
                    egui::TextEdit::singleline(text)
                        .hint_text(RichText::new(hint).color(p.fg_a4))
                        .frame(false)
                        .text_color(p.fg_a1)
                        .font(font::regular(14.0))
                        .desired_width(ui.available_width()),
                )
            })
            .inner
        })
        .inner
}

/// Numeric field with a unit suffix ("0 px", "-4.000 s"). Drag or type.
pub fn number_field(
    ui: &mut Ui,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    decimals: usize,
    unit: &str,
    width: f32,
) -> Response {
    let p = pal(ui.ctx());
    egui::Frame::new()
        .fill(p.bg_b2)
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(14, 8))
        .show(ui, |ui| {
            ui.set_width(width - 28.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let speed = ((range.end() - range.start()) / 300.0).max(10f64.powi(-(decimals as i32)));
                let r = ui.add(
                    egui::DragValue::new(value)
                        .range(range)
                        .speed(speed)
                        .max_decimals(decimals)
                        .min_decimals(decimals)
                        .custom_formatter(move |v, _| format!("{v:.decimals$}")),
                );
                if !unit.is_empty() {
                    ui.label(RichText::new(unit).color(p.fg_a3).font(font::regular(14.0)));
                }
                r
            })
            .inner
        })
        .inner
}

/// "#RRGGBB" with a swatch that opens a picker.
pub fn color_field(ui: &mut Ui, rgb: &mut u32, width: f32) -> bool {
    let p = pal(ui.ctx());
    let mut changed = false;
    egui::Frame::new()
        .fill(p.bg_b2)
        .corner_radius(6)
        .inner_margin(egui::Margin { left: 14, right: 6, top: 4, bottom: 4 })
        .show(ui, |ui| {
            ui.set_width(width - 20.0);
            ui.horizontal(|ui| {
                let mut text = format!("#{:06X}", *rgb & 0xFFFFFF);
                let r = ui.add(
                    egui::TextEdit::singleline(&mut text)
                        .frame(false)
                        .text_color(p.fg_a1)
                        .font(font::regular(14.0))
                        .desired_width(ui.available_width() - 40.0),
                );
                if r.changed()
                    && let Ok(v) = u32::from_str_radix(text.trim().trim_start_matches('#'), 16)
                {
                    *rgb = v & 0xFFFFFF;
                    changed = true;
                }
                let mut c = Color32::from_rgb((*rgb >> 16) as u8, (*rgb >> 8) as u8, *rgb as u8);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let resp = egui::widgets::color_picker::color_edit_button_srgba(
                        ui,
                        &mut c,
                        egui::color_picker::Alpha::Opaque,
                    );
                    if resp.changed() {
                        *rgb = ((c.r() as u32) << 16) | ((c.g() as u32) << 8) | c.b() as u32;
                        changed = true;
                    }
                });
            });
        });
    changed
}

// ---- layout pieces ---------------------------------------------------------------

/// Section heading in settings and panels (Medium 16, FG/2).
pub fn section(ui: &mut Ui, title: &str) {
    let p = pal(ui.ctx());
    ui.add_space(10.0);
    ui.label(RichText::new(title).font(font::medium(16.0)).color(p.fg2));
    ui.add_space(4.0);
}

/// Label with an optional info bubble on the left, control on the right.
pub fn row<R>(ui: &mut Ui, label: &str, info: Option<&str>, control: impl FnOnce(&mut Ui) -> R) -> R {
    let p = pal(ui.ctx());
    ui.horizontal(|ui| {
        ui.set_min_height(42.0);
        ui.label(RichText::new(label).font(font::regular(14.0)).color(p.fg_a2));
        if let Some(text) = info {
            ui.add_space(2.0);
            icon(ui, Icon::Info, 20.0, p.fg_a3).on_hover_text(text);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), control).inner
    })
    .inner
}

pub fn divider(ui: &mut Ui) {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 1.0), Sense::hover());
    ui.painter().rect_filled(r, 0, p.divider_dim);
}

/// The collapsible effect card: "Expand Settings" / "Collapse Settings".
pub fn collapsible(ui: &mut Ui, id: impl std::hash::Hash, add: impl FnOnce(&mut Ui)) {
    let p = pal(ui.ctx());
    let id = ui.make_persistent_id(id);
    let mut open = ui.data_mut(|d| *d.get_persisted_mut_or(id, false));
    egui::Frame::new().fill(p.bg_a3).corner_radius(8).inner_margin(egui::Margin::symmetric(16, 10)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::click());
        let text = if open { "Collapse Settings" } else { "Expand Settings" };
        ui.painter().text(
            pos2(rect.min.x, rect.center().y),
            Align2::LEFT_CENTER,
            text,
            font::regular(14.0),
            if resp.hovered() { p.fg_a1 } else { p.fg_a3 },
        );
        let chev = if open { Icon::ChevronDown } else { Icon::ChevronRight };
        paint_icon(
            ui,
            Rect::from_center_size(pos2(rect.max.x - 12.0, rect.center().y), Vec2::splat(20.0)),
            chev,
            p.fg_a3,
        );
        if resp.clicked() {
            open = !open;
            ui.data_mut(|d| d.insert_persisted(id, open));
        }
        if open {
            ui.add_space(8.0);
            add(ui);
        }
    });
}

/// Tab strip with an accent underline (quick settings, side panels).
pub fn tabs(ui: &mut Ui, selected: &mut usize, items: &[(Option<Icon>, &str)]) -> bool {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 52.0), Sense::hover());
    ui.painter().rect_filled(rect, 0, p.bg_b2);
    let n = items.len().max(1) as f32;
    let tw = w / n;
    let mut changed = false;
    for (i, (ic, label)) in items.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.min.x + i as f32 * tw, rect.min.y), vec2(tw, rect.height()));
        let resp = ui.interact(r, ui.id().with(("tab", i)), Sense::click());
        if resp.clicked() && *selected != i {
            *selected = i;
            changed = true;
        }
        let on = *selected == i;
        let col = if on {
            p.fg1
        } else if resp.hovered() {
            p.fg_a1
        } else {
            p.fg_a3
        };
        let galley = ui.painter().layout_no_wrap(label.to_string(), font::regular(14.0), col);
        let iw = if ic.is_some() { 30.0 } else { 0.0 };
        let x0 = r.center().x - (galley.size().x + iw) / 2.0;
        if let Some(ic) = ic {
            paint_icon(
                ui,
                Rect::from_min_size(pos2(x0, r.center().y - 12.0), Vec2::splat(24.0)),
                *ic,
                if on { p.accent } else { col },
            );
        }
        ui.painter().galley(pos2(x0 + iw, r.center().y - galley.size().y / 2.0), galley, col);
        if on {
            let ul = Rect::from_min_max(pos2(r.min.x + 8.0, r.max.y - 3.0), pos2(r.max.x - 8.0, r.max.y));
            ui.painter().rect_filled(ul, CornerRadius::same(1), p.accent);
        }
    }
    changed
}

// ---- menus ---------------------------------------------------------------------

pub fn menu_frame(p: &Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(p.bg_a1)
        .corner_radius(12)
        .inner_margin(egui::Margin::same(6))
        .stroke(Stroke::new(1.0, p.divider_dim))
        .shadow(egui::epaint::Shadow { offset: [0, 8], blur: 30, spread: 0, color: Color32::from_black_alpha(118) })
}

/// A context-menu row: icon, label, and a shortcut or submenu chevron on the right.
pub fn menu_row(ui: &mut Ui, icon_: Option<Icon>, label: &str, right: &str, selected: bool) -> Response {
    let p = pal(ui.ctx());
    let w = ui.available_width().max(200.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 40.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(6), p.hover);
    }
    let mut x = rect.min.x + 12.0;
    if let Some(i) = icon_ {
        paint_icon(ui, Rect::from_min_size(pos2(x, rect.center().y - 12.0), Vec2::splat(24.0)), i, p.fg_a2);
        x += 36.0;
    }
    let col = if selected {
        p.accent
    } else if resp.hovered() {
        p.fg1
    } else {
        p.fg_a1
    };
    ui.painter().text(pos2(x, rect.center().y), Align2::LEFT_CENTER, label, font::regular(15.0), col);
    if right == ">" {
        paint_icon(
            ui,
            Rect::from_center_size(pos2(rect.max.x - 18.0, rect.center().y), Vec2::splat(20.0)),
            Icon::ChevronRight,
            p.fg_a3,
        );
    } else if !right.is_empty() {
        ui.painter().text(
            pos2(rect.max.x - 12.0, rect.center().y),
            Align2::RIGHT_CENTER,
            right,
            font::regular(14.0),
            p.fg_a3,
        );
    } else if selected {
        paint_icon(
            ui,
            Rect::from_center_size(pos2(rect.max.x - 18.0, rect.center().y), Vec2::splat(20.0)),
            Icon::Check,
            p.accent,
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A submenu row that opens its contents to the side on hover. Only valid
/// inside a menu (context menu or `Popup::menu`).
pub fn submenu(ui: &mut Ui, icon_: Option<Icon>, label: &str, add: impl FnOnce(&mut Ui)) {
    let resp = menu_row(ui, icon_, label, ">", false);
    egui::containers::menu::SubMenu::new().show(ui, &resp, |ui| {
        ui.set_min_width(240.0);
        add(ui);
    });
}

// ---- misc ------------------------------------------------------------------------

/// A muted placeholder for empty lists.
pub fn empty_state(ui: &mut Ui, icon_: Icon, title: &str, body: &str) {
    let p = pal(ui.ctx());
    ui.vertical_centered(|ui| {
        ui.add_space(60.0);
        icon(ui, icon_, 48.0, p.fg_a4);
        ui.add_space(12.0);
        ui.label(RichText::new(title).font(font::medium(16.0)).color(p.fg_a1));
        ui.add_space(4.0);
        ui.label(RichText::new(body).font(font::regular(14.0)).color(p.fg_a3));
    });
}

/// Draws text clipped with an ellipsis to `max_w`.
pub fn text_clipped(
    ui: &Ui,
    pos: Pos2,
    align: Align2,
    text: &str,
    font: egui::FontId,
    color: Color32,
    max_w: f32,
) -> Rect {
    let mut job = egui::text::LayoutJob::single_section(text.to_string(), egui::TextFormat::simple(font, color));
    job.wrap =
        egui::text::TextWrapping {
            max_width: max_w, max_rows: 1, break_anywhere: true, overflow_character: Some('…')
        };
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let rect = align.anchor_size(pos, galley.size());
    ui.painter().galley(rect.min, galley, color);
    rect
}

/// A rounded panel surface with the design's blur-and-shadow look.
pub fn panel_frame(p: &Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(p.bg_b1_t)
        .corner_radius(14)
        .stroke(Stroke::new(1.0, p.divider_dim))
        .shadow(egui::epaint::Shadow { offset: [0, 8], blur: 30, spread: 0, color: Color32::from_black_alpha(118) })
}

pub fn stroke_rect(ui: &Ui, rect: Rect, r: u8, color: Color32) {
    ui.painter().rect_stroke(rect, CornerRadius::same(r), Stroke::new(1.0, color), StrokeKind::Inside);
}
