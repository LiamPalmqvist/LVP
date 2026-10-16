//! The 34pt header row above every library page: back/forward, breadcrumbs,
//! item count, sort and view options, search, add and the page menu.

use egui::{Align2, Color32, Rect, Response, Sense, Ui, pos2, vec2};

use crate::app::App;
use crate::domain::browse::SortOption;
use crate::domain::library::ViewMode;
use crate::ui::icons::Icon;
use crate::ui::state::UiState;
use crate::ui::theme::{font, with_alpha};
use crate::ui::widgets::{self, pal};

use super::Page;
use super::menus;
use super::model::{PageModel, items_label};

pub const HEIGHT: f32 = 34.0;

/// A 24pt icon drawn at `rect` that lights up on hover.
pub fn icon_at(ui: &mut Ui, rect: Rect, icon: Icon, enabled: bool, active: bool, tooltip: &str) -> Response {
    let p = pal(ui.ctx());
    let id = ui.id().with(("lib-icon", tooltip, rect.min.x as i32, rect.min.y as i32));
    let resp = ui.interact(rect.expand(4.0), id, if enabled { Sense::click() } else { Sense::hover() });
    if enabled && resp.hovered() {
        ui.painter().rect_filled(rect.expand(4.0), 4, p.hover);
    }
    let fg = if !enabled {
        with_alpha(p.fg_a2, 0.3)
    } else if active {
        p.accent
    } else if resp.hovered() {
        p.fg2
    } else {
        p.fg_a2
    };
    super::cards::icon(ui, rect, icon, fg);
    if enabled {
        let r = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        if tooltip.is_empty() { r } else { r.on_hover_text(tooltip) }
    } else {
        resp
    }
}

/// Back and forward arrows (24pt, 12 apart); returns their combined width.
/// With no history, Back goes up to the parent breadcrumb.
pub fn nav_arrows(ui: &mut Ui, st: &mut UiState, crumbs: &[(String, Option<Page>)], x: f32, cy: f32) -> f32 {
    let back = Rect::from_min_size(pos2(x, cy - 12.0), vec2(24.0, 24.0));
    let up = crumbs.iter().rev().skip(1).find_map(|c| c.1.clone()).filter(|p| *p != st.page);
    if icon_at(ui, back, Icon::ArrowLeft, !st.back.is_empty() || up.is_some(), false, "Back").clicked() {
        go_back_or_up(st, up);
    }
    let fwd = back.translate(vec2(36.0, 0.0));
    if icon_at(ui, fwd, Icon::ArrowRight, !st.forward.is_empty(), false, "Forward").clicked() {
        st.go_forward();
    }
    60.0
}

pub fn go_back_or_up(st: &mut UiState, up: Option<Page>) {
    if !st.back.is_empty() {
        st.go_back();
    } else if let Some(p) = up {
        st.navigate(p);
    }
}

/// Breadcrumbs from `x`, at most `max_w` wide, vertically centred on `cy`.
/// Leading segments collapse into "..." when the path does not fit.
pub fn crumbs(ui: &mut Ui, st: &mut UiState, crumbs: &[(String, Option<Page>)], x: f32, cy: f32, max_w: f32) {
    let p = pal(ui.ctx());
    let n = crumbs.len();
    let fonts: Vec<egui::FontId> =
        (0..n).map(|i| if i + 1 == n && n > 1 { font::semibold(16.0) } else { font::regular(16.0) }).collect();
    let widths: Vec<f32> = crumbs
        .iter()
        .zip(&fonts)
        .map(|((t, _), f)| ui.painter().layout_no_wrap(t.clone(), f.clone(), Color32::WHITE).size().x)
        .collect();
    const SEP: f32 = 8.0 + 12.0;
    let ell_w = ui.painter().layout_no_wrap("...".into(), font::regular(16.0), Color32::WHITE).size().x + SEP;
    let total = |from: usize| -> f32 {
        widths[from..].iter().sum::<f32>()
            + SEP * (n - from).saturating_sub(1) as f32
            + if from > 0 { ell_w } else { 0.0 }
    };
    let mut first = 0;
    while first + 1 < n && total(first) > max_w {
        first += 1;
    }
    let mut cx = x;
    let sep = |ui: &Ui, cx: f32| {
        let c = pos2(cx + 6.0 + 4.0, cy);
        let tri = vec![pos2(c.x - 2.5, c.y - 3.5), pos2(c.x + 3.0, c.y), pos2(c.x - 2.5, c.y + 3.5)];
        ui.painter().add(egui::Shape::convex_polygon(tri, p.fg_a4, egui::Stroke::NONE));
    };
    if first > 0 {
        ui.painter().text(pos2(cx, cy), Align2::LEFT_CENTER, "...", font::regular(16.0), p.fg_a2);
        cx += ell_w - SEP;
        sep(ui, cx);
        cx += SEP;
    }
    for i in first..n {
        let (label, page) = &crumbs[i];
        let last = i + 1 == n;
        let w = widths[i].min((x + max_w - cx).max(0.0));
        let r = Rect::from_min_size(pos2(cx, cy - 12.0), vec2(w, 24.0));
        let clickable = page.is_some() && !last;
        let resp = ui.interact(r, ui.id().with(("crumb", i)), if clickable { Sense::click() } else { Sense::hover() });
        let col = if last && n > 1 {
            p.fg_a2
        } else if clickable && resp.hovered() {
            p.fg2
        } else {
            p.fg_a2
        };
        widgets::text_clipped(ui, pos2(cx, cy), Align2::LEFT_CENTER, label, fonts[i].clone(), col, w.max(8.0));
        if clickable && resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            st.navigate(page.clone().unwrap_or_default());
        }
        cx += w;
        if !last {
            sep(ui, cx);
            cx += SEP;
        }
    }
}

