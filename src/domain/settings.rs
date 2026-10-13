//! User preferences, one struct per settings page in the design.
//!
//! Everything is `#[serde(default)]` so older or partial settings files load
//! cleanly. Enums carry their own display labels so the UI can render
//! dropdowns straight from `ALL`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::effects::{AudioEffects, DeinterlaceMode, VideoEffects};
use super::shortcuts::Shortcuts;
use super::toolbar::ToolbarLayout;

/// Declares a fieldless enum with a label per variant and an `ALL` list.
macro_rules! choice {
    ($(#[$m:meta])* $name:ident { $($(#[$vm:meta])* $var:ident => $label:expr),+ $(,)? } default $def:ident) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name { $($(#[$vm])* $var),+ }
        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$var),+];
            pub fn label(self) -> &'static str { match self { $($name::$var => $label),+ } }
        }
        impl Default for $name { fn default() -> Self { $name::$def } }
    };
}

choice!(Theme { Dark => "Dark", Light => "Light", System => "System" } default Dark);

choice!(
    /// The nine accent swatches on the General page.
    Accent {
        Orange => "Orange", Red => "Red", Olive => "Olive", Green => "Green", Teal => "Teal",
        Blue => "Blue", Purple => "Purple", Pink => "Pink", Gray => "Gray"
    } default Orange
);

impl Accent {
    /// The swatch colour as shown on the General page (sRGB hex).
    pub fn hex(self) -> u32 {
        match self {
            Accent::Orange => 0xD37918,
            Accent::Red => 0xE15032,
            Accent::Olive => 0xC49E23,
            Accent::Green => 0x19A234,
            Accent::Teal => 0x18AAD3,
            Accent::Blue => 0x185DD3,
            Accent::Purple => 0x8E42FF,
            Accent::Pink => 0xF052B3,
            Accent::Gray => 0xAFAFAF,
        }
    }
}

choice!(LogLevel { ErrorsOnly => "Errors only", Warnings => "Errors and warnings", Info => "Information", Debug => "Debug" } default ErrorsOnly);
choice!(LogFormat { PlainText => "Plain text", Html => "HTML" } default PlainText);
choice!(SyslogFacility {
    User => "User", Daemon => "Daemon", Local0 => "Local 0", Local1 => "Local 1", Local2 => "Local 2",
    Local3 => "Local 3", Local4 => "Local 4", Local5 => "Local 5", Local6 => "Local 6", Local7 => "Local 7"
} default User);

pub const LANGUAGES: &[&str] = &[
    "English",
    "Dansk",
    "Deutsch",
    "Español",
    "Français",
    "Italiano",
    "Nederlands",
    "Norsk",
    "Polski",
    "Português",
    "Suomi",
    "Svenska",
    "Türkçe",
    "Русский",
    "Українська",
    "日本語",
    "한국어",
    "简体中文",
    "繁體中文",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralSettings {
    pub language: String,
    pub theme: Theme,
    pub accent: Accent,
    pub show_dialogs: bool,
    pub check_updates: bool,
    pub developer_mode: bool,
    pub save_log: bool,
    pub log_file: String,
    pub log_level: LogLevel,
    pub show_minor_warnings: bool,
    pub extra_args: String,
    pub log_format: LogFormat,
    pub collect_stats: bool,
    pub syslog: bool,
    pub syslog_debug: bool,
    pub syslog_ident: String,
    pub syslog_facility: SyslogFacility,
    pub onboarding_done: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            language: "English".into(),
            theme: Theme::Dark,
            accent: Accent::Orange,
            show_dialogs: true,
            check_updates: false,
            developer_mode: false,
            save_log: false,
            log_file: String::new(),
            log_level: LogLevel::ErrorsOnly,
            show_minor_warnings: false,
            extra_args: String::new(),
            log_format: LogFormat::PlainText,
            collect_stats: true,
            syslog: false,
            syslog_debug: false,
            syslog_ident: "vlc".into(),
            syslog_facility: SyslogFacility::User,
            onboarding_done: false,
        }
    }
}

