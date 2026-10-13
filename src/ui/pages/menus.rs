//! Library menus: the 48pt option rows of the design's menu components,
//! the right-click menus for files (2404:13340), folders (2139:8935) and
//! playlists (2459:9389), the header's page menu, and the delete prompt.

use std::path::{Path, PathBuf};

use egui::{Align2, Color32, Rect, Response, Sense, Stroke, Ui, pos2, vec2};

use crate::app::App;
use crate::domain::media::{AUDIO_EXTENSIONS, MediaItem, MediaKind, SUBTITLE_EXTENSIONS};
use crate::domain::playlist_format::PlaylistFormat;
use crate::ui::icons::Icon;
use crate::ui::state::UiState;
use crate::ui::theme::{Palette, font};
use crate::ui::widgets::{self, ButtonKind, pal};

use super::model::{Act, PageKind, PageModel, SectionModel};
use super::{Page, play};

/// The menu surface: BG/A4, radius 10, 4pt padding.
pub fn frame(p: &Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(p.bg_a4)
        .corner_radius(10)
        .inner_margin(egui::Margin::same(4))
        .stroke(Stroke::new(1.0, p.divider_dim))
        .shadow(egui::epaint::Shadow { offset: [0, 8], blur: 30, spread: 0, color: Color32::from_black_alpha(118) })
}

/// One 48pt option: optional icon, label, and a check or chevron.
pub fn row(ui: &mut Ui, icon: Option<Icon>, label: &str, selected: bool, chevron: bool) -> Response {
    let p = pal(ui.ctx());
    let w = ui.available_width().max(200.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 48.0), Sense::click());
    if selected {
        ui.painter().rect_filled(rect, 6, p.bg_a3);
    }
    if resp.hovered() {
        ui.painter().rect_filled(rect, 6, p.hover);
    }
    let fg = if resp.hovered() { p.fg1 } else { p.fg_a2 };
    let mut x = rect.min.x + 14.0;
    if let Some(i) = icon {
        super::cards::icon(ui, Rect::from_min_size(pos2(x, rect.center().y - 12.0), vec2(24.0, 24.0)), i, fg);
        x += 34.0;
    }
    widgets::text_clipped(
        ui,
        pos2(x, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font::regular(14.0),
        fg,
        rect.max.x - x - 44.0,
    );
    let right = Rect::from_min_size(pos2(rect.max.x - 38.0, rect.center().y - 12.0), vec2(24.0, 24.0));
    if chevron {
        widgets::paint_icon(ui, right, Icon::ChevronRight, p.fg_a3);
    } else if selected {
        widgets::paint_icon(ui, right, Icon::Check, p.fg_a1);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn sub(ui: &mut Ui, icon: Option<Icon>, label: &str, add: impl FnOnce(&mut Ui)) {
    let resp = row(ui, icon, label, false, true);
    egui::containers::menu::SubMenu::new().show(ui, &resp, |ui| {
        ui.set_min_width(257.0);
        ui.spacing_mut().item_spacing.y = 0.0;
        add(ui);
    });
}

fn separator(ui: &mut Ui) {
    let p = pal(ui.ctx());
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 9.0), Sense::hover());
    ui.painter().hline(r.x_range().shrink(10.0), r.center().y, Stroke::new(1.0, p.divider_dim));
}

/// "Add to Playlist" submenu: a new playlist or any existing one.
fn add_to_playlist(ui: &mut Ui, app: &mut App, st: &mut UiState, items: Vec<MediaItem>) {
    let lists: Vec<(u64, String)> = app.library.playlists().iter().map(|p| (p.id, p.name.clone())).collect();
    sub(ui, Some(Icon::Playlist), "Add to Playlist", |ui| {
        if row(ui, Some(Icon::Add), "New Playlist...", false, false).clicked() {
            st.library.new_playlist.seed = items.clone();
            st.dialogs.new_playlist = true;
            ui.close();
        }
        for (id, name) in lists {
            if row(ui, Some(Icon::Lists), &name, false, false).clicked() {
                app.add_to_playlist(id, items.clone());
                st.library.invalidate();
                ui.close();
            }
        }
    });
}

