//! Add Subtitles to Playlist/Folder (O6, frames `1911:7545`, `3336:11929`,
//! `1919:8707`, `1919:9278`): pick where the subtitles go (the playing
//! file, a folder of videos, or the queue) and where they come from, match
//! them by name, adjust per video, then attach or copy them.

use std::path::{Path, PathBuf};

use egui::{Align2, CornerRadius, Rect, Sense, Stroke, Ui, pos2, vec2};

use super::frame::{self, Btn, Part};
use crate::app::App;
use crate::domain::subtitles::{self, SubtitleMatch};
use crate::ui::icons::Icon;
use crate::ui::theme::font;
use crate::ui::widgets::{self, pal};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Target {
    File,
    #[default]
    Folder,
    Playlist,
}

const TARGETS: [(Target, &str); 3] =
    [(Target::File, "To This File"), (Target::Folder, "To This Folder"), (Target::Playlist, "To This Playlist")];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Origin {
    #[default]
    Computer,
    OpenSubtitles,
}

const ORIGINS: [(Origin, &str); 2] =
    [(Origin::Computer, "From Computer"), (Origin::OpenSubtitles, "From OpenSubtitles")];

/// One video in the folder or queue and its subtitle choice.
struct Row {
    video: PathBuf,
    /// Queue index for the playlist target.
    index: usize,
    candidates: Vec<PathBuf>,
    chosen: Option<PathBuf>,
}

#[derive(Default)]
pub struct SubsState {
    primed: bool,
    target: Target,
    origin: Origin,
    rule: SubtitleMatch,
    video_dir: Option<PathBuf>,
    sub_dir: Option<PathBuf>,
    subs: Vec<PathBuf>,
    rows: Vec<Row>,
    /// The selected subtitle for "To This File".
    file_choice: Option<PathBuf>,
    /// Copy plan awaiting confirmation: (subtitle, destination, replaces).
    confirm: Option<Vec<(PathBuf, PathBuf, bool)>>,
    search: String,
    dirty: bool,
    /// Prime on "To This File" whatever the folder holds.
    force_file: bool,
}

impl SubsState {
    /// Starts on the playing file (`online`: the OpenSubtitles view).
    pub fn preset_file(&mut self, online: bool) {
        self.primed = false;
        self.force_file = true;
        if online {
            self.origin = Origin::OpenSubtitles;
        }
    }

    /// Starts on a folder of videos, its subtitles taken from `subs` (the
    /// same folder when `None`). Used by the snapshot tool and the library.
    pub fn preset_folder(&mut self, videos: PathBuf, subs: Option<PathBuf>) {
        self.sub_dir = Some(subs.unwrap_or_else(|| videos.clone()));
        self.video_dir = Some(videos);
        self.target = Target::Folder;
        self.primed = true;
        self.dirty = true;
    }
}

/// The last few components of a path, "Videos / Shows / The Wand / S01".
fn crumbs(p: &Path) -> String {
    let parts: Vec<String> = p
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    let start = parts.len().saturating_sub(4);
    parts[start..].join(" / ")
}

fn name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn rebuild(app: &App, s: &mut SubsState) {
    s.subs = s.sub_dir.as_deref().map(|d| app.subtitle_files(d)).unwrap_or_default();
    // Subtitles from the queue's own items (sidecar files) are not listed
    // twice; the folder scan already finds them.
    let videos: Vec<(usize, PathBuf)> = match s.target {
        Target::Folder => {
            s.video_dir.as_deref().map(|d| app.video_files(d)).unwrap_or_default().into_iter().enumerate().collect()
        }
        Target::Playlist => app
            .queue
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, it)| it.source.path().map(|p| (i, p.to_path_buf())))
            .collect(),
        Target::File => app
            .queue
            .current_item()
            .and_then(|it| it.source.path().map(|p| (app.queue.current.unwrap_or(0), p.to_path_buf())))
            .into_iter()
            .collect(),
    };
    let paths: Vec<PathBuf> = videos.iter().map(|(_, p)| p.clone()).collect();
    let pairs = subtitles::pair_all(&paths, &s.subs, s.rule);
    s.rows = videos
        .into_iter()
        .zip(pairs)
        .map(|((index, video), (candidates, pick))| {
            let chosen = pick.map(|i| candidates[i].clone());
            Row { video, index, candidates, chosen }
        })
        .collect();
    if s.target == Target::File {
        // Best matches first, then everything else in the folder.
        let mut list = s.rows.first().map(|r| r.candidates.clone()).unwrap_or_default();
        for sub in &s.subs {
            if !list.contains(sub) {
                list.push(sub.clone());
            }
        }
        s.file_choice = s.rows.first().and_then(|r| r.chosen.clone()).or_else(|| list.first().cloned());
        if let Some(r) = s.rows.first_mut() {
            r.candidates = list;
        }
    }
    s.dirty = false;
}

