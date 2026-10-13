//! Building blocks for the settings pages.
//!
//! Every page is one function over a [`Form`]. In draw mode the form lays out
//! rows the way the design does (label and info bubble on the left, a 320pt
//! control on the right); in count mode, used by search, it only counts the
//! rows whose label or explanation matches the query. One page definition
//! therefore serves both the page itself and the search results.

use std::ops::RangeInclusive;

use egui::{
    Align, Align2, Color32, CornerRadius, Id, Layout, Rect, Response, RichText, Sense, Stroke, StrokeKind, TextEdit,
    Ui, UiBuilder, Vec2, pos2, vec2,
};

use crate::ui::icons::Icon;
use crate::ui::theme::{font, mix, with_alpha};
use crate::ui::widgets::{self, paint_icon, pal};

/// Width of every right-hand control.
pub const CONTROL_W: f32 = 320.0;
/// Row heights from the design: 32pt controls, 36pt fields, labelled sliders.
pub const ROW_TOGGLE: f32 = 42.0;
pub const ROW_FIELD: f32 = 46.0;
pub const ROW_SLIDER: f32 = 57.0;
pub const ROW_NUMBER: f32 = 48.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Draw,
    Count,
}

pub struct Form<'u> {
    pub ui: &'u mut Ui,
    mode: Mode,
    query: String,
    /// Rows that matched the query (all rows when not searching).
    pub hits: usize,
    /// No row or section drawn yet in the current group.
    first: bool,
    /// The page's "Advanced Options" are expanded.
    pub advanced: bool,
}

impl<'u> Form<'u> {
    pub fn new(ui: &'u mut Ui, mode: Mode, query: &str, advanced: bool) -> Self {
        Self { ui, mode, query: query.trim().to_lowercase(), hits: 0, first: true, advanced }
    }

    pub fn searching(&self) -> bool {
        !self.query.is_empty()
    }

    pub fn drawing(&self) -> bool {
        self.mode == Mode::Draw
    }

    /// Whether a row with this label and explanation is part of the results.
    pub fn matches(&self, label: &str, info: Option<&str>) -> bool {
        self.query.is_empty()
            || label.to_lowercase().contains(&self.query)
            || info.is_some_and(|i| i.to_lowercase().contains(&self.query))
    }

    /// Section heading (Medium 16, FG/2) with the design's 20pt group gap.
    pub fn section(&mut self, title: &str) {
        if self.searching() || !self.drawing() {
            return;
        }
        if !self.first {
            self.ui.add_space(20.0);
        }
        self.first = false;
        heading(self.ui, title);
    }

    /// A row: label, optional info bubble, and `control` laid out right to
    /// left in the remaining space. Returns `None` when the row is filtered
    /// out or the form is only counting.
    pub fn row<R>(
        &mut self,
        label: &str,
        info: Option<&str>,
        height: f32,
        control: impl FnOnce(&mut Ui) -> R,
    ) -> Option<R> {
        if !self.matches(label, info) {
            return None;
        }
        self.hits += 1;
        if !self.drawing() {
            return None;
        }
        self.first = false;
        Some(row_ui(self.ui, label, info, height, control))
    }

    /// Free-form content that is listed in search results when `keywords`
    /// match.
    pub fn block(&mut self, keywords: &str, add: impl FnOnce(&mut Ui)) {
        if !self.matches(keywords, None) {
            return;
        }
        self.hits += 1;
        if self.drawing() {
            self.first = false;
            add(self.ui);
        }
    }

    /// The "Advanced Options" bar; `add` draws the rows it reveals. While
    /// searching, advanced rows are always included.
    pub fn advanced(&mut self, add: impl FnOnce(&mut Self)) {
        if self.searching() || !self.drawing() {
            add(self);
            return;
        }
        self.ui.add_space(20.0);
        if advanced_bar(self.ui, self.advanced).clicked() {
            self.advanced = !self.advanced;
        }
        if self.advanced {
            self.ui.add_space(20.0);
            self.first = true;
            add(self);
        }
    }

    // ---- typed rows ----------------------------------------------------------

    pub fn toggle(&mut self, label: &str, info: Option<&str>, v: &mut bool) -> bool {
        self.row(label, info, ROW_TOGGLE, |ui| widgets::toggle(ui, v).changed()).unwrap_or(false)
    }

