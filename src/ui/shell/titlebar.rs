//! The 40pt custom title bar: logo, app menu, sidebar toggle, location,
//! centred title, window buttons. On macOS the window's own traffic lights
//! sit at the left of the bar instead of the design's buttons.

use egui::{Align2, Rect, Sense, Ui, ViewportCommand, pos2, vec2};

use crate::app::{App, UiCommand};
use crate::ui::icons::Icon;
use crate::ui::state::{UiState, View};
use crate::ui::theme::font;
use crate::ui::widgets::{self, menu_row, pal, submenu};

pub const HEIGHT: f32 = 40.0;
/// Room for macOS's traffic lights: 13pt margins in the 40pt bar put the zoom
/// button's right edge at 73pt, and the bar's content starts 16pt after it.
const TRAFFIC_LIGHTS: f32 = 76.0;

pub fn show(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    let p = pal(ui.ctx());
    let rect = ui.max_rect();
    let fill = if st.view == View::Library { p.bg_a4 } else { p.chrome };
    ui.painter().rect_filled(rect, 0, fill);

    // Dragging empty space moves the window; double-click maximises.
    let drag = ui.interact(rect, ui.id().with("titlebar-drag"), Sense::click_and_drag());
    if drag.drag_started() {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if drag.double_clicked() {
        let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!max));
    }

    // Title, centred on the window.
    let title = window_title(app);
    widgets::text_clipped(
        ui,
        rect.center(),
        Align2::CENTER_CENTER,
        &title,
        font::regular(14.0),
        p.fg_a2,
        rect.width() * 0.4,
    );

    // In fullscreen macOS moves the traffic lights out of the window.
    let fullscreen = ui.ctx().input(|i| i.viewport().fullscreen.unwrap_or(false));
    let inset = if st.native_titlebar && !fullscreen { TRAFFIC_LIGHTS } else { 0.0 };
    let mut left = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_size(rect.min + vec2(6.0 + inset, 6.0), vec2(420.0, 28.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    left.spacing_mut().item_spacing.x = 7.0;
    left.add_space(7.0);
    let logo = widgets::icon(&mut left, Icon::Logo, 24.0, p.accent_logo);
    if logo.clicked() {
        st.navigate(crate::ui::pages::Page::Home);
    }
    left.add_space(4.0);
    let more = widgets::icon_button_sized(&mut left, Icon::More, vec2(28.0, 28.0), 24.0, false, "Menu");
    app_menu(&more, app, st);
    if st.view == View::Player {
        let icon = if st.side_panel { Icon::SidebarActive } else { Icon::SidebarLeft };
        if widgets::icon_button_sized(&mut left, icon, vec2(28.0, 28.0), 24.0, false, "Show the library").clicked() {
            st.side_panel = !st.side_panel;
        }
        let (r, _) = left.allocate_exact_size(vec2(1.0, 20.0), Sense::hover());
        left.painter().rect_filled(r, 0, p.divider_bright);
        if let Some(origin) = location_label(app) {
            left.add_space(4.0);
            let galley = left.painter().layout_no_wrap(origin, font::regular(14.0), p.fg_a2);
            let (r, resp) = left.allocate_exact_size(vec2(galley.size().x + 40.0, 28.0), Sense::click());
            if resp.hovered() {
                left.painter().rect_filled(r, 6, p.hover);
            }
            widgets::paint_icon(
                &left,
                Rect::from_min_size(pos2(r.min.x + 4.0, r.center().y - 12.0), vec2(24.0, 24.0)),
                Icon::Folder,
                p.fg_a2,
            );
            left.painter().galley(pos2(r.min.x + 32.0, r.center().y - galley.size().y / 2.0), galley, p.fg_a2);
            if resp.on_hover_text("Show in the library").clicked() {
                if let Some(dir) = app.queue.current_item().and_then(|i| i.source.path()).and_then(|p| p.parent()) {
                    st.navigate(crate::ui::pages::Page::Folder(dir.to_path_buf()));
                } else {
                    st.view = View::Library;
                }
            }
        }
    }

    if st.native_titlebar {
        return;
    }
    // Window buttons, right aligned, 36x24 each.
    let mut right = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_max(
                pos2(rect.max.x - 160.0, rect.min.y + 8.0),
                pos2(rect.max.x - 6.0, rect.max.y - 8.0),
            ))
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    right.spacing_mut().item_spacing.x = 0.0;
    let bsize = vec2(36.0, 24.0);
    if window_button(&mut right, Icon::Close, bsize, true).on_hover_text("Close").clicked() {
        st.command(UiCommand::Quit);
    }
    let fs_icon = if st.fullscreen { Icon::FullscreenActive } else { Icon::Fullscreen };
    if window_button(&mut right, fs_icon, bsize, false).on_hover_text("Fullscreen").clicked() {
        st.command(UiCommand::ToggleFullscreen);
    }
    if window_button(&mut right, Icon::Maximize, bsize, false).on_hover_text("Maximize").clicked() {
        let max = right.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
        right.ctx().send_viewport_cmd(ViewportCommand::Maximized(!max));
    }
    if window_button(&mut right, Icon::Minimize, bsize, false).on_hover_text("Minimize").clicked() {
        right.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
    }
}

