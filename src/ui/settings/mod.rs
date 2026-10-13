//! The settings window: thirteen categories in three groups, search across
//! every page, and the keyboard shortcuts editor.
//!
//! Pages edit a draft copy of `app.settings`. Clicks commit at once through
//! `App::update_settings`; slider drags and typing commit when the pointer is
//! released or the field loses focus, so libvlc is not rebuilt on every
//! keystroke. Effects are the exception: they go through live so the
//! equalizer and picture sliders can be heard and seen while dragging.

mod audio;
pub mod extensions;
mod general;
mod kit;
mod library;
mod media;
pub mod shortcuts;
mod system;

use std::time::Instant;

use egui::{Align2, Color32, CornerRadius, Id, Rect, Sense, Ui, UiBuilder, Vec2, pos2, vec2};

use crate::app::App;
use crate::app::ports::AudioDevice;
use crate::domain::settings::Settings;
use crate::ui::icons::Icon;
use crate::ui::state::UiState;
use crate::ui::theme::font;
use crate::ui::widgets::{self, paint_icon, pal};

use kit::{Form, Mode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Category {
    #[default]
    General,
    Interface,
    Privacy,
    Controls,
    Playback,
    Video,
    Audio,
    Subtitles,
    Library,
    Network,
    Performance,
    Recording,
    Extensions,
}

impl Category {
    pub const ALL: [Category; 13] = [
        Category::General,
        Category::Interface,
        Category::Privacy,
        Category::Controls,
        Category::Playback,
        Category::Video,
        Category::Audio,
        Category::Subtitles,
        Category::Library,
        Category::Network,
        Category::Performance,
        Category::Recording,
        Category::Extensions,
    ];

    /// The sidebar groups: (heading, categories).
    pub const GROUPS: [(&'static str, &'static [Category]); 3] = [
        ("App", &[Category::General, Category::Interface, Category::Privacy, Category::Controls]),
        ("Media", &[Category::Playback, Category::Video, Category::Audio, Category::Subtitles, Category::Library]),
        ("System", &[Category::Network, Category::Performance, Category::Recording, Category::Extensions]),
    ];

    pub fn name(self) -> &'static str {
        match self {
            Category::General => "General",
            Category::Interface => "Interface",
            Category::Privacy => "Privacy",
            Category::Controls => "Controls",
            Category::Playback => "Playback",
            Category::Video => "Video",
            Category::Audio => "Audio",
            Category::Subtitles => "Subtitles",
            Category::Library => "Library",
            Category::Network => "Network",
            Category::Performance => "Performance",
            Category::Recording => "Recording",
            Category::Extensions => "Extensions",
        }
    }

    pub fn from_name(s: &str) -> Option<Category> {
        Category::ALL.iter().copied().find(|c| c.name().eq_ignore_ascii_case(s))
    }

    /// The page title shown in the window header.
    pub fn title(self) -> &'static str {
        match self {
            Category::General => "Global Settings",
            Category::Interface => "Interface Settings",
            Category::Privacy => "Privacy Settings",
            Category::Controls => "Mouse, Keyboard & Remote Controls",
            Category::Playback => "Playback Preferences",
            Category::Video => "Video Defaults",
            Category::Audio => "Audio Defaults",
            Category::Subtitles => "Subtitle Defaults",
            Category::Library => "Playlists, Folders & Files",
            Category::Network => "Network Connections & Security",
            Category::Performance => "Performance Settings",
            Category::Recording => "Recording, Casting & Streaming Out",
            Category::Extensions => "Add-ons & Scripts",
        }
    }

    pub fn icon(self, active: bool) -> Icon {
        match (self, active) {
            (Category::General | Category::Performance, false) => Icon::Preferences,
            (Category::General | Category::Performance, true) => Icon::PreferencesActive,
            (Category::Interface, false) => Icon::Interface,
            (Category::Interface, true) => Icon::InterfaceActive,
            (Category::Privacy, false) => Icon::Privacy,
            (Category::Privacy, true) => Icon::PrivacyActive,
            (Category::Controls, false) => Icon::Hotkeys,
            (Category::Controls, true) => Icon::HotkeysActive,
            (Category::Playback, false) => Icon::Playback,
            (Category::Playback, true) => Icon::PlaybackActive,
            (Category::Video, false) => Icon::Video,
            (Category::Video, true) => Icon::VideoActive,
            (Category::Audio, false) => Icon::Audio,
            (Category::Audio, true) => Icon::AudioActive,
            (Category::Subtitles, false) => Icon::Subtitles,
            (Category::Subtitles, true) => Icon::SubtitlesActive,
            (Category::Library, false) => Icon::Folder,
            (Category::Library, true) => Icon::FolderActive,
            (Category::Network, false) => Icon::Streaming,
            (Category::Network, true) => Icon::StreamingActive,
            (Category::Recording, false) => Icon::Record,
            (Category::Recording, true) => Icon::RecordActive,
            (Category::Extensions, false) => Icon::Plugins,
            (Category::Extensions, true) => Icon::PluginsActive,
        }
    }

    /// Paints the sidebar icon. Interface, Privacy and Recording only have a
    /// usable outline in their "active" artwork, so their idle state draws
    /// that artwork in a single colour.
    pub fn paint_icon(self, ui: &Ui, rect: Rect, selected: bool, color: Color32) {
        let mono = matches!(self, Category::Interface | Category::Privacy | Category::Recording);
        if mono && !selected {
            let colors = crate::ui::icons::IconColors { fg: color, accent: color, accent_dim: color };
            crate::ui::icons::paint(ui, rect, self.icon(true), colors);
        } else {
            paint_icon(ui, rect, self.icon(selected), color);
        }
    }

    fn index(self) -> usize {
        Category::ALL.iter().position(|c| *c == self).unwrap_or(0)
    }
}

