//! The Keyboard Shortcuts dialog.

use crate::app::App;
use crate::ui::state::UiState;

#[derive(Default)]
pub struct ShortcutsState {}

pub fn show(_ctx: &egui::Context, _app: &mut App, st: &mut UiState) {
    if !st.dialogs.shortcuts {
        st.settings.shortcuts = Default::default();
    }
}
