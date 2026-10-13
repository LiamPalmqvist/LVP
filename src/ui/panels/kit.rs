//! Controls drawn the way the player popovers draw them. The settings kit in
//! `ui::widgets` is sized for the settings window; these follow the
//! "Controls" component frame (`491:1780`): 16pt tracks with 8x24 thumbs,
//! 140x36 value boxes, 48pt dropdowns, 50pt tab strips.

use egui::{Align2, Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2};

use crate::ui::icons::Icon;
use crate::ui::theme::{Palette, font, mix, with_alpha};
use crate::ui::widgets::{self, pal};

/// Popover width and the content's side margin.
pub const WIDTH: f32 = 400.0;
pub const MARGIN: f32 = 20.0;
pub const TAB_H: f32 = 50.0;
/// Temp-data flag that opens every effect card by default.
pub const EXPAND_ALL: &str = "panels-expand-cards";

/// The popover surface: BG/B1 at 95%, radius 10.
pub fn popover_frame(p: &Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(p.bg_b1_t)
        .corner_radius(10)
        .stroke(Stroke::new(1.0, p.divider_dim))
        .shadow(egui::epaint::Shadow { offset: [0, 8], blur: 30, spread: 0, color: Color32::from_black_alpha(118) })
}

/// The list surface of an open dropdown.
pub fn list_frame(p: &Palette) -> egui::Frame {
    egui::Frame::new().fill(p.bg_a4).corner_radius(6).inner_margin(egui::Margin::same(2)).shadow(egui::epaint::Shadow {
        offset: [0, 6],
        blur: 20,
        spread: 0,
        color: Color32::from_black_alpha(110),
    })
}

// ---- tabs -----------------------------------------------------------------

/// The popover tab strip: icon and label per tab, accent underline under the
/// selected one. `widths` gives each tab's width; `None` splits evenly.
pub fn tab_strip(ui: &mut Ui, selected: usize, items: &[(Icon, &str)], widths: Option<&[f32]>) -> Option<usize> {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, TAB_H), Sense::hover());
    let r = CornerRadius { nw: 10, ne: 10, sw: 0, se: 0 };
    ui.painter().rect_filled(rect, r, p.bg_b2);
    let inner = rect.shrink2(vec2(6.0, 0.0));
    let n = items.len().max(1) as f32;
    let even = (inner.width() - (n - 1.0)) / n;
    let mut x = inner.min.x;
    let mut chosen = None;
    for (i, (ic, label)) in items.iter().enumerate() {
        let tw = widths.and_then(|w| w.get(i).copied()).unwrap_or(even);
        let tr = Rect::from_min_size(pos2(x, rect.min.y), vec2(tw, TAB_H));
        x += tw + 1.0;
        let resp = ui.interact(tr, ui.id().with(("ptab", i)), Sense::click());
        if resp.clicked() && i != selected {
            chosen = Some(i);
        }
        let on = i == selected;
        let col = if on {
            p.fg2
        } else if resp.hovered() {
            p.fg_a1
        } else {
            p.fg_a3
        };
        let galley = ui.painter().layout_no_wrap(label.to_string(), font::regular(12.0), col);
        let cw = 20.0 + 8.0 + galley.size().x;
        let x0 = tr.center().x - cw / 2.0;
        let cy = tr.min.y + 24.0;
        widgets::paint_icon(
            ui,
            Rect::from_min_size(pos2(x0, cy - 10.0), Vec2::splat(20.0)),
            *ic,
            if on { p.accent } else { col },
        );
        ui.painter().galley(pos2(x0 + 28.0, cy - galley.size().y / 2.0), galley, col);
        if on {
            let ul = Rect::from_min_max(pos2(tr.min.x + 8.0, tr.max.y - 2.0), pos2(tr.max.x - 8.0, tr.max.y));
            ui.painter().rect_filled(ul, CornerRadius::same(1), p.accent);
        }
        let _ = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    chosen
}

// ---- headings and rows ----------------------------------------------------------

