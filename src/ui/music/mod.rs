//! The music view (cover art and visualiser), the waveform seek bar, and the
//! floating mini player.

mod art;
mod mini;
mod player;
mod visualiser;
mod waveform;

use crate::domain::media::MediaSource;

pub use mini::miniplayer;
pub use player::show_player;
pub use waveform::draw_waveform_seek;

#[derive(Default)]
pub struct MusicState {
    /// The mini player's queue list is open above the preview.
    pub mini_queue: bool,
    /// Scroll the mini player's queue to the current item on its next frame.
    pub mini_reveal: bool,
    /// Row selected in the mini player's queue, for its remove button.
    pub mini_selected: Option<usize>,
    /// Queue row being dragged by its handle.
    pub mini_drag: Option<usize>,
    /// The next press of the queue's sort button sorts Z to A.
    pub mini_sort_desc: bool,
    /// The mini player was closed while this source was current; it comes
    /// back when playback starts again.
    pub mini_closed: Option<MediaSource>,
    /// Bar levels of the music view's visualiser.
    pub visualiser: visualiser::Visualiser,
}