    pub fn choice<T: Copy + PartialEq + 'static>(
        &mut self,
        label: &str,
        info: Option<&str>,
        v: &mut T,
        all: &[T],
        name: impl Fn(T) -> &'static str,
    ) -> bool {
        self.row(label, info, ROW_FIELD, |ui| widgets::choice(ui, label, v, all, name)).unwrap_or(false)
    }

    /// A dropdown over arbitrary options; returns the chosen index.
    pub fn dropdown(&mut self, label: &str, info: Option<&str>, selected: &str, options: &[String]) -> Option<usize> {
        self.row(label, info, ROW_FIELD, |ui| widgets::dropdown(ui, label, selected, options, CONTROL_W)).flatten()
    }

    /// A dropdown that stores the chosen option's text.
    pub fn pick(&mut self, label: &str, info: Option<&str>, v: &mut String, options: &[&str]) -> bool {
        let opts: Vec<String> = options.iter().map(|s| s.to_string()).collect();
        match self.dropdown(label, info, v, &opts) {
            Some(i) if opts[i] != *v => {
                *v = opts[i].clone();
                true
            }
            _ => false,
        }
    }

    pub fn segmented(&mut self, label: &str, info: Option<&str>, sel: &mut usize, labels: &[&str]) -> bool {
        self.row(label, info, ROW_FIELD, |ui| widgets::segmented(ui, sel, labels, CONTROL_W)).unwrap_or(false)
    }

    pub fn slider(
        &mut self,
        label: &str,
        info: Option<&str>,
        v: &mut f32,
        range: RangeInclusive<f32>,
        spec: &SliderSpec,
    ) -> bool {
        let h = if spec.labels.is_empty() { ROW_TOGGLE } else { ROW_SLIDER };
        self.row(label, info, h, |ui| slider(ui, v, range, spec, CONTROL_W).changed()).unwrap_or(false)
    }

    pub fn number(
        &mut self,
        label: &str,
        info: Option<&str>,
        v: &mut f64,
        range: RangeInclusive<f64>,
        decimals: usize,
        unit: &str,
    ) -> bool {
        self.row(label, info, ROW_NUMBER, |ui| number_field(ui, v, range, decimals, unit, CONTROL_W)).unwrap_or(false)
    }

    /// Integer convenience over [`Form::number`].
    pub fn int<T>(&mut self, label: &str, info: Option<&str>, v: &mut T, range: RangeInclusive<i64>, unit: &str) -> bool
    where
        T: Copy + TryFrom<i64> + Into<i64>,
    {
        let mut f = (*v).into() as f64;
        if self.number(label, info, &mut f, *range.start() as f64..=*range.end() as f64, 0, unit)
            && let Ok(n) = T::try_from(f.round() as i64)
        {
            *v = n;
            return true;
        }
        false
    }

    pub fn text(&mut self, label: &str, info: Option<&str>, v: &mut String, hint: &str) -> bool {
        self.row(label, info, ROW_FIELD, |ui| text_field(ui, v, hint, CONTROL_W, false).changed()).unwrap_or(false)
    }

    pub fn password(&mut self, label: &str, info: Option<&str>, v: &mut String, hint: &str) -> bool {
        self.row(label, info, ROW_FIELD, |ui| text_field(ui, v, hint, CONTROL_W, true).changed()).unwrap_or(false)
    }

    /// A path field with a Browse button that picks a folder or a file.
    pub fn path(&mut self, label: &str, info: Option<&str>, v: &mut String, pick: Pick) -> bool {
        self.row(label, info, ROW_FIELD, |ui| path_field(ui, v, pick)).unwrap_or(false)
    }

    /// A row whose control is a single button; true when it was clicked.
    pub fn action(&mut self, label: &str, info: Option<&str>, button: &str, kind: Btn) -> bool {
        self.row(label, info, ROW_TOGGLE, |ui| action_button(ui, button, kind, 100.0).clicked()).unwrap_or(false)
    }

    /// Like [`Form::action`] with a button as wide as the other controls.
    pub fn action_wide(&mut self, label: &str, info: Option<&str>, button: &str, kind: Btn) -> bool {
        self.row(label, info, ROW_TOGGLE, |ui| action_button(ui, button, kind, CONTROL_W).clicked()).unwrap_or(false)
    }

    pub fn color(&mut self, label: &str, info: Option<&str>, v: &mut u32) -> bool {
        self.row(label, info, ROW_TOGGLE, |ui| color_field(ui, v, CONTROL_W)).unwrap_or(false)
    }
}

// ---- layout pieces ------------------------------------------------------------

