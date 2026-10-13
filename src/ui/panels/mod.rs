//! Player popovers (queue, tracks, quick settings and effects), the player
//! context menu and the customise-toolbar dialog.
//!
//! Every popover follows the "Controls" component frame (`491:1780`): 400pt
//! wide, right-aligned 20pt in from the bar's right end and 20pt above it,
//! a 50pt tab strip on top.

#[allow(dead_code)] // parts of the kit are used by later panels
mod kit;
mod queue;
mod tracks;

use egui::{Pos2, Rect, Response, Ui, pos2, vec2};

use crate::app::App;
use crate::ui::state::{PlayerPanel, UiState};
use crate::ui::widgets::pal;

#[derive(Default)]
pub struct PanelsState {
    /// Opens the player menu at this point (from the toolbar's More button).
    pub context_menu_at: Option<Pos2>,
    pub customize_open: bool,
    /// Quick settings > Playback > "Show Subtitle Timeline on Seekbar".
    pub subtitle_timeline: bool,
    /// Quick settings > Playback > "Show Statistics": overlay on the video.
    pub show_stats: bool,
    /// The video area, recorded each frame for overlays.
    pub video_rect: Option<Rect>,
    queue: queue::QueueUi,
    tracks: tracks::TracksUi,
}

/// Design heights of the popovers.
fn design_height(panel: PlayerPanel) -> f32 {
    match panel {
        PlayerPanel::Tracks(_) => 524.0,
        PlayerPanel::Queue(_) | PlayerPanel::QuickSettings(_) => 946.0,
    }
}

/// Draws the open popover anchored above the right end of the control bar.
pub fn show_popovers(ui: &mut Ui, bar: Rect, app: &mut App, st: &mut UiState) {
    let Some(panel) = st.panel else { return };
    let p = pal(ui.ctx());
    let top = ui.max_rect().min.y + 12.0;
    let bottom = bar.min.y - 20.0;
    let h = design_height(panel).min(bottom - top).max(160.0);
    let rect = Rect::from_min_size(pos2(bar.max.x - 20.0 - kit::WIDTH, bottom - h), vec2(kit::WIDTH, h));
    egui::Area::new(ui.id().with("popover")).order(egui::Order::Middle).fixed_pos(rect.min).show(ui.ctx(), |ui| {
        kit::popover_frame(&p).show(ui, |ui| {
            ui.set_min_size(rect.size());
            ui.set_max_size(rect.size());
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            match panel {
                PlayerPanel::Queue(tab) => queue::show(ui, app, st, tab),
                PlayerPanel::Tracks(tab) => tracks::show(ui, app, st, tab),
                PlayerPanel::QuickSettings(tab) => {
                    let _ = tab;
                }
            }
        });
    });
}

/// Right-click menu on the video.
pub fn player_context_menu(resp: &Response, _app: &mut App, st: &mut UiState) {
    st.panels.video_rect = Some(resp.rect);
}
