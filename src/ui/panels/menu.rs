//! The player's right-click menu (`6065:13616`): 292pt wide, 48pt rows,
//! shortcuts on the right, submenus for every area of the player.

use egui::{Align2, Color32, CornerRadius, Rect, Response, Sense, Ui, Vec2, pos2, vec2};

use super::{kit, queue};
use crate::app::{App, UiCommand};
use crate::domain::effects::{ASPECT_RATIOS, CROP_RATIOS, DeinterlaceMode, StereoMode, ZOOM_LEVELS};
use crate::domain::settings::{RepeatMode, SPEED_STEPS, Visualizer};
use crate::domain::shortcuts::Action;
use crate::ui::icons::Icon;
use crate::ui::state::{PlayerPanel, UiState};
use crate::ui::theme::{Palette, font};
use crate::ui::widgets::{self, pal};

const WIDTH: f32 = 292.0;
const ROW_H: f32 = 48.0;

fn frame(p: &Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(p.bg_a1)
        .corner_radius(10)
        .inner_margin(egui::Margin::same(4))
        .stroke(egui::Stroke::new(1.0, p.divider_dim))
        .shadow(egui::epaint::Shadow { offset: [0, 8], blur: 30, spread: 0, color: Color32::from_black_alpha(118) })
}

/// Styles a menu ui so submenus open with the same look and rows touch.
fn style_menu(ui: &mut Ui) {
    let p = pal(ui.ctx());
    let s = ui.style_mut();
    s.spacing.item_spacing = vec2(0.0, 0.0);
    s.spacing.menu_margin = egui::Margin::same(4);
    s.visuals.menu_corner_radius = CornerRadius::same(10);
    s.visuals.window_fill = p.bg_a1;
    ui.set_min_width(WIDTH - 8.0);
}

enum Right<'a> {
    None,
    Text(&'a str),
    Chevron,
    Check,
}

fn row(ui: &mut Ui, icon: Option<(Icon, Color32)>, label: &str, right: Right) -> Response {
    let p = pal(ui.ctx());
    let w = ui.available_width().max(WIDTH - 8.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, ROW_H), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(6), p.hover);
    }
    let mut x = rect.min.x + 14.0;
    if let Some((i, c)) = icon {
        widgets::paint_icon(ui, Rect::from_min_size(pos2(x, rect.center().y - 12.0), Vec2::splat(24.0)), i, c);
        x += 34.0;
    }
    let fg = if resp.hovered() { p.fg2 } else { p.fg_a2 };
    let right_w = match right {
        Right::Text(t) => ui.painter().layout_no_wrap(t.to_string(), font::regular(14.0), fg).size().x + 24.0,
        Right::None => 14.0,
        _ => 44.0,
    };
    widgets::text_clipped(
        ui,
        pos2(x, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font::regular(14.0),
        fg,
        rect.max.x - right_w - x,
    );
    let rc = Rect::from_center_size(pos2(rect.max.x - 26.0, rect.center().y), Vec2::splat(24.0));
    match right {
        Right::Text(t) => {
            ui.painter().text(
                pos2(rect.max.x - 14.0, rect.center().y),
                Align2::RIGHT_CENTER,
                t,
                font::regular(14.0),
                p.fg_a3,
            );
        }
        Right::Chevron => widgets::paint_icon(ui, rc, Icon::ChevronRight, p.fg_a2),
        Right::Check => kit::paint_check(ui, rc, p.accent),
        Right::None => {}
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn divider(ui: &mut Ui) {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 1.0), Sense::hover());
    ui.painter().rect_filled(r, 0, p.bg_a1);
}

fn sub(ui: &mut Ui, icon: Option<(Icon, Color32)>, label: &str, add: impl FnOnce(&mut Ui)) {
    let resp = row(ui, icon, label, Right::Chevron);
    egui::containers::menu::SubMenu::new().show(ui, &resp, |ui| {
        style_menu(ui);
        egui::ScrollArea::vertical().max_height(560.0).show(ui, add);
    });
}

/// A plain choice row with a check when selected.
fn pick(ui: &mut Ui, label: &str, on: bool) -> bool {
    row(ui, None, label, if on { Right::Check } else { Right::None }).clicked()
}

fn keys(app: &App, a: Action) -> String {
    app.settings.controls.shortcuts.keys(a).first().map(|k| k.to_string()).unwrap_or_default()
}

