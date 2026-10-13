//! The Library page: opening files, metadata, and the library folders.

use std::path::PathBuf;

use egui::{Align2, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};

use super::Cx;
use super::kit::{self, Btn, Form};
use crate::domain::settings::*;
use crate::ui::icons::Icon;
use crate::ui::theme::font;
use crate::ui::widgets::{self, pal, text_clipped};

pub fn library(f: &mut Form, s: &mut Settings, _cx: &mut Cx) {
    let l = &mut s.library;
    f.section("Opening Files");
    f.toggle(
        "Only allow one VLC window",
        Some("Opening a file while the app runs plays it in the open window instead of starting a second one."),
        &mut l.one_instance,
    );
    f.toggle(
        "Apply to files opened from the file manager",
        Some("Use the single window for files opened from Finder too."),
        &mut l.one_instance_from_file_manager,
    );
    f.toggle(
        "Add to the playlist instead of replacing it",
        Some("Files opened from outside are queued after what is playing."),
        &mut l.enqueue,
    );
    f.choice(
        "When opening a folder",
        Some("Whether files in subfolders are played too."),
        &mut l.folder_open,
        FolderOpen::ALL,
        FolderOpen::label,
    );
    f.toggle("Show hidden files", Some("List files and folders whose names start with a dot."), &mut l.show_hidden);

    f.section("Metadata & Artwork");
    f.choice(
        "Fetch titles and artwork",
        Some("Where titles, artists and cover art come from."),
        &mut l.artwork,
        ArtworkFetch::ALL,
        ArtworkFetch::label,
    );
    f.toggle(
        "Use the media library",
        Some("Index your folders so the Videos and Music pages can list everything in them."),
        &mut l.use_media_library,
    );

    f.advanced(|f| {
        f.block("video folders library movies", |ui| folder_list(ui, "Video folders", &mut l.video_folders));
        f.block("music folders library songs", |ui| {
            ui.add_space(20.0);
            folder_list(ui, "Music folders", &mut l.music_folders)
        });
    });
}

/// A heading with an "Add Folder" button and the folders below it, each with
/// a remove button.
fn folder_list(ui: &mut Ui, title: &str, folders: &mut Vec<PathBuf>) {
    let p = pal(ui.ctx());
    let mut add = false;
    kit::row_ui(ui, title, Some("The library pages show the media found in these folders."), kit::ROW_TOGGLE, |ui| {
        add = kit::action_button(ui, "Add Folder", Btn::Normal, 100.0).clicked();
    });
    if add && let Some(dirs) = rfd::FileDialog::new().set_title(format!("Add to {title}")).pick_folders() {
        for d in dirs {
            if !folders.contains(&d) {
                folders.push(d);
            }
        }
    }
    let w = ui.available_width();
    let rows = folders.len().max(1);
    let (rect, _) = ui.allocate_exact_size(vec2(w, rows as f32 * 40.0 + 12.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(6), p.bg_b2);
    ui.painter().rect_stroke(rect, CornerRadius::same(6), Stroke::new(2.0, p.bg_a4), StrokeKind::Inside);
    if folders.is_empty() {
        ui.painter().text(
            pos2(rect.min.x + 16.0, rect.center().y),
            Align2::LEFT_CENTER,
            "No folders yet. Use Add Folder to choose one.",
            font::regular(14.0),
            p.fg_a3,
        );
        return;
    }
    let mut remove = None;
    for (i, dir) in folders.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.min.x + 6.0, rect.min.y + 6.0 + i as f32 * 40.0), vec2(w - 12.0, 40.0));
        let hovered = ui.rect_contains_pointer(r);
        if hovered {
            ui.painter().rect_filled(r, CornerRadius::same(4), p.hover);
        }
        widgets::paint_icon(
            ui,
            Rect::from_min_size(pos2(r.min.x + 8.0, r.center().y - 12.0), Vec2::splat(24.0)),
            Icon::Folder,
            p.fg_a2,
        );
        let exists = dir.is_dir();
        text_clipped(
            ui,
            pos2(r.min.x + 42.0, r.center().y),
            Align2::LEFT_CENTER,
            &dir.to_string_lossy(),
            font::regular(14.0),
            if exists { p.fg_a1 } else { p.fg_a3 },
            r.width() - 100.0,
        );
        if !exists {
            ui.painter().text(
                pos2(r.max.x - 46.0, r.center().y),
                Align2::RIGHT_CENTER,
                "Missing",
                font::regular(12.0),
                p.danger,
            );
        }
        let br = Rect::from_min_size(pos2(r.max.x - 38.0, r.center().y - 16.0), vec2(32.0, 32.0));
        let clicked = ui
            .scope_builder(egui::UiBuilder::new().max_rect(br), |ui| {
                widgets::icon_button_sized(ui, Icon::Remove, vec2(32.0, 32.0), 20.0, false, "Remove from the library")
                    .clicked()
            })
            .inner;
        if clicked {
            remove = Some(i);
        }
    }
    if let Some(i) = remove {
        folders.remove(i);
    }
}
