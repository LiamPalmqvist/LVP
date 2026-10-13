//! The Queue / Bookmarks / Chapters popover.

use egui::{Align2, CornerRadius, Rect, Sense, Stroke, Ui, UiBuilder, Vec2, pos2, vec2};

use super::kit;
use crate::app::App;
use crate::domain::media::MediaItem;
use crate::domain::settings::RepeatMode;
use crate::domain::time::{format_hms, format_short};
use crate::ui::icons::Icon;
use crate::ui::state::{PlayerPanel, UiState};
use crate::ui::theme::font;
use crate::ui::widgets::{self, pal};

#[derive(Default)]
pub struct QueueUi {
    /// Queue row being dragged.
    drag_from: Option<usize>,
    /// Bookmark row last clicked (the target of "-").
    bm_selected: Option<usize>,
    /// Bookmark being renamed and its draft name.
    bm_edit: Option<(usize, String)>,
}

pub fn show(ui: &mut Ui, app: &mut App, st: &mut UiState, tab: usize) {
    let tabs = [(Icon::Queue2tone, "Queue"), (Icon::Bookmarks, "Bookmarks"), (Icon::Chapters, "Chapters")];
    if let Some(t) = kit::tab_strip(ui, tab, &tabs, None) {
        st.panel = Some(PlayerPanel::Queue(t));
    }
    match tab {
        0 => queue_tab(ui, app, st),
        1 => bookmarks_tab(ui, app, st),
        _ => chapters_tab(ui, app),
    }
}

/// The caption band under the tabs: text on the left, actions on the right.
fn header(ui: &mut Ui, h: f32, text: &str, actions: impl FnOnce(&mut Ui)) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    kit::caption(ui, pos2(rect.min.x + 28.0, rect.center().y), text);
    let right = Rect::from_min_max(pos2(rect.min.x + 160.0, rect.min.y), pos2(rect.max.x - 20.0, rect.max.y));
    let mut child =
        ui.new_child(UiBuilder::new().max_rect(right).layout(egui::Layout::right_to_left(egui::Align::Center)));
    child.spacing_mut().item_spacing.x = 12.0;
    actions(&mut child);
}

fn list_area(ui: &mut Ui, top: f32, add: impl FnOnce(&mut Ui)) {
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        egui::Frame::new().inner_margin(egui::Margin { left: 20, right: 20, top: top as i8, bottom: 20 }).show(
            ui,
            |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing = vec2(0.0, 12.0);
                add(ui);
            },
        );
    });
}

fn duration(app: &App, item: &MediaItem) -> Option<i64> {
    item.meta.duration_ms.or_else(|| app.library.meta(&item.source).and_then(|m| m.duration_ms)).filter(|d| *d > 0)
}

fn hours_minutes(ms: i64) -> String {
    let m = ms / 60_000;
    if m >= 60 { format!("{}h {}m", m / 60, m % 60) } else { format!("{m}m") }
}

/// The design's bookmark time: `HH:MM:SS:mmm`.
fn format_hmsm(ms: i64) -> String {
    format!("{}:{:03}", format_hms(ms), ms.max(0) % 1000)
}

enum RowAction {
    Play(usize),
    PlayNext(usize),
    Remove(usize),
}

