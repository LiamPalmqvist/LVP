//! The Extensions page: VLC Lua add-ons in the user's VLC Lua folders.

use std::path::{Path, PathBuf};

use crate::app::App;
use crate::domain::settings::Settings;

use super::Cx;
use super::kit::Form;

#[derive(Default)]
pub struct ExtensionsState {}

pub fn page(f: &mut Form, s: &mut Settings, _cx: &mut Cx) {
    f.section("Add-ons");
    f.toggle("Load add-ons on startup", None, &mut s.extensions.load_on_startup);
}

pub fn remove(app: &mut App, _st: &mut ExtensionsState, path: &Path) {
    let _: PathBuf = path.to_path_buf();
    if let Err(e) = std::fs::remove_file(path) {
        app.toast_error(format!("Could not remove {}: {e}", path.display()));
    }
}
