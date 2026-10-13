//! Open Network Stream (`6216:13614`): one or more URLs, an optional
//! playlist name, VLC's per-input options, recent URLs.

use egui::{RichText, vec2};

use super::frame::{self, Btn, Part};
use crate::app::App;
use crate::domain::media::{MediaItem, MediaSource, SUBTITLE_EXTENSIONS};
use crate::domain::time::parse_clock;
use crate::ui::state::{UiState, View};
use crate::ui::theme::font;
use crate::ui::widgets::pal;

/// What each protocol chip inserts. VLC's Lua playlist parsers turn
/// YouTube and similar page links into playable streams.
const EXAMPLES: &[(&str, &str)] = &[
    ("HTTP", "https://www.example.com/stream.mp4"),
    ("HLS", "https://www.example.com/live/playlist.m3u8"),
    ("RTSP", "rtsp://192.168.1.10:554/stream"),
    ("RTP", "rtp://@239.0.0.1:5004"),
    ("UDP", "udp://@239.0.0.1:1234"),
    ("MMS", "mms://www.example.com/stream"),
    ("SMB", "smb://server/share/movie.mkv"),
    ("FTP", "ftp://ftp.example.com/video.mp4"),
    ("YouTube", "https://www.youtube.com/watch?v="),
];

pub struct NetworkState {
    urls: String,
    playlist: String,
    caching_ms: f64,
    start: String,
    subtitle: String,
    primed: bool,
}

impl Default for NetworkState {
    fn default() -> Self {
        Self {
            urls: String::new(),
            playlist: String::new(),
            caching_ms: 1000.0,
            start: String::new(),
            subtitle: String::new(),
            primed: false,
        }
    }
}

impl NetworkState {
    fn urls(&self) -> Vec<String> {
        self.urls.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect()
    }

    /// The `:option`s added to every item, VLC style.
    fn options(&self, default_caching: u32) -> Vec<String> {
        let mut o = Vec::new();
        if self.caching_ms.round() as u32 != default_caching {
            o.push(format!(":network-caching={}", self.caching_ms.round() as u32));
        }
        if let Some(ms) = parse_clock(&self.start).filter(|ms| *ms > 0) {
            o.push(format!(":start-time={:.3}", ms as f64 / 1000.0));
        }
        if !self.subtitle.trim().is_empty() {
            o.push(format!(":sub-file={}", self.subtitle.trim()));
        }
        o
    }
}

fn append_line(text: &mut String, line: &str) {
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(line);
}

