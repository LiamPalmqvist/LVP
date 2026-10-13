//! Convert / Save and Stream (O2) with the profile editor (O8). Sources are
//! the current item, files picked here, or items handed over by Open Disc,
//! Open Capture Device or a context menu; running jobs are listed with
//! their progress and can be cancelled.

use std::path::{Path, PathBuf};

use egui::{Align2, RichText, Sense, vec2};

use super::frame::{self, Btn, Part};
use crate::app::App;
use crate::domain::convert::{
    self, AudioCodec, Container, Destination, Profile, StreamProtocol, SubtitleCodec, SubtitleMode, VideoCodec,
};
use crate::domain::media::{AUDIO_EXTENSIONS, MediaItem, VIDEO_EXTENSIONS};
use crate::ui::theme::font;
use crate::ui::widgets::{self, pal};

/// A job as listed: id, label, progress, and the outcome once finished.
type JobRow = (u64, String, f32, Option<(bool, String)>);

const SAMPLE_RATES: &[u32] = &[0, 8000, 11025, 22050, 32000, 44100, 48000, 96000];

pub struct ConvertState {
    primed: bool,
    profiles: Vec<Profile>,
    profile: usize,
    /// Empty means "the current item".
    sources: Vec<MediaItem>,
    stream: bool,
    /// A file for one source, a folder for several.
    dest: String,
    protocol: StreamProtocol,
    address: String,
    port: f64,
    path: String,
    /// The profile being edited and its index (None for a new one).
    editing: Option<(Option<usize>, Profile)>,
    /// Opens the editor on the selected profile once profiles are loaded.
    pub open_editor: bool,
}

impl Default for ConvertState {
    fn default() -> Self {
        Self {
            primed: false,
            profiles: Vec::new(),
            profile: 0,
            sources: Vec::new(),
            stream: false,
            dest: String::new(),
            protocol: StreamProtocol::Http,
            address: "239.0.0.1".into(),
            port: 8080.0,
            path: "stream".into(),
            editing: None,
            open_editor: false,
        }
    }
}

impl ConvertState {
    fn current(&self) -> Profile {
        self.profiles.get(self.profile).cloned().unwrap_or_default()
    }

    fn destination(&self, dest: &str) -> Destination {
        if self.stream {
            Destination::Stream {
                protocol: self.protocol,
                address: self.address.clone(),
                port: self.port.round().clamp(1.0, 65535.0) as u16,
                path: self.path.clone(),
            }
        } else {
            Destination::File(dest.to_string())
        }
    }
}

/// Where one source's output goes when `dest` is a folder (or empty: next
/// to the source).
fn file_for(source: &MediaItem, dest: &str, profile: &Profile, several: bool) -> String {
    let stem = source
        .source
        .path()
        .and_then(|p| p.file_stem())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| sanitize(&source.title()));
    let ext = source.source.path().and_then(|p| p.extension()).map(|e| e.to_string_lossy().into_owned());
    let name = convert::output_name(&stem, ext.as_deref().unwrap_or(""), profile);
    let dest = dest.trim();
    if !several && !dest.is_empty() && !Path::new(dest).is_dir() {
        return dest.to_string();
    }
    let dir = if dest.is_empty() {
        source
            .source
            .path()
            .and_then(|p| p.parent())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join("Movies"))
    } else {
        PathBuf::from(dest)
    };
    dir.join(name).display().to_string()
}

fn sanitize(s: &str) -> String {
    s.chars().map(|c| if c == '/' || c == ':' { '-' } else { c }).collect()
}