/// "Defaults", "Synchronization"...: Medium 16 on a 39pt band.
pub fn section(ui: &mut Ui, title: &str) {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 39.0), Sense::hover());
    ui.painter().text(pos2(rect.min.x + 4.0, rect.min.y + 20.0), Align2::LEFT_TOP, title, font::medium(16.0), p.fg_a1);
}

/// A full-width rule between sections; reaches past the content margin to
/// the popover's edges.
pub fn rule(ui: &mut Ui) {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 1.0), Sense::hover());
    let r = Rect::from_min_max(pos2(r.min.x - MARGIN, r.min.y), pos2(r.max.x + MARGIN, r.max.y));
    ui.painter().rect_filled(r, 0, p.divider_dim);
}

/// A rule inside a card, from edge to edge of the card.
pub fn card_rule(ui: &mut Ui) {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 1.0), Sense::hover());
    let r = Rect::from_min_max(pos2(r.min.x - 16.0, r.min.y), pos2(r.max.x + 16.0, r.max.y));
    ui.painter().rect_filled(r, 0, p.divider_dim);
}

/// A label on the left (Regular 14, FG/A3, inset 4) and a control on the
/// right, `h` tall.
pub fn row<R>(ui: &mut Ui, label: &str, h: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    row_with(ui, label, None, h, add)
}

pub fn row_with<R>(ui: &mut Ui, label: &str, tip: Option<&str>, h: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let label_resp = ui.interact(
        Rect::from_min_size(rect.min, vec2(rect.width() * 0.55, h)),
        ui.id().with(("rowlabel", label)),
        Sense::hover(),
    );
    ui.painter().text(
        pos2(rect.min.x + 4.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font::regular(14.0),
        p.fg_a3,
    );
    if let Some(t) = tip {
        label_resp.on_hover_text(t);
    }
    let mut child =
        ui.new_child(UiBuilder::new().max_rect(rect).layout(egui::Layout::right_to_left(egui::Align::Center)));
    add(&mut child)
}

/// A check mark drawn directly (the exported "check" icon is an empty frame).
pub fn paint_check(ui: &Ui, rect: Rect, color: Color32) {
    let k = rect.width() / 24.0;
    let at = |x: f32, y: f32| rect.min + vec2(x * k, y * k);
    ui.painter()
        .add(egui::Shape::line(vec![at(5.0, 11.5), at(9.7, 17.0), at(19.0, 6.5)], Stroke::new(1.75 * k, color)));
}

/// A label with a value printed on the right, above a slider (37pt).
pub fn label_value(ui: &mut Ui, label: &str, value: &str) {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 37.0), Sense::hover());
    let y = rect.min.y + 10.0;
    ui.painter().text(pos2(rect.min.x, y), Align2::LEFT_TOP, label, font::regular(14.0), p.fg_a3);
    if !value.is_empty() {
        ui.painter().text(pos2(rect.max.x, y), Align2::RIGHT_TOP, value, font::regular(14.0), p.fg_a3);
    }
}

/// A muted caption ("Embedded Tracks", "4 items - 2h 3m").
pub fn caption(ui: &Ui, pos: egui::Pos2, text: &str) {
    let p = pal(ui.ctx());
    ui.painter().text(pos, Align2::LEFT_CENTER, text, font::regular(14.0), p.fg_a3);
}

// ---- value boxes ------------------------------------------------------------------

fn paint_box(ui: &Ui, rect: Rect, focused: bool, hovered: bool) {
    let p = pal(ui.ctx());
    ui.painter().rect_filled(rect, CornerRadius::same(6), p.bg_b2);
    let stroke = if focused {
        with_alpha(p.accent, 0.6)
    } else if hovered {
        p.bg_a1
    } else {
        p.bg_a4
    };
    ui.painter().rect_stroke(rect, CornerRadius::same(6), Stroke::new(2.0, stroke), StrokeKind::Inside);
}

/// The 140x36 "text_input" box showing `display`. Clicking edits it; the
/// typed text comes back when editing ends. Scrolling over it returns
/// `Scroll(±1)` so numeric boxes can step.
pub enum BoxEvent {
    Commit(String),
    Scroll(f32),
}