pub fn show(ctx: &egui::Context, app: &mut App, st: &mut UiState) {
    let ns = &mut st.dialogs.network;
    let default_caching = app.settings.network.network_caching_ms;
    if !ns.primed {
        ns.caching_ms = default_caching as f64;
        ns.primed = true;
    }
    let urls = ns.urls();
    let start_ok = ns.start.trim().is_empty() || parse_clock(&ns.start).is_some();
    let can_open = !urls.is_empty() && start_ok;
    let recents: Vec<String> = app.settings.network.recent_urls.iter().take(6).cloned().collect();
    let mut action: Option<bool> = None; // Some(true) play, Some(false) enqueue
    let mut cancel = false;
    let mut clear_recents = false;
    let primary = if ns.playlist.trim().is_empty() { "Play" } else { "Save and Play" };
    let out =
        frame::modal(ctx, "open-network", "Open Network Stream", vec2(1048.0, 700.0), true, |ui, part| match part {
            Part::Body => {
                let p = pal(ui.ctx());
                egui::ScrollArea::vertical().id_salt("net-body").auto_shrink([false, false]).show(ui, |ui| {
                    let w = ui.available_width();
                    frame::heading(ui, "Network URL", "You can enter multiple links to load.");
                    ui.add_space(15.0);
                    frame::area(ui, &mut ns.urls, "e.g., https://www.example.com/stream.avi", vec2(w, 100.0));
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                        ui.spacing_mut().interact_size.y = 29.0;
                        frame::caption(ui, "Examples", 64.0);
                        for (name, url) in EXAMPLES {
                            if frame::chip(ui, name, false).on_hover_text(*url).clicked() {
                                append_line(&mut ns.urls, url);
                            }
                        }
                    });
                    if !recents.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                            ui.spacing_mut().interact_size.y = 29.0;
                            frame::caption(ui, "Recent", 64.0);
                            for u in &recents {
                                let short: String = if u.chars().count() > 48 {
                                    format!("{}…", u.chars().take(47).collect::<String>())
                                } else {
                                    u.clone()
                                };
                                let on = urls.iter().any(|x| x == u);
                                if frame::chip(ui, &short, on).on_hover_text(u).clicked() && !on {
                                    append_line(&mut ns.urls, u);
                                }
                            }
                            if frame::chip(ui, "Clear", false).clicked() {
                                clear_recents = true;
                            }
                        });
                    }
                    ui.add_space(10.0);
                    frame::rule(ui);
                    ui.add_space(10.0);
                    frame::heading(
                        ui,
                        "Playlist Name (optional)",
                        "The playlist will be saved and displayed with this title.",
                    );
                    ui.add_space(15.0);
                    frame::field(ui, &mut ns.playlist, "e.g., Favorite TV Channels", w);
                    ui.add_space(4.0);
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 20.0;
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 6.0;
                            frame::label(ui, "Network caching")
                                .on_hover_text("How much VLC buffers before playing (network-caching).");
                            frame::number(ui, &mut ns.caching_ms, 0.0..=60_000.0, "ms", 160.0);
                        });
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 6.0;
                            frame::label(ui, "Start at").on_hover_text("Start playing at this position (start-time).");
                            frame::field(ui, &mut ns.start, "HH:MM:SS", 160.0);
                        });
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 6.0;
                            frame::label(ui, "Subtitle file")
                                .on_hover_text("Load this subtitle file with the stream (sub-file).");
                            ui.horizontal(|ui| {
                                let fw = ui.available_width() - 118.0;
                                frame::field(ui, &mut ns.subtitle, "None", fw);
                                if frame::btn_sized(
                                    ui,
                                    "Browse",
                                    Btn::Plain,
                                    true,
                                    vec2(100.0, 37.0),
                                    font::regular(14.0),
                                )
                                .clicked()
                                    && let Some(f) = rfd::FileDialog::new()
                                        .set_title("Subtitle File")
                                        .add_filter("Subtitles", SUBTITLE_EXTENSIONS)
                                        .pick_file()
                                {
                                    ns.subtitle = f.display().to_string();
                                }
                            });
                        });
                    });
                    let opts = ns.options(default_caching);
                    if !start_ok {
                        ui.label(
                            RichText::new("Start time must look like HH:MM:SS.")
                                .font(font::regular(13.0))
                                .color(p.danger),
                        );
                    } else if !opts.is_empty() {
                        ui.label(RichText::new(opts.join(" ")).font(font::mono(13.0)).color(p.fg_a3));
                    }
                });
            }
            Part::Footer => {
                if frame::cancel(ui) {
                    cancel = true;
                }
                frame::right(ui, |ui| {
                    if frame::btn(ui, primary, Btn::Accent, can_open).clicked() {
                        action = Some(true);
                    }
                    if frame::btn(ui, "Enqueue", Btn::Plain, can_open).clicked() {
                        action = Some(false);
                    }
                });
            }
        });
    if clear_recents {
        app.settings.network.recent_urls.clear();
        app.mark_settings_dirty();
    }
    if let Some(play) = action {
        let opts = ns.options(default_caching);
        let items: Vec<MediaItem> = urls
            .iter()
            .map(|u| {
                let mut item = MediaItem::new(MediaSource::from_user_input(u));
                item.options = opts.clone();
                item
            })
            .collect();
        let recents = &mut app.settings.network.recent_urls;
        for u in urls.iter().rev() {
            recents.retain(|x| x != u);
            recents.insert(0, u.clone());
        }
        recents.truncate(20);
        app.mark_settings_dirty();
        let name = ns.playlist.trim().to_string();
        if !name.is_empty() {
            app.library.create_playlist(&name, items.clone());
            app.library.save();
            app.toast(format!("Saved the playlist \"{name}\""));
        }
        if play {
            app.play_items(items, 0, (!name.is_empty()).then_some(name));
            st.view = View::Player;
        } else {
            app.enqueue(items);
        }
        st.dialogs.network = NetworkState::default();
        st.dialogs.open_network = false;
        return;
    }
    if cancel || out.close {
        st.dialogs.open_network = false;
        st.dialogs.network.primed = false;
    }
}