fn queue_tab(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    let p = pal(ui.ctx());
    let n = app.queue.len();
    let total: i64 = app.queue.items.iter().filter_map(|i| duration(app, i)).sum();
    let mut text = format!("{n} item{}", if n == 1 { "" } else { "s" });
    if total > 0 {
        text.push_str(&format!(" - {}", hours_minutes(total)));
    }
    header(ui, 68.0, &text, |ui| {
        let add = kit::icon_action(ui, Icon::Add, "Add to the queue", false);
        add_menu(ui, &add, app);
        if kit::icon_action(ui, Icon::Remove, "Clear the queue", false).clicked() && !app.queue.is_empty() {
            app.stop();
            app.queue.clear();
            app.save_queue();
        }
        let shuffle = app.queue.shuffle;
        let icon = if shuffle { Icon::ShuffleRight2tone } else { Icon::ShuffleRight };
        if kit::icon_action(ui, icon, "Shuffle", shuffle).clicked() {
            app.set_shuffle(!shuffle);
        }
        let repeat = app.queue.repeat;
        let icon = if repeat != RepeatMode::Off { Icon::RepeatRight2tone } else { Icon::RepeatRight };
        let r = kit::icon_action(ui, icon, repeat.label(), repeat != RepeatMode::Off);
        if repeat == RepeatMode::One {
            ui.painter().text(
                r.rect.right_top() + vec2(1.0, 1.0),
                Align2::CENTER_CENTER,
                "1",
                font::bold(9.0),
                p.accent,
            );
        }
        if r.clicked() {
            app.set_repeat(repeat.cycle());
        }
        let sort = kit::icon_action(ui, Icon::Sort, "Sort the queue", false);
        sort_menu(ui, &sort, app);
    });
    if n == 0 {
        widgets::empty_state(ui, Icon::Queue, "The queue is empty", "Open files or add them from the library.");
        return;
    }
    let mut action = None;
    let q = &mut st.panels.queue;
    let mut rows: Vec<Rect> = Vec::with_capacity(n);
    list_area(ui, 0.0, |ui| {
        let w = ui.available_width();
        for i in 0..n {
            let item = app.queue.items[i].clone();
            if duration(app, &item).is_none() {
                app.request_meta(&item);
            }
            let (rect, resp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click_and_drag());
            rows.push(rect);
            let current = app.queue.current == Some(i);
            let dragging = q.drag_from == Some(i);
            if resp.hovered() || dragging {
                ui.painter().rect_filled(rect.expand2(vec2(6.0, 4.0)), CornerRadius::same(6), p.hover);
            }
            let fg = if current { p.accent } else { p.fg_a1 };
            let handle = Rect::from_min_size(rect.min, vec2(32.0, 32.0));
            widgets::paint_icon(
                ui,
                Rect::from_center_size(handle.center(), Vec2::splat(24.0)),
                Icon::Move,
                if resp.hovered() { p.fg2 } else { p.fg_a2 },
            );
            let dur = duration(app, &item).map(format_short).unwrap_or_default();
            let dur_w = if dur.is_empty() { 0.0 } else { 50.0 };
            widgets::text_clipped(
                ui,
                pos2(rect.min.x + 42.0, rect.center().y),
                Align2::LEFT_CENTER,
                &item.title(),
                font::regular(14.0),
                fg,
                w - 42.0 - dur_w - 10.0,
            );
            if !dur.is_empty() {
                ui.painter().text(
                    pos2(rect.max.x - 4.0, rect.center().y),
                    Align2::RIGHT_CENTER,
                    dur,
                    font::regular(14.0),
                    if current { p.accent } else { p.fg_a1 },
                );
            }
            if resp.drag_started() {
                q.drag_from = Some(i);
            }
            if resp.clicked() {
                action = Some(RowAction::Play(i));
            }
            egui::Popup::context_menu(&resp).frame(widgets::menu_frame(&p)).show(|ui| {
                ui.set_min_width(220.0);
                if widgets::menu_row(ui, Some(Icon::Play), "Play", "", false).clicked() {
                    action = Some(RowAction::Play(i));
                }
                if widgets::menu_row(ui, Some(Icon::Next), "Play Next", "", false).clicked() {
                    action = Some(RowAction::PlayNext(i));
                }
                if widgets::menu_row(ui, Some(Icon::Remove), "Remove from Queue", "", false).clicked() {
                    action = Some(RowAction::Remove(i));
                }
            });
        }
    });
    // Drag to reorder: a line marks where the row will land.
    if let Some(from) = q.drag_from {
        let pointer = ui.ctx().pointer_interact_pos().map(|p| p.y);
        let slot = pointer.map(|y| rows.iter().filter(|r| r.center().y < y).count()).unwrap_or(from);
        if let Some(r) = rows.get(slot.min(rows.len().saturating_sub(1))) {
            let y = if slot >= rows.len() { r.max.y + 6.0 } else { r.min.y - 6.0 };
            ui.painter().line_segment([pos2(r.min.x, y), pos2(r.max.x, y)], Stroke::new(2.0, p.accent));
        }
        if !ui.input(|i| i.pointer.any_down()) {
            q.drag_from = None;
            let to = if slot > from { slot - 1 } else { slot };
            if to != from {
                app.queue.move_item(from, to.min(n - 1));
                app.save_queue();
            }
        }
    }
    match action {
        Some(RowAction::Play(i)) => app.play_index(i),
        Some(RowAction::PlayNext(i)) => {
            if let Some(c) = app.queue.current {
                let to = if i > c { c + 1 } else { c };
                app.queue.move_item(i, to);
                app.save_queue();
            }
        }
        Some(RowAction::Remove(i)) => {
            if app.queue.current == Some(i) {
                app.stop();
            }
            app.queue.remove(i);
            app.save_queue();
        }
        None => {}
    }
}