choice!(ResizeToVideo { Never => "Never", Always => "Always", FirstVideo => "First video only" } default Never);
choice!(FullscreenScreen { Current => "Current screen", Primary => "Primary screen" } default Current);
choice!(AutoHideControls { Never => "Never", FullscreenOnly => "In fullscreen only", Always => "Always" } default FullscreenOnly);
choice!(TrackPopup { Never => "Never", WhenMinimized => "When minimized", Always => "Always" } default WhenMinimized);
choice!(BringToFront { Never => "Never", Video => "Video", Always => "Always" } default Video);
choice!(Sensitivity { Low => "Low", Medium => "Medium", High => "High" } default Medium);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InterfaceSettings {
    pub custom_skin: bool,
    pub skin_file: String,
    pub video_in_main_window: bool,
    pub resize_to_video: ResizeToVideo,
    pub remember_window: bool,
    pub always_on_top: bool,
    pub show_decorations: bool,
    pub filename_in_title: bool,
    pub fullscreen_screen: FullscreenScreen,
    pub start_compact: bool,
    pub auto_hide_controls: AutoHideControls,
    pub hide_after_s: f32,
    pub controls_in_fullscreen: bool,
    pub tray_icon: bool,
    pub start_minimized: bool,
    pub track_popup: TrackPopup,
    pub popup_duration_s: f32,
    pub playlist_tree: bool,
    pub window_opacity: f32,
    pub pause_when_minimized: bool,
    pub bring_to_front: BringToFront,
    pub cover_art_for_audio: bool,
    pub file_browser_in_open: bool,
    pub dynamic_app_icon: bool,
    pub fs_controls_opacity: f32,
    pub fs_controls_sensitivity: Sensitivity,
    /// Library sidebar expanded (labels) or collapsed (icons only).
    pub sidebar_expanded: bool,
    pub window_size: Option<[f32; 2]>,
    pub window_pos: Option<[f32; 2]>,
    pub toolbar: ToolbarLayout,
}

