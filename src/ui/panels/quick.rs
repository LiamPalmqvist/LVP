//! The quick settings popover: Playback / Video / Audio / Subtitles, with the
//! effect preset bar and "Open Advanced Settings" at the foot.

use egui::{Rect, Ui, UiBuilder, vec2};

use super::{audio_fx, kit, presets, video_fx};
use crate::app::App;
use crate::app::ports::{SubtitleTiming, Track};
use crate::domain::settings::{FontSize, Outline, SizeUnits, SubtitleSettings, Visualizer};
use crate::domain::time::format_delay;
use crate::ui::icons::Icon;
use crate::ui::settings::Category;
use crate::ui::state::{PlayerPanel, UiState};
use crate::ui::widgets;

const FOOT_H: f32 = 125.0;
/// The design's dropdowns in this panel.
const DROP: egui::Vec2 = egui::vec2(220.0, 48.0);
/// The value boxes on the right of a row.
const BOX_W: f32 = 140.0;

pub const SLEEP_CHOICES: &[Option<u32>] = &[None, Some(5), Some(10), Some(15), Some(30), Some(45), Some(60), Some(90)];

pub fn show(ui: &mut Ui, app: &mut App, st: &mut UiState, tab: usize) {
    let tabs =
        [(Icon::Playback, "Playback"), (Icon::Video, "Video"), (Icon::Audio, "Audio"), (Icon::Subtitles, "Subtitles")];
    if let Some(t) = kit::tab_strip(ui, tab, &tabs, Some(&[108.0, 85.0, 85.0, 107.0])) {
        st.panel = Some(PlayerPanel::QuickSettings(t));
    }
    let w = ui.available_width();
    let body_h = (ui.available_height() - FOOT_H).max(80.0);
    let body = Rect::from_min_size(ui.cursor().min, vec2(w, body_h));
    ui.scope_builder(UiBuilder::new().max_rect(body), |ui| {
        egui::ScrollArea::vertical().id_salt(("quick", tab)).auto_shrink([false, false]).max_height(body_h).show(
            ui,
            |ui| {
                egui::Frame::new().inner_margin(egui::Margin { left: 20, right: 20, top: 0, bottom: 16 }).show(
                    ui,
                    |ui| {
                        ui.set_width(w - 40.0);
                        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                        match tab {
                            0 => playback_tab(ui, app, st),
                            1 => video_fx::show(ui, app, st),
                            2 => audio_fx::show(ui, app, st),
                            _ => subtitles_tab(ui, app, st),
                        }
                    },
                );
            },
        );
    });
    footer(ui, app, st, tab);
}

fn footer(ui: &mut Ui, app: &mut App, st: &mut UiState, tab: usize) {
    let p = widgets::pal(ui.ctx());
    let w = ui.available_width();
    let top = ui.cursor().min;
    ui.painter().hline(top.x..=top.x + w, top.y, egui::Stroke::new(1.0, p.divider_dim));
    egui::Frame::new().inner_margin(egui::Margin { left: 20, right: 20, top: 21, bottom: 20 }).show(ui, |ui| {
        ui.set_width(w - 40.0);
        ui.spacing_mut().item_spacing = vec2(0.0, 10.0);
        presets::bar(ui, app, st);
        if kit::foot_button(ui, Some(Icon::Settings), "Open Advanced Settings").clicked() {
            st.settings.open = true;
            st.settings.category = match tab {
                0 => Category::Playback,
                1 => Category::Video,
                2 => Category::Audio,
                _ => Category::Subtitles,
            };
            st.panel = None;
        }
    });
}

/// Gap above a group of rows (the design's 10pt group padding).
fn gap(ui: &mut Ui, h: f32) {
    ui.add_space(h);
}

