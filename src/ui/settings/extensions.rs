//! The Extensions page: VLC Lua add-ons (playlist parsers, metadata fetchers,
//! service discovery, extensions) in the user's VLC Lua folders.
//!
//! libvlc loads every script it finds in these folders, so installing means
//! copying the file into the folder for its kind, and disabling means
//! renaming it to `*.disabled` so VLC skips it.

use std::path::{Path, PathBuf};

use egui::{Align2, CornerRadius, Rect, RichText, Sense, Stroke, StrokeKind, Ui, UiBuilder, pos2, vec2};

use super::kit::{self, Btn, Form};
use super::{Confirm, Cx};
use crate::app::App;
use crate::domain::settings::{Extension, Settings};
use crate::ui::icons::Icon;
use crate::ui::theme::font;
use crate::ui::widgets::{self, pal, text_clipped};

/// Sub-folder of the Lua folder and the name shown for each kind of add-on.
pub const KINDS: [(&str, &str); 7] = [
    ("playlist", "Playlist parser"),
    ("meta/art", "Art fetcher"),
    ("meta/fetcher", "Metadata fetcher"),
    ("meta/reader", "Metadata reader"),
    ("sd", "Service discovery"),
    ("extensions", "Extension"),
    ("intf", "Interface"),
];

const DISABLED: &str = ".disabled";

#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub name: String,
    pub description: String,
    pub kind: &'static str,
    pub path: PathBuf,
    pub enabled: bool,
}

#[derive(Default)]
pub struct ExtensionsState {
    pub search: String,
    pub selected: Option<PathBuf>,
    found: Option<Vec<Found>>,
}

impl ExtensionsState {
    /// Drops the cached folder listing so the next frame rescans.
    pub fn refresh(&mut self) {
        self.found = None;
    }

    fn list(&mut self) -> &[Found] {
        self.found.get_or_insert_with(|| lua_root().map(|r| scan(&r)).unwrap_or_default())
    }
}

/// VLC's per-user Lua folder.
pub fn lua_root() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if cfg!(target_os = "macos") {
        home.map(|h| h.join("Library/Application Support/org.videolan.vlc/lua"))
    } else if cfg!(windows) {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("vlc/lua"))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home.map(|h| h.join(".local/share")))
            .map(|d| d.join("vlc/lua"))
    }
}

fn is_script(name: &str) -> bool {
    let base = name.strip_suffix(DISABLED).unwrap_or(name);
    base.ends_with(".lua") || base.ends_with(".luac")
}

/// Lists the scripts in every kind folder under `root`.
pub fn scan(root: &Path) -> Vec<Found> {
    let mut out = Vec::new();
    for (dir, kind) in KINDS {
        let Ok(rd) = std::fs::read_dir(root.join(dir)) else { continue };
        let mut files: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.file_name().and_then(|n| n.to_str()).is_some_and(is_script))
            .collect();
        files.sort();
        for path in files {
            let file = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
            let enabled = !file.ends_with(DISABLED);
            let stem = file.strip_suffix(DISABLED).unwrap_or(&file);
            let stem = stem.rsplit_once('.').map(|(s, _)| s).unwrap_or(stem).to_string();
            let src = std::fs::read(&path).ok().map(|b| String::from_utf8_lossy(&b[..b.len().min(65536)]).into_owned());
            let field = |key: &str| src.as_deref().and_then(|s| lua_string_field(s, key));
            let name = field("title").unwrap_or_else(|| stem.replace(['_', '-'], " "));
            let description = field("description").or_else(|| field("shortdesc")).unwrap_or_default();
            out.push(Found { name, description, kind, path, enabled });
        }
    }
    out
}

