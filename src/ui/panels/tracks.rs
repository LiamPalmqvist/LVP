//! The Video / Audio / Subtitles track popover.

use std::path::PathBuf;

use egui::{Align2, CornerRadius, Rect, Sense, Ui, UiBuilder, Vec2, pos2, vec2};

use super::kit;
use crate::app::ports::Track;
use crate::app::{App, UiCommand};
use crate::domain::media::{MediaKind, MediaSource, kind_for_path, matching_subtitles};
use crate::ui::icons::Icon;
use crate::ui::settings::Category;
use crate::ui::state::{PlayerPanel, UiState};
use crate::ui::theme::font;
use crate::ui::widgets::{self, pal};

/// Subtitle files found next to the playing video, cached per source.
#[derive(Default)]
pub struct TracksUi {
    scanned: Option<MediaSource>,
    external: Vec<PathBuf>,
    loaded: Vec<PathBuf>,
}

pub fn show(ui: &mut Ui, app: &mut App, st: &mut UiState, tab: usize) {
    let tabs = [(Icon::Video, "Video"), (Icon::Audio, "Audio"), (Icon::Subtitles, "Subtitles")];
    if let Some(t) = kit::tab_strip(ui, tab, &tabs, None) {
        st.panel = Some(PlayerPanel::Tracks(t));
    }
    let foot_h = 72.0;
    let body_h = (ui.available_height() - foot_h).max(60.0);
    let w = ui.available_width();
    let body = Rect::from_min_size(ui.cursor().min, vec2(w, body_h));
    ui.scope_builder(UiBuilder::new().max_rect(body), |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).max_height(body_h).show(ui, |ui| {
            ui.set_width(w);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            match tab {
                0 => video_tab(ui, app),
                1 => audio_tab(ui, app),
                _ => subtitles_tab(ui, app, st),
            }
        });
    });
    ui.add_space(20.0);
    egui::Frame::new().inner_margin(egui::Margin::symmetric(20, 0)).show(ui, |ui| {
        ui.set_width(w - 40.0);
        if kit::foot_button(ui, None, "Show Preferences").clicked() {
            st.settings.open = true;
            st.settings.category = match tab {
                0 => Category::Video,
                1 => Category::Audio,
                _ => Category::Subtitles,
            };
            st.panel = None;
        }
    });
}

/// A 60pt caption band ("Embedded Tracks") with optional actions on the right.
fn band(ui: &mut Ui, text: &str, actions: impl FnOnce(&mut Ui)) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 60.0), Sense::hover());
    kit::caption(ui, pos2(rect.min.x + 28.0, rect.center().y), text);
    let right = Rect::from_min_max(pos2(rect.min.x + 200.0, rect.min.y), pos2(rect.max.x - 28.0, rect.max.y));
    let mut child =
        ui.new_child(UiBuilder::new().max_rect(right).layout(egui::Layout::right_to_left(egui::Align::Center)));
    child.spacing_mut().item_spacing.x = 20.0;
    actions(&mut child);
}

/// A radio-style list row: a check in the 32pt slot when selected.
fn track_row(ui: &mut Ui, name: &str, selected: bool) -> bool {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect.expand2(vec2(6.0, 4.0)), CornerRadius::same(6), p.hover);
    }
    if selected {
        kit::paint_check(
            ui,
            Rect::from_center_size(pos2(rect.min.x + 16.0, rect.center().y), Vec2::splat(24.0)),
            p.fg_a1,
        );
    }
    widgets::text_clipped(
        ui,
        pos2(rect.min.x + 42.0, rect.center().y),
        Align2::LEFT_CENTER,
        name,
        font::regular(14.0),
        if selected { p.fg2 } else { p.fg_a1 },
        w - 46.0,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn list(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    egui::Frame::new().inner_margin(egui::Margin { left: 20, right: 20, top: 0, bottom: 8 }).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing = vec2(0.0, 12.0);
        add(ui);
    });
}

fn track_name(t: &Track) -> String {
    if t.id < 0 { "None".to_string() } else { t.name.clone() }
}

