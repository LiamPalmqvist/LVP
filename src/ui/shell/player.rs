//! The player view: optional library column, the picture, overlays and the
//! control bar.

use std::time::{Duration, Instant};

use egui::{Align2, Color32, Rect, Sense, Ui, pos2, vec2};

use crate::app::App;
use crate::domain::settings::{AutoHideControls, ClickAction, WheelAction};
use crate::ui::icons::Icon;
use crate::ui::state::{UiState, View};
use crate::ui::theme::font;
use crate::ui::widgets::{self, pal};
use crate::ui::{Resources, music, pages, panels};

use super::{controls, nav};

pub const SIDE_PANEL_WIDTH: f32 = 377.0;

/// Whether the control bar should currently be visible.
pub fn controls_visible(app: &App, st: &UiState, ctx: &egui::Context) -> bool {
    if st.panel.is_some() || !app.is_playing() {
        return true;
    }
    let auto_hide = st.hide_ui
        || match app.settings.interface.auto_hide_controls {
            AutoHideControls::Never => false,
            AutoHideControls::FullscreenOnly => st.fullscreen,
            AutoHideControls::Always => true,
        };
    if !auto_hide {
        return true;
    }
    if st.fullscreen && !app.settings.interface.controls_in_fullscreen {
        return false;
    }
    let now = Instant::now();
    if now < st.controls_until {
        ctx.request_repaint_after(st.controls_until - now);
        return true;
    }
    false
}