pub fn show(ctx: &egui::Context, app: &mut App, open: &mut bool, s: &mut SubsState) {
    if !s.primed {
        let dir = app.queue.current_item().and_then(|i| i.source.path()).and_then(Path::parent).map(Path::to_path_buf);
        let several = dir.as_deref().is_some_and(|d| app.video_files(d).len() > 1);
        s.target = if several && !std::mem::take(&mut s.force_file) { Target::Folder } else { Target::File };
        s.video_dir = dir.clone();
        s.sub_dir = dir;
        s.primed = true;
        s.dirty = true;
    }
    if s.dirty {
        rebuild(app, s);
    }
    let mut cancel = false;
    let mut save = false;
    let mut apply_copy = false;
    let mut back = false;
    let can_save = s.origin == Origin::Computer
        && match s.target {
            Target::File => s.file_choice.is_some() && app.has_media(),
            _ => s.rows.iter().any(|r| r.chosen.is_some()),
        };
    let confirming = s.confirm.is_some();
    let out =
        frame::modal_with_toolbar(ctx, "add-subtitles", "Add Subtitles", vec2(1048.0, 800.0), |ui, part| match part {
            Part::Toolbar => toolbar(ui, app, s),
            Part::Body => {
                if let Some(plan) = &s.confirm {
                    confirm_body(ui, plan);
                } else if s.origin == Origin::OpenSubtitles {
                    widgets::empty_state(
                        ui,
                        Icon::Search,
                        "Searching OpenSubtitles is not available",
                        "VLC does this with its VLSub Lua extension, which libvlc cannot run. Download the files and \
                         use From Computer.",
                    );
                } else if s.sub_dir.is_none() {
                    widgets::empty_state(
                        ui,
                        Icon::Folder,
                        "Choose a folder with subtitle files",
                        "Click the folder field above to pick where the .srt, .ass or .vtt files are.",
                    );
                } else if s.target == Target::File {
                    file_list(ui, app, s);
                } else {
                    pair_list(ui, s);
                }
            }
            Part::Footer => {
                if confirming {
                    if frame::btn(ui, "Back", Btn::Plain, true).clicked() {
                        back = true;
                    }
                    frame::right(ui, |ui| {
                        if frame::btn(ui, "Copy Files", Btn::Accent, true).clicked() {
                            apply_copy = true;
                        }
                    });
                } else {
                    if frame::cancel(ui) {
                        cancel = true;
                    }
                    frame::right(ui, |ui| {
                        if frame::btn(ui, "Save", Btn::Accent, can_save).clicked() {
                            save = true;
                        }
                    });
                }
            }
        });
    if back {
        s.confirm = None;
        return;
    }
    if apply_copy {
        let pairs: Vec<(PathBuf, PathBuf)> =
            s.rows.iter().filter_map(|r| r.chosen.clone().map(|c| (r.video.clone(), c))).collect();
        app.copy_subtitles(&pairs);
        s.confirm = None;
        s.primed = false;
        *open = false;
        return;
    }
    if save {
        match s.target {
            Target::File => {
                if let Some(f) = s.file_choice.clone() {
                    let idx = app.queue.current.unwrap_or(0);
                    if app.queue.current_item().is_some_and(|i| i.source.path().is_some()) {
                        app.attach_subtitles(&[(idx, f)]);
                    } else {
                        app.add_subtitle(f);
                    }
                }
                s.primed = false;
                *open = false;
            }
            Target::Playlist => {
                let pairs: Vec<(usize, PathBuf)> =
                    s.rows.iter().filter_map(|r| r.chosen.clone().map(|c| (r.index, c))).collect();
                app.attach_subtitles(&pairs);
                s.primed = false;
                *open = false;
            }
            Target::Folder => {
                let pairs: Vec<(PathBuf, PathBuf)> =
                    s.rows.iter().filter_map(|r| r.chosen.clone().map(|c| (r.video.clone(), c))).collect();
                s.confirm = Some(app.subtitle_copy_plan(&pairs));
            }
        }
        return;
    }
    if cancel || out.close {
        if confirming {
            s.confirm = None;
        } else {
            s.primed = false;
            *open = false;
        }
    }
}

