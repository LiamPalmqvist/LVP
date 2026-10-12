//! Library navigation: the 50pt icon rail (player view) and the 217pt
//! labelled sidebar (library view).

use egui::{Align2, Rect, Sense, Ui, pos2, vec2};

use crate::app::{App, UiCommand};
use crate::ui::icons::Icon;
use crate::ui::pages::{Page, Section};
use crate::ui::state::{UiState, View};
use crate::ui::theme::font;
use crate::ui::widgets::{self, pal};

pub const RAIL_WIDTH: f32 = 50.0;
pub const SIDEBAR_WIDTH: f32 = 217.0;

struct Entry {
    section: Option<Section>,
    label: &'static str,
    icon: Icon,
    selected: Icon,
}

const ENTRIES: &[Entry] = &[
    Entry { section: Some(Section::Home), label: "Home", icon: Icon::Home, selected: Icon::HomeSelected },
    Entry { section: Some(Section::Videos), label: "Videos", icon: Icon::Videos, selected: Icon::VideosSelected },
    Entry { section: Some(Section::Music), label: "Music", icon: Icon::Songs, selected: Icon::SongsSelected },
    Entry {
        section: Some(Section::Network),
        label: "Network",
        icon: Icon::Streaming,
        selected: Icon::StreamingSelected,
    },
    Entry {
        section: Some(Section::Playlists),
        label: "Playlists",
        icon: Icon::Playlist,
        selected: Icon::Playlist2tone,
    },
    Entry { section: Some(Section::Recent), label: "Recent", icon: Icon::Recent, selected: Icon::Recent2tone },
    Entry { section: None, label: "Add item", icon: Icon::Add, selected: Icon::Add },
];

fn activate(entry: &Entry, app: &mut App, st: &mut UiState) {
    match entry.section {
        Some(s) => {
            st.navigate(Page::of(s));
            if st.view == View::Player {
                // In player view the rail drives the side column.
                st.view = View::Player;
                st.side_panel = true;
            }
        }
        None => add_item_menu_fallback(app, st),
    }
}

fn add_item_menu_fallback(_app: &mut App, st: &mut UiState) {
    st.command(UiCommand::OpenFile);
}

/// The icon rail used next to the video.
pub fn rail(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    let p = pal(ui.ctx());
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0, p.bg_a4);
    let mut y = rect.min.y + 8.0;
    let current = st.page.section();
    for e in ENTRIES {
        let r = Rect::from_min_size(pos2(rect.min.x + 9.0, y), vec2(32.0, 32.0));
        let resp = ui.interact(r, ui.id().with(("rail", e.label)), Sense::click()).on_hover_text(e.label);
        let on = e.section.is_some() && e.section == current && st.side_panel;
        if resp.hovered() {
            ui.painter().rect_filled(r, 4, p.hover);
        }
        let icon = if on { e.selected } else { e.icon };
        let fg = if on {
            p.accent
        } else if resp.hovered() {
            p.fg2
        } else {
            p.fg_a2
        };
        widgets::paint_icon(ui, Rect::from_center_size(r.center(), vec2(24.0, 24.0)), icon, fg);
        if resp.clicked() {
            if e.section.is_some() && on {
                st.side_panel = false;
            } else {
                activate(e, app, st);
            }
        }
        y += 46.0;
    }
}

/// The labelled sidebar in library view.
pub fn sidebar(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    let p = pal(ui.ctx());
    let outer = ui.max_rect();
    ui.painter().rect_filled(outer, 0, p.bg_b1);
    let panel = outer.shrink(8.0);
    ui.painter().rect_filled(panel, 6, p.bg_a4);
    let mut y = panel.min.y + 12.0;
    let current = st.page.section();
    for e in ENTRIES {
        let r = Rect::from_min_size(pos2(panel.min.x + 8.0, y), vec2(panel.width() - 16.0, 32.0));
        let resp = ui.interact(r, ui.id().with(("side", e.label)), Sense::click());
        let on = e.section.is_some() && e.section == current;
        if resp.hovered() || on {
            ui.painter().rect_filled(r, 4, if on { p.bg_a1 } else { p.hover });
        }
        let icon = if on { e.selected } else { e.icon };
        let fg = if on { p.accent } else { p.fg_a2 };
        widgets::paint_icon(
            ui,
            Rect::from_min_size(pos2(r.min.x + 4.0, r.center().y - 12.0), vec2(24.0, 24.0)),
            icon,
            fg,
        );
        let tc = if on {
            p.fg1
        } else if resp.hovered() {
            p.fg2
        } else {
            p.fg_a2
        };
        ui.painter().text(pos2(r.min.x + 38.0, r.center().y), Align2::LEFT_CENTER, e.label, font::regular(14.0), tc);
        if resp.clicked() {
            activate(e, app, st);
        }
        y += 46.0;
    }
    // Playlists under the main entries, like VLC's playlist tree.
    let lists: Vec<(u64, String)> = app.library.playlists().iter().map(|pl| (pl.id, pl.name.clone())).collect();
    if !lists.is_empty() {
        y += 8.0;
        ui.painter().text(
            pos2(panel.min.x + 16.0, y + 8.0),
            Align2::LEFT_CENTER,
            "Playlists",
            font::regular(12.0),
            p.fg_a3,
        );
        y += 22.0;
        for (id, name) in lists {
            if y > panel.max.y - 40.0 {
                break;
            }
            let r = Rect::from_min_size(pos2(panel.min.x + 8.0, y), vec2(panel.width() - 16.0, 30.0));
            let resp = ui.interact(r, ui.id().with(("pl", id)), Sense::click());
            let on = st.page == Page::Playlist(id);
            if resp.hovered() || on {
                ui.painter().rect_filled(r, 4, if on { p.bg_a1 } else { p.hover });
            }
            widgets::paint_icon(
                ui,
                Rect::from_min_size(pos2(r.min.x + 6.0, r.center().y - 10.0), vec2(20.0, 20.0)),
                Icon::Lists,
                p.fg_a3,
            );
            widgets::text_clipped(
                ui,
                pos2(r.min.x + 36.0, r.center().y),
                Align2::LEFT_CENTER,
                &name,
                font::regular(14.0),
                if on { p.fg1 } else { p.fg_a2 },
                r.width() - 44.0,
            );
            if resp.clicked() {
                st.navigate(Page::Playlist(id));
            }
            y += 32.0;
        }
    }
}