fn add_menu(ui: &Ui, resp: &egui::Response, app: &mut App) {
    let p = pal(ui.ctx());
    egui::Popup::menu(resp).frame(widgets::menu_frame(&p)).show(|ui| {
        ui.set_min_width(240.0);
        if widgets::menu_row(ui, Some(Icon::File), "Add Files...", "", false).clicked()
            && let Some(files) = rfd::FileDialog::new().set_title("Add to Queue").pick_files()
        {
            let items = app.expand_paths(&files);
            app.enqueue(items);
        }
        if widgets::menu_row(ui, Some(Icon::Folder), "Add Folder...", "", false).clicked()
            && let Some(dir) = rfd::FileDialog::new().set_title("Add Folder to Queue").pick_folder()
        {
            let items = app.expand_paths(&[dir]);
            app.enqueue(items);
        }
        if widgets::menu_row(ui, Some(Icon::Playlist), "Save as Playlist", "", false).clicked() {
            save_as_playlist(app);
        }
    });
}

/// Saves the queue as a new playlist named after its origin.
pub fn save_as_playlist(app: &mut App) {
    if app.queue.is_empty() {
        app.toast("The queue is empty");
        return;
    }
    let base = app.queue.origin.clone().unwrap_or_else(|| "Queue".into());
    let taken = |n: &str| app.library.playlists().iter().any(|p| p.name == n);
    let mut name = base.clone();
    let mut k = 2;
    while taken(&name) {
        name = format!("{base} {k}");
        k += 1;
    }
    let items = app.queue.items.clone();
    app.library.create_playlist(&name, items);
    app.toast(format!("Saved the queue as \"{name}\""));
}

fn sort_menu(ui: &Ui, resp: &egui::Response, app: &mut App) {
    let p = pal(ui.ctx());
    egui::Popup::menu(resp).frame(widgets::menu_frame(&p)).show(|ui| {
        ui.set_min_width(220.0);
        if widgets::menu_row(ui, None, "Title", "", false).clicked() {
            app.queue.sort_by_key(|i| i.title().to_lowercase());
            app.save_queue();
        }
        if widgets::menu_row(ui, None, "Duration", "", false).clicked() {
            let keys: Vec<(crate::domain::media::MediaSource, i64)> =
                app.queue.items.iter().map(|i| (i.source.clone(), duration(app, i).unwrap_or(0))).collect();
            app.queue.sort_by_key(|i| keys.iter().find(|k| k.0 == i.source).map(|k| k.1).unwrap_or(0));
            app.save_queue();
        }
        if widgets::menu_row(ui, None, "Reverse Order", "", false).clicked() {
            let n = app.queue.len();
            for i in 0..n {
                app.queue.move_item(n - 1, i);
            }
            app.save_queue();
        }
    });
}

fn bookmarks_tab(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    let p = pal(ui.ctx());
    let marks = app.bookmarks();
    let q = &mut st.panels.queue;
    let n = marks.len();
    let text = format!("{n} Bookmark{}", if n == 1 { "" } else { "s" });
    let has_media = app.current_source().is_some();
    header(ui, 64.0, &text, |ui| {
        if kit::icon_action(ui, Icon::Add, "Bookmark the current time", false).clicked() && has_media {
            app.add_bookmark(None);
        }
        if kit::icon_action(ui, Icon::Remove, "Remove the selected bookmark", false).clicked() && n > 0 {
            let i = q.bm_selected.filter(|i| *i < n).unwrap_or(n - 1);
            app.remove_bookmark(i);
            q.bm_selected = None;
        }
    });
    if n == 0 {
        let body = if has_media { "Press + to bookmark the current time." } else { "Play something to add bookmarks." };
        widgets::empty_state(ui, Icon::Bookmarks, "No bookmarks yet", body);
        return;
    }
    enum Act {
        Seek(usize),
        Rename(usize),
        Remove(usize),
        Commit(usize, String),
    }
    let mut act = None;
    list_area(ui, 0.0, |ui| {
        let w = ui.available_width();
        for (i, b) in marks.iter().enumerate() {
            if let Some((ei, draft)) = q.bm_edit.as_mut()
                && *ei == i
            {
                let r = kit::value_box_sized(ui, ("bm-edit", i), draft, vec2(w, 32.0));
                let te_id = ui.make_persistent_id(("bm-edit", i)).with("te");
                if !ui.memory(|m| m.has_focus(te_id)) && r.is_none() {
                    ui.memory_mut(|m| m.request_focus(te_id));
                }
                if let Some(kit::BoxEvent::Commit(name)) = r {
                    act = Some(Act::Commit(i, name));
                } else if ui.input(|inp| inp.key_pressed(egui::Key::Escape)) {
                    act = Some(Act::Commit(i, b.name.clone()));
                }
                continue;
            }
            let (rect, resp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click());
            let sel = q.bm_selected == Some(i);
            if resp.hovered() || sel {
                ui.painter().rect_filled(
                    rect.expand2(vec2(6.0, 4.0)),
                    CornerRadius::same(6),
                    if resp.hovered() { p.hover } else { p.bg_a5 },
                );
            }
            widgets::text_clipped(
                ui,
                pos2(rect.min.x, rect.center().y),
                Align2::LEFT_CENTER,
                &b.name,
                font::regular(14.0),
                if sel { p.fg2 } else { p.fg_a1 },
                w - 110.0,
            );
            ui.painter().text(
                pos2(rect.max.x - 4.0, rect.center().y),
                Align2::RIGHT_CENTER,
                format_hmsm(b.time_ms),
                font::regular(14.0),
                p.fg_a3,
            );
            let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            if resp.double_clicked() {
                act = Some(Act::Rename(i));
            } else if resp.clicked() {
                act = Some(Act::Seek(i));
            }
            egui::Popup::context_menu(&resp).frame(widgets::menu_frame(&p)).show(|ui| {
                ui.set_min_width(200.0);
                if widgets::menu_row(ui, Some(Icon::Play), "Go to Bookmark", "", false).clicked() {
                    act = Some(Act::Seek(i));
                }
                if widgets::menu_row(ui, Some(Icon::Edit), "Rename", "", false).clicked() {
                    act = Some(Act::Rename(i));
                }
                if widgets::menu_row(ui, Some(Icon::Remove), "Remove", "", false).clicked() {
                    act = Some(Act::Remove(i));
                }
            });
        }
    });
    match act {
        Some(Act::Seek(i)) => {
            q.bm_selected = Some(i);
            app.seek_ms(marks[i].time_ms);
        }
        Some(Act::Rename(i)) => q.bm_edit = Some((i, marks[i].name.clone())),
        Some(Act::Remove(i)) => {
            app.remove_bookmark(i);
            q.bm_selected = None;
        }
        Some(Act::Commit(i, name)) => {
            q.bm_edit = None;
            let name = name.trim().to_string();
            if !name.is_empty() && name != marks[i].name {
                app.rename_bookmark(i, name);
            }
        }
        None => {}
    }
}