/// Finds `key = "value"` (or single quotes) in a Lua source.
pub fn lua_string_field(src: &str, key: &str) -> Option<String> {
    let mut rest = src;
    while let Some(i) = rest.find(key) {
        let before_ok = rest[..i].chars().last().is_none_or(|c| !c.is_alphanumeric() && c != '_');
        let after = rest[i + key.len()..].trim_start();
        rest = &rest[i + key.len()..];
        if !before_ok {
            continue;
        }
        let Some(after) = after.strip_prefix('=') else { continue };
        let after = after.trim_start();
        let Some(q) = after.chars().next().filter(|c| *c == '"' || *c == '\'') else { continue };
        let body = &after[1..];
        if let Some(end) = body.find(q) {
            let v = body[..end].trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn to_settings(found: &[Found]) -> Vec<Extension> {
    found
        .iter()
        .map(|f| Extension {
            name: f.name.clone(),
            description: f.description.clone(),
            path: f.path.clone(),
            enabled: f.enabled,
        })
        .collect()
}

pub fn page(f: &mut Form, s: &mut Settings, cx: &mut Cx) {
    f.section("Add-ons");
    if f.drawing() && f.matches("add-ons extensions lua scripts plugins", None) {
        let list = cx.ss.extensions.list().to_vec();
        let installed = to_settings(&list);
        if s.extensions.installed != installed {
            s.extensions.installed = installed;
        }
    }
    f.block("add-ons extensions lua scripts plugins", |ui| {
        list_box(ui, cx);
        ui.add_space(6.0);
        toolbar(ui, cx);
        ui.add_space(11.0);
    });
    f.toggle("Load add-ons on startup", None, &mut s.extensions.load_on_startup);
    f.advanced(|f| {
        if f.action(
            "Open the add-ons folder",
            Some("Shows VLC's Lua folder, where add-ons are installed by kind."),
            "Open",
            Btn::Accent,
        ) && let Some(root) = lua_root()
        {
            cx.later(move |app, _| {
                let _ = std::fs::create_dir_all(&root);
                if let Err(e) = open::that(&root) {
                    app.toast_error(format!("Could not open {}: {e}", root.display()));
                }
            });
        }
    });
}

fn list_box(ui: &mut Ui, cx: &mut Cx) {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 194.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(6), p.bg_b2);
    ui.painter().rect_stroke(rect, CornerRadius::same(6), Stroke::new(2.0, p.bg_a4), StrokeKind::Inside);
    let st = &mut cx.ss.extensions;
    let query = st.search.trim().to_lowercase();
    let items: Vec<Found> = st
        .list()
        .iter()
        .filter(|f| {
            query.is_empty()
                || f.name.to_lowercase().contains(&query)
                || f.description.to_lowercase().contains(&query)
                || f.kind.to_lowercase().contains(&query)
        })
        .cloned()
        .collect();
    if items.is_empty() {
        let text = if query.is_empty() {
            "No add-ons installed. Use Add to install a .lua script, or Find more extensions to browse VLC's add-ons."
        } else {
            "No add-ons match the search."
        };
        let inner = rect.shrink(24.0);
        kit::place(ui, UiBuilder::new().max_rect(inner), |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(inner.height() / 2.0 - 30.0);
                widgets::icon(ui, Icon::Plugins, 28.0, p.fg_a4);
                ui.add_space(6.0);
                ui.label(RichText::new(text).font(font::regular(14.0)).color(p.fg_a3));
            });
        });
        return;
    }
    let mut toggled: Option<Found> = None;
    kit::place(ui, UiBuilder::new().max_rect(rect.shrink(6.0)), |ui| {
        egui::ScrollArea::vertical().id_salt("ext-list").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
            for it in &items {
                let rw = ui.available_width();
                let (r, resp) = ui.allocate_exact_size(vec2(rw, 44.0), Sense::click());
                let selected = st.selected.as_ref() == Some(&it.path);
                if selected {
                    ui.painter().rect_filled(r, CornerRadius::same(4), p.bg_a1);
                } else if resp.hovered() {
                    ui.painter().rect_filled(r, CornerRadius::same(4), p.hover);
                }
                if resp.clicked() {
                    st.selected = Some(it.path.clone());
                }
                let name_col = if it.enabled { p.fg1 } else { p.fg_a3 };
                let nr = text_clipped(
                    ui,
                    pos2(r.min.x + 12.0, r.min.y + 6.0),
                    Align2::LEFT_TOP,
                    &it.name,
                    font::regular(14.0),
                    name_col,
                    rw - 260.0,
                );
                ui.painter().text(
                    pos2(nr.max.x + 10.0, r.min.y + 7.0),
                    Align2::LEFT_TOP,
                    it.kind,
                    font::regular(12.0),
                    p.accent_dim,
                );
                let sub = if it.description.is_empty() {
                    it.path.to_string_lossy().into_owned()
                } else {
                    it.description.clone()
                };
                text_clipped(
                    ui,
                    pos2(r.min.x + 12.0, r.max.y - 6.0),
                    Align2::LEFT_BOTTOM,
                    &sub,
                    font::regular(12.0),
                    p.fg_a3,
                    rw - 110.0,
                );
                let tr = Rect::from_min_size(pos2(r.max.x - 78.0, r.center().y - 16.0), vec2(68.0, 32.0));
                let mut on = it.enabled;
                let changed =
                    kit::place(ui, UiBuilder::new().max_rect(tr), |ui| widgets::toggle(ui, &mut on).changed());
                if changed {
                    toggled = Some(it.clone());
                }
            }
        });
    });
    if let Some(it) = toggled {
        let target = if it.enabled {
            PathBuf::from(format!("{}{DISABLED}", it.path.to_string_lossy()))
        } else {
            let s = it.path.to_string_lossy();
            PathBuf::from(s.strip_suffix(DISABLED).unwrap_or(&s).to_string())
        };
        match std::fs::rename(&it.path, &target) {
            Ok(()) => {
                if st.selected.as_ref() == Some(&it.path) {
                    st.selected = Some(target);
                }
                st.refresh();
            }
            Err(e) => cx.toast(format!("Could not change {}: {e}", it.name)),
        }
    }
}