/// Section heading: 39pt tall, Medium 16 in FG/2.
pub fn heading(ui: &mut Ui, title: &str) {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 39.0), Sense::hover());
    ui.painter().text(pos2(rect.min.x, rect.min.y + 10.0), Align2::LEFT_TOP, title, font::medium(16.0), p.fg2);
}

/// Draws one settings row into a fresh full-width rect.
pub fn row_ui<R>(ui: &mut Ui, label: &str, info: Option<&str>, height: f32, control: impl FnOnce(&mut Ui) -> R) -> R {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let galley = ui.painter().layout_no_wrap(label.to_string(), font::regular(14.0), p.fg_a2);
    let lw = galley.size().x;
    ui.painter().galley(pos2(rect.min.x, rect.center().y - galley.size().y / 2.0), galley, p.fg_a2);
    if let Some(text) = info {
        let ir = Rect::from_min_size(pos2(rect.min.x + lw + 10.0, rect.center().y - 9.0), Vec2::splat(18.0));
        let id = ui.id().with(("info", label, rect.min.y as i32));
        let resp = ui.interact(ir.expand(3.0), id, Sense::hover());
        paint_icon(ui, ir, Icon::Info, if resp.hovered() { p.fg1 } else { p.fg_a2 });
        resp.on_hover_ui(|ui| {
            ui.set_max_width(300.0);
            ui.label(RichText::new(text).font(font::regular(13.0)).color(p.fg1));
        });
    }
    let mut child = ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::right_to_left(Align::Center)));
    child.spacing_mut().item_spacing = vec2(10.0, 0.0);
    control(&mut child)
}

/// The full-width "Advanced Options >" bar.
pub fn advanced_bar(ui: &mut Ui, open: bool) -> Response {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 38.0), Sense::click());
    let bg = if resp.hovered() { p.bg_a3 } else { p.bg_a5 };
    ui.painter().rect_filled(rect, CornerRadius::same(2), bg);
    let col = if resp.hovered() {
        p.fg1
    } else if open {
        p.fg_a3
    } else {
        p.fg_a2
    };
    let galley = ui.painter().layout_no_wrap("Advanced Options".into(), font::regular(14.0), col);
    let total = galley.size().x + 10.0 + 24.0;
    let x = rect.center().x - total / 2.0;
    let gy = rect.center().y - galley.size().y / 2.0;
    let gw = galley.size().x;
    ui.painter().galley(pos2(x, gy), galley, col);
    let chev = if open { Icon::ChevronDown } else { Icon::ChevronRight };
    paint_icon(ui, Rect::from_min_size(pos2(x + gw + 10.0, rect.center().y - 12.0), Vec2::splat(24.0)), chev, col);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

// ---- buttons -------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Btn {
    /// Raised grey (Browse, Rescan, Add).
    Normal,
    /// Accent text on the dark accent tint (Open).
    Accent,
    /// Red text (Reset).
    Danger,
}

/// The design's 32pt action button (SemiBold 14), at least `min_w` wide.
pub fn action_button(ui: &mut Ui, text: &str, kind: Btn, min_w: f32) -> Response {
    let p = pal(ui.ctx());
    let galley = ui.painter().layout_no_wrap(text.to_string(), font::semibold(14.0), Color32::PLACEHOLDER);
    let w = (galley.size().x + 40.0).max(min_w);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click());
    let hovered = resp.hovered();
    let (bg, fg) = match kind {
        Btn::Normal => (if hovered { p.bg_a1 } else { p.bg_a4 }, if hovered { p.fg1 } else { p.fg_a2 }),
        Btn::Accent => {
            (if hovered { p.bg_accent } else { p.bg_accent_dark }, if hovered { p.accent_bright } else { p.accent })
        }
        Btn::Danger => (
            with_alpha(p.danger, if hovered { 0.16 } else { 0.06 }),
            if hovered { p.danger } else { mix(p.danger, p.accent, 0.3) },
        ),
    };
    ui.painter().rect_filled(rect, CornerRadius::same(6), bg);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, fg);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

// ---- slider ---------------------------------------------------------------------

/// Tick marks and value labels for [`slider`].
#[derive(Clone, Default)]
pub struct SliderSpec {
    /// Number of intervals between tick marks (0 for none).
    pub ticks: usize,
    /// Labels under the track: (value, text).
    pub labels: Vec<(f32, String)>,
    /// Snap step (0 for continuous).
    pub step: f32,
    /// Where the fill starts (the range start when `None`).
    pub origin: Option<f32>,
}

