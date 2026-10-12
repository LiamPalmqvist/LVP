//! The music view (cover art and visualiser), the waveform seek bar data,
//! and the floating mini player.

use egui::{Rect, TextureId, Ui};

use crate::app::App;
use crate::ui::state::UiState;
use crate::ui::widgets::pal;

#[derive(Default)]
pub struct MusicState {}

/// Shown in the player when the current item has no picture.
pub fn show_player(ui: &mut Ui, rect: Rect, app: &mut App, _st: &mut UiState) {
    let p = pal(ui.ctx());
    let art = app.queue.current_item().cloned().and_then(|i| app.thumbnail(&i));
    let side = rect.height().min(rect.width()) * 0.5;
    let r = Rect::from_center_size(rect.center(), egui::vec2(side, side));
    if !art.is_some_and(|a| crate::ui::images::paint_cover(ui, r, &a, 12)) {
        ui.painter().rect_filled(r, 12, p.bg_a3);
    }
}

/// The floating player in the library's bottom-right corner.
pub fn miniplayer(
    _ui: &mut Ui,
    _content: Rect,
    _app: &mut App,
    _st: &mut UiState,
    _frame: Option<(TextureId, [u32; 2])>,
) {
}