pub fn show(ui: &mut Ui, app: &mut App, st: &mut UiState, res: &mut Resources) {
    let p = pal(ui.ctx());
    let full = ui.max_rect();
    let show_chrome = !st.hide_ui && !st.fullscreen && !st.pip;
    let bar_h = controls::height(app);
    let visible = controls_visible(app, st, ui.ctx());
    let overlay = !show_chrome;
    // The control bar spans the full width below the rail and the picture.
    let mut area =
        if overlay || !visible { full } else { Rect::from_min_max(full.min, pos2(full.max.x, full.max.y - bar_h)) };

    // Library rail and column on the left.
    if show_chrome {
        let rail = Rect::from_min_size(area.min, vec2(nav::RAIL_WIDTH, area.height()));
        ui.scope_builder(egui::UiBuilder::new().max_rect(rail), |ui| nav::rail(ui, app, st));
        area.min.x += nav::RAIL_WIDTH;
        if st.side_panel {
            let col = Rect::from_min_size(area.min, vec2(SIDE_PANEL_WIDTH, area.height()));
            ui.painter().rect_filled(col, 0, p.bg_b1);
            ui.scope_builder(egui::UiBuilder::new().max_rect(col.shrink(20.0)), |ui| pages::side_panel(ui, app, st));
            area.min.x += SIDE_PANEL_WIDTH;
        }
    }
    let video_rect = area;

    // The picture (or the music view for audio).
    ui.painter().rect_filled(video_rect, 0, p.bg_b2);
    let has_picture = res.video_frame.is_some();
    if has_picture {
        if let Some((tex, [w, h])) = res.video_frame {
            let (dest, uv) = crate::ui::video::fit(
                w,
                h,
                app.status.video_sar,
                video_rect,
                &app.geometry,
                &app.settings.video.effects.crop,
            );
            ui.painter().with_clip_rect(video_rect).image(tex, dest, uv, Color32::WHITE);
        }
    } else if app.has_media() {
        music::show_player(ui, video_rect, app, st);
    } else {
        empty_player(ui, video_rect, st);
    }

    // Mouse on the picture: click, double-click, wheel, context menu.
    let resp = ui.interact(video_rect, ui.id().with("video-area"), Sense::click());
    if ui.input(|i| i.pointer.is_moving()) && ui.rect_contains_pointer(full) {
        st.bump_controls(app.settings.interface.hide_after_s.max(1.0));
    }
    handle_video_mouse(ui, &resp, app, st);
    panels::player_context_menu(&resp, app, st);

    // On-screen messages.
    if let Some(osd) = &app.osd {
        let galley = ui.painter().layout_no_wrap(osd.text.clone(), font::medium(18.0), p.fg1);
        let r = Rect::from_min_size(video_rect.min + vec2(24.0, 24.0), galley.size() + vec2(24.0, 14.0));
        ui.painter().rect_filled(r, 8, crate::ui::theme::with_alpha(Color32::BLACK, 0.6));
        ui.painter().galley(r.min + vec2(12.0, 7.0), galley, p.fg1);
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
    if matches!(app.status.state, crate::app::ports::PlayerState::Opening | crate::app::ports::PlayerState::Buffering)
        && !has_picture
    {
        ui.painter().text(
            video_rect.center() + vec2(0.0, 60.0),
            Align2::CENTER_CENTER,
            "Loading...",
            font::regular(14.0),
            p.fg_a3,
        );
    }

    if visible {
        let bar = Rect::from_min_max(pos2(full.min.x, full.max.y - bar_h), full.max);
        controls::show(ui, bar, app, st, overlay);
        panels::show_popovers(ui, bar, app, st);
    } else {
        ui.ctx().set_cursor_icon(egui::CursorIcon::None);
    }
}

fn empty_player(ui: &mut Ui, r: Rect, st: &mut UiState) {
    let p = pal(ui.ctx());
    let c = r.center();
    widgets::paint_icon(ui, Rect::from_center_size(c - vec2(0.0, 40.0), vec2(72.0, 72.0)), Icon::Vlc, p.accent_logo);
    ui.painter().text(
        c + vec2(0.0, 20.0),
        Align2::CENTER_TOP,
        "Drop a file here, or open something to play",
        font::regular(16.0),
        p.fg_a2,
    );
    let br = Rect::from_center_size(c + vec2(0.0, 76.0), vec2(160.0, 36.0));
    ui.scope_builder(egui::UiBuilder::new().max_rect(br), |ui| {
        if widgets::button_with(ui, Some(Icon::File), "Open media", widgets::ButtonKind::Primary, vec2(160.0, 36.0))
            .clicked()
        {
            st.command(crate::app::UiCommand::OpenFile);
        }
    });
}

fn handle_video_mouse(ui: &Ui, resp: &egui::Response, app: &mut App, st: &mut UiState) {
    let c = &app.settings.controls;
    let id = resp.id.with("pending-click");
    // Delay single clicks so a double click does not also toggle pause.
    if resp.double_clicked() {
        ui.ctx().data_mut(|d| d.remove::<Instant>(id));
        run_click(c.double_click, app, st);
    } else if resp.clicked() {
        if st.panel.is_some() {
            st.panel = None;
        } else {
            ui.ctx().data_mut(|d| d.insert_temp(id, Instant::now()));
        }
    }
    let pending: Option<Instant> = ui.ctx().data(|d| d.get_temp(id));
    if let Some(t) = pending {
        let delay = Duration::from_millis(280);
        if t.elapsed() >= delay {
            ui.ctx().data_mut(|d| d.remove::<Instant>(id));
            let action = app.settings.controls.single_click;
            run_click(action, app, st);
        } else {
            ui.ctx().request_repaint_after(delay - t.elapsed());
        }
    }
    if resp.hovered() {
        let (dy, dx) = ui.input(|i| (i.raw_scroll_delta.y, i.raw_scroll_delta.x));
        let rev = if app.settings.controls.reverse_scroll { -1.0 } else { 1.0 };
        let apply = |action: WheelAction, delta: f32, app: &mut App| match action {
            WheelAction::Nothing => {}
            WheelAction::Volume => app.change_volume(if delta * rev > 0.0 { 5 } else { -5 }),
            WheelAction::Seek => {
                let s = app.settings.controls.skip_very_short_s as i64 * 1000;
                app.jump(if delta * rev > 0.0 { s } else { -s });
            }
        };
        if dy.abs() > 2.0 {
            apply(app.settings.controls.scroll_wheel, dy, app);
        }
        if dx.abs() > 2.0 {
            apply(app.settings.controls.horizontal_scroll, -dx, app);
        }
    }
}

fn run_click(action: ClickAction, app: &mut App, st: &mut UiState) {
    match action {
        ClickAction::Nothing => {}
        ClickAction::PlayPause => app.toggle_pause(),
        ClickAction::ToggleFullscreen => st.command(crate::app::UiCommand::ToggleFullscreen),
        ClickAction::ToggleControls => {
            if Instant::now() < st.controls_until {
                st.controls_until = Instant::now();
            } else {
                st.bump_controls(app.settings.interface.hide_after_s.max(1.0));
            }
        }
    }
}

/// Library view: sidebar, page, and the floating mini player.
pub fn library(ui: &mut Ui, app: &mut App, st: &mut UiState, res: &mut Resources) {
    let full = ui.max_rect();
    let side = Rect::from_min_size(
        full.min,
        vec2(if st.sidebar_expanded { nav::SIDEBAR_WIDTH } else { nav::RAIL_WIDTH }, full.height()),
    );
    ui.scope_builder(egui::UiBuilder::new().max_rect(side), |ui| {
        if st.sidebar_expanded { nav::sidebar(ui, app, st) } else { nav::rail(ui, app, st) }
    });
    let content = Rect::from_min_max(pos2(side.max.x, full.min.y), full.max);
    let p = pal(ui.ctx());
    ui.painter().rect_filled(content, 0, p.bg_b1);
    ui.scope_builder(egui::UiBuilder::new().max_rect(content.shrink(20.0)), |ui| pages::show(ui, app, st));
    if app.has_media() && st.view == View::Library {
        music::miniplayer(ui, content, app, st, res.video_frame);
    }
}