fn pick_and_attach(app: &mut App, st: &mut UiState, item: &MediaItem, subtitle: bool) {
    let (title, exts) =
        if subtitle { ("Add Subtitle Track", SUBTITLE_EXTENSIONS) } else { ("Add Audio Track", AUDIO_EXTENSIONS) };
    let Some(file) = rfd::FileDialog::new().set_title(title).add_filter(title, exts).pick_file() else { return };
    let current = app.current_source() == Some(&item.source);
    if subtitle && current {
        app.add_subtitle(file);
        return;
    }
    let mut it = item.clone();
    if subtitle {
        it.options.push(format!(":sub-file={}", file.display()));
    } else {
        it.options.push(format!(":input-slave={}", crate::domain::media::path_to_file_url(&file)));
    }
    play(app, st, vec![it], 0, None);
}

/// The right-click menu for a playable item (frame 2404:13340).
pub fn item_menu(resp: &Response, app: &mut App, st: &mut UiState, kind: &PageKind, sec: &SectionModel, index: usize) {
    let Some(item) = sec.playable.get(index).cloned() else { return };
    let p = pal(&resp.ctx);
    egui::Popup::context_menu(resp).frame(frame(&p)).width(265.0).show(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        if row(ui, Some(Icon::Play), "Play", false, false).clicked() {
            play(app, st, sec.playable.clone(), index, sec.origin.clone());
        }
        if row(ui, Some(Icon::Next), "Play Next", false, false).clicked() {
            app.play_next(item.clone());
            app.toast(format!("{} plays next", item.title()));
        }
        let record = app.library.record(&item.source);
        let watched = record.finished();
        if item.source.path().is_some() {
            let label = if watched { "Mark as Unwatched" } else { "Mark as Watched" };
            if row(ui, Some(Icon::CheckSelected), label, false, false).clicked() {
                app.set_watched(&item.source, !watched);
                st.library.invalidate();
            }
        }
        let fav = record.favorite;
        if row(
            ui,
            Some(if fav { Icon::FavoriteActive } else { Icon::Favorite }),
            if fav { "Remove from Favorites" } else { "Favorite" },
            false,
            false,
        )
        .clicked()
        {
            app.library.toggle_favorite(&item.source);
            st.library.invalidate();
        }
        if row(ui, Some(Icon::Streaming), "Stream", false, false).clicked() {
            st.library.action_item = Some(item.clone());
            st.dialogs.convert = true;
        }
        if row(ui, Some(Icon::Swap), "Convert", false, false).clicked() {
            st.library.action_item = Some(item.clone());
            st.dialogs.convert = true;
        }
        if row(ui, Some(Icon::Queue), "Add to Queue", false, false).clicked() {
            app.enqueue(vec![item.clone()]);
            app.toast(format!("Added {} to the queue", item.title()));
        }
        add_to_playlist(ui, app, st, vec![item.clone()]);
        if let Some(path) = item.source.path() {
            if row(ui, Some(Icon::Folder), "Show in Finder", false, false).clicked() {
                app.reveal_in_file_manager(path);
            }
            let on_folder = matches!(kind, PageKind::Folder { path: d, .. } if path.parent() == Some(d.as_path()))
                || matches!(kind, PageKind::Root { .. });
            if !on_folder
                && let Some(dir) = path.parent()
                && row(ui, Some(Icon::ArrowRight), "Go to Source", false, false).clicked()
            {
                st.navigate(super::model::folder_page(app, dir));
            }
        }
        if item.kind() != MediaKind::Audio
            && row(ui, Some(Icon::Subtitle), "Add Subtitle Tracks", false, false).clicked()
        {
            pick_and_attach(app, st, &item, true);
        }
        if row(ui, Some(Icon::Audio), "Add Audio Tracks", false, false).clicked() {
            pick_and_attach(app, st, &item, false);
        }
        if matches!(kind, PageKind::Recent | PageKind::HomeList | PageKind::Home)
            && row(ui, Some(Icon::Remove), "Remove from History", false, false).clicked()
        {
            app.library.remove_from_recent(&item.source);
            st.library.invalidate();
        }
        if let PageKind::Playlist(id) = kind
            && row(ui, Some(Icon::Remove), "Remove from Playlist", false, false).clicked()
        {
            app.remove_playlist_item(*id, index);
            st.library.invalidate();
        }
        if row(ui, Some(Icon::Info), "Properties & Metadata", false, false).clicked() {
            st.library.action_item = Some(item.clone());
            st.dialogs.media_info = true;
        }
    });
}