fn window_button(ui: &mut Ui, icon: Icon, size: egui::Vec2, danger: bool) -> egui::Response {
    let p = pal(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.hovered() {
        let c = if danger { crate::ui::theme::with_alpha(p.danger, 0.8) } else { p.hover };
        ui.painter().rect_filled(rect, 4, c);
    }
    let fg = if resp.hovered() { p.fg1 } else { p.fg_a2 };
    widgets::paint_icon(ui, Rect::from_center_size(rect.center(), vec2(24.0, 24.0)), icon, fg);
    resp
}

pub fn window_title(app: &App) -> String {
    match app.queue.current_item() {
        Some(item) if app.settings.interface.filename_in_title && app.status.state.is_active() => {
            let t = item.title();
            match &app.status.meta.now_playing {
                Some(np) => format!("{t} - {np}"),
                None => t,
            }
        }
        _ => "LVP".to_string(),
    }
}

/// The folder or playlist the current item came from.
fn location_label(app: &App) -> Option<String> {
    if let Some(o) = &app.queue.origin {
        return Some(o.clone());
    }
    let item = app.queue.current_item()?;
    let dir = item.source.path()?.parent()?;
    Some(dir.file_name()?.to_string_lossy().into_owned())
}

/// VLC's Media / Playback / Tools / Help menus folded into one.
fn app_menu(button: &egui::Response, app: &mut App, st: &mut UiState) {
    let p = pal(&button.ctx);
    egui::Popup::menu(button).frame(widgets::menu_frame(&p)).show(|ui| {
        ui.set_min_width(260.0);
        if menu_row(
            ui,
            Some(Icon::File),
            "Open File...",
            &shortcut(app, crate::domain::shortcuts::Action::OpenFile),
            false,
        )
        .clicked()
        {
            st.command(UiCommand::OpenFile);
        }
        if menu_row(
            ui,
            Some(Icon::Folder),
            "Open Folder...",
            &shortcut(app, crate::domain::shortcuts::Action::OpenFolder),
            false,
        )
        .clicked()
        {
            st.command(UiCommand::OpenFolder);
        }
        if menu_row(
            ui,
            Some(Icon::Disc),
            "Open Disc...",
            &shortcut(app, crate::domain::shortcuts::Action::OpenDisc),
            false,
        )
        .clicked()
        {
            st.command(UiCommand::OpenDisc);
        }
        if menu_row(
            ui,
            Some(Icon::Streaming),
            "Open Network Stream...",
            &shortcut(app, crate::domain::shortcuts::Action::OpenNetwork),
            false,
        )
        .clicked()
        {
            st.command(UiCommand::OpenNetwork);
        }
        if menu_row(
            ui,
            Some(Icon::CaptureDevice),
            "Open Capture Device...",
            &shortcut(app, crate::domain::shortcuts::Action::OpenCapture),
            false,
        )
        .clicked()
        {
            st.command(UiCommand::OpenCapture);
        }
        submenu(ui, Some(Icon::Recent), "Open Recent Media", |ui| {
            let recent = app.library.recent();
            if recent.is_empty() {
                ui.label(egui::RichText::new("Nothing played yet").color(p.fg_a3));
            }
            for (item, _) in recent.into_iter().take(12) {
                if menu_row(ui, None, &item.title(), "", false).clicked() {
                    app.play_items(vec![item], 0, None);
                    st.view = View::Player;
                }
            }
        });
        widgets::divider(ui);
        if menu_row(ui, Some(Icon::Save), "Save Queue as Playlist...", "", false).clicked()
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("XSPF playlist", &["xspf"])
                .add_filter("M3U playlist", &["m3u8", "m3u"])
                .add_filter("PLS playlist", &["pls"])
                .set_file_name("Playlist.xspf")
                .save_file()
        {
            let items = app.queue.items.clone();
            app.export_items("Queue", &items, &path);
        }
        if menu_row(
            ui,
            Some(Icon::Swap),
            "Convert / Save...",
            &shortcut(app, crate::domain::shortcuts::Action::ConvertSave),
            false,
        )
        .clicked()
        {
            st.command(UiCommand::ConvertSave);
        }
        if menu_row(
            ui,
            Some(Icon::Info),
            "Media Information",
            &shortcut(app, crate::domain::shortcuts::Action::MediaInfo),
            false,
        )
        .clicked()
        {
            st.command(UiCommand::MediaInfo);
        }
        if menu_row(
            ui,
            Some(Icon::Eq),
            "Effects and Filters",
            &shortcut(app, crate::domain::shortcuts::Action::EffectsFilters),
            false,
        )
        .clicked()
        {
            st.command(UiCommand::EffectsFilters);
        }
        widgets::divider(ui);
        if menu_row(
            ui,
            Some(Icon::Settings),
            "Preferences",
            &shortcut(app, crate::domain::shortcuts::Action::Preferences),
            false,
        )
        .clicked()
        {
            st.command(UiCommand::Preferences);
        }
        if menu_row(
            ui,
            Some(Icon::Hotkeys),
            "Keyboard Shortcuts",
            &shortcut(app, crate::domain::shortcuts::Action::KeyboardShortcuts),
            false,
        )
        .clicked()
        {
            st.command(UiCommand::KeyboardShortcuts);
        }
        if menu_row(ui, Some(Icon::Logo), "About LVP", "", false).clicked() {
            st.dialogs.about = true;
        }
        if menu_row(ui, Some(Icon::Quit), "Quit", &shortcut(app, crate::domain::shortcuts::Action::Quit), false)
            .clicked()
        {
            st.command(UiCommand::Quit);
        }
    });
}

pub fn shortcut(app: &App, a: crate::domain::shortcuts::Action) -> String {
    app.settings.controls.shortcuts.keys(a).first().map(|k| k.to_string()).unwrap_or_default()
}
