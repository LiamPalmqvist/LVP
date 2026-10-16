//! The egui front end. Draws the design and turns input into calls on
//! `app::App`; never talks to libvlc directly.

pub mod dialogs;
pub mod icons;
pub mod images;
pub mod music;
pub mod pages;
pub mod panels;
pub mod settings;
pub mod shell;
pub mod state;
pub mod theme;
pub mod video;
pub mod widgets;

use std::path::PathBuf;
use std::time::Duration;

use egui::{Align2, Color32, Key, Rect, TextureId, ViewportCommand, pos2, vec2};

use crate::app::{App, UiCommand};
use crate::domain::media::{MediaKind, SUBTITLE_EXTENSIONS, kind_for_path};
use crate::domain::settings::{Accent, Theme};
use crate::domain::shortcuts::KeyChord;
use state::{PlayerPanel, UiState, View};
use theme::{Palette, font};

/// Per-frame resources owned by the UI.
pub struct Resources {
    pub video_tex: video::VideoTexture,
    /// The current picture, if any, as an egui texture and its pixel size.
    pub video_frame: Option<(TextureId, [u32; 2])>,
}

pub struct NotVlcApp {
    pub app: App,
    pub st: UiState,
    pub res: Resources,
    palette_key: Option<(Theme, Accent, bool)>,
    last_title: String,
}

impl NotVlcApp {
    pub fn new(ctx: &egui::Context, app: App, open: Vec<PathBuf>) -> Self {
        theme::install_fonts(ctx);
        let repaint = ctx.clone();
        app.set_repaint(move || repaint.request_repaint());
        let mut st = UiState { sidebar_expanded: app.settings.interface.sidebar_expanded, ..Default::default() };
        if !app.settings.general.onboarding_done {
            st.dialogs.onboarding = true;
        }
        let mut me = Self {
            app,
            st,
            res: Resources { video_tex: video::VideoTexture::new(), video_frame: None },
            palette_key: None,
            last_title: String::new(),
        };
        if !open.is_empty() {
            me.app.open_paths(open);
            me.st.view = View::Player;
        }
        if me.app.settings.video.start_fullscreen {
            me.st.fullscreen = true;
            ctx.send_viewport_cmd(ViewportCommand::Fullscreen(true));
        }
        me
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        let system_dark = ctx.input(|i| i.raw.system_theme).map(|t| t == egui::Theme::Dark).unwrap_or(true);
        let g = &self.app.settings.general;
        let key = (g.theme, g.accent, system_dark);
        if self.palette_key != Some(key) {
            let p = Palette::for_settings(g.theme, g.accent, system_dark);
            p.apply(ctx);
            widgets::set_palette(ctx, p);
            self.palette_key = Some(key);
        }
    }

    /// One full frame of UI. Separate from `eframe::App` so snapshots can
    /// drive it without a window.
    pub fn frame(&mut self, ctx: &egui::Context, rs: Option<&egui_wgpu::RenderState>) {
        self.app.tick();
        self.apply_theme(ctx);
        if let Some(rs) = rs {
            self.res.video_frame = self.res.video_tex.update(rs, &self.app.engine.video());
        }
        self.handle_drops(ctx);
        self.handle_keys(ctx);
        let p = widgets::pal(ctx);

        let chrome =
            !self.st.fullscreen && !self.st.hide_ui && !self.st.pip && self.app.settings.interface.show_decorations;
        egui::CentralPanel::default().frame(egui::Frame::new().fill(p.bg_b1)).show(ctx, |ui| {
            let full = ui.max_rect();
            let mut body = full;
            if chrome {
                let bar = Rect::from_min_size(full.min, vec2(full.width(), shell::titlebar::HEIGHT));
                ui.scope_builder(egui::UiBuilder::new().max_rect(bar), |ui| {
                    shell::titlebar::show(ui, &mut self.app, &mut self.st)
                });
                body.min.y += shell::titlebar::HEIGHT;
            }
            ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| match self.st.view {
                View::Library => shell::player::library(ui, &mut self.app, &mut self.st, &mut self.res),
                View::Player => shell::player::show(ui, &mut self.app, &mut self.st, &mut self.res),
            });
        });

        settings::show(ctx, &mut self.app, &mut self.st);
        dialogs::show(ctx, &mut self.app, &mut self.st);
        self.toasts(ctx);