/// Destructive actions waiting for a yes/no.
#[derive(Debug, Clone, PartialEq)]
pub enum Confirm {
    ResetSettings,
    ResetShortcuts,
    RemoveExtension(std::path::PathBuf),
}

#[derive(Default)]
pub struct SettingsState {
    pub open: bool,
    pub category: Category,
    pub search: String,
    /// "Advanced Options" expanded, per category.
    pub advanced: [bool; 13],
    /// The Audio page's equalizer is open.
    pub eq_open: bool,
    /// Scroll the content to this offset on the next frame (snapshots).
    pub scroll_to: Option<f32>,
    pub confirm: Option<Confirm>,
    pub extensions: extensions::ExtensionsState,
    pub shortcuts: shortcuts::ShortcutsState,
    /// Edits not committed yet, and `app.settings` when the draft started.
    draft: Option<(Settings, Settings)>,
    was_open: bool,
    devices: Option<(Instant, Vec<AudioDevice>)>,
}

impl SettingsState {
    pub fn set_advanced(&mut self, c: Category, open: bool) {
        self.advanced[c.index()] = open;
    }
}

/// Deferred work that needs the whole app, run after the pages are drawn.
pub type Later = Box<dyn FnOnce(&mut App, &mut UiState)>;

/// What pages see besides the settings they edit.
pub struct Cx<'a> {
    pub app: &'a App,
    pub ss: &'a mut SettingsState,
    pub later: Vec<Later>,
}

impl Cx<'_> {
    pub fn later(&mut self, f: impl FnOnce(&mut App, &mut UiState) + 'static) {
        self.later.push(Box::new(f));
    }

    pub fn toast(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.later(move |app, _| app.toast(text));
    }

    /// Output devices, refreshed every few seconds while the window is open.
    pub fn audio_devices(&mut self) -> Vec<AudioDevice> {
        let stale = self.ss.devices.as_ref().is_none_or(|(t, _)| t.elapsed().as_secs() >= 3);
        if stale {
            self.ss.devices = Some((Instant::now(), self.app.engine.audio_devices()));
        }
        self.ss.devices.as_ref().map(|(_, d)| d.clone()).unwrap_or_default()
    }
}

fn draw_page(c: Category, f: &mut Form, s: &mut Settings, cx: &mut Cx) {
    match c {
        Category::General => general::general(f, s, cx),
        Category::Interface => general::interface(f, s, cx),
        Category::Privacy => general::privacy(f, s, cx),
        Category::Controls => general::controls(f, s, cx),
        Category::Playback => media::playback(f, s, cx),
        Category::Video => media::video(f, s, cx),
        Category::Audio => audio::audio(f, s, cx),
        Category::Subtitles => media::subtitles(f, s, cx),
        Category::Library => library::library(f, s, cx),
        Category::Network => system::network(f, s, cx),
        Category::Performance => system::performance(f, s, cx),
        Category::Recording => system::recording(f, s, cx),
        Category::Extensions => extensions::page(f, s, cx),
    }
}