pub fn format_rate(r: f64) -> String {
    let s = format!("{r:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    format!("{s}x")
}

fn playback_tab(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    gap(ui, 10.0);
    let ps = &mut st.panels;
    kit::toggle_row(ui, "Show Subtitle Timeline on Seekbar", &mut ps.subtitle_timeline, 52.0);
    kit::toggle_row(ui, "Show Statistics", &mut ps.show_stats, 52.0);
    kit::row(ui, "Reverse", 52.0, |ui| {
        kit::disabled_toggle(ui, "Reverse playback is not available with VLC's engine.");
    });
    // Sleep timer: the chosen length, cleared when the timer runs out.
    if app.sleep_until.is_none() {
        ps.sleep_minutes = None;
    }
    kit::row(ui, "Sleep Timer", 78.0, |ui| {
        let labels: Vec<String> =
            SLEEP_CHOICES.iter().map(|c| c.map(|m| format!("{m} min")).unwrap_or_else(|| "Off".into())).collect();
        let sel = SLEEP_CHOICES.iter().position(|c| *c == ps.sleep_minutes);
        let cur = sel.map(|i| labels[i].clone()).unwrap_or_else(|| "Off".into());
        if let Some(i) = kit::dropdown(ui, "sleep", DROP, sel, &cur, &kit::opts(&labels)) {
            ps.sleep_minutes = SLEEP_CHOICES[i];
            app.set_sleep_timer(SLEEP_CHOICES[i]);
            app.show_osd(match SLEEP_CHOICES[i] {
                Some(m) => format!("Sleep in {m} minutes"),
                None => "Sleep timer off".into(),
            });
        }
    });
    kit::row(ui, "Audio Visualizer", 68.0, |ui| {
        if let Some(v) = kit::choice(ui, "visualizer", DROP, app.settings.audio.visualizer, Visualizer::ALL, |v| {
            v.label().to_string()
        }) {
            app.set_visualizer(v);
        }
    });
    let mut rate = if app.status.rate > 0.0 { app.status.rate as f64 } else { 1.0 };
    let mut changed =
        kit::row(ui, "Speed", 44.0, |ui| kit::number_box(ui, "speed", &mut rate, 0.25..=4.0, 0.05, format_rate, BOX_W));
    gap(ui, 8.0);
    let labels = [
        (0.25, "0.25"),
        (0.5, "0.5"),
        (0.75, "0.75"),
        (1.0, "1"),
        (1.25, "1.25"),
        (1.5, "1.5"),
        (1.75, "1.75"),
        (2.0, "2"),
    ];
    let mut slider_rate = rate.min(2.0);
    if kit::slider(ui, &mut slider_rate, 0.25..=2.0, &kit::Sl::default().ticks(7).step(0.05).labels(&labels).reset(1.0))
        .changed()
    {
        rate = slider_rate;
        changed = true;
    }
    if changed {
        app.set_rate(rate as f32);
    }
}

/// 1-based position of the current track among real tracks, or "Off".
fn track_number(tracks: &[Track], current: i32) -> String {
    tracks
        .iter()
        .filter(|t| t.id >= 0)
        .position(|t| t.id == current)
        .map(|i| (i + 1).to_string())
        .unwrap_or_else(|| "Off".into())
}

/// Edits a track-number box; returns the id to switch to.
pub fn track_number_box(ui: &mut Ui, id: &str, tracks: &[Track], current: i32) -> Option<i32> {
    let real: Vec<&Track> = tracks.iter().filter(|t| t.id >= 0).collect();
    let shown = track_number(tracks, current);
    let pos = real.iter().position(|t| t.id == current);
    match kit::value_box(ui, id, &shown, BOX_W) {
        Some(kit::BoxEvent::Commit(t)) => {
            let n = kit::parse_number(&t).map(|v| v as i64).unwrap_or(0);
            if n <= 0 { Some(-1) } else { real.get(n as usize - 1).map(|t| t.id) }
        }
        Some(kit::BoxEvent::Scroll(d)) if !real.is_empty() => {
            let i = match pos {
                Some(i) if d > 0.0 => (i + 1).min(real.len() - 1),
                Some(i) => i.saturating_sub(1),
                None => 0,
            };
            Some(real[i].id)
        }
        _ => None,
    }
}

/// A language text box bound to a setting.
pub fn language_box(ui: &mut Ui, id: &str, value: &str) -> Option<String> {
    match kit::value_box(ui, id, value, BOX_W) {
        Some(kit::BoxEvent::Commit(t)) if !t.trim().is_empty() => Some(t.trim().to_string()),
        _ => None,
    }
}

/// The delay box and its -10..+10 s slider. Returns the new delay in µs.
pub fn delay_editor(ui: &mut Ui, id: &str, label: &str, us: i64) -> Option<i64> {
    let mut secs = us as f64 / 1_000_000.0;
    let mut changed = kit::row(ui, label, 56.0, |ui| {
        kit::number_box(ui, id, &mut secs, -60.0..=60.0, 0.05, |v| format_delay((v * 1e6) as i64), BOX_W)
    });
    gap(ui, 5.0);
    let mut s = secs.clamp(-10.0, 10.0);
    let style =
        kit::Sl::default().origin(0.0).step(0.05).reset(0.0).labels(&[(-10.0, "-10 s"), (0.0, "0 s"), (10.0, "+10 s")]);
    if kit::slider(ui, &mut s, -10.0..=10.0, &style).changed() {
        secs = s;
        changed = true;
    }
    changed.then(|| (secs * 1_000_000.0).round() as i64)
}

fn subtitles_tab(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    kit::section(ui, "Defaults");
    gap(ui, 10.0);
    let tracks = app.status.subtitle_tracks.clone();
    if let Some(id) =
        kit::row(ui, "Subtitle Track", 48.0, |ui| track_number_box(ui, "sub-track", &tracks, app.status.subtitle_track))
    {
        app.set_subtitle_track(id);
    }
    gap(ui, 5.0);
    let lang = app.settings.subtitles.preferred_language.clone();
    if let Some(l) = kit::row(ui, "Subtitle Language", 48.0, |ui| language_box(ui, "sub-lang", &lang)) {
        app.update_settings(|s| s.subtitles.preferred_language = l);
    }
    gap(ui, 10.0);
    kit::rule(ui);

    kit::section(ui, "Synchronization");
    gap(ui, 10.0);
    if let Some(us) = delay_editor(ui, "sub-delay", "Subtitle Track Delay", app.status.subtitle_delay_us) {
        app.set_subtitle_delay(us);
    }
    let mut t: SubtitleTiming = app.subtitle_timing;
    let mut speed = t.speed as f64;
    let mut changed = kit::row(ui, "Subtitle Speed", 56.0, |ui| {
        kit::number_box(ui, "sub-speed", &mut speed, 0.1..=10.0, 0.05, |v| format!("{v:.3} fps"), BOX_W)
    });
    gap(ui, 5.0);
    changed |= kit::slider(ui, &mut speed, 0.1..=10.0, &kit::Sl::default().step(0.05).reset(1.0)).changed();
    let mut factor = t.duration_factor as f64;
    changed |= kit::row(ui, "Subtitle Duration Factor", 56.0, |ui| {
        kit::number_box(ui, "sub-factor", &mut factor, 0.0..=20.0, 0.1, |v| format!("{v:.3}"), BOX_W)
    });
    gap(ui, 5.0);
    changed |= kit::slider(ui, &mut factor, 0.0..=20.0, &kit::Sl::default().step(0.1).reset(0.0)).changed();
    if changed {
        t.speed = speed as f32;
        t.duration_factor = factor as f32;
        app.set_subtitle_timing(t);
    }
    kit::row_with(
        ui,
        "Manual Sync",
        Some("Mark when you hear a line, then when you see it; Apply sets the subtitle delay from the difference."),
        48.0,
        |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if kit::chip(ui, "Reset", "Clear the marks and the subtitle delay").clicked() {
                app.subtitle_sync_reset();
            }
            if kit::chip(ui, "Apply", "Apply the difference as subtitle delay").clicked() {
                app.subtitle_sync_apply();
            }
            if kit::chip(ui, "Sub", "Mark the moment the subtitle appears").clicked() {
                app.subtitle_sync_mark_subtitle();
            }
            if kit::chip(ui, "Audio", "Mark the moment you hear the line").clicked() {
                app.subtitle_sync_mark_audio();
            }
        },
    );
    gap(ui, 10.0);
    kit::rule(ui);

    kit::section(ui, "Appearance");
    gap(ui, 10.0);
    appearance(ui, app, st);
}

