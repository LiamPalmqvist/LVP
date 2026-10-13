//! Media Information: General (tags and artwork), Codec (elementary
//! streams) and Statistics (VLC's input counters, refreshed live).

use std::time::Duration;

use egui::{Align2, CornerRadius, RichText, Sense, vec2};

use super::frame::{self, Btn, Part};
use crate::app::App;
use crate::domain::media::{CodecInfo, MediaMeta};
use crate::domain::time::format_hms;
use crate::ui::icons::Icon;
use crate::ui::images;
use crate::ui::theme::font;
use crate::ui::widgets::{self, pal};

#[derive(Default)]
pub struct InfoState {
    tab: usize,
}

impl InfoState {
    pub fn set_tab(&mut self, tab: usize) {
        self.tab = tab;
    }
}

fn opt(v: &Option<String>) -> &str {
    v.as_deref().unwrap_or("")
}

fn bytes(n: i64) -> String {
    let n = n.max(0) as f64;
    if n >= 1024.0 * 1024.0 * 1024.0 {
        format!("{:.2} GiB", n / (1024.0 * 1024.0 * 1024.0))
    } else if n >= 1024.0 * 1024.0 {
        format!("{:.1} MiB", n / (1024.0 * 1024.0))
    } else if n >= 1024.0 {
        format!("{:.0} KiB", n / 1024.0)
    } else {
        format!("{n:.0} B")
    }
}

pub fn show(ctx: &egui::Context, app: &mut App, open: &mut bool, st: &mut InfoState) {
    let item = app.queue.current_item().cloned();
    let mut meta: MediaMeta = item.as_ref().map(|i| i.meta.clone()).unwrap_or_default();
    if let Some(i) = &item
        && let Some(m) = app.meta_for(&i.source)
    {
        meta.merge(m);
    }
    if app.has_media() {
        meta.merge(app.status.meta.clone());
    }
    let art = item.as_ref().and_then(|i| {
        let mut i = i.clone();
        i.meta.artwork_url = meta.artwork_url.clone().or(i.meta.artwork_url);
        app.thumbnail(&i)
    });
    let title = meta
        .title
        .clone()
        .filter(|t| !t.trim().is_empty())
        .or_else(|| item.as_ref().map(|i| i.title()))
        .unwrap_or_default();
    let location = item
        .as_ref()
        .map(|i| i.source.path().map(|p| p.display().to_string()).unwrap_or_else(|| i.source.mrl()))
        .unwrap_or_default();
    let stats = app.status.stats;
    let collecting = app.settings.general.collect_stats;
    let length = app.status.length_ms.max(meta.duration_ms.unwrap_or(0));
    let mut close = false;
    let out = frame::modal(ctx, "media-info", "Media Information", vec2(1048.0, 680.0), true, |ui, part| match part {
        Part::Body => {
            let p = pal(ui.ctx());
            if item.is_none() {
                widgets::empty_state(ui, Icon::Info, "Nothing is playing", "Open a file to see its information.");
                return;
            }
            widgets::tabs(ui, &mut st.tab, &[(None, "General"), (None, "Codec"), (None, "Statistics")]);
            ui.add_space(6.0);
            egui::ScrollArea::vertical().id_salt("info-body").auto_shrink([false, false]).show(ui, |ui| match st.tab {
                0 => {
                    ui.horizontal_top(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(220.0, 220.0), Sense::hover());
                        ui.painter().rect_filled(r, CornerRadius::same(10), p.bg_a3);
                        let shown = art.as_deref().is_some_and(|a| images::paint_cover(ui, r, a, 10));
                        if !shown {
                            widgets::paint_icon(
                                ui,
                                egui::Rect::from_center_size(r.center(), vec2(64.0, 64.0)),
                                if item.as_ref().is_some_and(|i| i.is_audio()) { Icon::Music } else { Icon::Video },
                                p.fg_a4,
                            );
                        }
                        ui.add_space(14.0);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.label(RichText::new(&title).font(font::light(24.0)).color(p.fg1));
                            ui.add_space(10.0);
                            let k = 130.0;
                            frame::info_row(ui, "Artist", opt(&meta.artist), k);
                            frame::info_row(ui, "Album", opt(&meta.album), k);
                            frame::info_row(ui, "Album artist", opt(&meta.album_artist), k);
                            frame::info_row(ui, "Genre", opt(&meta.genre), k);
                            frame::info_row(ui, "Date", opt(&meta.date), k);
                            let track = match (&meta.track_number, &meta.disc_number) {
                                (Some(t), Some(d)) => format!("{t} (disc {d})"),
                                (Some(t), None) => t.clone(),
                                _ => String::new(),
                            };
                            frame::info_row(ui, "Track", &track, k);
                            if meta.show_name.is_some() || meta.season.is_some() || meta.episode.is_some() {
                                let ep = [
                                    meta.show_name.clone(),
                                    meta.season.as_ref().map(|s| format!("season {s}")),
                                    meta.episode.as_ref().map(|e| format!("episode {e}")),
                                ]
                                .into_iter()
                                .flatten()
                                .collect::<Vec<_>>()
                                .join(", ");
                                frame::info_row(ui, "Show", &ep, k);
                            }
                            frame::info_row(
                                ui,
                                "Length",
                                &if length > 0 { format_hms(length) } else { String::new() },
                                k,
                            );
                        });
                    });
                    ui.add_space(10.0);
                    frame::rule(ui);
                    ui.add_space(4.0);
                    let k = 130.0;
                    ui.spacing_mut().item_spacing.y = 0.0;
                    frame::info_row(ui, "Description", opt(&meta.description), k);
                    frame::info_row(ui, "Copyright", opt(&meta.copyright), k);
                    frame::info_row(ui, "Publisher", opt(&meta.publisher), k);
                    frame::info_row(ui, "Encoded by", opt(&meta.encoded_by), k);
                    frame::info_row(ui, "Language", opt(&meta.language), k);
                    if meta.director.is_some() || meta.actors.is_some() {
                        frame::info_row(ui, "Director", opt(&meta.director), k);
                        frame::info_row(ui, "Actors", opt(&meta.actors), k);
                    }
                    if meta.now_playing.is_some() {
                        frame::info_row(ui, "Now playing", opt(&meta.now_playing), k);
                    }
                    frame::info_row(ui, "Artwork", opt(&meta.artwork_url), k);
                    frame::info_row(ui, "Location", &location, k);
                }
                1 => codec_tab(ui, &meta.codecs),
                _ => {
                    ui.ctx().request_repaint_after(Duration::from_millis(500));
                    if !collecting {
                        frame::note(
                            ui,
                            "Statistics are turned off in Settings > General (Collect statistics), so VLC reports \
                             zeros.",
                        );
                    }
                    let k = 240.0;
                    ui.spacing_mut().item_spacing.y = 0.0;
                    widgets::section(ui, "Input");
                    frame::info_row(ui, "Media data size", &bytes(stats.read_bytes), k);
                    frame::info_row(ui, "Input bitrate", &format!("{:.0} kb/s", stats.input_bitrate_kbps), k);
                    frame::info_row(ui, "Content bitrate", &format!("{:.0} kb/s", stats.demux_bitrate_kbps), k);
                    frame::info_row(ui, "Corrupted packets", &stats.demux_corrupted.to_string(), k);
                    frame::info_row(ui, "Discontinuities", &stats.demux_discontinuity.to_string(), k);
                    widgets::section(ui, "Video");
                    frame::info_row(ui, "Decoded blocks", &stats.decoded_video.to_string(), k);
                    frame::info_row(ui, "Displayed frames", &stats.displayed_pictures.to_string(), k);
                    frame::info_row(ui, "Lost frames", &stats.lost_pictures.to_string(), k);
                    widgets::section(ui, "Audio");
                    frame::info_row(ui, "Decoded blocks", &stats.decoded_audio.to_string(), k);
                    frame::info_row(ui, "Played buffers", &stats.played_abuffers.to_string(), k);
                    frame::info_row(ui, "Lost buffers", &stats.lost_abuffers.to_string(), k);
                }
            });
        }
        Part::Footer => {
            frame::right(ui, |ui| {
                if frame::btn(ui, "Close", Btn::Accent, true).clicked() {
                    close = true;
                }
            });
        }
    });
    if close || out.close {
        *open = false;
    }
}

