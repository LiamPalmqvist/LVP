//! Modal dialogs: VLC's own prompts, open network/disc/capture, convert,
//! media information, jump to time, onboarding and about.

mod about;
mod capture;
mod convert;
mod disc;
pub mod frame;
mod jump;
mod media_info;
mod network;
mod vlc;

use crate::app::App;
use crate::domain::media::MediaItem;
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
    network: network::NetworkState,
    disc: disc::DiscState,
    capture_state: capture::CaptureState,
    info: media_info::InfoState,
    convert_state: convert::ConvertState,
    /// Items handed to Convert / Save by another dialog.
    convert_source: Option<Vec<MediaItem>>,
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
            "convert_profile" => {
                self.convert_state.open_editor = true;
                &mut self.convert
            }
            "media_info_codec" | "media_info_stats" => {
                self.info.set_tab(if name.ends_with("codec") { 1 } else { 2 });
                &mut self.media_info
            }
            _ => return false,
        };
        *flag = true;
        true
    }

    /// Opens Convert / Save with these items as the source.
    pub fn convert_items(&mut self, items: Vec<MediaItem>) {
        self.convert_source = Some(items);
        self.convert = true;
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
    if st.dialogs.open_network {
        network::show(ctx, app, st);
    }
    if st.dialogs.open_disc {
        disc::show(ctx, app, st);
    }
    if st.dialogs.capture {
        capture::show(ctx, app, st);
    }
    if st.dialogs.convert {
        convert::show(ctx, app, &mut st.dialogs);
    }
    let d = &mut st.dialogs;
    if d.media_info {
        media_info::show(ctx, app, &mut d.media_info, &mut d.info);
    }
    if d.jump_to_time {
        jump::show(ctx, app, &mut d.jump_to_time, &mut d.jump);
    }
    if d.about {
        about::show(ctx, app, &mut d.about);
    }
    // VLC's prompts go last so they sit above anything the user opened.
    vlc::show(ctx, app, &mut d.login);
}