/// Subtitle style options. They rebuild the engine, so edits collect in a
/// draft until the pointer is released and no field is being typed in.
fn appearance(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    let mut s: SubtitleSettings = st.panels.sub_draft.clone().unwrap_or_else(|| app.settings.subtitles.clone());
    let mut embedded = !s.ignore_ass_styling;
    if kit::toggle_row(ui, "Embedded Sub Format", &mut embedded, 48.0) {
        s.ignore_ass_styling = !embedded;
    }
    gap(ui, 5.0);
    if let Some(f) = kit::row(ui, "Subtitle Font", 48.0, |ui| language_box(ui, "sub-font", &s.font)) {
        s.font = f;
    }
    gap(ui, 5.0);
    kit::row(ui, "Text Color", 48.0, |ui| kit::color_box(ui, "sub-text-color", &mut s.text_color, vec2(BOX_W, 36.0)));
    gap(ui, 5.0);
    kit::row(ui, "Outline Color", 48.0, |ui| {
        kit::color_box(ui, "sub-outline-color", &mut s.outline_color, vec2(BOX_W, 36.0))
    });
    gap(ui, 5.0);
    kit::row(ui, "Size Values", 50.0, |ui| {
        let sel = if s.size_units == SizeUnits::Pixels { 1 } else { 0 };
        if let Some(i) = kit::segmented(ui, "sub-units", sel, &["Relative", "Pixels"], vec2(180.0, 38.0)) {
            s.size_units = if i == 1 { SizeUnits::Pixels } else { SizeUnits::Relative };
        }
    });
    gap(ui, 10.0);
    if s.size_units == SizeUnits::Pixels {
        let text = if s.font_size_px == 0 { "Auto".to_string() } else { format!("{} px", s.font_size_px) };
        kit::label_value(ui, "Font Size", &text);
        let mut px = s.font_size_px as i32;
        if kit::slider_i32(ui, &mut px, 0..=96, &kit::Sl::default().ticks(8)).changed() {
            s.font_size_px = px.max(0) as u32;
        }
    } else {
        kit::label_value(ui, "Font Size", s.font_size.label());
        let mut i = FontSize::ALL.iter().position(|f| *f == s.font_size).unwrap_or(0) as i32;
        let n = FontSize::ALL.len() as i32 - 1;
        if kit::slider_i32(ui, &mut i, 0..=n, &kit::Sl::default().ticks(n as usize)).changed() {
            s.font_size = FontSize::ALL[i.clamp(0, n) as usize];
        }
    }
    gap(ui, 15.0);
    kit::label_value(ui, "Outline Thickness", s.outline.label());
    let mut i = Outline::ALL.iter().position(|f| *f == s.outline).unwrap_or(0) as i32;
    let n = Outline::ALL.len() as i32 - 1;
    if kit::slider_i32(ui, &mut i, 0..=n, &kit::Sl::default().ticks(n as usize)).changed() {
        s.outline = Outline::ALL[i.clamp(0, n) as usize];
    }
    gap(ui, 15.0);
    let mut pct = (s.background_opacity * 100.0).round() as i32;
    kit::label_value(ui, "Background Opacity", &format!("{pct} %"));
    if kit::slider_i32(ui, &mut pct, 0..=100, &kit::Sl::default()).changed() {
        s.background_opacity = pct as f32 / 100.0;
        if pct > 0 && s.background == crate::domain::settings::SubBackground::None {
            s.background = crate::domain::settings::SubBackground::Box;
        }
    }
    gap(ui, 10.0);

    if s == app.settings.subtitles {
        st.panels.sub_draft = None;
        return;
    }
    let busy = ui.input(|i| i.pointer.any_down()) || ui.ctx().memory(|m| m.focused().is_some());
    if busy {
        st.panels.sub_draft = Some(s);
    } else {
        st.panels.sub_draft = None;
        app.update_settings(|x| x.subtitles = s);
    }
}