fn media_kinds(music: bool) -> Vec<MediaKind> {
    if music { vec![MediaKind::Audio] } else { vec![MediaKind::Video, MediaKind::Audio] }
}

/// Everything playable below a folder, recursively when the settings ask.
pub fn folder_items(app: &App, dir: &Path, music: bool) -> Vec<MediaItem> {
    let recursive = app.settings.library.folder_open == crate::domain::settings::FolderOpen::IncludeSubfolders;
    app.library.collect_media(dir, &media_kinds(music), recursive, app.settings.library.show_hidden)
}

fn play_folder(app: &mut App, st: &mut UiState, dir: &Path, music: bool, shuffle: bool) {
    let mut items = folder_items(app, dir, music);
    if items.is_empty() {
        app.toast_error("Nothing playable was found.");
        return;
    }
    if shuffle {
        app.set_shuffle(true);
    }
    let name = dir.file_name().map(|n| n.to_string_lossy().into_owned());
    let start = if shuffle { fastrand_index(items.len()) } else { 0 };
    items.truncate(10_000);
    play(app, st, items, start, name);
}

fn fastrand_index(n: usize) -> usize {
    let nanos =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
    (nanos as usize) % n.max(1)
}

/// The right-click menu for a folder (frame 2139:8935).
pub fn folder_menu(resp: &Response, app: &mut App, st: &mut UiState, dir: PathBuf, music: bool, is_root: bool) {
    let p = pal(&resp.ctx);
    egui::Popup::context_menu(resp).frame(frame(&p)).width(265.0).show(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        if row(ui, Some(Icon::Play), "Play", false, false).clicked() {
            play_folder(app, st, &dir, music, false);
        }
        if row(ui, Some(Icon::ShuffleRight), "Shuffle", false, false).clicked() {
            play_folder(app, st, &dir, music, true);
        }
        if row(ui, Some(Icon::Queue), "Add to Queue", false, false).clicked() {
            let items = folder_items(app, &dir, music);
            let n = items.len();
            app.enqueue(items);
            app.toast(format!("Added {n} item{} to the queue", if n == 1 { "" } else { "s" }));
        }
        add_to_playlist(ui, app, st, folder_items(app, &dir, music));
        separator(ui);
        if row(ui, Some(Icon::Subtitle), "Edit Subtitle Tracks", false, false).clicked() {
            st.library.action_item = Some(MediaItem::file(&dir));
            st.dialogs.add_subtitles = true;
        }
        if row(ui, Some(Icon::Folder), "Show in Finder", false, false).clicked() {
            app.reveal_in_file_manager(&dir);
        }
        if row(ui, Some(Icon::Refresh), "Resync", false, false).clicked() {
            st.library.invalidate();
        }
        if is_root && row(ui, Some(Icon::Remove), "Remove from Library", false, false).clicked() {
            app.remove_library_folder(&dir, music);
            st.library.invalidate();
        }
    });
}

fn export_playlist(app: &mut App, id: u64) {
    let Some(pl) = app.library.playlist(id).cloned() else { return };
    if let Some(path) = rfd::FileDialog::new()
        .set_title("Export Playlist")
        .set_file_name(format!("{}.xspf", pl.name))
        .add_filter("XSPF", &["xspf"])
        .add_filter("M3U", &["m3u", "m3u8"])
        .add_filter("PLS", &["pls"])
        .save_file()
    {
        app.export_items(&pl.name, &pl.items, &path);
    }
}

pub fn import_playlist(app: &mut App, st: &mut UiState) {
    let exts: Vec<&str> =
        ["xspf", "m3u", "m3u8", "pls"].into_iter().filter(|e| PlaylistFormat::from_extension(e).is_some()).collect();
    if let Some(files) = rfd::FileDialog::new().set_title("Import Playlist").add_filter("Playlists", &exts).pick_files()
    {
        for f in files {
            match app.import_playlist(&f) {
                Some(_) => app.toast(format!("Imported {}", f.display())),
                None => app.toast_error(format!("Could not read {}", f.display())),
            }
        }
        st.library.invalidate();
    }
}