fn toolbar(ui: &mut Ui, app: &App, s: &mut SubsState) {
    let p = pal(ui.ctx());
    let tl = TARGETS.iter().find(|t| t.0 == s.target).map(|t| t.1).unwrap_or("");
    let labels: Vec<&str> = TARGETS.iter().map(|t| t.1).collect();
    if let Some(i) = frame::select(ui, "subs-target", tl, &labels, 200.0) {
        let t = TARGETS[i].0;
        if t == Target::Folder
            && (s.video_dir.is_none() || s.target == Target::Folder)
            && let Some(d) = rfd::FileDialog::new().set_title("Folder of Videos").pick_folder()
        {
            if s.sub_dir.is_none() || s.sub_dir == s.video_dir {
                s.sub_dir = Some(d.clone());
            }
            s.video_dir = Some(d);
        }
        s.target = t;
        s.confirm = None;
        s.dirty = true;
    }
    let ol = ORIGINS.iter().find(|o| o.0 == s.origin).map(|o| o.1).unwrap_or("");
    let labels: Vec<&str> = ORIGINS.iter().map(|o| o.1).collect();
    if let Some(i) = frame::select(ui, "subs-origin", ol, &labels, 200.0) {
        s.origin = ORIGINS[i].0;
        if s.origin == Origin::OpenSubtitles && s.search.is_empty() {
            s.search = app.queue.current_item().map(|i| i.title()).unwrap_or_default();
        }
    }
    let rest = ui.available_width();
    let rule_w = if s.target == Target::File { 0.0 } else { 210.0 };
    let field_w = rest - rule_w;
    if s.origin == Origin::OpenSubtitles {
        // Search box as in 1919:9278 (disabled: no OpenSubtitles client).
        let (r, _) = ui.allocate_exact_size(vec2(field_w - 210.0, 34.0), Sense::hover());
        ui.painter().rect_filled(r, CornerRadius::same(6), p.bg_a5);
        widgets::paint_icon(ui, Rect::from_min_size(r.min + vec2(8.0, 5.0), vec2(24.0, 24.0)), Icon::Search, p.fg_a3);
        let mut c = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_max(r.min + vec2(42.0, 8.0), r.max - vec2(8.0, 6.0)))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        c.add(
            egui::TextEdit::singleline(&mut s.search)
                .frame(false)
                .margin(egui::Margin::ZERO)
                .text_color(p.fg_a1)
                .font(font::regular(14.0))
                .desired_width(r.width() - 50.0),
        );
        frame::select(ui, "subs-lang", "English", &["English"], 200.0);
        return;
    }
    // The subtitle folder (Frame 122): folder icon and a breadcrumb.
    let (r, resp) = ui.allocate_exact_size(vec2(field_w, 34.0), Sense::click());
    ui.painter().rect_filled(r, CornerRadius::same(6), if resp.hovered() { p.bg_a3 } else { p.bg_a5 });
    widgets::paint_icon(ui, Rect::from_min_size(r.min + vec2(8.0, 5.0), vec2(24.0, 24.0)), Icon::Folder, p.fg_a2);
    let text = s.sub_dir.as_deref().map(crumbs).unwrap_or_else(|| "Choose the subtitles folder".into());
    widgets::text_clipped(
        ui,
        pos2(r.min.x + 42.0, r.center().y),
        Align2::LEFT_CENTER,
        &text,
        font::regular(14.0),
        p.fg_a3,
        r.width() - 50.0,
    );
    let resp = resp.on_hover_text("Subtitles folder").on_hover_cursor(egui::CursorIcon::PointingHand);
    if resp.clicked() {
        let mut dlg = rfd::FileDialog::new().set_title("Subtitles Folder");
        if let Some(d) = s.sub_dir.as_deref() {
            dlg = dlg.set_directory(d);
        }
        if let Some(d) = dlg.pick_folder() {
            s.sub_dir = Some(d);
            s.dirty = true;
        }
    }
    if rule_w > 0.0 {
        let labels: Vec<&str> = SubtitleMatch::ALL.iter().map(|m| m.label()).collect();
        if let Some(i) = frame::select(ui, "subs-rule", s.rule.label(), &labels, 200.0) {
            s.rule = SubtitleMatch::ALL[i];
            s.dirty = true;
        }
    }
}

