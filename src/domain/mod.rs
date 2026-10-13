//! Pure types and rules. Nothing in here touches the filesystem, libvlc or
//! the UI, and nothing in here imports from other layers of this crate.

pub mod convert;
pub mod disc;
pub mod effects;
pub mod library;
pub mod media;
pub mod playlist;
pub mod playlist_format;
pub mod settings;
pub mod shortcuts;
pub mod subtitles;
pub mod time;
pub mod toolbar;