fn playlist_items(app: &App, id: u64) -> Vec<MediaItem> {
    app.library.playlist(id).map(|p| p.items.clone()).unwrap_or_default()
}

fn play_playlist(app: &mut App, st: &mut UiState, id: u64, shuffle: bool) {
    let items = playlist_items(app, id);
    if items.is_empty() {
        app.toast_error("This playlist is empty.");
        return;
    }
    let name = app.library.playlist(id).map(|p| p.name.clone());
    if shuffle {
        app.set_shuffle(true);
    }
    let start = if shuffle { fastrand_index(items.len()) } else { 0 };
    play(app, st, items, start, name);
}

/// The right-click menu for a playlist (frame 2459:9389).
pub fn playlist_menu(resp: &Response, app: &mut App, st: &mut UiState, id: u64) {
    let p = pal(&resp.ctx);
    egui::Popup::context_menu(resp).frame(frame(&p)).width(265.0).show(|ui| playlist_rows(ui, app, st, id));
}

fn playlist_rows(ui: &mut Ui, app: &mut App, st: &mut UiState, id: u64) {
    ui.spacing_mut().item_spacing.y = 0.0;
    if row(ui, Some(Icon::Play), "Play", false, false).clicked() {
        play_playlist(app, st, id, false);
    }
    if row(ui, Some(Icon::ShuffleRight), "Shuffle", false, false).clicked() {
        play_playlist(app, st, id, true);
    }
    if row(ui, Some(Icon::Save), "Export Playlist", false, false).clicked() {
        export_playlist(app, id);
    }
    if row(ui, Some(Icon::Queue), "Add to Queue", false, false).clicked() {
        let items = playlist_items(app, id);
        app.enqueue(items);
    }
    if row(ui, Some(Icon::Subtitle), "Add Subtitle Tracks", false, false).clicked() {
        st.library.action_item = None;
        st.dialogs.add_subtitles = true;
    }
    separator(ui);
    if row(ui, Some(Icon::Edit), "Rename", false, false).clicked() {
        let name = app.library.playlist(id).map(|p| p.name.clone()).unwrap_or_default();
        st.library.rename = Some((id, name));
        if st.page != Page::Playlist(id) {
            st.navigate(Page::Playlist(id));
            st.library.seen_page = Some(Page::Playlist(id));
        }
    }
    if row(ui, Some(Icon::Remove), "Delete", false, false).clicked() {
        st.library.confirm_delete = Some(id);
    }
}

/// The header's "..." menu, with what applies to the page.
pub fn page_menu(button: &Response, app: &mut App, st: &mut UiState, m: &PageModel) {
    let p = pal(&button.ctx);
    egui::Popup::menu(button).frame(frame(&p)).width(265.0).gap(6.0).show(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let playable: Vec<MediaItem> = m.sections.iter().flat_map(|s| s.playable.iter().cloned()).collect();
        match &m.kind {
            PageKind::Folder { path, music } => {
                folder_page_rows(ui, app, st, path.clone(), *music);
            }
            PageKind::Root { music } if app.settings.library.video_folders.len() == 1 || *music => {
                let roots = if *music {
                    app.settings.library.music_folders.clone()
                } else {
                    app.settings.library.video_folders.clone()
                };
                if let [only] = roots.as_slice() {
                    folder_page_rows(ui, app, st, only.clone(), *music);
                }
                if row(ui, Some(Icon::NewFolder), "Add Folder...", false, false).clicked() {
                    super::view::run_act(app, st, Act::AddFolder { music: *music });
                }
            }
            PageKind::Playlist(id) => playlist_rows(ui, app, st, *id),
            PageKind::Playlists => {
                if row(ui, Some(Icon::Add), "Create Playlist...", false, false).clicked() {
                    st.dialogs.new_playlist = true;
                }
                if row(ui, Some(Icon::Open), "Import Playlist...", false, false).clicked() {
                    import_playlist(app, st);
                }
            }
            PageKind::Network | PageKind::Service | PageKind::NetFolder => {
                if row(ui, Some(Icon::Streaming), "Open Network Stream...", false, false).clicked() {
                    st.dialogs.open_network = true;
                }
                if !playable.is_empty() && row(ui, Some(Icon::Play), "Play All", false, false).clicked() {
                    play(app, st, playable.clone(), 0, m.crumbs.last().map(|c| c.0.clone()));
                }
                if row(ui, Some(Icon::Refresh), "Refresh", false, false).clicked() {
                    super::network::refresh(&mut st.library.net);
                    st.library.invalidate();
                }
            }
            PageKind::Recent | PageKind::Home | PageKind::HomeList => {
                if !playable.is_empty() && row(ui, Some(Icon::Play), "Play All", false, false).clicked() {
                    play(app, st, playable.clone(), 0, m.crumbs.last().map(|c| c.0.clone()));
                }
                if row(ui, Some(Icon::Remove), "Clear History", false, false).clicked() {
                    app.clear_history();
                    st.library.invalidate();
                }
                if row(ui, Some(Icon::Refresh), "Refresh", false, false).clicked() {
                    st.library.invalidate();
                }
            }
            _ => {
                if !playable.is_empty() {
                    if row(ui, Some(Icon::Play), "Play All", false, false).clicked() {
                        play(app, st, playable.clone(), 0, m.crumbs.last().map(|c| c.0.clone()));
                    }
                    if row(ui, Some(Icon::Queue), "Add to Queue", false, false).clicked() {
                        app.enqueue(playable.clone());
                    }
                }
                if row(ui, Some(Icon::Refresh), "Refresh", false, false).clicked() {
                    super::music::rescan(&mut st.library.music);
                    st.library.invalidate();
                }
            }
        }
    });
}