pub fn show(ctx: &egui::Context, app: &mut App, st: &mut super::DialogsState) {
    if let Some(items) = st.convert_source.take() {
        st.convert_state.sources = items;
        st.convert_state.dest.clear();
    }
    let cs = &mut st.convert_state;
    if !cs.primed {
        cs.profiles = app.conversion_profiles();
        cs.profile = cs.profile.min(cs.profiles.len().saturating_sub(1));
        cs.primed = true;
    }
    if std::mem::take(&mut cs.open_editor) {
        cs.editing = Some((Some(cs.profile), cs.current()));
    }
    if cs.editing.is_some() {
        editor(ctx, app, cs);
        return;
    }
    let current = app.queue.current_item().cloned();
    let sources: Vec<MediaItem> =
        if cs.sources.is_empty() { current.into_iter().collect() } else { cs.sources.clone() };
    let profile = cs.current();
    let several = sources.len() > 1;
    let preview_dest = sources.first().map(|s| file_for(s, &cs.dest, &profile, several)).unwrap_or_default();
    let preview = convert::sout(&profile, &cs.destination(&preview_dest));
    let can_start = !sources.is_empty() && (!cs.stream || !several);
    let mut start = false;
    let mut close = false;
    let mut cancel_job = None;
    let mut clear_jobs = false;
    let jobs: Vec<JobRow> = app.jobs.iter().map(|j| (j.id, j.label.clone(), j.progress, j.finished.clone())).collect();
    if jobs.iter().any(|j| j.3.is_none()) {
        ctx.request_repaint_after(std::time::Duration::from_millis(400));
    }
    let title = if cs.stream { "Stream" } else { "Convert / Save" };
    let out = frame::modal(ctx, "convert", title, vec2(1048.0, 760.0), true, |ui, part| match part {
        Part::Body => {
            let p = pal(ui.ctx());
            egui::ScrollArea::vertical().id_salt("convert-body").auto_shrink([false, false]).show(ui, |ui| {
                let w = ui.available_width();
                // ---- source
                frame::heading(ui, "Source", "");
                ui.horizontal(|ui| {
                    let text = match sources.len() {
                        0 => "Nothing is playing. Add files to convert.".to_string(),
                        1 => sources[0].title(),
                        n => format!("{} and {} more", sources[0].title(), n - 1),
                    };
                    let (r, _) = ui.allocate_exact_size(vec2(w - 370.0, 37.0), Sense::hover());
                    ui.painter().rect_filled(r, 6, p.bg_a5);
                    widgets::text_clipped(
                        ui,
                        r.left_center() + vec2(12.0, 0.0),
                        Align2::LEFT_CENTER,
                        &text,
                        font::regular(14.0),
                        if sources.is_empty() { p.fg_a3 } else { p.fg2 },
                        r.width() - 24.0,
                    );
                    let small = |ui: &mut egui::Ui, l: &str, on: bool| {
                        frame::btn_sized(ui, l, Btn::Plain, on, vec2(100.0, 37.0), font::regular(14.0)).clicked()
                    };
                    if small(ui, "Add Files", true) {
                        let mut exts: Vec<&str> = VIDEO_EXTENSIONS.to_vec();
                        exts.extend(AUDIO_EXTENSIONS);
                        if let Some(files) = rfd::FileDialog::new()
                            .set_title("Files to Convert")
                            .add_filter("Media files", &exts)
                            .add_filter("All files", &["*"])
                            .pick_files()
                        {
                            cs.sources.extend(files.into_iter().map(MediaItem::file));
                        }
                    }
                    if small(ui, "Current Item", !cs.sources.is_empty()) {
                        cs.sources.clear();
                    }
                    if small(ui, "Queue", !app.queue.is_empty()) {
                        cs.sources = app.queue.items.clone();
                    }
                });
                ui.add_space(6.0);
                // ---- profile
                frame::heading(ui, "Profile", "");
                ui.horizontal(|ui| {
                    let names: Vec<String> = cs.profiles.iter().map(|p| p.name.clone()).collect();
                    if let Some(i) = widgets::dropdown(ui, "convert-profile", &profile.name, &names, 460.0) {
                        cs.profile = i;
                    }
                    let small = |ui: &mut egui::Ui, l: &str, on: bool| {
                        frame::btn_sized(ui, l, Btn::Plain, on, vec2(84.0, 36.0), font::regular(14.0)).clicked()
                    };
                    if small(ui, "Edit", !cs.profiles.is_empty()) {
                        cs.editing = Some((Some(cs.profile), profile.clone()));
                    }
                    if small(ui, "New", true) {
                        cs.editing = Some((None, Profile::default()));
                    }
                    if small(ui, "Duplicate", !cs.profiles.is_empty()) {
                        let mut copy = profile.clone();
                        copy.name = format!("{} (copy)", copy.name);
                        cs.editing = Some((None, copy));
                    }
                    if small(ui, "Delete", cs.profiles.len() > 1) {
                        cs.profiles.remove(cs.profile);
                        cs.profile = cs.profile.min(cs.profiles.len() - 1);
                        app.save_conversion_profiles(&cs.profiles);
                    }
                });
                ui.label(RichText::new(profile.summary()).font(font::regular(13.0)).color(p.fg_a3));
                ui.add_space(6.0);
                // ---- destination
                frame::heading(ui, "Destination", "");
                let mut mode = cs.stream as usize;
                if widgets::segmented(ui, &mut mode, &["Save as File", "Stream"], 320.0) {
                    cs.stream = mode == 1;
                }
                if cs.stream {
                    let mut pi = StreamProtocol::ALL.iter().position(|x| *x == cs.protocol).unwrap_or(0);
                    let labels: Vec<&str> = StreamProtocol::ALL.iter().map(|x| x.label()).collect();
                    if widgets::segmented(ui, &mut pi, &labels, 640.0) {
                        cs.protocol = StreamProtocol::ALL[pi];
                        cs.port = cs.protocol.default_port() as f64;
                    }
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 20.0;
                        if cs.protocol.sends() {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 6.0;
                                frame::label(ui, "Address");
                                frame::field(ui, &mut cs.address, "239.0.0.1", 260.0);
                            });
                        }
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 6.0;
                            frame::label(ui, "Port");
                            frame::number(ui, &mut cs.port, 1.0..=65535.0, "", 120.0);
                        });
                        if !cs.protocol.sends() {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 6.0;
                                frame::label(ui, "Path");
                                frame::field(ui, &mut cs.path, "stream", 260.0);
                            });
                        }
                    });
                    let port = cs.port.round() as u16;
                    let hint = match cs.protocol {
                        StreamProtocol::Http => {
                            format!("Other devices open http://<this Mac>:{port}/{}", cs.path.trim_matches('/'))
                        }
                        StreamProtocol::Rtsp => {
                            format!("Other devices open rtsp://<this Mac>:{port}/{}", cs.path.trim_matches('/'))
                        }
                        StreamProtocol::Rtp => format!("Receivers open rtp://@{}:{port}", cs.address.trim()),
                        StreamProtocol::Udp => format!("Receivers open udp://@{}:{port}", cs.address.trim()),
                    };
                    frame::note(ui, &hint);
                    if !profile.container.streamable() {
                        frame::note(ui, "This profile's container cannot be streamed live; MPEG-TS is used instead.");
                    }
                    if several {
                        ui.label(
                            RichText::new("Stream one source at a time.").font(font::regular(13.0)).color(p.danger),
                        );
                    }
                } else {
                    ui.horizontal(|ui| {
                        let hint = if several { "Next to each source" } else { "Next to the source" };
                        frame::field(ui, &mut cs.dest, hint, w - 110.0);
                        if frame::btn_sized(ui, "Browse", Btn::Plain, true, vec2(100.0, 37.0), font::regular(14.0))
                            .clicked()
                        {
                            if several {
                                if let Some(d) = rfd::FileDialog::new().set_title("Destination Folder").pick_folder() {
                                    cs.dest = d.display().to_string();
                                }
                            } else {
                                let suggested = Path::new(&preview_dest)
                                    .file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_default();
                                let mut dlg = rfd::FileDialog::new()
                                    .set_title("Save As")
                                    .set_file_name(&suggested)
                                    .add_filter(profile.container.label(), &[profile.extension()]);
                                if let Some(dir) = Path::new(&preview_dest).parent() {
                                    dlg = dlg.set_directory(dir);
                                }
                                if let Some(f) = dlg.save_file() {
                                    cs.dest = f.display().to_string();
                                }
                            }
                        }
                    });
                    if !preview_dest.is_empty() {
                        frame::note(ui, &format!("Saves to {preview_dest}"));
                    }
                }
                ui.add(egui::Label::new(RichText::new(&preview).font(font::mono(12.0)).color(p.fg_a3)).wrap());
                // ---- jobs
                if !jobs.is_empty() {
                    ui.add_space(4.0);
                    frame::rule(ui);
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(200.0, 28.0), Sense::hover());
                        ui.painter().text(r.left_center(), Align2::LEFT_CENTER, "Jobs", font::regular(16.0), p.fg2);
                        frame::right(ui, |ui| {
                            if jobs.iter().any(|j| j.3.is_some()) && frame::chip(ui, "Clear finished", false).clicked()
                            {
                                clear_jobs = true;
                            }
                        });
                    });
                    for (id, label, progress, finished) in &jobs {
                        ui.horizontal(|ui| {
                            ui.set_min_height(32.0);
                            let lw = 420.0;
                            let (r, _) = ui.allocate_exact_size(vec2(lw, 20.0), Sense::hover());
                            widgets::text_clipped(
                                ui,
                                r.left_center(),
                                Align2::LEFT_CENTER,
                                label,
                                font::regular(14.0),
                                p.fg2,
                                lw,
                            );
                            frame::progress(ui, Some(*progress), 300.0);
                            match finished {
                                None => {
                                    ui.label(
                                        RichText::new(format!("{:.0}%", progress * 100.0))
                                            .font(font::mono(13.0))
                                            .color(p.fg_a2),
                                    );
                                    frame::right(ui, |ui| {
                                        if frame::chip(ui, "Cancel", false).clicked() {
                                            cancel_job = Some(*id);
                                        }
                                    });
                                }
                                Some((ok, msg)) => {
                                    ui.label(
                                        RichText::new(if *ok { "Done" } else { msg.as_str() })
                                            .font(font::regular(13.0))
                                            .color(if *ok { p.accent } else { p.danger }),
                                    );
                                }
                            }
                        });
                    }
                }
            });
        }
        _ => {
            if frame::btn(ui, "Close", Btn::Plain, true).clicked() {
                close = true;
            }
            frame::right(ui, |ui| {
                if frame::btn(ui, if cs.stream { "Stream" } else { "Start" }, Btn::Accent, can_start).clicked() {
                    start = true;
                }
            });
        }
    });
    if let Some(id) = cancel_job {
        app.cancel_job(id);
    }
    if clear_jobs {
        app.clear_finished_jobs();
    }
    if start {
        for s in &sources {
            let file = file_for(s, &cs.dest, &profile, several);
            let dest = cs.destination(&file);
            if let Destination::File(f) = &dest
                && s.source.path().is_some_and(|p| p == Path::new(f))
            {
                app.toast_error("The destination is the source file; choose another name.");
                continue;
            }
            app.start_conversion(s.clone(), &profile, &dest);
        }
    }
    if close || out.close {
        st.convert = false;
        st.convert_state.primed = false;
    }
}