/// Runs a shortcut action; window actions come back as commands.
fn run(app: &mut App, st: &mut UiState, a: Action) {
    if let Some(c) = app.perform(a) {
        st.command(c);
    }
}

/// A row for a shortcut action with its key on the right.
fn action(ui: &mut Ui, app: &mut App, st: &mut UiState, icon: Option<(Icon, Color32)>, label: &str, a: Action) {
    let k = keys(app, a);
    if row(ui, icon, label, Right::Text(&k)).clicked() {
        run(app, st, a);
    }
}

pub fn show(resp: &Response, app: &mut App, st: &mut UiState) {
    let p = pal(&resp.ctx);
    let id = egui::Popup::default_response_id(resp);
    if let Some(pos) = st.panels.context_menu_at.take() {
        // The same call egui's context menu makes on a right click, so the
        // menu opens at the More button exactly as it would at the pointer.
        #[allow(deprecated)]
        resp.ctx.memory_mut(|m| m.open_popup_at(id, Some(pos)));
    }
    egui::Popup::context_menu(resp).frame(frame(&p)).width(WIDTH).show(|ui| {
        style_menu(ui);
        contents(ui, app, st, &p);
    });
}

fn contents(ui: &mut Ui, app: &mut App, st: &mut UiState, p: &Palette) {
    let orange = p.accent_dim;
    let grey = p.fg_a2;
    let playing = app.is_playing();
    let (icon, label) = if playing { (Icon::Pause, "Pause") } else { (Icon::Playback, "Play") };
    action(ui, app, st, Some((icon, orange)), label, Action::PlayPause);
    action(ui, app, st, Some((Icon::Stop2tone, orange)), "Stop", Action::Stop);
    action(ui, app, st, Some((Icon::Previous2tone, orange)), "Previous Item", Action::Previous);
    action(ui, app, st, Some((Icon::Next2tone, orange)), "Next Item", Action::Next);
    let rec = if app.status.recording { "Stop Recording" } else { "Record" };
    action(ui, app, st, Some((Icon::Record2tone, grey)), rec, Action::Record);
    divider(ui);
    action(ui, app, st, Some((Icon::Ui2tone, grey)), "Hide UI", Action::HideUi);
    divider(ui);
    sub(ui, Some((Icon::PlaybackSelected2tone, grey)), "Playback", |ui| playback_menu(ui, app, st));
    sub(ui, Some((Icon::Video2tone, grey)), "Video", |ui| video_menu(ui, app, st));
    sub(ui, Some((Icon::Audio2tone, grey)), "Audio", |ui| audio_menu(ui, app, st));
    sub(ui, Some((Icon::Subtitles2tone, grey)), "Subtitle", |ui| subtitle_menu(ui, app, st));
    sub(ui, Some((Icon::Settings2tone, grey)), "More Settings", |ui| {
        action(ui, app, st, None, "Effects and Filters", Action::EffectsFilters);
        action(ui, app, st, None, "Media Information", Action::MediaInfo);
        action(ui, app, st, None, "Keyboard Shortcuts", Action::KeyboardShortcuts);
        action(ui, app, st, None, "Convert / Save", Action::ConvertSave);
        action(ui, app, st, None, "Preferences", Action::Preferences);
        if row(ui, None, "Customize Player", Right::None).clicked() {
            st.panels.customize_open = true;
        }
    });
    sub(ui, Some((Icon::Playlist2tone, grey)), "Playlist", |ui| playlist_menu(ui, app, st));
    sub(ui, Some((Icon::File2tone, grey)), "Open Media", |ui| open_menu(ui, app, st));
    action(ui, app, st, Some((Icon::Quit2tone, grey)), "Quit VLC", Action::Quit);
}