pub fn show(ctx: &egui::Context, app: &mut App, st: &mut UiState) {
    // Escape closes the top-most surface: when the shell closed the window
    // while the shortcuts editor sat on top of it, close the editor instead.
    if st.settings.was_open && !st.settings.open && st.dialogs.shortcuts {
        st.settings.open = true;
        st.dialogs.shortcuts = false;
        st.settings.shortcuts = Default::default();
    }
    st.settings.was_open = st.settings.open;
    if st.settings.open {
        window(ctx, app, st);
    } else if st.settings.draft.is_some() {
        commit(ctx, app, &mut st.settings, true);
    }
    if st.settings.open && st.settings.confirm.is_some() {
        confirmations(ctx, app, st);
    }
    shortcuts::show(ctx, app, st);
}

fn confirmations(ctx: &egui::Context, app: &mut App, st: &mut UiState) {
    let Some(c) = st.settings.confirm.clone() else { return };
    let (title, body, ok) = match &c {
        Confirm::ResetSettings => (
            "Reset all settings?",
            "Every preference on every page goes back to its default. Your library, playlists and history are kept.",
            "Reset",
        ),
        Confirm::ResetShortcuts => {
            ("Reset all shortcuts?", "Every keyboard shortcut goes back to its default key.", "Reset")
        }
        Confirm::RemoveExtension(_) => {
            ("Remove this add-on?", "The script file is deleted from VLC's add-ons folder.", "Remove")
        }
    };
    match kit::confirm(ctx, "settings", title, body, ok) {
        Some(true) => {
            st.settings.confirm = None;
            st.settings.draft = None;
            match c {
                Confirm::ResetSettings => app.reset_settings(),
                Confirm::ResetShortcuts => {
                    app.update_settings(|s| s.controls.shortcuts.reset());
                    app.toast("Keyboard shortcuts were reset");
                }
                Confirm::RemoveExtension(path) => extensions::remove(app, &mut st.settings.extensions, &path),
            }
        }
        Some(false) => st.settings.confirm = None,
        None => {}
    }
}

/// Window size: the design's 1048x865, shrunk to fit small windows.
fn window_size(ctx: &egui::Context) -> Vec2 {
    let screen = ctx.content_rect();
    vec2(1048f32.min(screen.width() - 24.0), 865f32.min(screen.height() - 24.0)).max(vec2(600.0, 400.0))
}

fn window(ctx: &egui::Context, app: &mut App, st: &mut UiState) {
    let size = window_size(ctx);
    egui::Modal::new(Id::new("settings-window"))
        .frame(egui::Frame::NONE)
        .backdrop_color(Color32::from_black_alpha(128))
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            body(ui, rect, app, st);
        });
}