fn pick<T: Copy + PartialEq>(ui: &mut egui::Ui, id: &str, value: &mut T, all: &[T], label: impl Fn(T) -> String) {
    let names: Vec<String> = all.iter().map(|v| label(*v)).collect();
    if let Some(i) = widgets::dropdown(ui, id, &label(*value), &names, 300.0) {
        *value = all[i];
    }
}

fn labelled(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        frame::label(ui, label);
        add(ui);
    });
}

fn number_u32(ui: &mut egui::Ui, v: &mut u32, max: f64, unit: &str) {
    let mut f = *v as f64;
    frame::number(ui, &mut f, 0.0..=max, unit, 150.0);
    *v = f.round() as u32;
}

/// The profile editor, shown in place of the Convert dialog.
fn editor(ctx: &egui::Context, app: &mut App, cs: &mut ConvertState) {
    let Some((index, mut prof)) = cs.editing.take() else { return };
    let mut save = false;
    let mut cancel = false;
    let mut reset = false;
    let title = if index.is_some() { "Edit Profile" } else { "New Profile" };
    let out = frame::modal(ctx, "convert-profile", title, vec2(1048.0, 760.0), true, |ui, part| match part {
        Part::Body => {
            let p = pal(ui.ctx());
            egui::ScrollArea::vertical().id_salt("profile-body").auto_shrink([false, false]).show(ui, |ui| {
                let w = ui.available_width();
                frame::heading(ui, "Profile name", "");
                frame::field(ui, &mut prof.name, "My profile", w);
                ui.add_space(4.0);
                frame::heading(ui, "Encapsulation", "");
                pick(ui, "prof-container", &mut prof.container, Container::ALL, |c| c.label().to_string());
                ui.add_space(4.0);
                frame::rule(ui);
                frame::heading(ui, "Video", "");
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 20.0;
                    labelled(ui, "Codec", |ui| {
                        pick(ui, "prof-vcodec", &mut prof.video, VideoCodec::ALL, |c| c.label().to_string())
                    });
                    if !matches!(prof.video, VideoCodec::Keep | VideoCodec::Disabled) {
                        labelled(ui, "Bitrate", |ui| number_u32(ui, &mut prof.video_bitrate_kbps, 100_000.0, "kb/s"));
                        labelled(ui, "Frame rate", |ui| {
                            let mut f = prof.fps as f64;
                            frame::number(ui, &mut f, 0.0..=240.0, "fps", 150.0);
                            prof.fps = f as f32;
                        });
                    }
                });
                if !matches!(prof.video, VideoCodec::Keep | VideoCodec::Disabled) {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 20.0;
                        labelled(ui, "Scale", |ui| {
                            let scales = [0.0f32, 0.25, 0.5, 0.75, 1.25, 1.5, 2.0];
                            pick(ui, "prof-scale", &mut prof.scale, &scales, |s| {
                                if s == 0.0 { "Auto".to_string() } else { format!("{s}x") }
                            });
                        });
                        labelled(ui, "Width", |ui| number_u32(ui, &mut prof.width, 7680.0, "px"));
                        labelled(ui, "Height", |ui| number_u32(ui, &mut prof.height, 4320.0, "px"));
                    });
                    frame::note(ui, "Zero keeps the source's value; set only the width to keep the aspect ratio.");
                }
                ui.add_space(4.0);
                frame::rule(ui);
                frame::heading(ui, "Audio", "");
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 20.0;
                    labelled(ui, "Codec", |ui| {
                        pick(ui, "prof-acodec", &mut prof.audio, AudioCodec::ALL, |c| c.label().to_string())
                    });
                    if !matches!(prof.audio, AudioCodec::Keep | AudioCodec::Disabled) {
                        labelled(ui, "Bitrate", |ui| number_u32(ui, &mut prof.audio_bitrate_kbps, 1536.0, "kb/s"));
                        labelled(ui, "Channels", |ui| number_u32(ui, &mut prof.channels, 8.0, ""));
                    }
                });
                if !matches!(prof.audio, AudioCodec::Keep | AudioCodec::Disabled) {
                    labelled(ui, "Sample rate", |ui| {
                        pick(ui, "prof-rate", &mut prof.sample_rate, SAMPLE_RATES, |r| {
                            if r == 0 { "Same as source".to_string() } else { format!("{r} Hz") }
                        })
                    });
                }
                ui.add_space(4.0);
                frame::rule(ui);
                frame::heading(ui, "Subtitles", "");
                let mut si = SubtitleMode::ALL.iter().position(|m| *m == prof.subtitles).unwrap_or(0);
                let labels: Vec<&str> = SubtitleMode::ALL.iter().map(|m| m.label()).collect();
                if widgets::segmented(ui, &mut si, &labels, 480.0) {
                    prof.subtitles = SubtitleMode::ALL[si];
                }
                if prof.subtitles == SubtitleMode::Encode {
                    pick(ui, "prof-scodec", &mut prof.subtitle_codec, SubtitleCodec::ALL, |c| c.label().to_string());
                }
                if prof.subtitles == SubtitleMode::Overlay {
                    frame::note(ui, "Burns the selected subtitles into the picture (needs video re-encoding).");
                }
                ui.add_space(4.0);
                let chain = prof.transcode().unwrap_or_else(|| "Remux only (nothing is re-encoded)".into());
                ui.add(egui::Label::new(RichText::new(chain).font(font::mono(12.0)).color(p.fg_a3)).wrap());
            });
        }
        _ => {
            if frame::cancel(ui) {
                cancel = true;
            }
            if frame::btn(ui, "Reset All", Btn::Plain, true).on_hover_text("Restore VLC's standard profiles").clicked()
            {
                reset = true;
            }
            frame::right(ui, |ui| {
                if frame::btn(ui, "Save", Btn::Accent, !prof.name.trim().is_empty()).clicked() {
                    save = true;
                }
            });
        }
    });
    if reset {
        cs.profiles = app.reset_conversion_profiles();
        cs.profile = 0;
        return;
    }
    if save {
        prof.name = prof.name.trim().to_string();
        match index.filter(|i| *i < cs.profiles.len()) {
            Some(i) => {
                cs.profiles[i] = prof;
                cs.profile = i;
            }
            None => {
                cs.profiles.push(prof);
                cs.profile = cs.profiles.len() - 1;
            }
        }
        app.save_conversion_profiles(&cs.profiles);
        return;
    }
    if cancel || out.close {
        return;
    }
    cs.editing = Some((index, prof));
}
