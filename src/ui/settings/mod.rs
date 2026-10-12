//! The settings window: thirteen categories in three groups.

use crate::app::App;
use crate::ui::state::UiState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Category {
    #[default]
    General,
    Interface,
    Privacy,
    Controls,
    Playback,
    Video,
    Audio,
    Subtitles,
    Library,
    Network,
    Performance,
    Recording,
    Extensions,
}

impl Category {
    pub const ALL: [Category; 13] = [
        Category::General,
        Category::Interface,
        Category::Privacy,
        Category::Controls,
        Category::Playback,
        Category::Video,
        Category::Audio,
        Category::Subtitles,
        Category::Library,
        Category::Network,
        Category::Performance,
        Category::Recording,
        Category::Extensions,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Category::General => "General",
            Category::Interface => "Interface",
            Category::Privacy => "Privacy",
            Category::Controls => "Controls",
            Category::Playback => "Playback",
            Category::Video => "Video",
            Category::Audio => "Audio",
            Category::Subtitles => "Subtitles",
            Category::Library => "Library",
            Category::Network => "Network",
            Category::Performance => "Performance",
            Category::Recording => "Recording",
            Category::Extensions => "Extensions",
        }
    }

    pub fn from_name(s: &str) -> Option<Category> {
        Category::ALL.iter().copied().find(|c| c.name().eq_ignore_ascii_case(s))
    }
}

#[derive(Default)]
pub struct SettingsState {
    pub open: bool,
    pub category: Category,
    pub search: String,
}

pub fn show(ctx: &egui::Context, _app: &mut App, st: &mut UiState) {
    if !st.settings.open {
        return;
    }
    let mut open = true;
    egui::Window::new("Settings").open(&mut open).show(ctx, |ui| {
        ui.label("Settings are being built.");
    });
    st.settings.open = open;
}