        let pending = std::mem::take(&mut self.st.pending);
        for c in pending {
            self.handle_command(ctx, c);
        }
        if self.app.quit_requested {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        let title = shell::titlebar::window_title(&self.app);
        if title != self.last_title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
        if self.app.is_playing() {
            // Clock and seek bar updates; new pictures repaint on their own.
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }

    fn handle_drops(&mut self, ctx: &egui::Context) {
        // The New Playlist dialog takes dropped files itself.
        if self.st.dialogs.new_playlist {
            return;
        }
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if dropped.is_empty() {
            let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
            if hovering {
                let p = widgets::pal(ctx);
                let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("drop")));
                let r = ctx.content_rect();
                painter.rect_filled(r, 0, theme::with_alpha(Color32::BLACK, 0.6));
                painter.text(r.center(), Align2::CENTER_CENTER, "Drop to play", font::medium(24.0), p.fg1);
            }
            return;
        }
        let subs_only = dropped.iter().all(|p| kind_for_path(p) == MediaKind::Subtitle);
        if subs_only && self.app.has_media() {
            for s in dropped {
                self.app.add_subtitle(s);
            }
        } else {
            self.app.open_paths(dropped);
            self.st.view = View::Player;
        }
    }

    /// True while a modal surface owns the keyboard.
    fn modal_open(&self) -> bool {
        self.st.settings.open
            || self.st.dialogs.any_open()
            || self.st.dialogs.shortcuts
            || self.st.dialogs.new_playlist
            || self.st.panels.customize_open
            || !self.app.dialogs.is_empty()
    }

    fn handle_keys(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() {
            return;
        }
        let modal = self.modal_open();
        let events: Vec<(Key, egui::Modifiers)> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Key { key, pressed: true, modifiers, .. } => Some((*key, *modifiers)),
                    _ => None,
                })
                .collect()
        });
        for (key, m) in events {
            let Some(name) = key_name(key) else { continue };
            let chord = KeyChord { ctrl: m.command, alt: m.alt, shift: m.shift, key: name.to_string() };
            // Escape also closes whatever is open on top.
            if name == "Esc" && self.close_topmost() {
                continue;
            }
            // Modals handle their own keys (Esc, Enter); nothing reaches the player.
            if modal {
                continue;
            }
            if let Some(action) = self.app.settings.controls.shortcuts.action_for(&chord)
                && let Some(cmd) = self.app.perform(action)
            {
                self.st.command(cmd);
            }
        }
    }

    /// Closes the top-most transient surface. Returns whether anything closed.
    fn close_topmost(&mut self) -> bool {
        if self.st.settings.open {
            self.st.settings.open = false;
            return true;
        }
        if self.st.panel.is_some() {
            self.st.panel = None;
            return true;
        }
        false
    }

    pub fn handle_command(&mut self, ctx: &egui::Context, c: UiCommand) {
        let st = &mut self.st;
        match c {
            UiCommand::ToggleFullscreen => {
                st.fullscreen = !st.fullscreen;
                if st.fullscreen {
                    st.view = View::Player;
                }
                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(st.fullscreen));
            }
            UiCommand::LeaveFullscreen => {
                if st.fullscreen {
                    st.fullscreen = false;
                    ctx.send_viewport_cmd(ViewportCommand::Fullscreen(false));
                } else if st.hide_ui {
                    st.hide_ui = false;
                } else if st.pip {
                    self.toggle_pip(ctx);
                }
            }
            UiCommand::ShowControls => st.bump_controls(3.0),
            UiCommand::HideUi => {
                st.hide_ui = !st.hide_ui;
                st.view = View::Player;
            }
            UiCommand::ToggleSidebar => match st.view {
                View::Library => {
                    st.sidebar_expanded = !st.sidebar_expanded;
                    self.app.settings.interface.sidebar_expanded = st.sidebar_expanded;
                    self.app.mark_settings_dirty();
                }
                View::Player => st.side_panel = !st.side_panel,
            },
            UiCommand::ToggleQueue => {
                st.view = View::Player;
                st.toggle_panel(PlayerPanel::Queue(0));
            }
            UiCommand::ToggleMiniPlayer | UiCommand::PictureInPicture => self.toggle_pip(ctx),
            UiCommand::OpenFile => {
                let mut exts: Vec<&str> = crate::domain::media::VIDEO_EXTENSIONS.to_vec();
                exts.extend(crate::domain::media::AUDIO_EXTENSIONS);
                exts.extend(crate::domain::media::PLAYLIST_EXTENSIONS);
                if let Some(files) = rfd::FileDialog::new()
                    .set_title("Open Media")
                    .add_filter("Media files", &exts)
                    .add_filter("Subtitles", SUBTITLE_EXTENSIONS)
                    .add_filter("All files", &["*"])
                    .pick_files()
                {
                    self.app.open_paths(files);
                    self.st.view = View::Player;
                }
            }
            UiCommand::OpenFolder => {
                if let Some(dir) = rfd::FileDialog::new().set_title("Open Folder").pick_folder() {
                    self.app.open_paths(vec![dir]);
                    self.st.view = View::Player;
                }
            }
            UiCommand::OpenDisc => st.dialogs.open_disc = true,
            UiCommand::OpenNetwork => st.dialogs.open_network = true,
            UiCommand::OpenCapture => st.dialogs.capture = true,
            UiCommand::ConvertSave => st.dialogs.convert = true,
            UiCommand::Preferences => st.settings.open = true,
            UiCommand::MediaInfo => st.dialogs.media_info = true,
            UiCommand::EffectsFilters => {
                st.view = View::Player;
                st.panel = Some(PlayerPanel::QuickSettings(2));
            }
            UiCommand::KeyboardShortcuts => st.dialogs.shortcuts = true,
            UiCommand::JumpToTime => st.dialogs.jump_to_time = true,
            UiCommand::AlwaysOnTop => {
                st.always_on_top = !st.always_on_top;
                let level = if st.always_on_top { egui::WindowLevel::AlwaysOnTop } else { egui::WindowLevel::Normal };
                ctx.send_viewport_cmd(ViewportCommand::WindowLevel(level));
                self.app.show_osd(if st.always_on_top { "Always on top" } else { "Normal window" });
            }
            UiCommand::Wallpaper => self.app.toast("Wallpaper mode is only available on Windows."),
            UiCommand::AddSubtitleFile => {
                if let Some(f) = rfd::FileDialog::new()
                    .set_title("Add Subtitle File")
                    .add_filter("Subtitles", SUBTITLE_EXTENSIONS)
                    .pick_file()
                {
                    self.app.add_subtitle(f);
                }
            }
            UiCommand::Quit => ctx.send_viewport_cmd(ViewportCommand::Close),
        }
    }

    fn toggle_pip(&mut self, ctx: &egui::Context) {
        self.st.pip = !self.st.pip;
        self.st.view = View::Player;
        if self.st.pip {
            ctx.send_viewport_cmd(ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(480.0, 300.0)));
        } else {
            let level = if self.st.always_on_top { egui::WindowLevel::AlwaysOnTop } else { egui::WindowLevel::Normal };
            ctx.send_viewport_cmd(ViewportCommand::WindowLevel(level));
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(1280.0, 800.0)));
        }
    }

    fn toasts(&mut self, ctx: &egui::Context) {
        if self.app.toasts.is_empty() {
            return;
        }
        let p = widgets::pal(ctx);
        let screen = ctx.content_rect();
        let mut y = screen.max.y - 90.0;
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("toasts")));
        for t in self.app.toasts.iter().rev().take(4) {
            let galley =
                painter.layout(t.text.clone(), font::regular(14.0), if t.error { p.danger } else { p.fg1 }, 520.0);
            let r = Rect::from_center_size(
                pos2(screen.center().x, y - galley.size().y / 2.0),
                galley.size() + vec2(32.0, 18.0),
            );
            painter.rect_filled(r, 10, p.bg_a1);
            painter.rect_stroke(r, 10, egui::Stroke::new(1.0, p.divider_bright), egui::StrokeKind::Inside);
            painter.galley(r.min + vec2(16.0, 9.0), galley, p.fg1);
            y = r.min.y - 8.0;
        }
        ctx.request_repaint_after(Duration::from_millis(200));
    }
}