impl SliderSpec {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn ticks(mut self, n: usize) -> Self {
        self.ticks = n;
        self
    }
    pub fn step(mut self, s: f32) -> Self {
        self.step = s;
        self
    }
    pub fn origin(mut self, o: f32) -> Self {
        self.origin = Some(o);
        self
    }
    pub fn labels(mut self, l: &[(f32, &str)]) -> Self {
        self.labels = l.iter().map(|(v, t)| (*v, t.to_string())).collect();
        self
    }
}

/// The settings slider: a 16pt black track with an accent-tinted fill, an
/// 8x24 thumb and Regular 12 labels underneath.
pub fn slider(ui: &mut Ui, v: &mut f32, range: RangeInclusive<f32>, spec: &SliderSpec, width: f32) -> Response {
    let p = pal(ui.ctx());
    let h = if spec.labels.is_empty() { 24.0 } else { 47.0 };
    let (outer, mut resp) = ui.allocate_exact_size(vec2(width, h), Sense::click_and_drag());
    let track = Rect::from_min_size(outer.min + vec2(0.0, 4.0), vec2(width, 16.0));
    let inner = track.shrink(2.0);
    let (lo, hi) = (*range.start(), *range.end());
    let span = (hi - lo).max(f32::EPSILON);
    let x_of = |val: f32| egui::lerp(inner.min.x + 4.0..=inner.max.x - 4.0, ((val - lo) / span).clamp(0.0, 1.0));
    let snap = |val: f32| {
        let val = if spec.step > 0.0 { lo + ((val - lo) / spec.step).round() * spec.step } else { val };
        val.clamp(lo, hi)
    };
    if (resp.dragged() || resp.clicked())
        && let Some(pos) = resp.interact_pointer_pos()
    {
        let t = ((pos.x - inner.min.x - 4.0) / (inner.width() - 8.0)).clamp(0.0, 1.0);
        let nv = snap(lo + t * span);
        if nv != *v {
            *v = nv;
            resp.mark_changed();
        }
    }
    if resp.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y + i.raw_scroll_delta.x);
        if scroll.abs() > 0.5 {
            let step = if spec.step > 0.0 { spec.step } else { span / 100.0 };
            let nv = snap(*v + step * scroll.signum());
            if nv != *v {
                *v = nv;
                resp.mark_changed();
            }
        }
    }
    let painter = ui.painter();
    painter.rect_filled(track, CornerRadius::same(6), p.bg_b2);
    let x = x_of(*v);
    let fill = match spec.origin {
        None => Rect::from_min_max(inner.min, pos2(x, inner.max.y)),
        Some(o) => {
            let ox = x_of(o);
            Rect::from_min_max(pos2(ox.min(x), inner.min.y), pos2(ox.max(x), inner.max.y))
        }
    };
    if fill.width() > 0.5 {
        painter.rect_filled(fill, CornerRadius::same(4), p.bg_accent);
    }
    if spec.ticks > 1 {
        for i in 1..spec.ticks {
            let tx = egui::lerp(inner.min.x..=inner.max.x, i as f32 / spec.ticks as f32);
            painter.line_segment(
                [pos2(tx, inner.center().y - 3.0), pos2(tx, inner.center().y + 3.0)],
                Stroke::new(2.0, p.bg_accent_dark),
            );
        }
    }
    let thumb = Rect::from_center_size(pos2(x, track.center().y), vec2(8.0, 24.0));
    let tc = if resp.hovered() || resp.dragged() { mix(p.accent_dim, p.accent_bright, 0.5) } else { p.accent_dim };
    painter.rect_filled(thumb, CornerRadius::same(4), tc);
    for (lv, text) in &spec.labels {
        let (pos, align) = if (*lv - lo).abs() < 1e-6 {
            (pos2(track.min.x, track.max.y + 8.0), Align2::LEFT_TOP)
        } else if (*lv - hi).abs() < 1e-6 {
            (pos2(track.max.x, track.max.y + 8.0), Align2::RIGHT_TOP)
        } else {
            (pos2(x_of(*lv), track.max.y + 8.0), Align2::CENTER_TOP)
        };
        painter.text(pos, align, text, font::regular(12.0), p.fg_a3);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

// ---- fields ------------------------------------------------------------------------

/// Paints the field well: black fill with a 2pt BG/A4 border.
fn field_bg(ui: &Ui, rect: Rect, focused: bool) {
    let p = pal(ui.ctx());
    ui.painter().rect_filled(rect, CornerRadius::same(6), p.bg_b2);
    let stroke = if focused { with_alpha(p.accent, 0.6) } else { p.bg_a4 };
    ui.painter().rect_stroke(rect, CornerRadius::same(6), Stroke::new(2.0, stroke), StrokeKind::Inside);
}

pub fn text_field(ui: &mut Ui, v: &mut String, hint: &str, width: f32, password: bool) -> Response {
    let p = pal(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
    let id = ui.id().with(("text", rect.min.y as i32, rect.min.x as i32));
    let focused = ui.memory(|m| m.has_focus(id));
    field_bg(ui, rect, focused);
    let inner = Rect::from_min_max(pos2(rect.min.x + 12.0, rect.min.y), pos2(rect.max.x - 12.0, rect.max.y));
    let edit = TextEdit::singleline(v)
        .id(id)
        .password(password)
        .hint_text(RichText::new(hint).color(p.fg_a4))
        .frame(false)
        .text_color(p.fg_a2)
        .font(font::regular(14.0))
        .vertical_align(Align::Center)
        .margin(egui::Margin::ZERO)
        .desired_width(inner.width());
    ui.put(inner, edit)
}

/// Text field plus Browse button.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pick {
    Folder,
    File,
}

fn path_field(ui: &mut Ui, v: &mut String, pick: Pick) -> bool {
    let mut changed = false;
    if action_button(ui, "Browse", Btn::Normal, 100.0).clicked() {
        let dlg = rfd::FileDialog::new();
        let start = crate::app::expand_home(v);
        let dlg = if start.is_empty() { dlg } else { dlg.set_directory(&start) };
        let picked = match pick {
            Pick::Folder => dlg.pick_folder(),
            Pick::File => dlg.save_file(),
        };
        if let Some(path) = picked {
            *v = path.to_string_lossy().into_owned();
            changed = true;
        }
    }
    changed |= text_field(ui, v, "", CONTROL_W - 110.0, false).changed();
    changed
}

/// A numeric field showing "value unit". Typing parses as you go; the mouse
/// wheel steps by one unit of the last decimal.
pub fn number_field(
    ui: &mut Ui,
    v: &mut f64,
    range: RangeInclusive<f64>,
    decimals: usize,
    unit: &str,
    width: f32,
) -> bool {
    let p = pal(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
    let id = ui.id().with(("num", rect.min.y as i32, rect.min.x as i32));
    let buf_id = id.with("buf");
    let focused = ui.memory(|m| m.has_focus(id));
    field_bg(ui, rect, focused);
    let fmt = |x: f64| format!("{x:.decimals$}");
    let mut text = if focused { ui.data(|d| d.get_temp::<String>(buf_id)).unwrap_or_else(|| fmt(*v)) } else { fmt(*v) };
    let galley = ui.painter().layout_no_wrap(text.clone(), font::regular(14.0), p.fg_a2);
    let text_w = (galley.size().x + 6.0).min(width - 60.0);
    let inner = Rect::from_min_max(pos2(rect.min.x + 12.0, rect.min.y), pos2(rect.max.x - 12.0, rect.max.y));
    let edit_w = if unit.is_empty() || focused { inner.width() } else { text_w };
    let edit = TextEdit::singleline(&mut text)
        .id(id)
        .frame(false)
        .text_color(p.fg_a2)
        .font(font::regular(14.0))
        .vertical_align(Align::Center)
        .margin(egui::Margin::ZERO)
        .desired_width(edit_w);
    let er = ui.put(Rect::from_min_size(inner.min, vec2(edit_w, inner.height())), edit);
    let mut changed = false;
    if er.changed()
        && let Ok(n) = text.trim().trim_end_matches(unit).trim().replace(',', ".").parse::<f64>()
    {
        let n = n.clamp(*range.start(), *range.end());
        if n != *v {
            *v = n;
            changed = true;
        }
    }
    if er.has_focus() {
        ui.data_mut(|d| d.insert_temp(buf_id, text));
    } else {
        ui.data_mut(|d| d.remove::<String>(buf_id));
    }
    if !unit.is_empty() && !er.has_focus() {
        ui.painter().text(
            pos2(inner.min.x + text_w - 2.0, rect.center().y),
            Align2::LEFT_CENTER,
            unit,
            font::regular(14.0),
            p.fg_a2,
        );
    }
    if (resp.hovered() || er.hovered()) && !er.has_focus() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll.abs() > 0.5 {
            let step = 10f64.powi(-(decimals as i32));
            let n = (*v + step * scroll.signum() as f64).clamp(*range.start(), *range.end());
            if n != *v {
                *v = n;
                changed = true;
            }
        }
    }
    changed
}

/// "#RRGGBB" with a swatch that opens a colour picker.
pub fn color_field(ui: &mut Ui, rgb: &mut u32, width: f32) -> bool {
    let p = pal(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(width, 32.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(6), p.bg_b2);
    let id = ui.id().with(("color", rect.min.y as i32));
    let mut changed = false;
    let mut text =
        ui.data(|d| d.get_temp::<String>(id.with("buf"))).unwrap_or_else(|| format!("#{:06X}", *rgb & 0xFFFFFF));
    let inner = Rect::from_min_max(pos2(rect.min.x + 12.0, rect.min.y), pos2(rect.max.x - 40.0, rect.max.y));
    let er = ui.put(
        inner,
        TextEdit::singleline(&mut text)
            .id(id)
            .frame(false)
            .text_color(p.fg_a3)
            .font(font::regular(14.0))
            .vertical_align(Align::Center)
            .margin(egui::Margin::ZERO)
            .desired_width(inner.width()),
    );
    if er.changed()
        && let Ok(v) = u32::from_str_radix(text.trim().trim_start_matches('#'), 16)
        && text.trim().trim_start_matches('#').len() == 6
    {
        *rgb = v & 0xFFFFFF;
        changed = true;
    }
    if er.has_focus() {
        ui.data_mut(|d| d.insert_temp(id.with("buf"), text));
    } else {
        ui.data_mut(|d| d.remove::<String>(id.with("buf")));
    }
    let sw = Rect::from_min_size(pos2(rect.max.x - 27.0, rect.center().y - 11.0), Vec2::splat(22.0));
    let sresp = ui.interact(sw, id.with("swatch"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    let mut c = Color32::from_rgb((*rgb >> 16) as u8, (*rgb >> 8) as u8, *rgb as u8);
    ui.painter().rect_filled(sw, CornerRadius::same(2), c);
    ui.painter().rect_stroke(
        sw,
        CornerRadius::same(2),
        Stroke::new(1.5, with_alpha(Color32::WHITE, if sresp.hovered() { 0.5 } else { 0.21 })),
        StrokeKind::Outside,
    );
    egui::Popup::menu(&sresp)
        .id(id.with("picker"))
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .frame(widgets::menu_frame(&p))
        .show(|ui| {
            if egui::color_picker::color_picker_color32(ui, &mut c, egui::color_picker::Alpha::Opaque) {
                *rgb = ((c.r() as u32) << 16) | ((c.g() as u32) << 8) | c.b() as u32;
                changed = true;
            }
        });
    changed
}

/// A small modal asking to confirm a destructive action. Returns
/// `Some(true)` on confirm, `Some(false)` on cancel.
pub fn confirm(ctx: &egui::Context, id: &str, title: &str, body: &str, ok: &str) -> Option<bool> {
    let p = pal(ctx);
    let mut out = None;
    let resp = egui::Modal::new(Id::new(("settings-confirm", id)))
        .backdrop_color(Color32::from_black_alpha(90))
        .frame(
            egui::Frame::new()
                .fill(p.bg_a3)
                .corner_radius(12)
                .inner_margin(egui::Margin::same(20))
                .stroke(Stroke::new(1.0, p.divider_dim)),
        )
        .show(ctx, |ui| {
            ui.set_width(380.0);
            ui.label(RichText::new(title).font(font::medium(16.0)).color(p.fg1));
            ui.add_space(8.0);
            ui.label(RichText::new(body).font(font::regular(14.0)).color(p.fg_a1));
            ui.add_space(18.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                if action_button(ui, ok, Btn::Danger, 100.0).clicked() {
                    out = Some(true);
                }
                if action_button(ui, "Cancel", Btn::Normal, 100.0).clicked() {
                    out = Some(false);
                }
            });
        });
    if out.is_none() && resp.should_close() {
        out = Some(false);
    }
    out
}

/// Runs `add` in a child placed at `builder`'s rect without moving the
/// parent's cursor (unlike `Ui::scope_builder`), for content drawn inside a
/// rect that was already allocated.
pub fn place<R>(ui: &mut Ui, builder: UiBuilder, add: impl FnOnce(&mut Ui) -> R) -> R {
    let mut child = ui.new_child(builder);
    add(&mut child)
}
