//! The "New Playlist" dialog (frame 6259:14954): a name, an optional cover
//! picture, and files dropped or picked into the box.

use std::path::PathBuf;

use egui::{Align2, Color32, Rect, Sense, Stroke, StrokeKind, pos2, vec2};

use crate::app::App;
use crate::domain::media::{AUDIO_EXTENSIONS, IMAGE_EXTENSIONS, MediaItem, PLAYLIST_EXTENSIONS, VIDEO_EXTENSIONS};
use crate::ui::icons::Icon;
use crate::ui::images;
use crate::ui::state::UiState;
use crate::ui::theme::{font, with_alpha};
use crate::ui::widgets::{self, pal};

use super::Page;

#[derive(Default)]
pub struct NewPlaylist {
    pub name: String,
    pub files: Vec<PathBuf>,
    pub cover: Option<PathBuf>,
    /// Items to put in the new playlist (from "Add to Playlist > New Playlist").
    pub seed: Vec<MediaItem>,
}

const W: f32 = 713.0;

fn media_exts() -> Vec<&'static str> {
    let mut v: Vec<&str> = VIDEO_EXTENSIONS.to_vec();
    v.extend(AUDIO_EXTENSIONS);
    v.extend(PLAYLIST_EXTENSIONS);
    v
}

/// A small dark square button with a "+" (the design's 30pt add chip).
fn add_chip(ui: &egui::Ui, r: Rect, hovered: bool) {
    let p = pal(ui.ctx());
    ui.painter().rect_filled(r, 5, if hovered { p.bg_a1 } else { p.bg_a5 });
    super::cards::icon(ui, Rect::from_center_size(r.center(), vec2(24.0, 24.0)), Icon::Add, p.fg_a2);
}