pub fn show(ui: &mut Ui, app: &mut App, st: &mut UiState, rect: Rect, m: &PageModel) {
    let p = pal(ui.ctx());
    let cy = rect.center().y;
    let arrows = nav_arrows(ui, st, &m.crumbs, rect.min.x, cy);

    // Right-hand controls, laid out from the right edge.
    let mut x = rect.max.x;
    let more_r = Rect::from_min_size(pos2(x - 24.0, cy - 12.0), vec2(24.0, 24.0));
    let more = icon_at(ui, more_r, Icon::More, true, false, "More");
    menus::page_menu(&more, app, st, m);
    x -= 24.0;
    if let Some((tip, act)) = m.add {
        x -= 12.0;
        let r = Rect::from_min_size(pos2(x - 24.0, cy - 12.0), vec2(24.0, 24.0));
        if icon_at(ui, r, Icon::Add, true, false, tip).clicked() {
            super::view::run_act(app, st, act);
        }
        x -= 24.0;
    }
    x -= 20.0;
    let search_w = 325.0f32.min((rect.width() * 0.3).max(160.0));
    let sr = Rect::from_min_max(pos2(x - search_w, rect.min.y), pos2(x, rect.max.y));
    ui.scope_builder(egui::UiBuilder::new().max_rect(sr), |ui| {
        widgets::search_field(ui, &mut st.library.search, "Search", search_w);
    });
    x = sr.min.x - 20.0;
    if m.viewable {
        let r = Rect::from_min_size(pos2(x - 24.0, cy - 12.0), vec2(24.0, 24.0));
        let icon = if st.library.view == ViewMode::Grid { Icon::Grid } else { Icon::List };
        let resp = icon_at(ui, r, icon, true, false, "View options");
        view_popup(&resp, st);
        x -= 44.0;
    }
    if m.sortable {
        let r = Rect::from_min_size(pos2(x - 24.0, cy - 12.0), vec2(24.0, 24.0));
        let icon = if st.library.sort.is_some() { Icon::SortActive } else { Icon::Sort };
        let resp = icon_at(ui, r, icon, true, false, "Sorting options");
        sort_popup(&resp, st);
        x -= 44.0;
    }
    let count = items_label(m.count());
    let cr = ui.painter().text(pos2(x, cy), Align2::RIGHT_CENTER, count, font::regular(14.0), p.fg_a3);

    let cx = rect.min.x + arrows + 10.0;
    crumbs(ui, st, &m.crumbs, cx, cy, (cr.min.x - 20.0 - cx).max(40.0));
}

/// "Sorting Options" (856:2215). Picking the checked option again returns
/// to the page's natural order.
pub fn sort_popup(button: &Response, st: &mut UiState) {
    let p = pal(&button.ctx);
    egui::Popup::menu(button).frame(menus::frame(&p)).width(265.0).gap(6.0).show(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        for &opt in SortOption::ALL {
            let on = st.library.sort == Some(opt);
            if menus::row(ui, None, opt.label(), on, false).clicked() {
                st.library.sort = if on { None } else { Some(opt) };
                st.library.invalidate();
            }
        }
    });
}

/// "View Options" (858:3290): grid or list.
pub fn view_popup(button: &Response, st: &mut UiState) {
    let p = pal(&button.ctx);
    egui::Popup::menu(button).frame(menus::frame(&p)).width(265.0).gap(6.0).show(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        for (mode, icon, label) in
            [(ViewMode::Grid, Icon::Grid, "Grid View"), (ViewMode::List, Icon::List, "List View")]
        {
            if menus::row(ui, Some(icon), label, st.library.view == mode, false).clicked() {
                st.library.view = mode;
            }
        }
    });
}