impl Default for InterfaceSettings {
    fn default() -> Self {
        Self {
            custom_skin: false,
            skin_file: String::new(),
            video_in_main_window: true,
            resize_to_video: ResizeToVideo::Never,
            remember_window: true,
            always_on_top: false,
            show_decorations: true,
            filename_in_title: true,
            fullscreen_screen: FullscreenScreen::Current,
            start_compact: false,
            auto_hide_controls: AutoHideControls::FullscreenOnly,
            hide_after_s: 3.0,
            controls_in_fullscreen: true,
            tray_icon: false,
            start_minimized: false,
            track_popup: TrackPopup::WhenMinimized,
            popup_duration_s: 5.0,
            playlist_tree: true,
            window_opacity: 1.0,
            pause_when_minimized: false,
            bring_to_front: BringToFront::Video,
            cover_art_for_audio: true,
            file_browser_in_open: true,
            dynamic_app_icon: true,
            fs_controls_opacity: 1.0,
            fs_controls_sensitivity: Sensitivity::Medium,
            sidebar_expanded: true,
            window_size: None,
            window_pos: None,
            toolbar: ToolbarLayout::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PrivacySettings {
    pub save_recent: bool,
    pub remember_position: bool,
    pub never_remember_pattern: String,
    pub fetch_online_meta: bool,
    pub lastfm_submit: bool,
    pub lastfm_username: String,
    pub lastfm_password: String,
    pub lastfm_url: String,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            save_recent: true,
            remember_position: true,
            never_remember_pattern: String::new(),
            fetch_online_meta: false,
            lastfm_submit: false,
            lastfm_username: String::new(),
            lastfm_password: String::new(),
            lastfm_url: "post.audioscrobbler.com".into(),
        }
    }
}

choice!(ClickAction {
    Nothing => "Nothing", PlayPause => "Play / Pause", ToggleFullscreen => "Toggle fullscreen",
    ToggleControls => "Show / hide controls"
} default PlayPause);
choice!(WheelAction { Nothing => "Nothing", Volume => "Volume", Seek => "Seek" } default Volume);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ControlsSettings {
    pub shortcuts: Shortcuts,
    pub skip_very_short_s: u32,
    pub skip_short_s: u32,
    pub skip_medium_s: u32,
    pub skip_long_s: u32,
    pub single_click: ClickAction,
    pub double_click: ClickAction,
    pub scroll_wheel: WheelAction,
    pub horizontal_scroll: WheelAction,
    pub reverse_scroll: bool,
    pub web_remote: bool,
    pub web_remote_port: u16,
    pub web_remote_password: String,
}

impl Default for ControlsSettings {
    fn default() -> Self {
        Self {
            shortcuts: Shortcuts::default(),
            skip_very_short_s: 3,
            skip_short_s: 10,
            skip_medium_s: 60,
            skip_long_s: 300,
            single_click: ClickAction::PlayPause,
            double_click: ClickAction::ToggleFullscreen,
            scroll_wheel: WheelAction::Volume,
            horizontal_scroll: WheelAction::Seek,
            reverse_scroll: false,
            web_remote: false,
            web_remote_port: 8080,
            web_remote_password: String::new(),
        }
    }
}

choice!(ContinuePlayback { Never => "Never", Ask => "Ask", Always => "Always" } default Always);
choice!(VideoQuality { Best => "Best available", P2160 => "2160p", P1440 => "1440p", P1080 => "1080p", P720 => "720p", P576 => "576p", P360 => "360p", P240 => "240p" } default Best);
choice!(RepeatMode { Off => "Off", One => "Repeat one", All => "Repeat all" } default Off);
choice!(WhenFinished { Stop => "Stop", ReturnHome => "Return to library", Quit => "Quit" } default Stop);
choice!(AskAlways { Ask => "Ask", Always => "Always fix", Never => "Never fix" } default Ask);
choice!(BufferSize { Small => "Small", Medium => "Medium", Large => "Large" } default Medium);

impl VideoQuality {
    /// Value for VLC's `preferred-resolution` (-1 = best).
    pub fn vlc_value(self) -> i32 {
        match self {
            VideoQuality::Best => -1,
            VideoQuality::P2160 => 2160,
            VideoQuality::P1440 => 1440,
            VideoQuality::P1080 => 1080,
            VideoQuality::P720 => 720,
            VideoQuality::P576 => 576,
            VideoQuality::P360 => 360,
            VideoQuality::P240 => 240,
        }
    }
}

impl RepeatMode {
    pub fn cycle(self) -> Self {
        match self {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        }
    }
}

pub const SPEED_STEPS: &[f32] = &[0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlaybackSettings {
    pub continue_playback: ContinuePlayback,
    pub autoplay: bool,
    pub default_speed: f32,
    pub pause_on_interrupt: bool,
    pub preferred_quality: VideoQuality,
    pub menu_language: String,
    pub default_audio_track: i32,
    pub default_subtitle_track: i32,
    pub repeat: RepeatMode,
    pub shuffle: bool,
    pub stop_after_each: bool,
    pub when_finished: WhenFinished,
    pub seek_precisely: bool,
    pub scrub_thumbnails: bool,
    pub optical_drive: String,
    pub repair_avi: AskAlways,
    pub load_matching_mkv: bool,
    pub timeshift_dir: String,
    pub timeshift_size: BufferSize,
}

impl Default for PlaybackSettings {
    fn default() -> Self {
        Self {
            continue_playback: ContinuePlayback::Always,
            autoplay: true,
            default_speed: 1.0,
            pause_on_interrupt: true,
            preferred_quality: VideoQuality::Best,
            menu_language: "English".into(),
            default_audio_track: 1,
            default_subtitle_track: 1,
            repeat: RepeatMode::Off,
            shuffle: false,
            stop_after_each: false,
            when_finished: WhenFinished::Stop,
            seek_precisely: true,
            scrub_thumbnails: true,
            optical_drive: String::new(),
            repair_avi: AskAlways::Ask,
            load_matching_mkv: true,
            timeshift_dir: String::new(),
            timeshift_size: BufferSize::Medium,
        }
    }
}

choice!(SnapshotFormat { Png => "PNG", Jpg => "JPG", Tiff => "TIFF" } default Png);
choice!(SnapshotSize { SameAsVideo => "Same as video", Half => "Half size", Window => "Window size" } default SameAsVideo);
choice!(VideoPosition {
    Center => "Center", Top => "Top", Bottom => "Bottom", Left => "Left", Right => "Right"
} default Center);

impl SnapshotFormat {
    pub fn ext(self) -> &'static str {
        match self {
            SnapshotFormat::Png => "png",
            SnapshotFormat::Jpg => "jpg",
            SnapshotFormat::Tiff => "tiff",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VideoSettings {
    pub enabled: bool,
    pub start_fullscreen: bool,
    pub stop_screensaver: bool,
    pub wallpaper: bool,
    pub deinterlace: DeinterlaceMode,
    /// A key of `effects::DEINTERLACE_METHODS`.
    pub deinterlace_method: String,
    /// `None` keeps the source's aspect ratio.
    pub aspect_ratio: Option<String>,
    pub crop: Option<String>,
    pub scale_to_window: bool,
    /// 0 fits the window, otherwise a fixed scale.
    pub zoom: f32,
    pub position: VideoPosition,
    pub snapshot_dir: String,
    pub snapshot_prefix: String,
    pub snapshot_format: SnapshotFormat,
    pub snapshot_size: SnapshotSize,
    pub snapshot_sequential: bool,
    pub snapshot_preview: bool,
    pub custom_aspect_ratios: String,
    pub custom_crop_ratios: String,
    pub monitor_par: String,
    /// Persisted defaults for the per-file effects panel.
    pub effects: VideoEffects,
}

impl Default for VideoSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            start_fullscreen: false,
            stop_screensaver: true,
            wallpaper: false,
            deinterlace: DeinterlaceMode::Automatic,
            deinterlace_method: "auto".into(),
            aspect_ratio: None,
            crop: None,
            scale_to_window: true,
            zoom: 0.0,
            position: VideoPosition::Center,
            snapshot_dir: String::new(),
            snapshot_prefix: "vlcsnap-".into(),
            snapshot_format: SnapshotFormat::Png,
            snapshot_size: SnapshotSize::SameAsVideo,
            snapshot_sequential: true,
            snapshot_preview: true,
            custom_aspect_ratios: String::new(),
            custom_crop_ratios: String::new(),
            monitor_par: "1:1".into(),
            effects: VideoEffects::default(),
        }
    }
}

choice!(Visualizer {
    Disabled => "Disabled", Spectrometer => "Spectrometer", Scope => "Scope", Spectrum => "Spectrum",
    VuMeter => "VU meter", Goom => "Goom", ProjectM => "projectM", Glspectrum => "3D spectrum"
} default Disabled);
choice!(DolbyDetect { Automatic => "Automatic", On => "On", Off => "Off" } default Automatic);
choice!(ResampleQuality { Fast => "Fast", Balanced => "Balanced", Best => "Best" } default Balanced);
choice!(MediaRole {
    Video => "Video", Music => "Music", Communication => "Communication", Game => "Game",
    Notification => "Notification", Animation => "Animation", Production => "Production",
    Accessibility => "Accessibility", Test => "Test"
} default Video);
choice!(VisualSize { MatchWindow => "Match window", Small => "Small", Large => "Large" } default MatchWindow);
choice!(SpeakerSetup {
    Automatic => "Automatic", Stereo => "Stereo", Mono => "Mono", Headphones => "Headphones",
    ReverseStereo => "Reverse stereo", DolbySurround => "Dolby Surround"
} default Automatic);
choice!(ReplayGain { Off => "Off", Track => "Track", Album => "Album" } default Off);

impl SpeakerSetup {
    /// VLC's `stereo-mode` value (0 leaves the decoder's layout alone).
    pub fn vlc_value(self) -> i32 {
        match self {
            SpeakerSetup::Automatic => 0,
            SpeakerSetup::Stereo => 1,
            SpeakerSetup::ReverseStereo => 2,
            SpeakerSetup::DolbySurround => 5,
            SpeakerSetup::Headphones => 6,
            SpeakerSetup::Mono => 7,
        }
    }
}

impl ReplayGain {
    /// VLC's `audio-replay-gain-mode` value.
    pub fn vlc_value(self) -> &'static str {
        match self {
            ReplayGain::Off => "none",
            ReplayGain::Track => "track",
            ReplayGain::Album => "album",
        }
    }
}

