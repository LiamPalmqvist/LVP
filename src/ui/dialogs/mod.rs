//! Modal dialogs: VLC's own prompts, open network/disc/capture, convert,
//! media information, jump to time, onboarding and about.

use crate::app::App;
use crate::ui::state::UiState;

#[derive(Default)]
pub struct DialogsState {
    pub about: bool,
    pub open_network: bool,
    pub open_disc: bool,
    pub capture: bool,
    pub convert: bool,
    pub media_info: bool,
    pub jump_to_time: bool,
    pub shortcuts: bool,
    pub onboarding: bool,
    pub add_subtitles: bool,
    pub new_playlist: bool,
}

impl DialogsState {
    /// Opens a dialog by its field name; used by the snapshot tool.
    pub fn open_by_name(&mut self, name: &str) -> bool {
        let flag = match name {
            "about" => &mut self.about,
            "open_network" => &mut self.open_network,
            "open_disc" => &mut self.open_disc,
            "capture" => &mut self.capture,
            "convert" => &mut self.convert,
            "media_info" => &mut self.media_info,
            "jump_to_time" => &mut self.jump_to_time,
            "shortcuts" => &mut self.shortcuts,
            "onboarding" => &mut self.onboarding,
            "add_subtitles" => &mut self.add_subtitles,
            "new_playlist" => &mut self.new_playlist,
            _ => return false,
        };
        *flag = true;
        true
    }
}

pub fn show(ctx: &egui::Context, app: &mut App, st: &mut UiState) {
    if st.dialogs.about {
        let mut open = true;
        egui::Window::new("About").open(&mut open).show(ctx, |ui| {
            ui.label(format!("NotVLC {} on libvlc {}", env!("CARGO_PKG_VERSION"), app.engine.version()));
        });
        st.dialogs.about = open;
    }
}
