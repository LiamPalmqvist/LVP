//! Adapters to the outside world: libvlc and the filesystem. Implements the
//! traits in `app::ports`; only `main.rs` wires these in.

pub mod media_controls;
pub mod platform;
#[cfg(unix)]
pub mod single_instance;
pub mod store;
pub mod vlc;