/// Returns the id of a clicked track.
fn track_list(ui: &mut Ui, tracks: &[Track], current: i32) -> Option<i32> {
    let p = pal(ui.ctx());
    let mut chosen = None;
    if tracks.is_empty() {
        list(ui, |ui| {
            ui.label(egui::RichText::new("No tracks").font(font::regular(14.0)).color(p.fg_a4));
        });
        return None;
    }
    list(ui, |ui| {
        for t in tracks {
            if track_row(ui, &track_name(t), t.id == current) && t.id != current {
                chosen = Some(t.id);
            }
        }
    });
    chosen
}

fn video_tab(ui: &mut Ui, app: &mut App) {
    band(ui, "Embedded Tracks", |_| {});
    let tracks = app.status.video_tracks.clone();
    if let Some(id) = track_list(ui, &tracks, app.status.video_track) {
        app.set_video_track(id);
    }
}

fn audio_tab(ui: &mut Ui, app: &mut App) {
    band(ui, "Embedded Tracks", |_| {});
    let tracks = app.status.audio_tracks.clone();
    if let Some(id) = track_list(ui, &tracks, app.status.audio_track) {
        app.set_audio_track(id);
    }
    let mut refresh = false;
    band(ui, "Output Device", |ui| {
        refresh = kit::icon_action(ui, Icon::Refresh, "Refresh the device list", false).clicked();
    });
    let id = ui.make_persistent_id("audio-devices");
    let cached: Option<Vec<(String, String)>> = ui.data(|d| d.get_temp(id));
    let devices = match cached {
        Some(d) if !refresh => d,
        _ => {
            let d: Vec<(String, String)> = app.engine.audio_devices().into_iter().map(|d| (d.id, d.name)).collect();
            ui.data_mut(|m| m.insert_temp(id, d.clone()));
            d
        }
    };
    let current = app.status.audio_device.clone().or_else(|| app.settings.audio.output_device.clone());
    let mut chosen = None;
    list(ui, |ui| {
        let default_on = current.as_deref().is_none_or(str::is_empty);
        if !devices.iter().any(|(i, _)| i.is_empty()) && track_row(ui, "System Default", default_on) {
            chosen = Some(None);
        }
        for (dev_id, name) in &devices {
            let on = current.as_deref() == Some(dev_id.as_str()) || (dev_id.is_empty() && default_on);
            let label = if name.is_empty() { "System Default" } else { name.as_str() };
            if track_row(ui, label, on) && !on {
                chosen = Some(if dev_id.is_empty() { None } else { Some(dev_id.clone()) });
            }
        }
    });
    if let Some(dev) = chosen {
        app.set_audio_device(dev);
    }
}

fn scan_external(app: &App, ui_state: &mut TracksUi) {
    let src = app.current_source().cloned();
    if ui_state.scanned == src {
        return;
    }
    ui_state.scanned = src.clone();
    ui_state.external.clear();
    ui_state.loaded.clear();
    let Some(path) = src.as_ref().and_then(|s| s.path()).map(|p| p.to_path_buf()) else { return };
    let Some(dir) = path.parent() else { return };
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let candidates: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    let mut found = matching_subtitles(&path, &candidates, true);
    if found.is_empty() {
        found = candidates.into_iter().filter(|c| kind_for_path(c) == MediaKind::Subtitle).take(12).collect();
    }
    found.sort();
    ui_state.external = found;
}

fn subtitles_tab(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    let p = pal(ui.ctx());
    band(ui, "Embedded Tracks", |_| {});
    let tracks = app.status.subtitle_tracks.clone();
    if let Some(id) = track_list(ui, &tracks, app.status.subtitle_track) {
        app.set_subtitle_track(id);
    }
    let mut add = false;
    band(ui, "External Tracks", |ui| {
        add = kit::icon_action(ui, Icon::Add, "Add a subtitle file", false).clicked();
    });
    if add {
        st.command(UiCommand::AddSubtitleFile);
    }
    let t = &mut st.panels.tracks;
    scan_external(app, t);
    let mut load = None;
    list(ui, |ui| {
        if t.external.is_empty() {
            ui.label(
                egui::RichText::new("No subtitle files next to this video").font(font::regular(14.0)).color(p.fg_a4),
            );
        }
        for f in &t.external {
            let name = f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if track_row(ui, &name, t.loaded.contains(f)) && !t.loaded.contains(f) {
                load = Some(f.clone());
            }
        }
    });
    if let Some(f) = load {
        t.loaded.push(f.clone());
        app.add_subtitle(f);
    }
}