pub fn value_box(ui: &mut Ui, id: impl std::hash::Hash, display: &str, width: f32) -> Option<BoxEvent> {
    value_box_sized(ui, id, display, vec2(width, 36.0))
}

pub fn value_box_sized(ui: &mut Ui, id: impl std::hash::Hash, display: &str, size: Vec2) -> Option<BoxEvent> {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    value_box_in(ui, rect, rect.shrink2(vec2(12.0, 0.0)), resp.hovered(), id, display)
}

/// A value box in an already allocated `rect`, editing text inside `text`.
pub fn value_box_in(
    ui: &mut Ui,
    rect: Rect,
    text: Rect,
    hovered: bool,
    id: impl std::hash::Hash,
    display: &str,
) -> Option<BoxEvent> {
    let p = pal(ui.ctx());
    let te_id = ui.make_persistent_id(id).with("te");
    let focused = ui.memory(|m| m.has_focus(te_id));
    paint_box(ui, rect, focused, hovered);
    let mut buf = if focused {
        ui.data(|d| d.get_temp::<String>(te_id)).unwrap_or_else(|| display.to_string())
    } else {
        display.to_string()
    };
    let inner = text;
    let te = egui::TextEdit::singleline(&mut buf)
        .id(te_id)
        .frame(false)
        .font(font::regular(14.0))
        .text_color(p.fg_a2)
        .vertical_align(egui::Align::Center)
        .margin(egui::Margin::ZERO)
        .desired_width(inner.width());
    let mut child =
        ui.new_child(UiBuilder::new().max_rect(inner).layout(egui::Layout::left_to_right(egui::Align::Center)));
    let r = child.add(te);
    let mut out = None;
    if r.has_focus() {
        ui.data_mut(|d| d.insert_temp(te_id, buf.clone()));
    } else if r.lost_focus() {
        ui.data_mut(|d| d.remove::<String>(te_id));
        if buf != display {
            out = Some(BoxEvent::Commit(buf));
        }
    }
    if out.is_none() && !focused && (hovered || r.hovered()) {
        let s = ui.input(|i| i.raw_scroll_delta.y);
        if s.abs() > 0.5 {
            out = Some(BoxEvent::Scroll(s.signum()));
        }
    }
    out
}

/// Parses the leading number of "-4.000 s", "1.25x", "90 deg".
pub fn parse_number(s: &str) -> Option<f64> {
    let t: String = s.trim().chars().take_while(|c| c.is_ascii_digit() || matches!(c, '-' | '+' | '.' | ',')).collect();
    t.replace(',', ".").parse().ok()
}

/// A numeric value box: shows `fmt(value)`, accepts typed numbers (in the
/// displayed unit, converted back with `parse`) and wheel steps.
pub fn number_box(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    step: f64,
    fmt: impl Fn(f64) -> String,
    width: f32,
) -> bool {
    let shown = fmt(*value);
    match value_box(ui, id, &shown, width) {
        Some(BoxEvent::Commit(text)) => match parse_number(&text) {
            Some(v) => {
                let v = v.clamp(*range.start(), *range.end());
                let changed = v != *value;
                *value = v;
                changed
            }
            None => false,
        },
        Some(BoxEvent::Scroll(d)) => {
            let v = (*value + step * d as f64).clamp(*range.start(), *range.end());
            let changed = v != *value;
            *value = v;
            changed
        }
        None => false,
    }
}

// ---- dropdown -----------------------------------------------------------------------

/// One entry of a dropdown list.
pub struct Opt {
    pub icon: Option<Icon>,
    pub label: String,
}

impl Opt {
    pub fn new(label: impl Into<String>) -> Self {
        Self { icon: None, label: label.into() }
    }
    pub fn icon(icon: Icon, label: impl Into<String>) -> Self {
        Self { icon: Some(icon), label: label.into() }
    }
}

pub fn opts<S: AsRef<str>>(labels: &[S]) -> Vec<Opt> {
    labels.iter().map(|l| Opt::new(l.as_ref())).collect()
}