fn body(ui: &mut Ui, rect: Rect, app: &mut App, st: &mut UiState) {
    let p = pal(ui.ctx());
    let painter = ui.painter();
    painter.add(
        egui::epaint::Shadow { offset: [0, 12], blur: 40, spread: 0, color: Color32::from_black_alpha(110) }
            .as_shape(rect, CornerRadius::same(14)),
    );
    let window_fill = if p.dark { p.bg_b1_t } else { crate::ui::theme::with_alpha(p.bg_b1, 0.98) };
    painter.rect_filled(rect, CornerRadius::same(14), window_fill);

    // Header: search, page title, close.
    let header = Rect::from_min_size(rect.min, vec2(rect.width(), 50.0));
    let header_fill = if p.dark { Color32::BLACK } else { p.chrome };
    painter.rect_filled(header, CornerRadius { nw: 14, ne: 14, sw: 0, se: 0 }, header_fill);
    let search_rect = Rect::from_min_size(rect.min + vec2(8.0, 8.0), vec2(200.0, 34.0));
    let before = st.settings.search.clone();
    ui.scope_builder(UiBuilder::new().max_rect(search_rect), |ui| {
        widgets::search_field(ui, &mut st.settings.search, "Search", 200.0);
    });
    if st.settings.search != before {
        st.settings.scroll_to = Some(0.0);
    }
    let searching = !st.settings.search.trim().is_empty();
    let title = if searching { "Search Results" } else { st.settings.category.title() };
    ui.painter().text(
        pos2(rect.min.x + 240.0, header.center().y),
        Align2::LEFT_CENTER,
        title,
        font::regular(16.0),
        p.fg_a1,
    );
    let close = Rect::from_min_size(pos2(rect.max.x - 46.0, rect.min.y + 9.0), vec2(32.0, 32.0));
    let closed = ui
        .scope_builder(UiBuilder::new().max_rect(close), |ui| {
            widgets::icon_button_sized(ui, Icon::Close, vec2(32.0, 32.0), 24.0, false, "Close").clicked()
        })
        .inner;

    // Sidebar.
    let side =
        Rect::from_min_max(pos2(rect.min.x + 8.0, rect.min.y + 58.0), pos2(rect.min.x + 208.0, rect.max.y - 8.0));
    ui.painter().rect_filled(side, CornerRadius::same(6), if p.dark { p.bg_a4 } else { p.bg_a2 });
    ui.scope_builder(UiBuilder::new().max_rect(side.shrink(6.0)), |ui| {
        egui::ScrollArea::vertical()
            .id_salt("settings-side")
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| sidebar(ui, &mut st.settings, searching));
    });

    // Content.
    let content = Rect::from_min_max(pos2(rect.min.x + 216.0, rect.min.y + 50.0), rect.max);
    let mut s = st.settings.draft.as_ref().map(|(d, _)| d.clone()).unwrap_or_else(|| app.settings.clone());
    let start = s.clone();
    let mut cx = Cx { app, ss: &mut st.settings, later: Vec::new() };
    ui.scope_builder(UiBuilder::new().max_rect(content), |ui| {
        ui.set_clip_rect(content.intersect(ui.clip_rect()));
        let cat = cx.ss.category;
        let mut area = egui::ScrollArea::vertical()
            .id_salt(("settings-content", if searching { None } else { Some(cat) }))
            .auto_shrink([false, false]);
        if let Some(y) = cx.ss.scroll_to.take() {
            area = area.vertical_scroll_offset(y);
        }
        area.show(ui, |ui| {
            egui::Frame::new().inner_margin(egui::Margin { left: 24, right: 24, top: 20, bottom: 24 }).show(ui, |ui| {
                ui.set_width(content.width() - 48.0);
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                if searching {
                    search_results(ui, &mut s, &mut cx);
                } else {
                    let i = cat.index();
                    let query = String::new();
                    let mut f = Form::new(ui, Mode::Draw, &query, cx.ss.advanced[i]);
                    draw_page(cat, &mut f, &mut s, &mut cx);
                    let adv = f.advanced;
                    cx.ss.advanced[i] = adv;
                }
            });
        });
    });
    let later = std::mem::take(&mut cx.later);
    drop(cx);

    if s != start {
        let base = st.settings.draft.take().map(|(_, b)| b).unwrap_or_else(|| app.settings.clone());
        st.settings.draft = Some((s, base));
    }
    commit(ui.ctx(), app, &mut st.settings, closed);
    for f in later {
        f(app, st);
    }
    if closed {
        st.settings.open = false;
    }
}

fn sidebar(ui: &mut Ui, ss: &mut SettingsState, searching: bool) {
    let p = pal(ui.ctx());
    ui.spacing_mut().item_spacing = vec2(0.0, 8.0);
    for (gi, (group, cats)) in Category::GROUPS.iter().enumerate() {
        let _ = gi;
        let (r, _) = ui.allocate_exact_size(vec2(188.0, 33.0), Sense::hover());
        ui.painter().text(
            pos2(r.min.x + 10.0, r.center().y),
            Align2::LEFT_CENTER,
            *group,
            font::regular(14.0),
            p.fg_a3,
        );
        for c in cats.iter().copied() {
            let (r, resp) = ui.allocate_exact_size(vec2(188.0, 40.0), Sense::click());
            let selected = c == ss.category && !searching;
            if selected {
                ui.painter().rect_filled(r, CornerRadius::same(2), if p.dark { p.bg_a1 } else { p.bg_b2_t });
            } else if resp.hovered() {
                ui.painter().rect_filled(r, CornerRadius::same(2), p.hover);
            }
            let icon_rect = Rect::from_center_size(pos2(r.min.x + 20.0, r.center().y), Vec2::splat(24.0));
            c.paint_icon(ui, icon_rect, selected, if selected { p.accent } else { p.fg_a2 });
            let col = if selected || resp.hovered() { p.fg1 } else { p.fg2 };
            ui.painter().text(
                pos2(r.min.x + 44.0, r.center().y),
                Align2::LEFT_CENTER,
                c.name(),
                font::regular(14.0),
                col,
            );
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                if ss.category != c {
                    ss.scroll_to = Some(0.0);
                }
                ss.category = c;
                ss.search.clear();
            }
        }
    }
}