fn playback_menu(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    let rate = if app.status.rate > 0.0 { app.status.rate } else { 1.0 };
    sub(ui, None, "Speed", |ui| {
        for s in SPEED_STEPS {
            if pick(ui, &super::quick::format_rate(*s as f64), (rate - s).abs() < 0.01) {
                app.set_rate(*s);
            }
        }
        divider(ui);
        action(ui, app, st, None, "Faster", Action::Faster);
        action(ui, app, st, None, "Slower", Action::Slower);
        action(ui, app, st, None, "Normal Speed", Action::NormalSpeed);
    });
    action(ui, app, st, None, "Jump to Time", Action::JumpToTime);
    action(ui, app, st, None, "Jump Forward", Action::JumpForwardShort);
    action(ui, app, st, None, "Jump Backward", Action::JumpBackwardShort);
    let ab = match (app.ab_loop.a, app.ab_loop.b) {
        (None, _) => "A-B Loop: Set A",
        (Some(_), None) => "A-B Loop: Set B",
        _ => "A-B Loop: Off",
    };
    action(ui, app, st, None, ab, Action::AbLoop);
    action(ui, app, st, None, "Next Frame", Action::NextFrame);
    let titles = app.status.titles.clone();
    if titles.len() > 1 {
        sub(ui, None, "Title", |ui| {
            for (i, t) in titles.iter().enumerate() {
                let name = if t.name.is_empty() { format!("Title {}", i + 1) } else { t.name.clone() };
                if pick(ui, &name, app.status.title == i as i32) {
                    app.set_title(i as i32);
                }
            }
        });
    }
    let chapters = app.status.chapters.clone();
    if !chapters.is_empty() {
        sub(ui, None, "Chapter", |ui| {
            for (i, c) in chapters.iter().enumerate() {
                let name = if c.name.is_empty() { format!("Chapter {}", i + 1) } else { c.name.clone() };
                if pick(ui, &name, app.status.chapter == i as i32) {
                    app.set_chapter(i as i32);
                }
            }
        });
    }
    sub(ui, None, "Navigation", |ui| {
        action(ui, app, st, None, "Disc Menu", Action::DiscMenu);
        action(ui, app, st, None, "Previous Title", Action::PreviousTitle);
        action(ui, app, st, None, "Next Title", Action::NextTitle);
        action(ui, app, st, None, "Previous Chapter", Action::PreviousChapter);
        action(ui, app, st, None, "Next Chapter", Action::NextChapter);
    });
    let marks = app.bookmarks();
    sub(ui, None, "Bookmarks", |ui| {
        action(ui, app, st, None, "Add Bookmark", Action::AddBookmark);
        if row(ui, None, "Manage Bookmarks", Right::None).clicked() {
            st.panel = Some(PlayerPanel::Queue(1));
        }
        if !marks.is_empty() {
            divider(ui);
        }
        for b in &marks {
            if row(ui, None, &b.name, Right::None).clicked() {
                app.seek_ms(b.time_ms);
            }
        }
    });
}

fn ratio_menu(ui: &mut Ui, value: &mut Option<String>, builtin: &[&str], custom: &str) {
    if pick(ui, "Default", value.is_none()) {
        *value = None;
    }
    let extra = custom.split(',').map(str::trim).filter(|s| !s.is_empty());
    for r in builtin.iter().copied().chain(extra) {
        if pick(ui, r, value.as_deref() == Some(r)) {
            *value = Some(r.to_string());
        }
    }
}

fn video_menu(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    let k = keys(app, Action::ToggleFullscreen);
    if row(ui, None, "Fullscreen", if st.fullscreen { Right::Check } else { Right::Text(&k) }).clicked() {
        st.command(UiCommand::ToggleFullscreen);
    }
    if row(ui, None, "Always on Top", if st.always_on_top { Right::Check } else { Right::None }).clicked() {
        st.command(UiCommand::AlwaysOnTop);
    }
    action(ui, app, st, None, "Picture in Picture", Action::PictureInPicture);
    let custom_aspect = app.settings.video.custom_aspect_ratios.clone();
    sub(ui, None, "Aspect Ratio", |ui| ratio_menu(ui, &mut app.geometry.aspect, ASPECT_RATIOS, &custom_aspect));
    let custom_crop = app.settings.video.custom_crop_ratios.clone();
    sub(ui, None, "Crop", |ui| ratio_menu(ui, &mut app.geometry.crop, CROP_RATIOS, &custom_crop));
    sub(ui, None, "Zoom", |ui| {
        for (label, z) in ZOOM_LEVELS {
            if pick(ui, label, (app.geometry.zoom - z).abs() < 0.001) {
                app.geometry.zoom = *z;
            }
        }
    });
    sub(ui, None, "Deinterlace", |ui| {
        let (mode, method) = (app.settings.video.deinterlace, app.settings.video.deinterlace_method.clone());
        for m in DeinterlaceMode::ALL {
            if pick(ui, m.label(), mode == m) {
                app.set_deinterlace(m, method.clone());
            }
        }
    });
    action(ui, app, st, None, "Take Snapshot", Action::Snapshot);
    let tracks = app.status.video_tracks.clone();
    sub(ui, None, "Video Track", |ui| {
        for t in &tracks {
            let name = if t.id < 0 { "None" } else { t.name.as_str() };
            if pick(ui, name, app.status.video_track == t.id) {
                app.set_video_track(t.id);
            }
        }
    });
}