fn toolbar(ui: &mut Ui, cx: &mut Cx) {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 36.0), Sense::hover());
    let search = Rect::from_min_size(rect.min, vec2(210.0, 36.0));
    kit::place(ui, UiBuilder::new().max_rect(search), |ui| {
        kit::text_field(ui, &mut cx.ss.extensions.search, "Search", 210.0, false);
    });
    let buttons = Rect::from_min_max(pos2(rect.max.x - 405.0, rect.min.y + 2.0), rect.max);
    kit::place(ui, UiBuilder::new().max_rect(buttons).layout(egui::Layout::right_to_left(egui::Align::Min)), |ui| {
        ui.spacing_mut().item_spacing = vec2(10.0, 0.0);
        let has_sel = cx.ss.extensions.selected.as_ref().is_some_and(|s| s.exists());
        let remove = kit::action_button(ui, "Remove", Btn::Normal, 100.0);
        if remove.clicked() {
            match cx.ss.extensions.selected.clone().filter(|_| has_sel) {
                Some(path) => cx.ss.confirm = Some(Confirm::RemoveExtension(path)),
                None => cx.toast("Select an add-on in the list first."),
            }
        }
        let add = kit::action_button(ui, "Add", Btn::Normal, 100.0);
        egui::Popup::menu(&add).id(ui.id().with("ext-add")).frame(widgets::menu_frame(&p)).show(|ui| {
            ui.set_min_width(220.0);
            ui.label(RichText::new("Install as").font(font::regular(12.0)).color(p.fg_a3));
            for (dir, label) in KINDS {
                if widgets::menu_row(ui, None, label, "", false).clicked() {
                    install(cx, dir, label);
                }
            }
        });
        if kit::action_button(ui, "Find more extensions", Btn::Normal, 185.0).clicked() {
            cx.later(|app, _| {
                if let Err(e) = open::that("https://addons.videolan.org") {
                    app.toast_error(format!("Could not open the browser: {e}"));
                }
            });
        }
    });
}

/// Picks a script and copies it into the folder for `dir`.
fn install(cx: &mut Cx, dir: &'static str, label: &'static str) {
    let Some(root) = lua_root() else {
        cx.toast("Could not find VLC's add-ons folder.");
        return;
    };
    let Some(src) = rfd::FileDialog::new()
        .set_title(format!("Add {label}"))
        .add_filter("Lua scripts", &["lua", "luac"])
        .pick_file()
    else {
        return;
    };
    let dest_dir = root.join(dir);
    let Some(name) = src.file_name() else { return };
    let dest = dest_dir.join(name);
    match std::fs::create_dir_all(&dest_dir).and_then(|_| std::fs::copy(&src, &dest)) {
        Ok(_) => {
            cx.ss.extensions.selected = Some(dest);
            cx.ss.extensions.refresh();
            cx.toast(format!("Installed {} as a {}", name.to_string_lossy(), label.to_lowercase()));
        }
        Err(e) => cx.toast(format!("Could not install {}: {e}", name.to_string_lossy())),
    }
}

/// Deletes an add-on after the user confirmed.
pub fn remove(app: &mut App, st: &mut ExtensionsState, path: &Path) {
    match std::fs::remove_file(path) {
        Ok(()) => {
            st.selected = None;
            st.refresh();
            let found = st.list().to_vec();
            app.update_settings(|s| s.extensions.installed = to_settings(&found));
            app.toast("Add-on removed");
        }
        Err(e) => app.toast_error(format!("Could not remove {}: {e}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_lua_descriptor_fields() {
        let src = r#"function descriptor()
            return { title = "YouTube Parser" ; version = "1.0",
                     description = 'Plays YouTube pages' }
        end
        local subtitle = "x""#;
        assert_eq!(lua_string_field(src, "title").as_deref(), Some("YouTube Parser"));
        assert_eq!(lua_string_field(src, "description").as_deref(), Some("Plays YouTube pages"));
        assert_eq!(lua_string_field(src, "shortdesc"), None);
    }

    #[test]
    fn scans_kind_folders() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("playlist")).unwrap();
        std::fs::create_dir_all(root.join("meta/art")).unwrap();
        std::fs::write(root.join("playlist/vimeo.lua"), "-- nothing").unwrap();
        std::fs::write(root.join("meta/art/covers.luac.disabled"), [0u8, 1, 2]).unwrap();
        std::fs::write(root.join("playlist/readme.txt"), "x").unwrap();
        let found = scan(root);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].kind, "Playlist parser");
        assert_eq!(found[0].name, "vimeo");
        assert!(found[0].enabled);
        assert_eq!(found[1].kind, "Art fetcher");
        assert!(!found[1].enabled);
    }
}