impl eframe::App for NotVlcApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.frame(ctx, frame.wgpu_render_state());
    }

    fn on_exit(&mut self) {
        self.app.shutdown();
    }
}

/// Maps egui keys to the names used by `KeyChord`.
pub fn key_name(k: Key) -> Option<&'static str> {
    use Key::*;
    Some(match k {
        A => "A",
        B => "B",
        C => "C",
        D => "D",
        E => "E",
        F => "F",
        G => "G",
        H => "H",
        I => "I",
        J => "J",
        K => "K",
        L => "L",
        M => "M",
        N => "N",
        O => "O",
        P => "P",
        Q => "Q",
        R => "R",
        S => "S",
        T => "T",
        U => "U",
        V => "V",
        W => "W",
        X => "X",
        Y => "Y",
        Z => "Z",
        Num0 => "0",
        Num1 => "1",
        Num2 => "2",
        Num3 => "3",
        Num4 => "4",
        Num5 => "5",
        Num6 => "6",
        Num7 => "7",
        Num8 => "8",
        Num9 => "9",
        F1 => "F1",
        F2 => "F2",
        F3 => "F3",
        F4 => "F4",
        F5 => "F5",
        F6 => "F6",
        F7 => "F7",
        F8 => "F8",
        F9 => "F9",
        F10 => "F10",
        F11 => "F11",
        F12 => "F12",
        Space => "Space",
        Enter => "Enter",
        Escape => "Esc",
        Tab => "Tab",
        Backspace => "Backspace",
        Delete => "Delete",
        Insert => "Insert",
        Home => "Home",
        End => "End",
        PageUp => "PageUp",
        PageDown => "PageDown",
        ArrowUp => "Up",
        ArrowDown => "Down",
        ArrowLeft => "Left",
        ArrowRight => "Right",
        Plus => "+",
        Minus => "-",
        Equals => "=",
        OpenBracket => "[",
        CloseBracket => "]",
        Backslash => "\\",
        Semicolon => ";",
        Quote => "'",
        Comma => ",",
        Period => ".",
        Slash => "/",
        Backtick => "`",
        _ => return None,
    })
}