/// Every page's matching rows, grouped under the page name.
fn search_results(ui: &mut Ui, s: &mut Settings, cx: &mut Cx) {
    let p = pal(ui.ctx());
    let query = cx.ss.search.clone();
    let mut any = false;
    for c in Category::ALL {
        let hits = {
            let mut f = Form::new(ui, Mode::Count, &query, true);
            draw_page(c, &mut f, s, cx);
            f.hits
        };
        if hits == 0 {
            continue;
        }
        if any {
            ui.add_space(20.0);
        }
        any = true;
        let w = ui.available_width();
        let (r, resp) = ui.allocate_exact_size(vec2(w, 39.0), Sense::click());
        let col = if resp.hovered() { p.accent } else { p.fg2 };
        c.paint_icon(ui, Rect::from_min_size(pos2(r.min.x, r.min.y + 8.0), Vec2::splat(22.0)), false, col);
        ui.painter().text(pos2(r.min.x + 30.0, r.min.y + 10.0), Align2::LEFT_TOP, c.name(), font::medium(16.0), col);
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Open this page").clicked() {
            cx.ss.category = c;
            cx.ss.scroll_to = Some(0.0);
            cx.later(|_, st| st.settings.search.clear());
        }
        ui.push_id(("search", c), |ui| {
            let mut f = Form::new(ui, Mode::Draw, &query, true);
            draw_page(c, &mut f, s, cx);
        });
    }
    if !any {
        ui.add_space(80.0);
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new("No settings match your search").font(font::medium(16.0)).color(p.fg_a1));
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Try a different word, such as \"subtitle\" or \"volume\".")
                    .font(font::regular(14.0))
                    .color(p.fg_a3),
            );
        });
    }
}

/// Writes the draft back when nothing is being dragged or typed (or `force`).
fn commit(ctx: &egui::Context, app: &mut App, ss: &mut SettingsState, force: bool) {
    let Some((draft, base)) = ss.draft.take() else { return };
    // Effects reach the engine live, without rebuilding anything.
    if draft.audio.effects != app.settings.audio.effects || draft.video.effects != app.settings.video.effects {
        let (a, v) = (draft.audio.effects.clone(), draft.video.effects.clone());
        app.update_settings(|s| {
            s.audio.effects = a;
            s.video.effects = v;
        });
    }
    let busy = ctx.input(|i| i.pointer.any_down()) || ctx.memory(|m| m.focused().is_some());
    if busy && !force {
        ss.draft = Some((draft, base));
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
        return;
    }
    let merged = merge_settings(&base, &draft, &app.settings);
    app.update_settings(|s| *s = merged);
}

/// Applies the fields that changed between `base` and `draft` on top of
/// `current`, so changes made elsewhere meanwhile (volume, window size) are
/// kept.
fn merge_settings(base: &Settings, draft: &Settings, current: &Settings) -> Settings {
    if base == current {
        return draft.clone();
    }
    let (Ok(b), Ok(d), Ok(mut c)) =
        (serde_json::to_value(base), serde_json::to_value(draft), serde_json::to_value(current))
    else {
        return draft.clone();
    };
    merge_value(&b, &d, &mut c);
    serde_json::from_value(c).unwrap_or_else(|_| draft.clone())
}

fn merge_value(base: &serde_json::Value, draft: &serde_json::Value, cur: &mut serde_json::Value) {
    use serde_json::Value;
    match (base, draft, cur) {
        (Value::Object(b), Value::Object(d), Value::Object(c)) => {
            for (k, dv) in d {
                match (b.get(k), c.get_mut(k)) {
                    (Some(bv), Some(cv)) if bv != dv => merge_value(bv, dv, cv),
                    (Some(_), Some(_)) => {}
                    _ => {
                        c.insert(k.clone(), dv.clone());
                    }
                }
            }
        }
        (b, d, c) => {
            if b != d {
                *c = d.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_keeps_concurrent_changes() {
        let base = Settings::default();
        let mut draft = base.clone();
        draft.general.extra_args = "--foo".into();
        let mut current = base.clone();
        current.audio.volume = 42;
        let m = merge_settings(&base, &draft, &current);
        assert_eq!(m.general.extra_args, "--foo");
        assert_eq!(m.audio.volume, 42);
    }

    #[test]
    fn categories_round_trip_names() {
        for c in Category::ALL {
            assert_eq!(Category::from_name(c.name()), Some(c));
            assert_eq!(Category::ALL[c.index()], c);
        }
        let grouped: usize = Category::GROUPS.iter().map(|(_, c)| c.len()).sum();
        assert_eq!(grouped, Category::ALL.len());
    }
}