fn chapters_tab(ui: &mut Ui, app: &mut App) {
    let p = pal(ui.ctx());
    let chapters = app.status.chapters.clone();
    let titles = app.status.titles.clone();
    if chapters.is_empty() && titles.len() <= 1 {
        widgets::empty_state(ui, Icon::Chapters, "No chapters", "This media has no chapter marks.");
        return;
    }
    let mut set_chapter = None;
    let mut set_title = None;
    list_area(ui, 20.0, |ui| {
        let w = ui.available_width();
        let numbered = |ui: &mut Ui, n: usize, name: &str, time: &str, current: bool| -> bool {
            let (rect, resp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click());
            if resp.hovered() || current {
                ui.painter().rect_filled(
                    rect.expand2(vec2(6.0, 4.0)),
                    CornerRadius::same(6),
                    if resp.hovered() { p.hover } else { p.bg_a5 },
                );
            }
            ui.painter().text(
                pos2(rect.min.x + 4.0, rect.center().y),
                Align2::LEFT_CENTER,
                n.to_string(),
                font::medium(14.0),
                p.accent,
            );
            widgets::text_clipped(
                ui,
                pos2(rect.min.x + 48.0, rect.center().y),
                Align2::LEFT_CENTER,
                name,
                font::regular(14.0),
                if current { p.accent } else { p.fg_a1 },
                w - 48.0 - 80.0,
            );
            ui.painter().text(
                pos2(rect.max.x - 4.0, rect.center().y),
                Align2::RIGHT_CENTER,
                time,
                font::regular(14.0),
                p.fg_a3,
            );
            resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
        };
        if titles.len() > 1 {
            for (i, t) in titles.iter().enumerate() {
                let name = if t.name.is_empty() {
                    if t.is_menu { "Menu".to_string() } else { format!("Title {}", i + 1) }
                } else {
                    t.name.clone()
                };
                let time = if t.duration_ms > 0 { format_hms(t.duration_ms) } else { String::new() };
                if numbered(ui, i + 1, &name, &time, app.status.title == i as i32) {
                    set_title = Some(i as i32);
                }
            }
            if !chapters.is_empty() {
                kit::rule(ui);
            }
        }
        for (i, c) in chapters.iter().enumerate() {
            let name = if c.name.is_empty() { format!("Chapter {}", i + 1) } else { c.name.clone() };
            if numbered(ui, i + 1, &name, &format_hms(c.start_ms), app.status.chapter == i as i32) {
                set_chapter = Some(i as i32);
            }
        }
    });
    if let Some(t) = set_title {
        app.set_title(t);
    }
    if let Some(c) = set_chapter {
        app.set_chapter(c);
    }
}
