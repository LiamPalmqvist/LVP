//! Modal dialogs: VLC's own prompts, open network/disc/capture, convert,
//! media information, jump to time, onboarding and about.

mod about;
pub mod frame;
mod jump;
mod vlc;

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
    login: vlc::LoginState,
    jump: jump::JumpState,
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

    /// True when one of this module's dialogs is showing.
    pub fn any_open(&self) -> bool {
        self.about
            || self.open_network
            || self.open_disc
            || self.capture
            || self.convert
            || self.media_info
            || self.jump_to_time
            || self.onboarding
            || self.add_subtitles
    }
}

pub fn show(ctx: &egui::Context, app: &mut App, st: &mut UiState) {
    let d = &mut st.dialogs;
    if d.jump_to_time {
        jump::show(ctx, app, &mut d.jump_to_time, &mut d.jump);
    }
    if d.about {
        about::show(ctx, app, &mut d.about);
    }
    // VLC's prompts go last so they sit above anything the user opened.
    vlc::show(ctx, app, &mut d.login);
}
