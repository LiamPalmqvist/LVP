//! Player popovers (queue, tracks, quick settings and effects), the player
//! context menu and the customise-toolbar dialog.

use egui::{Pos2, Rect, Response, Ui};

use crate::app::App;
use crate::ui::state::{PlayerPanel, UiState};
use crate::ui::widgets::{self, pal};

#[derive(Default)]
pub struct PanelsState {
    /// Opens the player menu at this point (from the toolbar's More button).
    pub context_menu_at: Option<Pos2>,
    pub customize_open: bool,
}

/// Draws the open popover anchored above the right end of the control bar.
pub fn show_popovers(ui: &mut Ui, bar: Rect, _app: &mut App, st: &mut UiState) {
    let Some(panel) = st.panel else { return };
    let p = pal(ui.ctx());
    egui::Area::new(ui.id().with("popover"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(bar.max.x - 380.0, bar.min.y - 520.0))
        .show(ui.ctx(), |ui| {
            widgets::panel_frame(&p).inner_margin(16).show(ui, |ui| {
                ui.set_min_size(egui::vec2(344.0, 480.0));
                ui.label(format!("{panel:?}"));
            });
        });
    let _ = PlayerPanel::Queue(0);
}

/// Right-click menu on the video.
pub fn player_context_menu(_resp: &Response, _app: &mut App, _st: &mut UiState) {}
