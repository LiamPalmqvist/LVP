//! Open Disc: DVD, Blu-ray, Audio CD and VCD from a drive, a mounted
//! volume, a VIDEO_TS/BDMV folder or an ISO image.

use std::path::{Path, PathBuf};

use egui::{RichText, vec2};

use super::frame::{self, Btn, Part};
use crate::app::App;
use crate::domain::disc::{DiscKind, DiscRequest};
use crate::domain::media::{MediaItem, MediaSource};
use crate::ui::state::{UiState, View};
use crate::ui::theme::font;
use crate::ui::widgets::{self, pal};

#[derive(Default)]
pub struct DiscState {
    req: DiscRequest,
    /// Discs found under /Volumes when the dialog opened.
    found: Vec<(String, DiscKind, PathBuf)>,
    primed: bool,
}

/// Looks for mounted discs: VIDEO_TS (DVD), BDMV (Blu-ray), MPEGAV/SVCD
/// (VCD) or AIFF tracks (an audio CD as macOS mounts it).
fn scan_volumes() -> Vec<(String, DiscKind, PathBuf)> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir("/Volumes") else { return out };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        let kind = if p.join("VIDEO_TS").is_dir() {
            Some(DiscKind::Dvd)
        } else if p.join("BDMV").is_dir() {
            Some(DiscKind::Bluray)
        } else if p.join("MPEGAV").is_dir() || p.join("SVCD").is_dir() {
            Some(DiscKind::Vcd)
        } else if has_aiff(&p) {
            Some(DiscKind::AudioCd)
        } else {
            None
        };
        if let Some(k) = kind {
            out.push((name, k, p));
        }
    }
    out
}

fn has_aiff(p: &Path) -> bool {
    std::fs::read_dir(p)
        .map(|rd| {
            rd.flatten()
                .take(40)
                .any(|e| e.path().extension().and_then(|x| x.to_str()).is_some_and(|x| x.eq_ignore_ascii_case("aiff")))
        })
        .unwrap_or(false)
}

/// Picks the disc kind from what the chosen path contains.
fn guess_kind(p: &Path) -> Option<DiscKind> {
    let name = p.file_name()?.to_string_lossy().to_ascii_uppercase();
    if name == "VIDEO_TS" || p.join("VIDEO_TS").is_dir() {
        Some(DiscKind::Dvd)
    } else if name == "BDMV" || p.join("BDMV").is_dir() {
        Some(DiscKind::Bluray)
    } else {
        None
    }
}

pub(super) fn number_row(ui: &mut egui::Ui, label: &str, value: &mut u32, hint: &str) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        frame::label(ui, label).on_hover_text(hint);
        let mut v = *value as f64;
        frame::number(ui, &mut v, 0.0..=999.0, if *value == 0 { "auto" } else { "" }, 150.0);
        *value = v.round() as u32;
    });
}

