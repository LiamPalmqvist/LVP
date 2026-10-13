//! The compact library column beside the video (frame 187:328 and the
//! "sidebar" component 848:1996): title row, item count with sort/view/
//! search, then a single-column list of rows.

use egui::{Align2, Rect, Sense, Stroke, Ui, pos2, vec2};

use crate::app::App;
use crate::domain::library::ViewMode;
use crate::ui::icons::Icon;
use crate::ui::state::{UiState, View};
use crate::ui::theme::font;
use crate::ui::widgets::{self, pal};

use super::header::{self, icon_at};
use super::model::{PageModel, TileKind, items_label};
use super::{cards, menus, view};

pub fn show(ui: &mut Ui, app: &mut App, st: &mut UiState, m: &PageModel) {
    let p = pal(ui.ctx());
    let rect = ui.max_rect();
    // Title row: arrows, breadcrumbs, "..." and pop-out.
    let cy = rect.min.y + 12.0;
    let arrows = header::nav_arrows(ui, st, &m.crumbs, rect.min.x, cy);
    let pop = Rect::from_min_size(pos2(rect.max.x - 24.0, cy - 12.0), vec2(24.0, 24.0));
    if icon_at(ui, pop, Icon::Open, true, false, "Open in the library").clicked() {
        st.view = View::Library;
    }
    let more = icon_at(ui, pop.translate(vec2(-36.0, 0.0)), Icon::More, true, false, "More");
    menus::page_menu(&more, app, st, m);
    let cx = rect.min.x + arrows + 10.0;
    header::crumbs(ui, st, &m.crumbs, cx, cy, (pop.min.x - 46.0 - cx).max(30.0));

    // Count row with sort, view and search.
    let cy2 = rect.min.y + 56.0;
    ui.painter().text(pos2(rect.min.x, cy2), Align2::LEFT_CENTER, items_label(m.count()), font::regular(14.0), p.fg_a3);
    let search_id = ui.id().with("side-search-open");
    let mut search_open: bool =
        ui.ctx().data(|d| d.get_temp(search_id)).unwrap_or(false) || !st.library.search.is_empty();
    let sr = Rect::from_min_size(pos2(rect.max.x - 24.0, cy2 - 12.0), vec2(24.0, 24.0));
    if icon_at(ui, sr, Icon::Search, true, search_open, "Search").clicked() {
        search_open = !search_open;
        if !search_open {
            st.library.search.clear();
        }
    }
    ui.ctx().data_mut(|d| d.insert_temp(search_id, search_open));
    let vr = sr.translate(vec2(-36.0, 0.0));
    let icon = if st.library.view == ViewMode::Grid { Icon::Grid } else { Icon::List };
    let vresp = icon_at(ui, vr, icon, true, false, "View options");
    header::view_popup(&vresp, st);
    if m.sortable {
        let srt = vr.translate(vec2(-36.0, 0.0));
        let ic = if st.library.sort.is_some() { Icon::SortActive } else { Icon::Sort };
        let sresp = icon_at(ui, srt, ic, true, false, "Sorting options");
        header::sort_popup(&sresp, st);
    }
    let mut top = rect.min.y + 85.0;
    if search_open {
        let r = Rect::from_min_size(pos2(rect.min.x, top - 7.0), vec2(rect.width(), 34.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(r), |ui| {
            widgets::search_field(ui, &mut st.library.search, "Search", rect.width());
        });
        top += 44.0;
    }
    ui.painter().hline(rect.x_range(), top + 0.5, Stroke::new(1.0, p.divider_dim));
    let body = Rect::from_min_max(pos2(rect.min.x, top + 1.0), rect.max);
    ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| rows(ui, app, st, m));
}

fn rows(ui: &mut Ui, app: &mut App, st: &mut UiState, m: &PageModel) {
    let p = pal(ui.ctx());
    if let Some(err) = &m.error {
        ui.add_space(20.0);
        ui.label(egui::RichText::new(err).font(font::regular(14.0)).color(p.fg_a3));
        return;
    }
    if m.loading && m.count() == 0 {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| ui.add(egui::Spinner::new().size(22.0)));
        return;
    }
    egui::ScrollArea::vertical().id_salt("side-rows").auto_shrink(false).show(ui, |ui| {
        let w = ui.available_width();
        ui.add_space(20.0);
        let has_action = m.sections.iter().flat_map(|s| &s.tiles).any(|t| matches!(t.kind, TileKind::Action(_)));
        if let Some((label, act)) = m.add
            && !has_action
        {
            let label = if label == "Add a folder" { "Add" } else { label };
            let (r, resp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(r.expand2(vec2(4.0, 0.0)), 6, p.hover);
            }
            widgets::paint_icon(
                ui,
                Rect::from_center_size(pos2(r.min.x + 16.0, r.center().y), vec2(24.0, 24.0)),
                Icon::Add,
                p.fg_a2,
            );
            ui.painter().text(
                pos2(r.min.x + 42.0, r.center().y),
                Align2::LEFT_CENTER,
                label,
                font::regular(14.0),
                p.fg_a3,
            );
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                view::run_act(app, st, act);
            }
            ui.add_space(10.0);
        }
        let flat = m.main().is_some_and(|s| s.title.is_none());
        for (si, sec) in m.sections.iter().enumerate() {
            if let Some(title) = &sec.title {
                if si > 0 {
                    ui.add_space(14.0);
                }
                let (r, resp) = ui.allocate_exact_size(vec2(w, 24.0), Sense::click());
                let hot = sec.link.is_some() && resp.hovered();
                ui.painter().text(
                    r.left_center(),
                    Align2::LEFT_CENTER,
                    title,
                    font::regular(16.0),
                    if hot { p.fg1 } else { p.fg_a1 },
                );
                if let Some(link) = &sec.link {
                    widgets::paint_icon(
                        ui,
                        Rect::from_min_size(pos2(r.max.x - 24.0, r.min.y), vec2(24.0, 24.0)),
                        Icon::ChevronRight,
                        p.fg_a2,
                    );
                    if resp.clicked() {
                        st.navigate(link.clone());
                    }
                }
                ui.add_space(16.0);
            }
            for (ti, t) in sec.tiles.iter().enumerate() {
                let (r, resp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click());
                if ui.is_rect_visible(r) {
                    let item = match t.kind {
                        TileKind::Media(i) => sec.playable.get(i),
                        _ => None,
                    };
                    if let Some(item) = item {
                        view::want_meta(app, st, item);
                    }
                    let selected = flat && st.library.selected == Some(ti);
                    cards::list_row(ui, app, r, t, item, resp.hovered(), selected, false);
                    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                    if resp.clicked() {
                        if flat {
                            st.library.selected = Some(ti);
                        }
                        if item.is_none() {
                            view::activate(app, st, sec, ti);
                        }
                    }
                    if resp.double_clicked() && item.is_some() {
                        view::activate(app, st, sec, ti);
                    }
                    view::tile_menu(&resp, app, st, m, sec, ti);
                }
                ui.add_space(10.0);
            }
        }
        if m.count() == 0 && m.add.is_none() {
            ui.label(egui::RichText::new(m.empty.0).font(font::regular(14.0)).color(p.fg_a3));
        }
        ui.add_space(20.0);
    });
}