/// A dropdown row in an open list: optional icon, label, check when selected.
pub fn option_row(ui: &mut Ui, icon: Option<Icon>, label: &str, selected: bool, muted: bool) -> Response {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 44.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(4), p.bg_a2);
    } else if selected {
        ui.painter().rect_filled(rect, CornerRadius::same(4), p.bg_a3);
    }
    let mut x = rect.min.x + 14.0;
    let col = if muted {
        p.fg_a3
    } else if resp.hovered() {
        p.fg1
    } else {
        p.fg_a2
    };
    if let Some(i) = icon {
        widgets::paint_icon(ui, Rect::from_min_size(pos2(x, rect.center().y - 12.0), Vec2::splat(24.0)), i, col);
        x += 34.0;
    }
    widgets::text_clipped(
        ui,
        pos2(x, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font::regular(14.0),
        col,
        rect.max.x - x - 44.0,
    );
    if selected {
        paint_check(ui, Rect::from_center_size(pos2(rect.max.x - 26.0, rect.center().y), Vec2::splat(24.0)), p.fg_a1);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// The closed dropdown box: BG/B2 outline, BG/A4 well, chevron on the right.
pub fn drop_box(ui: &mut Ui, size: Vec2, icon: Option<Icon>, text: &str, up: bool, centered: bool) -> Response {
    let p = pal(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    ui.painter().rect_filled(rect, CornerRadius::same(6), p.bg_b2);
    let inner = rect.shrink(2.0);
    let well = if resp.hovered() { p.bg_a2 } else { p.bg_a4 };
    ui.painter().rect_filled(inner, CornerRadius::same(4), well);
    let mut x = inner.min.x + 14.0;
    if let Some(i) = icon {
        widgets::paint_icon(ui, Rect::from_min_size(pos2(x, inner.center().y - 12.0), Vec2::splat(24.0)), i, p.fg_a2);
        x += 34.0;
    }
    let max_w = inner.max.x - 44.0 - x;
    if centered {
        widgets::text_clipped(
            ui,
            pos2(inner.center().x - 8.0, inner.center().y),
            Align2::CENTER_CENTER,
            text,
            font::regular(14.0),
            p.fg_a2,
            max_w,
        );
    } else {
        widgets::text_clipped(
            ui,
            pos2(x, inner.center().y),
            Align2::LEFT_CENTER,
            text,
            font::regular(14.0),
            p.fg_a2,
            max_w,
        );
    }
    widgets::paint_icon(
        ui,
        Rect::from_center_size(pos2(inner.max.x - 26.0, inner.center().y), Vec2::splat(24.0)),
        if up { Icon::ChevronUp } else { Icon::ChevronDown },
        p.fg_a2,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Dropdown with the design's list. Returns the chosen index.
pub fn dropdown(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    size: Vec2,
    selected: Option<usize>,
    current: &str,
    options: &[Opt],
) -> Option<usize> {
    let icon = selected.and_then(|i| options.get(i)).and_then(|o| o.icon);
    let resp = drop_box(ui, size, icon, current, false, false);
    drop_list(ui, &resp, id, size.x, false, |ui| {
        let mut chosen = None;
        for (i, o) in options.iter().enumerate() {
            if option_row(ui, o.icon, &o.label, Some(i) == selected, false).clicked() {
                chosen = Some(i);
            }
        }
        chosen
    })
    .flatten()
}

/// Shows the list popup for a dropdown box.
pub fn drop_list<R>(
    ui: &Ui,
    resp: &Response,
    id: impl std::hash::Hash,
    width: f32,
    up: bool,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let p = pal(ui.ctx());
    let id = ui.make_persistent_id(id).with("list");
    egui::Popup::menu(resp)
        .id(id)
        .width(width)
        .gap(8.0)
        .align(if up { egui::RectAlign::TOP_START } else { egui::RectAlign::BOTTOM_START })
        .frame(list_frame(&p))
        .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
        .show(|ui| {
            ui.set_min_width(width - 4.0);
            ui.set_max_width(width - 4.0);
            ui.spacing_mut().item_spacing.y = 2.0;
            egui::ScrollArea::vertical().max_height(360.0).show(ui, add).inner
        })
        .map(|r| r.inner)
}

/// Dropdown over a `choice!`-style list of values.
pub fn choice<T: Copy + PartialEq>(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    size: Vec2,
    value: T,
    all: &[T],
    label: impl Fn(T) -> String,
) -> Option<T> {
    let options: Vec<Opt> = all.iter().map(|v| Opt::new(label(*v))).collect();
    let sel = all.iter().position(|v| *v == value);
    dropdown(ui, id, size, sel, &label(value), &options).map(|i| all[i]).filter(|v| *v != value)
}

// ---- segmented ------------------------------------------------------------------------

/// The "Values" control: BG/B2 outline, segments with an accent tint when on.
pub fn segmented(ui: &mut Ui, id: impl std::hash::Hash, selected: usize, labels: &[&str], size: Vec2) -> Option<usize> {
    let p = pal(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(6), p.bg_b2);
    let n = labels.len().max(1) as f32;
    let seg_w = (rect.width() - 4.0 - 2.0 * (n - 1.0)) / n;
    let base = ui.make_persistent_id(id);
    let mut chosen = None;
    for (i, l) in labels.iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(rect.min.x + 2.0 + i as f32 * (seg_w + 2.0), rect.min.y + 2.0),
            vec2(seg_w, rect.height() - 4.0),
        );
        let resp = ui.interact(r, base.with(i), Sense::click());
        if resp.clicked() && selected != i {
            chosen = Some(i);
        }
        let on = selected == i;
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
        let _ = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    chosen
}

// ---- slider ------------------------------------------------------------------------------

#[derive(Clone, Default)]
pub struct Sl {
    /// Number of equal divisions marked with ticks (0 for none).
    pub ticks: usize,
    /// Value the fill starts from (bipolar sliders).
    pub origin: Option<f64>,
    pub step: f64,
    /// Labels under the track: (value, text).
    pub labels: Vec<(f64, String)>,
    pub disabled: bool,
    /// Value restored by a double click.
    pub reset: Option<f64>,
}

impl Sl {
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
    pub fn labels(mut self, l: &[(f64, &str)]) -> Self {
        self.labels = l.iter().map(|(v, t)| (*v, t.to_string())).collect();
        self
    }
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }
    pub fn reset(mut self, v: f64) -> Self {
        self.reset = Some(v);
        self
    }
}

/// The popover slider: a 16pt BG/B2 track, a 12pt tinted fill and an 8x24
/// thumb, filling the available width.
pub fn slider(ui: &mut Ui, value: &mut f64, range: std::ops::RangeInclusive<f64>, s: &Sl) -> Response {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let label_h = if s.labels.is_empty() { 0.0 } else { 25.0 };
    let (outer, mut resp) = ui.allocate_exact_size(vec2(w, 24.0 + label_h), Sense::click_and_drag());
    let track = Rect::from_min_size(outer.min + vec2(0.0, 4.0), vec2(w, 16.0));
    let (lo, hi) = (*range.start(), *range.end());
    let span = (hi - lo).max(1e-9);
    let usable = track.shrink2(vec2(2.0, 0.0));
    let to_x = |v: f64| egui::lerp(usable.min.x + 4.0..=usable.max.x - 4.0, ((v - lo) / span).clamp(0.0, 1.0) as f32);
    let snap = |v: f64| {
        let v = if s.step > 0.0 { lo + ((v - lo) / s.step).round() * s.step } else { v };
        v.clamp(lo, hi)
    };
    if !s.disabled {
        if let Some(r) = s.reset
            && resp.double_clicked()
        {
            *value = r;
            resp.mark_changed();
        } else if (resp.dragged() || resp.clicked())
            && let Some(pos) = resp.interact_pointer_pos()
        {
            let t = ((pos.x - usable.min.x - 4.0) / (usable.width() - 8.0)).clamp(0.0, 1.0) as f64;
            let v = snap(lo + t * span);
            if v != *value {
                *value = v;
                resp.mark_changed();
            }
        }
        if resp.hovered() {
            let d = ui.input(|i| i.raw_scroll_delta.y + i.raw_scroll_delta.x);
            if d.abs() > 0.5 {
                let step = if s.step > 0.0 { s.step } else { span / 100.0 };
                let v = snap(*value + step * d.signum() as f64);
                if v != *value {
                    *value = v;
                    resp.mark_changed();
                }
            }
        }
    }
    let painter = ui.painter();
    painter.rect_filled(track, CornerRadius::same(6), p.bg_b2);
    let x = to_x(*value);
    let ox = to_x(s.origin.unwrap_or(lo));
    let (a, b) = (ox.min(x), ox.max(x));
    let fill =
        Rect::from_min_max(pos2((a - 4.0).max(usable.min.x), track.min.y + 2.0), pos2(b + 4.0, track.max.y - 2.0));
    let fill_col = if s.disabled { p.bg_a1 } else { p.bg_accent };
    painter.rect_filled(fill, CornerRadius::same(4), fill_col);
    if s.ticks > 1 {
        for i in 1..s.ticks {
            let tx = egui::lerp(usable.min.x..=usable.max.x, i as f32 / s.ticks as f32);
            let on = tx >= a && tx <= b;
            let c = if on { mix(p.bg_accent, p.accent_dim, 0.35) } else { p.bg_a1 };
            painter.line_segment(
                [pos2(tx, track.center().y - 3.0), pos2(tx, track.center().y + 3.0)],
                Stroke::new(1.0, c),
            );
        }
    }
    let thumb = Rect::from_center_size(pos2(x, track.center().y), vec2(8.0, 24.0));
    let tc = if s.disabled { p.fg_a3 } else { p.accent_dim };
    let tc = if !s.disabled && (resp.hovered() || resp.dragged()) { mix(tc, p.accent_bright, 0.4) } else { tc };
    painter.rect_filled(thumb, CornerRadius::same(4), tc);
    for (v, text) in &s.labels {
        let (lx, align) = if (*v - lo).abs() < 1e-9 {
            (track.min.x, Align2::LEFT_TOP)
        } else if (*v - hi).abs() < 1e-9 {
            (track.max.x, Align2::RIGHT_TOP)
        } else {
            (to_x(*v), Align2::CENTER_TOP)
        };
        painter.text(pos2(lx, track.max.y + 10.0), align, text, font::regular(12.0), p.fg_a3);
    }
    if !s.disabled {
        resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    resp
}

pub fn slider_f32(ui: &mut Ui, value: &mut f32, range: std::ops::RangeInclusive<f32>, s: &Sl) -> Response {
    let mut v = *value as f64;
    let r = slider(ui, &mut v, *range.start() as f64..=*range.end() as f64, s);
    if r.changed() {
        *value = v as f32;
    }
    r
}

pub fn slider_i32(ui: &mut Ui, value: &mut i32, range: std::ops::RangeInclusive<i32>, s: &Sl) -> Response {
    let mut v = *value as f64;
    let s = Sl { step: s.step.max(1.0), ..s.clone() };
    let r = slider(ui, &mut v, *range.start() as f64..=*range.end() as f64, &s);
    if r.changed() {
        *value = v.round() as i32;
    }
    r
}

// ---- cards ---------------------------------------------------------------------------------

/// The effect card: a 48pt "Expand Settings" bar that opens into a BG/A3
/// panel ("Collapse Settings") holding `add`.
pub fn card(ui: &mut Ui, id: impl std::hash::Hash, add: impl FnOnce(&mut Ui)) {
    let p = pal(ui.ctx());
    let id = ui.make_persistent_id(id);
    let all = ui.data(|d| d.get_temp::<bool>(egui::Id::new(EXPAND_ALL))).unwrap_or(false);
    let mut open = ui.data_mut(|d| *d.get_persisted_mut_or(id, all));
    let fill = if open { p.bg_a3 } else { p.bg_b2 };
    egui::Frame::new().fill(fill).corner_radius(6).show(ui, |ui| {
        let w = ui.available_width();
        ui.set_width(w);
        ui.spacing_mut().item_spacing.y = 0.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(w, 48.0), Sense::click());
        let text = if open { "Collapse Settings" } else { "Expand Settings" };
        let col = if resp.hovered() { p.fg_a1 } else { p.fg_a3 };
        ui.painter().text(
            pos2(rect.min.x + 16.0, rect.center().y),
            Align2::LEFT_CENTER,
            text,
            font::regular(14.0),
            col,
        );
        widgets::paint_icon(
            ui,
            Rect::from_center_size(pos2(rect.max.x - 22.0, rect.center().y), Vec2::splat(24.0)),
            if open { Icon::ChevronDown } else { Icon::ChevronRight },
            p.fg_a2,
        );
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            open = !open;
            ui.data_mut(|d| d.insert_persisted(id, open));
        }
        if open {
            egui::Frame::new().inner_margin(egui::Margin { left: 16, right: 16, top: 0, bottom: 10 }).show(ui, |ui| {
                ui.set_width(w - 32.0);
                ui.spacing_mut().item_spacing.y = 5.0;
                add(ui);
            });
        }
    });
}

/// A slider row inside a card: label and value text, then the slider.
pub fn card_slider(
    ui: &mut Ui,
    label: &str,
    value_text: &str,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    s: &Sl,
) -> Response {
    label_value(ui, label, value_text);
    let r = slider(ui, value, range, s);
    ui.add_space(5.0);
    r
}

pub fn card_slider_f32(ui: &mut Ui, label: &str, value_text: &str, value: &mut f32, range: (f32, f32), s: &Sl) -> bool {
    let mut v = *value as f64;
    let changed = card_slider(ui, label, value_text, &mut v, range.0 as f64..=range.1 as f64, s).changed();
    if changed {
        *value = v as f32;
    }
    changed
}

/// Toggle on the right of a labelled row (the design's 68x32 switch).
pub fn toggle_row(ui: &mut Ui, label: &str, on: &mut bool, h: f32) -> bool {
    row(ui, label, h, |ui| widgets::toggle(ui, on).changed())
}

/// A toggle drawn disabled, with a tooltip saying why.
pub fn disabled_toggle(ui: &mut Ui, why: &str) {
    let p = pal(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(68.0, 32.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(6), p.bg_b2);
    let inner = rect.shrink(3.0);
    let label = Rect::from_min_size(pos2(inner.max.x - 33.0, inner.min.y), vec2(33.0, 26.0));
    painter.rect_filled(label, CornerRadius::same(3), p.bg_a5);
    painter.text(label.center(), Align2::CENTER_CENTER, "OFF", font::regular(12.0), p.fg_a4);
    painter.rect_filled(Rect::from_min_size(inner.min, vec2(26.0, 26.0)), CornerRadius::same(3), p.bg_a1);
    resp.on_hover_text(why);
}

// ---- small buttons -------------------------------------------------------------------------

/// A bare 24pt icon that lights up on hover (panel header actions).
pub fn icon_action(ui: &mut Ui, icon: Icon, tip: &str, active: bool) -> Response {
    let p = pal(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::click());
    let fg = if active {
        p.accent
    } else if resp.hovered() {
        p.fg2
    } else {
        p.fg_a2
    };
    if resp.hovered() {
        ui.painter().rect_filled(rect.expand(3.0), CornerRadius::same(4), p.hover);
    }
    if icon == Icon::Add {
        paint_plus(ui, rect, fg);
    } else {
        widgets::paint_icon(ui, rect, icon, fg);
    }
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if tip.is_empty() { resp } else { resp.on_hover_text(tip) }
}

/// The "add" glyph drawn directly: the exported SVG's vertical bar is black,
/// which the icon loader treats as a cut-out.
pub fn paint_plus(ui: &Ui, rect: Rect, color: Color32) {
    let c = rect.center();
    let h = rect.width() * 7.0 / 24.0;
    let s = Stroke::new(1.5, color);
    ui.painter().line_segment([pos2(c.x - h, c.y), pos2(c.x + h, c.y)], s);
    ui.painter().line_segment([pos2(c.x, c.y - h), pos2(c.x, c.y + h)], s);
}

/// The centred text button at the foot of a popover ("Show Preferences").
pub fn foot_button(ui: &mut Ui, icon: Option<Icon>, text: &str) -> Response {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click());
    let col = if resp.hovered() { p.fg_a1 } else { p.fg_a3 };
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(4), p.hover);
    }
    let galley = ui.painter().layout_no_wrap(text.to_string(), font::regular(14.0), col);
    let iw = if icon.is_some() { 26.0 } else { 0.0 };
    let x0 = rect.center().x - (galley.size().x + iw) / 2.0;
    if let Some(i) = icon {
        widgets::paint_icon(ui, Rect::from_min_size(pos2(x0, rect.center().y - 10.0), Vec2::splat(20.0)), i, col);
    }
    ui.painter().galley(pos2(x0 + iw, rect.center().y - galley.size().y / 2.0), galley, col);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A small text button in the panels' accent-tinted style ("Mark", "Apply").
pub fn chip(ui: &mut Ui, text: &str, tip: &str) -> Response {
    let p = pal(ui.ctx());
    let galley = ui.painter().layout_no_wrap(text.to_string(), font::regular(14.0), p.accent_dim);
    let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x + 24.0, 32.0), Sense::click());
    let bg = if resp.hovered() { mix(p.bg_accent, p.accent_dim, 0.2) } else { p.bg_accent };
    ui.painter().rect_filled(rect, CornerRadius::same(4), bg);
    let col = if resp.hovered() { p.accent_bright } else { p.accent_dim };
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, col);
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if tip.is_empty() { resp } else { resp.on_hover_text(tip) }
}