pub fn show(ctx: &egui::Context, app: &mut App, st: &mut UiState) {
    let ds = &mut st.dialogs.disc;
    if !ds.primed {
        ds.found = scan_volumes();
        if ds.req.path.is_empty()
            && let Some((_, k, p)) = ds.found.first()
        {
            ds.req.kind = *k;
            ds.req.path = p.display().to_string();
        }
        ds.primed = true;
    }
    let mut action: Option<u8> = None; // 0 play, 1 enqueue, 2 convert
    let mut cancel = false;
    let out = frame::modal(ctx, "open-disc", "Open Disc", vec2(1048.0, 600.0), true, |ui, part| match part {
        Part::Body => {
            let p = pal(ui.ctx());
            let w = ui.available_width();
            frame::heading(ui, "Disc", "");
            let mut kind_i = DiscKind::ALL.iter().position(|k| *k == ds.req.kind).unwrap_or(0);
            let labels: Vec<&str> = DiscKind::ALL.iter().map(|k| k.label()).collect();
            if widgets::segmented(ui, &mut kind_i, &labels, 520.0) {
                ds.req.kind = DiscKind::ALL[kind_i];
            }
            ui.add_space(6.0);
            frame::heading(
                ui,
                "Disc device",
                "A mounted disc, a VIDEO_TS or BDMV folder, or an ISO image. Leave it empty to use the default drive.",
            );
            ui.horizontal(|ui| {
                frame::field(ui, &mut ds.req.path, "Default drive", w - 2.0 * 110.0);
                if frame::btn_sized(ui, "Folder", Btn::Plain, true, vec2(100.0, 37.0), font::regular(14.0)).clicked()
                    && let Some(d) = rfd::FileDialog::new().set_title("Disc Folder").pick_folder()
                {
                    if let Some(k) = guess_kind(&d) {
                        ds.req.kind = k;
                    }
                    ds.req.path = d.display().to_string();
                }
                if frame::btn_sized(ui, "ISO Image", Btn::Plain, true, vec2(100.0, 37.0), font::regular(14.0)).clicked()
                    && let Some(f) = rfd::FileDialog::new()
                        .set_title("Disc Image")
                        .add_filter("Disc images", &["iso", "img", "bin", "cue", "nrg", "mdf"])
                        .pick_file()
                {
                    ds.req.path = f.display().to_string();
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                ui.spacing_mut().interact_size.y = 29.0;
                frame::caption(ui, "Found", 64.0);
                if ds.found.is_empty() {
                    frame::caption(ui, "No disc is mounted.", 200.0);
                }
                for (name, k, path) in &ds.found {
                    let on = ds.req.path == path.display().to_string();
                    if frame::chip(ui, &format!("{name} ({})", k.label()), on).clicked() {
                        ds.req.kind = *k;
                        ds.req.path = path.display().to_string();
                    }
                }
                if frame::chip(ui, "Rescan", false).clicked() {
                    ds.found = scan_volumes();
                }
            });
            ui.add_space(6.0);
            frame::rule(ui);
            ui.add_space(6.0);
            if ds.req.kind.has_menus() {
                ui.horizontal(|ui| {
                    widgets::toggle(ui, &mut ds.req.no_menus);
                    ui.label(RichText::new("No disc menus").font(font::regular(14.0)).color(p.fg2));
                    ui.label(
                        RichText::new("Starts the main title directly instead of the disc's menu.")
                            .font(font::regular(13.0))
                            .color(p.fg_a3),
                    );
                });
                ui.add_space(4.0);
            }
            frame::heading(ui, "Starting position", "");
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 20.0;
                match ds.req.kind {
                    DiscKind::AudioCd => number_row(ui, "Track", &mut ds.req.title, "The CD track to start with."),
                    _ => {
                        number_row(ui, "Title", &mut ds.req.title, "The disc title to start with.");
                        number_row(ui, "Chapter", &mut ds.req.chapter, "The chapter within that title.");
                    }
                }
                if ds.req.kind != DiscKind::AudioCd {
                    number_row(ui, "Audio track", &mut ds.req.audio_track, "1 is the disc's first audio track.");
                    number_row(ui, "Subtitle track", &mut ds.req.subtitle_track, "1 is the disc's first subtitle.");
                }
            });
            ui.add_space(4.0);
            let opts = ds.req.options();
            ui.label(
                RichText::new(format!("{} {}", ds.req.mrl(), opts.join(" ")).trim().to_string())
                    .font(font::mono(13.0))
                    .color(p.fg_a3),
            );
            if ds.req.kind == DiscKind::Bluray {
                frame::note(
                    ui,
                    "Encrypted Blu-ray discs need libaacs and its key database installed for VLC; menus need Java \
                     (libbluray BD-J).",
                );
            }
        }
        _ => {
            if frame::cancel(ui) {
                cancel = true;
            }
            frame::right(ui, |ui| {
                if frame::btn(ui, "Play", Btn::Accent, true).clicked() {
                    action = Some(0);
                }
                if frame::btn(ui, "Enqueue", Btn::Plain, true).clicked() {
                    action = Some(1);
                }
                if frame::btn(ui, "Convert", Btn::Plain, true).clicked() {
                    action = Some(2);
                }
            });
        }
    });
    if let Some(a) = action {
        let mut item = MediaItem::new(MediaSource::Location(ds.req.mrl()));
        item.options = ds.req.options();
        let name = Path::new(ds.req.path.trim())
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| ds.req.kind.label().to_string());
        item.meta.title = Some(name);
        ds.primed = false;
        st.dialogs.open_disc = false;
        match a {
            0 => {
                app.play_items(vec![item], 0, None);
                st.view = View::Player;
            }
            1 => app.enqueue(vec![item]),
            _ => st.dialogs.convert_items(vec![item]),
        }
        return;
    }
    if cancel || out.close {
        st.dialogs.open_disc = false;
        st.dialogs.disc.primed = false;
    }
}