fn codec_tab(ui: &mut egui::Ui, codecs: &[CodecInfo]) {
    let p = pal(ui.ctx());
    if codecs.is_empty() {
        frame::note(ui, "No stream information yet. It appears once VLC has opened the media.");
        return;
    }
    for (i, c) in codecs.iter().enumerate() {
        egui::Frame::new().fill(p.bg_a5).corner_radius(10).inner_margin(egui::Margin::symmetric(16, 12)).show(
            ui,
            |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 0.0;
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::hover());
                let kind = if c.kind.is_empty() { "Stream".to_string() } else { c.kind.clone() };
                ui.painter().text(
                    r.left_center(),
                    Align2::LEFT_CENTER,
                    format!("Stream {i}  ·  {kind}"),
                    font::medium(15.0),
                    p.fg2,
                );
                let k = 160.0;
                frame::info_row(ui, "Codec", &c.codec, k);
                if let Some(l) = &c.language {
                    frame::info_row(ui, "Language", l, k);
                }
                if let Some(d) = &c.description {
                    frame::info_row(ui, "Description", d, k);
                }
                if let (Some(w), Some(h)) = (c.width, c.height) {
                    frame::info_row(ui, "Resolution", &format!("{w} x {h}"), k);
                }
                if let Some(f) = c.frame_rate.filter(|f| *f > 0.0) {
                    frame::info_row(ui, "Frame rate", &format!("{f:.3} fps"), k);
                }
                if let Some(ch) = c.channels {
                    frame::info_row(ui, "Channels", &ch.to_string(), k);
                }
                if let Some(sr) = c.sample_rate {
                    frame::info_row(ui, "Sample rate", &format!("{sr} Hz"), k);
                }
                if c.bitrate > 0 {
                    frame::info_row(ui, "Bitrate", &format!("{} kb/s", c.bitrate / 1000), k);
                }
            },
        );
        ui.add_space(8.0);
    }
}