// ---- colour box ------------------------------------------------------------------------------

fn rgb_color(rgb: u32) -> Color32 {
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// "#RRGGBB" in a value box with a 22pt swatch that opens a picker.
pub fn color_box(ui: &mut Ui, id: impl std::hash::Hash + Copy, rgb: &mut u32, size: Vec2) -> bool {
    let p = pal(ui.ctx());
    let mut changed = false;
    let shown = format!("#{:06X}", *rgb & 0xFFFFFF);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    if let Some(BoxEvent::Commit(t)) = value_box_in(
        ui,
        rect,
        Rect::from_min_max(rect.min + vec2(12.0, 0.0), rect.max - vec2(42.0, 0.0)),
        resp.hovered(),
        id,
        &shown,
    ) && let Ok(v) = u32::from_str_radix(t.trim().trim_start_matches('#'), 16)
    {
        *rgb = v & 0xFFFFFF;
        changed = true;
    }
    let sw = Rect::from_center_size(pos2(rect.max.x - 18.0, rect.center().y), Vec2::splat(22.0));
    let resp = ui.interact(sw, ui.make_persistent_id(id).with("swatch"), Sense::click());
    ui.painter().rect_filled(sw, CornerRadius::same(2), rgb_color(*rgb));
    ui.painter().rect_stroke(
        sw,
        CornerRadius::same(2),
        Stroke::new(1.5, Color32::from_white_alpha(54)),
        StrokeKind::Inside,
    );
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    egui::Popup::menu(&resp)
        .frame(list_frame(&p).inner_margin(egui::Margin::same(8)))
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            let mut c = rgb_color(*rgb);
            if egui::widgets::color_picker::color_picker_color32(ui, &mut c, egui::color_picker::Alpha::Opaque) {
                *rgb = ((c.r() as u32) << 16) | ((c.g() as u32) << 8) | c.b() as u32;
                changed = true;
            }
        });
    changed
}

/// The quick-settings footer preset box: a plain BG/B2 bar with a chevron.
pub fn preset_box(ui: &mut Ui, text: &str) -> Response {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 42.0), Sense::click());
    ui.painter().rect_filled(rect, CornerRadius::same(6), if resp.hovered() { p.bg_a5 } else { p.bg_b2 });
    widgets::text_clipped(
        ui,
        pos2(rect.min.x + 16.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        font::regular(14.0),
        p.fg_a2,
        rect.width() - 70.0,
    );
    widgets::paint_icon(
        ui,
        Rect::from_center_size(pos2(rect.max.x - 28.0, rect.center().y), Vec2::splat(24.0)),
        Icon::ChevronUp,
        p.fg_a2,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}
