//! LVP: a media player built on libvlc with a redesigned interface.
//!
//! Layers, outermost first: `ui` (egui screens) -> `app` (state and use cases)
//! -> `domain` (pure types). `infra` implements `app::ports` with libvlc and
//! the filesystem and is wired in by `main.rs`.

pub mod app;
pub mod domain;
pub mod infra;
pub mod ui;