/// "To This File": one row per subtitle file (component "subtitle one").
fn file_list(ui: &mut Ui, app: &App, s: &mut SubsState) {
    let p = pal(ui.ctx());
    if !app.has_media() {
        widgets::empty_state(ui, Icon::Video, "Nothing is playing", "Play a video to add a subtitle file to it.");
        return;
    }
    let list = s.rows.first().map(|r| r.candidates.clone()).unwrap_or_default();
    ui.spacing_mut().item_spacing.y = 4.0;
    egui::ScrollArea::vertical().id_salt("subs-file").auto_shrink([false, false]).show(ui, |ui| {
        if list.is_empty() {
            frame::note(ui, "No subtitle files in this folder.");
        }
        for sub in &list {
            let w = ui.available_width();
            let (r, resp) = ui.allocate_exact_size(vec2(w, 36.0), Sense::click());
            let on = s.file_choice.as_ref() == Some(sub);
            if on {
                ui.painter().rect_filled(r, CornerRadius::same(6), p.bg_b2);
            } else if resp.hovered() {
                ui.painter().rect_filled(r, CornerRadius::same(6), p.hover);
            }
            widgets::paint_icon(
                ui,
                Rect::from_min_size(r.min + vec2(8.0, 6.0), vec2(24.0, 24.0)),
                Icon::Subtitle,
                p.fg_a2,
            );
            ui.painter().text(
                pos2(r.min.x + 46.0, r.center().y),
                Align2::LEFT_CENTER,
                name(sub),
                font::regular(14.0),
                if on { p.fg2 } else { p.fg_a1 },
            );
            if on {
                widgets::paint_icon(
                    ui,
                    Rect::from_min_size(pos2(r.max.x - 32.0, r.min.y + 6.0), vec2(24.0, 24.0)),
                    Icon::Check,
                    p.accent,
                );
            }
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                s.file_choice = Some(sub.clone());
            }
        }
    });
}