pub fn show(ctx: &egui::Context, app: &mut App, st: &mut UiState) {
    if !st.dialogs.new_playlist {
        return;
    }
    let p = pal(ctx);
    let mut close = false;
    let mut save = false;
    // Files dropped while the dialog is open belong to it.
    let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
    let np = &mut st.library.new_playlist;
    for f in dropped {
        if !np.files.contains(&f) {
            np.files.push(f);
        }
    }
    let modal = egui::Modal::new(egui::Id::new("lib-new-playlist"))
        .frame(egui::Frame::new().fill(p.bg_a3_t).corner_radius(14).stroke(Stroke::new(1.0, p.divider_dim)).shadow(
            egui::epaint::Shadow { offset: [0, 8], blur: 30, spread: 0, color: Color32::from_black_alpha(118) },
        ))
        .show(ctx, |ui| {
            ui.set_width(W);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            // Title bar.
            // Everything is placed absolutely inside one allocation.
            let (full, _) = ui.allocate_exact_size(vec2(W, 589.0), Sense::hover());
            let bar = Rect::from_min_size(full.min, vec2(W, 50.0));
            ui.painter().rect_filled(bar, egui::CornerRadius { nw: 14, ne: 14, sw: 0, se: 0 }, p.bg_b2);
            ui.painter().text(
                pos2(bar.min.x + 19.0, bar.center().y),
                Align2::LEFT_CENTER,
                "New Playlist",
                font::regular(14.0),
                p.fg_a2,
            );
            let xr = Rect::from_center_size(pos2(bar.max.x - 25.0, bar.center().y), vec2(32.0, 32.0));
            let xresp = ui.interact(xr, ui.id().with("np-close"), Sense::click());
            if xresp.hovered() {
                ui.painter().rect_filled(xr, 4, p.hover);
            }
            widgets::paint_icon(ui, Rect::from_center_size(xr.center(), vec2(24.0, 24.0)), Icon::Close, p.fg_a2);
            if xresp.clicked() {
                close = true;
            }

            let body = Rect::from_min_size(full.min + vec2(0.0, 50.0), vec2(W, 467.0));
            let x0 = body.min.x + 20.0;
            let mut y = body.min.y + 22.0;
            ui.painter().text(pos2(x0, y + 14.0), Align2::LEFT_CENTER, "Playlist Details", font::regular(16.0), p.fg2);
            ui.painter().text(
                pos2(x0, y + 45.0),
                Align2::LEFT_CENTER,
                "The playlist will be saved and displayed with this title and thumbnail.",
                font::regular(14.0),
                p.fg_a3,
            );
            // Name field.
            let field = Rect::from_min_size(pos2(x0, y + 80.0), vec2(541.0, 37.0));
            ui.painter().rect_filled(field, 6, p.bg_b2);
            ui.painter().rect_stroke(field, 6, Stroke::new(2.0, p.bg_a4), StrokeKind::Inside);
            {
                let ui = &mut ui.new_child(egui::UiBuilder::new().max_rect(field.shrink2(vec2(12.0, 9.0))));
                let r = ui.add(
                    egui::TextEdit::singleline(&mut np.name)
                        .hint_text(egui::RichText::new("e.g., Favorite Videos").color(p.fg_a4))
                        .frame(false)
                        .text_color(p.fg_a1)
                        .font(font::regular(14.0))
                        .desired_width(field.width() - 24.0),
                );
                if !np.name.is_empty() && r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    save = true;
                }
                if np.name.is_empty() && !r.has_focus() && ui.memory(|m| m.focused().is_none()) {
                    r.request_focus();
                }
            }
            // Cover picker.
            let cover = Rect::from_min_size(pos2(body.max.x - 20.0 - 112.0, y + 7.0), vec2(112.0, 112.0));
            let cresp = ui.interact(cover, ui.id().with("np-cover"), Sense::click());
            ui.painter().rect_filled(cover, 6, p.bg_b2);
            let has = np.cover.as_ref().is_some_and(|c| images::paint_cover(ui, cover.shrink(2.0), c, 5));
            ui.painter().rect_stroke(cover, 6, Stroke::new(2.0, p.bg_a4), StrokeKind::Inside);
            if !has || cresp.hovered() {
                add_chip(ui, Rect::from_center_size(cover.center(), vec2(30.0, 30.0)), cresp.hovered());
            }
            if cresp.on_hover_text("Choose a picture").clicked()
                && let Some(img) = rfd::FileDialog::new()
                    .set_title("Playlist Picture")
                    .add_filter("Images", IMAGE_EXTENSIONS)
                    .pick_file()
            {
                np.cover = Some(img);
            }

            y += 139.0;
            ui.painter().hline(x0..=body.max.x - 20.0, y + 0.5, Stroke::new(1.0, p.divider_dim));
            y += 20.0;
            ui.painter().text(pos2(x0, y + 14.0), Align2::LEFT_CENTER, "Select Files", font::regular(16.0), p.fg2);
            ui.painter().text(
                pos2(x0, y + 45.0),
                Align2::LEFT_CENTER,
                "These files will be added to the playlist.",
                font::regular(14.0),
                p.fg_a3,
            );
            y += 78.0;
            // Drop box.
            let dropbox = Rect::from_min_size(pos2(x0, y), vec2(673.0, 186.0));
            let hovering_files = ui.input(|i| !i.raw.hovered_files.is_empty());
            ui.painter().rect_filled(dropbox, 6, if hovering_files { with_alpha(p.accent, 0.08) } else { p.bg_b2 });
            dashed_rect(ui, dropbox, if hovering_files { p.accent } else { p.divider_bright });
            let count = np.files.len() + np.seed.len();
            if count == 0 {
                let resp = ui.interact(dropbox, ui.id().with("np-drop"), Sense::click());
                ui.painter().text(
                    pos2(dropbox.center().x, dropbox.min.y + 67.0),
                    Align2::CENTER_CENTER,
                    "Drag files into the box or click here to select files.",
                    font::regular(14.0),
                    p.fg_a4,
                );
                add_chip(
                    ui,
                    Rect::from_center_size(pos2(dropbox.center().x, dropbox.min.y + 112.0), vec2(84.0, 30.0)),
                    resp.hovered(),
                );
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    pick_files(np);
                }
            } else {
                let inner = dropbox.shrink2(vec2(12.0, 8.0));
                {
                    let ui = &mut ui.new_child(egui::UiBuilder::new().max_rect(inner));
                    egui::ScrollArea::vertical().id_salt("np-files").max_height(inner.height() - 38.0).show(ui, |ui| {
                        let mut remove = None;
                        let names: Vec<String> =
                            np.seed
                                .iter()
                                .map(|i| i.title())
                                .chain(np.files.iter().map(|f| {
                                    f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
                                }))
                                .collect();
                        for (i, name) in names.iter().enumerate() {
                            let (r, resp) = ui.allocate_exact_size(vec2(inner.width() - 8.0, 26.0), Sense::hover());
                            if resp.hovered() {
                                ui.painter().rect_filled(r, 4, p.hover);
                            }
                            widgets::paint_icon(
                                ui,
                                Rect::from_min_size(pos2(r.min.x + 4.0, r.center().y - 9.0), vec2(18.0, 18.0)),
                                Icon::File,
                                p.fg_a3,
                            );
                            widgets::text_clipped(
                                ui,
                                pos2(r.min.x + 30.0, r.center().y),
                                Align2::LEFT_CENTER,
                                name,
                                font::regular(14.0),
                                p.fg_a1,
                                r.width() - 64.0,
                            );
                            let xr = Rect::from_center_size(pos2(r.max.x - 14.0, r.center().y), vec2(20.0, 20.0));
                            let x = ui.interact(xr, ui.id().with(("np-x", i)), Sense::click());
                            widgets::paint_icon(ui, xr, Icon::Close, if x.hovered() { p.fg1 } else { p.fg_a3 });
                            if x.clicked() {
                                remove = Some(i);
                            }
                        }
                        if let Some(i) = remove {
                            if i < np.seed.len() {
                                np.seed.remove(i);
                            } else {
                                np.files.remove(i - np.seed.len());
                            }
                        }
                    });
                    let br = Rect::from_min_size(pos2(inner.min.x, inner.max.y - 30.0), vec2(84.0, 30.0));
                    let resp = ui.interact(br, ui.id().with("np-more"), Sense::click());
                    add_chip(ui, br, resp.hovered());
                    if resp.on_hover_text("Add more files").clicked() {
                        pick_files(np);
                    }
                }
            }

            // Footer.
            let foot = Rect::from_min_size(full.min + vec2(0.0, 517.0), vec2(W, 72.0));
            let cancel = Rect::from_min_size(pos2(foot.min.x + 20.0, foot.min.y + 20.0), vec2(100.0, 32.0));
            let cr = ui.interact(cancel, ui.id().with("np-cancel"), Sense::click());
            ui.painter().rect_filled(cancel, 6, if cr.hovered() { p.bg_a1 } else { p.bg_a3 });
            ui.painter().text(cancel.center(), Align2::CENTER_CENTER, "Cancel", font::semibold(14.0), p.fg_a2);
            if cr.clicked() {
                close = true;
            }
            let ok = !np.name.trim().is_empty();
            let saver = Rect::from_min_size(pos2(foot.max.x - 120.0, foot.min.y + 20.0), vec2(100.0, 32.0));
            let sr = ui.interact(saver, ui.id().with("np-save"), if ok { Sense::click() } else { Sense::hover() });
            let a = if ok { 1.0 } else { 0.5 };
            let fill = if ok && sr.hovered() { p.accent_dim } else { p.bg_accent };
            ui.painter().rect_filled(saver, 6, with_alpha(fill, a));
            ui.painter().text(
                saver.center(),
                Align2::CENTER_CENTER,
                "Save",
                font::semibold(14.0),
                with_alpha(p.accent, a),
            );
            if ok && sr.clicked() {
                save = true;
            }
        });
    if save {
        let np = std::mem::take(&mut st.library.new_playlist);
        let mut items = np.seed;
        items.extend(app.expand_paths(&np.files));
        let id = app.library.create_playlist(&np.name, items);
        if let Some(c) = &np.cover {
            app.set_playlist_cover(id, c);
        }
        st.dialogs.new_playlist = false;
        st.library.invalidate();
        st.navigate(Page::Playlist(id));
    } else if close || modal.should_close() {
        st.library.new_playlist = NewPlaylist::default();
        st.dialogs.new_playlist = false;
    }
}

fn pick_files(np: &mut NewPlaylist) {
    if let Some(files) =
        rfd::FileDialog::new().set_title("Select Files").add_filter("Media", &media_exts()).pick_files()
    {
        for f in files {
            if !np.files.contains(&f) {
                np.files.push(f);
            }
        }
    }
}

/// The drop box's dashed 2pt border.
fn dashed_rect(ui: &egui::Ui, r: Rect, color: Color32) {
    let r = r.shrink(1.0);
    let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    for w in pts.windows(2) {
        ui.painter().extend(egui::Shape::dashed_line(&[w[0], w[1]], Stroke::new(2.0, color), 4.0, 4.0));
    }
}