fn audio_menu(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    let tracks = app.status.audio_tracks.clone();
    sub(ui, None, "Audio Track", |ui| {
        for t in &tracks {
            let name = if t.id < 0 { "None" } else { t.name.as_str() };
            if pick(ui, name, app.status.audio_track == t.id) {
                app.set_audio_track(t.id);
            }
        }
    });
    let devices = app.engine.audio_devices();
    let current = app.status.audio_device.clone().or_else(|| app.settings.audio.output_device.clone());
    sub(ui, None, "Audio Device", |ui| {
        for d in &devices {
            let name = if d.name.is_empty() { "System Default" } else { d.name.as_str() };
            if pick(ui, name, current.as_deref() == Some(d.id.as_str())) {
                app.set_audio_device(if d.id.is_empty() { None } else { Some(d.id.clone()) });
            }
        }
    });
    sub(ui, None, "Stereo Mode", |ui| {
        for m in StereoMode::ALL {
            if pick(ui, m.label(), app.stereo_mode == m) {
                app.set_stereo_mode(m);
            }
        }
    });
    sub(ui, None, "Visualization", |ui| {
        for v in Visualizer::ALL {
            if pick(ui, v.label(), app.settings.audio.visualizer == *v) {
                app.set_visualizer(*v);
            }
        }
    });
    divider(ui);
    action(ui, app, st, None, "Volume Up", Action::VolumeUp);
    action(ui, app, st, None, "Volume Down", Action::VolumeDown);
    action(ui, app, st, None, if app.status.muted { "Unmute" } else { "Mute" }, Action::Mute);
}

fn subtitle_menu(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    let tracks = app.status.subtitle_tracks.clone();
    sub(ui, None, "Subtitle Track", |ui| {
        for t in &tracks {
            let name = if t.id < 0 { "None" } else { t.name.as_str() };
            if pick(ui, name, app.status.subtitle_track == t.id) {
                app.set_subtitle_track(t.id);
            }
        }
    });
    action(ui, app, st, None, "Add Subtitle File...", Action::AddSubtitleFile);
    action(ui, app, st, None, "Toggle Subtitles", Action::ToggleSubtitles);
    action(ui, app, st, None, "Delay Up", Action::SubtitleDelayUp);
    action(ui, app, st, None, "Delay Down", Action::SubtitleDelayDown);
    if row(ui, None, "Subtitle Settings", Right::None).clicked() {
        st.panel = Some(PlayerPanel::QuickSettings(3));
    }
}

fn playlist_menu(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    action(ui, app, st, None, "Show Queue", Action::ToggleQueue);
    let repeat = app.queue.repeat;
    sub(ui, None, "Repeat", |ui| {
        for r in RepeatMode::ALL {
            if pick(ui, r.label(), repeat == *r) {
                app.set_repeat(*r);
            }
        }
    });
    let shuffle = app.queue.shuffle;
    let k = keys(app, Action::ToggleShuffle);
    if row(ui, None, "Shuffle", if shuffle { Right::Check } else { Right::Text(&k) }).clicked() {
        app.set_shuffle(!shuffle);
    }
    if row(ui, None, "Save Queue as Playlist", Right::None).clicked() {
        queue::save_as_playlist(app);
    }
}

fn open_menu(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    action(ui, app, st, None, "Open File...", Action::OpenFile);
    action(ui, app, st, None, "Open Folder...", Action::OpenFolder);
    action(ui, app, st, None, "Open Disc...", Action::OpenDisc);
    action(ui, app, st, None, "Open Network Stream...", Action::OpenNetwork);
    action(ui, app, st, None, "Open Capture Device...", Action::OpenCapture);
    let recent: Vec<_> = app.library.recent().into_iter().take(10).map(|(i, _)| i).collect();
    sub(ui, None, "Open Recent", |ui| {
        if recent.is_empty() {
            row(ui, None, "Nothing yet", Right::None);
        }
        for item in &recent {
            if row(ui, None, &item.title(), Right::None).clicked() {
                app.play_items(vec![item.clone()], 0, None);
            }
        }
    });
}