/// Folder and playlist targets: video on the left, a dashed connector, the
/// chosen subtitle on the right (components "subtitle one/multiple").
fn pair_list(ui: &mut Ui, s: &mut SubsState) {
    let p = pal(ui.ctx());
    if s.rows.is_empty() {
        let what = if s.target == Target::Playlist { "The queue has no video files" } else { "No videos here" };
        widgets::empty_state(ui, Icon::Video, what, "Choose another folder with the To This Folder menu.");
        return;
    }
    let all_subs = s.subs.clone();
    egui::ScrollArea::vertical().id_salt("subs-pairs").auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        let w = ui.available_width();
        let col = (w + 2.0 * frame::PAD_X) / 2.0 - frame::PAD_X;
        for (i, row) in s.rows.iter_mut().enumerate() {
            let (r, resp) = ui.allocate_exact_size(vec2(w, 36.0), Sense::hover());
            let hovered = resp.hovered();
            let left = Rect::from_min_size(r.min + vec2(10.0, 0.0), vec2(col - 10.0, r.height()));
            widgets::paint_icon(
                ui,
                Rect::from_min_size(left.min + vec2(0.0, 6.0), vec2(24.0, 24.0)),
                Icon::Video,
                p.fg_a2,
            );
            let text = widgets::text_clipped(
                ui,
                pos2(left.min.x + 38.0, left.center().y),
                Align2::LEFT_CENTER,
                &name(&row.video),
                font::regular(14.0),
                if hovered { p.fg1 } else { p.fg_a1 },
                col - 140.0,
            );
            if !hovered && row.chosen.is_some() {
                dashed(ui, text.max.x + 14.0, left.max.x, r.center().y, p.divider_bright);
            }
            // Right column: the chosen subtitle, or a prompt to pick one.
            let right = Rect::from_min_max(pos2(r.min.x + col + 18.0, r.min.y), r.max);
            let rr = ui.interact(right, ui.id().with(("subs-pick", i)), Sense::click());
            let several = row.candidates.len() > 1;
            if rr.hovered() {
                ui.painter().rect_filled(right, CornerRadius::same(6), p.bg_b2);
            }
            let icon_r = Rect::from_min_size(right.min + vec2(8.0, 6.0), vec2(24.0, 24.0));
            match &row.chosen {
                Some(c) => {
                    widgets::paint_icon(ui, icon_r, Icon::Subtitle, p.fg_a2);
                    widgets::text_clipped(
                        ui,
                        pos2(right.min.x + 46.0, right.center().y),
                        Align2::LEFT_CENTER,
                        &name(c),
                        font::regular(14.0),
                        p.fg_a1,
                        right.width() - 90.0,
                    );
                }
                None => {
                    ui.painter().text(
                        pos2(right.min.x + 46.0, right.center().y),
                        Align2::LEFT_CENTER,
                        if row.candidates.is_empty() { "No match" } else { "None" },
                        font::regular(14.0),
                        p.fg_a4,
                    );
                }
            }
            if rr.hovered() && (several || !all_subs.is_empty()) {
                widgets::paint_icon(
                    ui,
                    Rect::from_min_size(pos2(right.max.x - 32.0, right.min.y + 6.0), vec2(24.0, 24.0)),
                    Icon::Swap,
                    p.fg_a2,
                );
            } else if several && row.chosen.is_some() {
                ui.painter().text(
                    pos2(right.max.x - 12.0, right.center().y),
                    Align2::RIGHT_CENTER,
                    format!("{} matches", row.candidates.len()),
                    font::regular(12.0),
                    p.fg_a3,
                );
            }
            let rr = rr.on_hover_cursor(egui::CursorIcon::PointingHand);
            egui::Popup::menu(&rr)
                .id(ui.make_persistent_id(("subs-menu", i)))
                .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
                .frame(widgets::menu_frame(&p))
                .show(|ui| {
                    ui.set_min_width(right.width().min(460.0));
                    egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                        // Matches first, then the rest of the folder.
                        let mut opts = row.candidates.clone();
                        for sub in &all_subs {
                            if !opts.contains(sub) {
                                opts.push(sub.clone());
                            }
                        }
                        for o in opts {
                            let label = name(&o);
                            let on = row.chosen.as_ref() == Some(&o);
                            if widgets::menu_row(ui, Some(Icon::Subtitle), &label, "", on).clicked() {
                                row.chosen = Some(o);
                            }
                        }
                        if widgets::menu_row(ui, None, "No subtitle", "", row.chosen.is_none()).clicked() {
                            row.chosen = None;
                        }
                    });
                });
        }
        let chosen: Vec<Option<PathBuf>> = s.rows.iter().map(|r| r.chosen.clone()).collect();
        let left_over = subtitles::unmatched(&all_subs, &chosen);
        if !left_over.is_empty() {
            ui.add_space(8.0);
            frame::rule(ui);
            ui.add_space(4.0);
            frame::label(ui, &format!("Not matched ({})", left_over.len()));
            for sub in left_over.iter().take(50) {
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    widgets::icon(ui, Icon::Subtitle, 24.0, p.fg_a4);
                    ui.label(egui::RichText::new(name(sub)).font(font::regular(14.0)).color(p.fg_a3));
                });
            }
        }
        if s.target == Target::Folder {
            ui.add_space(6.0);
            frame::note(
                ui,
                "Save copies each subtitle next to its video under the video's name, so VLC loads it automatically.",
            );
        }
    });
}

fn dashed(ui: &Ui, x0: f32, x1: f32, y: f32, color: egui::Color32) {
    if x1 <= x0 {
        return;
    }
    let shapes = egui::Shape::dashed_line(&[pos2(x0, y), pos2(x1, y)], Stroke::new(1.0, color), 6.0, 5.0);
    ui.painter().extend(shapes);
}

fn confirm_body(ui: &mut Ui, plan: &[(PathBuf, PathBuf, bool)]) {
    let p = pal(ui.ctx());
    let replaced = plan.iter().filter(|x| x.2).count();
    frame::heading(
        ui,
        &format!("Copy {} subtitle file{} next to the videos?", plan.len(), if plan.len() == 1 { "" } else { "s" }),
        if replaced > 0 { "Files with the same name already exist and will be replaced." } else { "" },
    );
    egui::ScrollArea::vertical().id_salt("subs-confirm").auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for (sub, to, exists) in plan {
            ui.horizontal(|ui| {
                ui.set_min_height(28.0);
                ui.label(egui::RichText::new(name(sub)).font(font::regular(14.0)).color(p.fg_a2));
                ui.label(egui::RichText::new("→").font(font::regular(14.0)).color(p.fg_a4));
                ui.label(egui::RichText::new(name(to)).font(font::regular(14.0)).color(p.fg2));
                if *exists {
                    ui.label(egui::RichText::new("replaces a file").font(font::regular(12.0)).color(p.danger));
                }
            });
        }
    });
}