impl Visualizer {
    /// Value for `--audio-visual`, `--effect-list` style selection.
    pub fn vlc_module(self) -> Option<(&'static str, Option<&'static str>)> {
        match self {
            Visualizer::Disabled => None,
            Visualizer::Spectrometer => Some(("visual", Some("spectrometer"))),
            Visualizer::Scope => Some(("visual", Some("scope"))),
            Visualizer::Spectrum => Some(("visual", Some("spectrum"))),
            Visualizer::VuMeter => Some(("visual", Some("vuMeter"))),
            Visualizer::Goom => Some(("goom", None)),
            Visualizer::ProjectM => Some(("projectm", None)),
            Visualizer::Glspectrum => Some(("glspectrum", None)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    pub effects: AudioEffects,
    pub preferred_language: String,
    pub delay_ms: i64,
    pub visualizer: Visualizer,
    pub passthrough: bool,
    pub dolby_surround: DolbyDetect,
    pub resample_quality: ResampleQuality,
    pub media_role: MediaRole,
    pub visual_size: VisualSize,
    pub output_device: Option<String>,
    pub volume: i32,
    pub max_volume: i32,
    pub enabled: bool,
    pub speaker_setup: SpeakerSetup,
    /// Start at the last used volume, or always at `startup_volume`.
    pub remember_volume: bool,
    pub startup_volume: i32,
    /// Percentage points per volume up/down shortcut.
    pub volume_step: i32,
    /// Compress loud passages (drives the compressor with a gentle preset).
    pub night_mode: bool,
    pub replay_gain: ReplayGain,
    pub replay_gain_preamp: f32,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            effects: AudioEffects::default(),
            preferred_language: "System language".into(),
            delay_ms: 0,
            visualizer: Visualizer::Disabled,
            passthrough: false,
            dolby_surround: DolbyDetect::Automatic,
            resample_quality: ResampleQuality::Balanced,
            media_role: MediaRole::Video,
            visual_size: VisualSize::MatchWindow,
            output_device: None,
            volume: 100,
            max_volume: 125,
            enabled: true,
            speaker_setup: SpeakerSetup::Automatic,
            remember_volume: true,
            startup_volume: 100,
            volume_step: 5,
            night_mode: false,
            replay_gain: ReplayGain::Off,
            replay_gain_preamp: 0.0,
        }
    }
}

choice!(SubMatching {
    Exact => "Exact match", ExtraChars => "Title with extra characters", ContainsTitle => "Title anywhere",
    AllInFolder => "All in folder"
} default ExtraChars);
choice!(SizeUnits { Relative => "Relative", Pixels => "Pixels" } default Relative);
choice!(FontSize { Automatic => "Automatic", Smaller => "Smaller", Small => "Small", Normal => "Normal", Large => "Large", Larger => "Larger" } default Automatic);
choice!(Outline { None => "None", Thin => "Thin", Normal => "Normal", Thick => "Thick" } default Normal);
choice!(ShadowDistance { Small => "Small", Medium => "Medium", Large => "Large" } default Small);
choice!(SubBackground { None => "None", Box => "Box", Line => "Line" } default None);
choice!(SubPosition { Bottom => "Bottom", Top => "Top", Center => "Center" } default Bottom);
choice!(TextDirection { Automatic => "Automatic", LeftToRight => "Left to right", RightToLeft => "Right to left" } default Automatic);

pub const TEXT_ENCODINGS: &[&str] = &[
    "Automatic",
    "UTF-8",
    "UTF-16",
    "Windows-1250",
    "Windows-1251",
    "Windows-1252",
    "Windows-1253",
    "Windows-1254",
    "Windows-1255",
    "Windows-1256",
    "Windows-1257",
    "ISO-8859-1",
    "ISO-8859-2",
    "ISO-8859-5",
    "ISO-8859-7",
    "ISO-8859-15",
    "KOI8-R",
    "Shift_JIS",
    "EUC-JP",
    "GB18030",
    "Big5",
    "EUC-KR",
];

impl FontSize {
    /// VLC's `freetype-rel-fontsize` values (0 = automatic).
    pub fn rel_value(self) -> i32 {
        match self {
            FontSize::Automatic => 0,
            FontSize::Smaller => 20,
            FontSize::Small => 18,
            FontSize::Normal => 16,
            FontSize::Large => 12,
            FontSize::Larger => 6,
        }
    }
}

impl Outline {
    pub fn vlc_value(self) -> i32 {
        match self {
            Outline::None => 0,
            Outline::Thin => 2,
            Outline::Normal => 4,
            Outline::Thick => 6,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SubtitleSettings {
    pub enabled: bool,
    pub preferred_language: String,
    pub encoding: String,
    pub autoload: bool,
    pub matching: SubMatching,
    pub font: String,
    pub size_units: SizeUnits,
    pub font_size: FontSize,
    pub font_size_px: u32,
    pub bold: bool,
    pub text_color: u32,
    pub text_opacity: f32,
    pub outline: Outline,
    pub outline_color: u32,
    pub shadow: bool,
    pub shadow_color: u32,
    pub shadow_distance: ShadowDistance,
    pub background: SubBackground,
    pub background_color: u32,
    pub background_opacity: f32,
    pub position: SubPosition,
    pub delay_ms: i64,
    pub osd_feedback: bool,
    pub osd_feedback_s: f32,
    pub osd_filename: bool,
    pub osd_filename_s: f32,
    pub subtitle_dirs: String,
    pub ignore_ass_styling: bool,
    pub shadow_angle: f32,
    pub text_direction: TextDirection,
    pub monospace_font: String,
    /// Subtitle speed multiplier (VLC's `sub-fps` ratio) and display duration factor.
    pub speed: f32,
    pub duration_factor: f32,
}

impl Default for SubtitleSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            preferred_language: "System language".into(),
            encoding: "Automatic".into(),
            autoload: true,
            matching: SubMatching::ExtraChars,
            font: "System, sans-serif".into(),
            size_units: SizeUnits::Relative,
            font_size: FontSize::Automatic,
            font_size_px: 0,
            bold: false,
            text_color: 0xFFFFFF,
            text_opacity: 1.0,
            outline: Outline::Normal,
            outline_color: 0x000000,
            shadow: true,
            shadow_color: 0x000000,
            shadow_distance: ShadowDistance::Small,
            background: SubBackground::None,
            background_color: 0x000000,
            background_opacity: 0.0,
            position: SubPosition::Bottom,
            delay_ms: 0,
            osd_feedback: true,
            osd_feedback_s: 1.5,
            osd_filename: true,
            osd_filename_s: 5.0,
            subtitle_dirs: ".\\subtitles, .\\subs".into(),
            ignore_ass_styling: false,
            shadow_angle: -45.0,
            text_direction: TextDirection::Automatic,
            monospace_font: "System monospace".into(),
            speed: 1.0,
            duration_factor: 0.0,
        }
    }
}

choice!(FolderOpen { IncludeSubfolders => "Include subfolders", TopLevel => "Top level only" } default IncludeSubfolders);
choice!(ArtworkFetch { FileOnly => "From the file only", Internet => "From the file and the internet", Never => "Never" } default FileOnly);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LibrarySettings {
    pub one_instance: bool,
    pub one_instance_from_file_manager: bool,
    pub enqueue: bool,
    pub folder_open: FolderOpen,
    pub show_hidden: bool,
    pub artwork: ArtworkFetch,
    pub use_media_library: bool,
    pub video_folders: Vec<PathBuf>,
    pub music_folders: Vec<PathBuf>,
}

impl Default for LibrarySettings {
    fn default() -> Self {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
        Self {
            one_instance: false,
            one_instance_from_file_manager: false,
            enqueue: false,
            folder_open: FolderOpen::IncludeSubfolders,
            show_hidden: false,
            artwork: ArtworkFetch::FileOnly,
            use_media_library: true,
            video_folders: vec![home.join("Movies")],
            music_folders: vec![home.join("Music")],
        }
    }
}

choice!(BufferingPreset { Lowest => "Lowest latency", Low => "Low", Normal => "Normal", High => "High", Highest => "Highest" } default Normal);
choice!(CipherPreference { Balanced => "Balanced", Security => "Prefer security", Speed => "Prefer speed" } default Balanced);

impl BufferingPreset {
    pub fn caching_ms(self) -> [u32; 4] {
        match self {
            BufferingPreset::Lowest => [100, 100, 100, 100],
            BufferingPreset::Low => [200, 200, 500, 200],
            BufferingPreset::Normal => [300, 300, 1000, 300],
            BufferingPreset::High => [600, 600, 2000, 600],
            BufferingPreset::Highest => [1000, 1000, 5000, 1000],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkSettings {
    pub buffering: BufferingPreset,
    pub file_caching_ms: u32,
    pub disc_caching_ms: u32,
    pub network_caching_ms: u32,
    pub live_caching_ms: u32,
    pub http_proxy: String,
    pub http_proxy_password: String,
    pub verify_certs: bool,
    pub cipher: CipherPreference,
    pub user_agent: String,
    pub recent_urls: Vec<String>,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            buffering: BufferingPreset::Normal,
            file_caching_ms: 300,
            disc_caching_ms: 300,
            network_caching_ms: 1000,
            live_caching_ms: 300,
            http_proxy: String::new(),
            http_proxy_password: String::new(),
            verify_certs: true,
            cipher: CipherPreference::Balanced,
            user_agent: String::new(),
            recent_urls: Vec::new(),
        }
    }
}

choice!(HwDecoding { Automatic => "Automatic", VideoToolbox => "VideoToolbox", Disabled => "Disabled" } default Automatic);
choice!(PostProcessing { Off => "Off", Low => "Low", Medium => "Medium", High => "High", Max => "Maximum" } default Off);
choice!(SkipLoopFilter { Never => "Never", NonRef => "Non-reference frames", Bidir => "Bidirectional frames", NonKey => "Non-key frames", All => "All frames" } default Never);
choice!(ClockSync { Default => "Default", Disabled => "Disabled", Enabled => "Enabled" } default Default);

impl PostProcessing {
    pub fn quality(self) -> i32 {
        match self {
            PostProcessing::Off => 0,
            PostProcessing::Low => 1,
            PostProcessing::Medium => 3,
            PostProcessing::High => 5,
            PostProcessing::Max => 6,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PerformanceSettings {
    pub hw_decoding: HwDecoding,
    pub post_processing: PostProcessing,
    pub skip_loop_filter: SkipLoopFilter,
    pub decoding_threads: u32,
    pub drop_late_frames: bool,
    pub realtime_priority: bool,
    pub process_priority: i32,
    pub encoder: String,
    pub decoder: String,
    pub clock_sync: ClockSync,
    pub clock_jitter_ms: u32,
    pub clock_averaging: u32,
}

impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            hw_decoding: HwDecoding::Automatic,
            post_processing: PostProcessing::Off,
            skip_loop_filter: SkipLoopFilter::Never,
            decoding_threads: 0,
            drop_late_frames: true,
            realtime_priority: false,
            process_priority: 0,
            encoder: "Automatic".into(),
            decoder: "Automatic".into(),
            clock_sync: ClockSync::Default,
            clock_jitter_ms: 5000,
            clock_averaging: 40,
        }
    }
}

choice!(CastQuality { Automatic => "Automatic", Low => "Low (CPU)", Medium => "Medium", High => "High", Ultra => "Ultra" } default Automatic);
choice!(EncodingPreset {
    Ultrafast => "Ultrafast", Superfast => "Superfast", Veryfast => "Veryfast", Faster => "Faster", Fast => "Fast",
    Medium => "Medium", Slow => "Slow", Slower => "Slower", Veryslow => "Veryslow"
} default Medium);
choice!(EncodingTune {
    None => "None", Film => "Film", Animation => "Animation", Grain => "Grain", StillImage => "Still image",
    FastDecode => "Fast decode", ZeroLatency => "Zero latency"
} default None);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RecordingSettings {
    pub record_dir: String,
    pub record_no_reencode: bool,
    pub discover_cast: bool,
    pub cast_quality: CastQuality,
    pub stream_play_locally: bool,
    pub stream_audio: bool,
    pub stream_video: bool,
    pub stream_subtitles: bool,
    pub preset: EncodingPreset,
    pub tune: EncodingTune,
}

impl Default for RecordingSettings {
    fn default() -> Self {
        Self {
            record_dir: "~/Movies".into(),
            record_no_reencode: true,
            discover_cast: true,
            cast_quality: CastQuality::Automatic,
            stream_play_locally: true,
            stream_audio: true,
            stream_video: true,
            stream_subtitles: true,
            preset: EncodingPreset::Medium,
            tune: EncodingTune::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Extension {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExtensionSettings {
    pub load_on_startup: bool,
    pub installed: Vec<Extension>,
}

impl Default for ExtensionSettings {
    fn default() -> Self {
        Self { load_on_startup: true, installed: Vec::new() }
    }
}

/// All persisted preferences.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub general: GeneralSettings,
    pub interface: InterfaceSettings,
    pub privacy: PrivacySettings,
    pub controls: ControlsSettings,
    pub playback: PlaybackSettings,
    pub video: VideoSettings,
    pub audio: AudioSettings,
    pub subtitles: SubtitleSettings,
    pub library: LibrarySettings,
    pub network: NetworkSettings,
    pub performance: PerformanceSettings,
    pub recording: RecordingSettings,
    pub extensions: ExtensionSettings,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_json_fills_defaults() {
        let s: Settings = serde_json::from_str(r#"{"general":{"theme":"Light"}}"#).unwrap();
        assert_eq!(s.general.theme, Theme::Light);
        assert_eq!(s.general.accent, Accent::Orange);
        assert_eq!(s.controls.skip_short_s, 10);
    }

    #[test]
    fn round_trips() {
        let mut s = Settings::default();
        s.audio.effects.equalizer.enabled = true;
        let back: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn repeat_cycles() {
        assert_eq!(RepeatMode::Off.cycle(), RepeatMode::All);
        assert_eq!(RepeatMode::All.cycle(), RepeatMode::One);
        assert_eq!(RepeatMode::One.cycle(), RepeatMode::Off);
    }
}