fn folder_page_rows(ui: &mut Ui, app: &mut App, st: &mut UiState, dir: PathBuf, music: bool) {
    if row(ui, Some(Icon::Play), "Play All", false, false).clicked() {
        play_folder(app, st, &dir, music, false);
    }
    if row(ui, Some(Icon::ShuffleRight), "Shuffle All", false, false).clicked() {
        play_folder(app, st, &dir, music, true);
    }
    if row(ui, Some(Icon::Queue), "Add to Queue", false, false).clicked() {
        let items = folder_items(app, &dir, music);
        app.enqueue(items);
    }
    if row(ui, Some(Icon::Subtitle), "Add Subtitles to Folder", false, false).clicked() {
        st.library.action_item = Some(MediaItem::file(&dir));
        st.dialogs.add_subtitles = true;
    }
    if row(ui, Some(Icon::Folder), "Show in Finder", false, false).clicked() {
        app.reveal_in_file_manager(&dir);
    }
    if row(ui, Some(Icon::Refresh), "Refresh", false, false).clicked() {
        st.library.invalidate();
    }
}

/// "Delete playlist?" prompt.
pub fn confirm_delete(ctx: &egui::Context, app: &mut App, st: &mut UiState) {
    let Some(id) = st.library.confirm_delete else { return };
    let Some(name) = app.library.playlist(id).map(|p| p.name.clone()) else {
        st.library.confirm_delete = None;
        return;
    };
    let p = pal(ctx);
    let mut close = false;
    let modal = egui::Modal::new(egui::Id::new("lib-delete-playlist"))
        .frame(widgets::panel_frame(&p).fill(p.bg_a4).inner_margin(20))
        .show(ctx, |ui| {
            ui.set_width(380.0);
            ui.label(egui::RichText::new("Delete playlist?").font(font::medium(16.0)).color(p.fg2));
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(format!("\"{name}\" will be removed. The files themselves are not deleted."))
                    .font(font::regular(14.0))
                    .color(p.fg_a3),
            );
            ui.add_space(18.0);
            ui.horizontal(|ui| {
                if widgets::button_with(ui, None, "Cancel", ButtonKind::Secondary, vec2(100.0, 32.0)).clicked() {
                    close = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::button_with(ui, None, "Delete", ButtonKind::Danger, vec2(100.0, 32.0)).clicked() {
                        app.delete_playlist(id);
                        if st.page == Page::Playlist(id) {
                            st.navigate(Page::Playlists);
                        }
                        st.library.invalidate();
                        close = true;
                    }
                });
            });
        });
    if close || modal.should_close() {
        st.library.confirm_delete = None;
    }
}
