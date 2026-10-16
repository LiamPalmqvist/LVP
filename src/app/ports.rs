//! Interfaces the application layer needs from the outside world. The libvlc
//! engine and the JSON store in `infra` implement these; tests use fakes.

use std::path::PathBuf;
use std::sync::Arc;

use crate::app::video::VideoOutput;
use crate::domain::effects::{AudioEffects, DeinterlaceMode, StereoMode, VideoEffects};
use crate::domain::media::{MediaItem, MediaMeta, MediaSource};
use crate::domain::settings::{Settings, Visualizer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayerState {
    #[default]
    Idle,
    Opening,
    Buffering,
    Playing,
    Paused,
    Stopped,
    Ended,
    Error,
}

impl PlayerState {
    pub fn is_active(self) -> bool {
        matches!(self, PlayerState::Opening | PlayerState::Buffering | PlayerState::Playing | PlayerState::Paused)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Chapter {
    pub name: String,
    pub start_ms: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Title {
    pub name: String,
    pub duration_ms: i64,
    pub is_menu: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RendererInfo {
    pub id: u64,
    pub name: String,
    pub kind: String,
    pub can_video: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PlaybackStats {
    pub input_bitrate_kbps: f32,
    pub demux_bitrate_kbps: f32,
    pub read_bytes: i64,
    pub decoded_video: i64,
    pub decoded_audio: i64,
    pub displayed_pictures: i64,
    pub lost_pictures: i64,
    pub played_abuffers: i64,
    pub lost_abuffers: i64,
    pub demux_corrupted: i64,
    pub demux_discontinuity: i64,
}

/// A snapshot of the player, refreshed by the engine thread. Reading it never
/// blocks on libvlc.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EngineStatus {
    pub state: PlayerState,
    pub mrl: Option<String>,
    pub time_ms: i64,
    pub length_ms: i64,
    pub position: f32,
    pub rate: f32,
    pub volume: i32,
    pub muted: bool,
    pub seekable: bool,
    pub pausable: bool,
    pub has_video: bool,
    /// Decoded picture size and sample aspect ratio of the current video.
    pub video_size: Option<(u32, u32)>,
    pub video_sar: (u32, u32),
    pub fps: f32,
    pub audio_tracks: Vec<Track>,
    pub audio_track: i32,
    pub video_tracks: Vec<Track>,
    pub video_track: i32,
    pub subtitle_tracks: Vec<Track>,
    pub subtitle_track: i32,
    pub chapters: Vec<Chapter>,
    pub chapter: i32,
    pub titles: Vec<Title>,
    pub title: i32,
    pub audio_delay_us: i64,
    pub subtitle_delay_us: i64,
    pub buffering: f32,
    pub recording: bool,
    pub stereo_mode: StereoMode,
    pub audio_device: Option<String>,
    pub renderer: Option<u64>,
    pub meta: MediaMeta,
    pub stats: PlaybackStats,
    pub scrambled: bool,
    pub teletext_page: i32,
    /// True when live filter changes reach VLC (libvlccore available).
    pub live_effects: bool,
}

/// Disc and DVD-menu navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    Activate,
    Up,
    Down,
    Left,
    Right,
    Popup,
}

/// Subtitle synchronisation extras that VLC applies on the input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SubtitleTiming {
    pub speed: f32,
    pub duration_factor: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenRequest {
    pub item: MediaItem,
    pub start_ms: Option<i64>,
    pub paused: bool,
    pub subtitles: Vec<PathBuf>,
    /// Track ids chosen last time for this file (-1 = subtitles off).
    pub audio_track: Option<i32>,
    pub subtitle_track: Option<i32>,
    pub audio_delay_us: i64,
    pub subtitle_delay_us: i64,
    /// The file is known to carry subtitles. With hardware decoding set to
    /// Automatic this opens it with software decoding, because VLC 3 cannot
    /// draw subtitles onto hardware-decoded pictures in this output mode.
    pub expect_subtitles: bool,
}

impl OpenRequest {
    pub fn new(item: MediaItem) -> Self {
        Self {
            item,
            start_ms: None,
            paused: false,
            subtitles: Vec::new(),
            audio_track: None,
            subtitle_track: None,
            audio_delay_us: 0,
            subtitle_delay_us: 0,
            expect_subtitles: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EngineCommand {
    Open(Box<OpenRequest>),
    Play,
    Pause,
    TogglePause,
    Stop,
    SeekMs {
        ms: i64,
        precise: bool,
    },
    SeekPosition(f32),
    SetRate(f32),
    NextFrame,
    SetVolume(i32),
    SetMute(bool),
    SetAudioTrack(i32),
    SetVideoTrack(i32),
    SetSubtitleTrack(i32),
    AddSubtitle(PathBuf),
    AddAudioTrack(PathBuf),
    SetAudioDelay(i64),
    SetSubtitleDelay(i64),
    SetSubtitleTiming(SubtitleTiming),
    /// Subtitle size as a percentage of normal, applied live.
    SetSubtitleScale(i32),
    SetChapter(i32),
    SetTitle(i32),
    Navigate(Nav),
    SetAudioEffects(Box<AudioEffects>),
    SetVideoEffects(Box<VideoEffects>),
    SetDeinterlace(DeinterlaceMode, String),
    SetStereoMode(StereoMode),
    SetAudioDevice(Option<String>),
    SetRenderer(Option<u64>),
    SetRecording(bool),
    SetTeletext(i32),
    SetVisualizer(Visualizer),
    /// Rebuilds the libvlc instance from settings (codecs, caching, subtitle
    /// style...), resuming the current item where it was.
    Reconfigure(Box<Settings>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DialogRequest {
    Error { title: String, text: String },
    Login { id: u64, title: String, text: String, default_username: String, ask_store: bool },
    Question { id: u64, title: String, text: String, cancel: String, action1: Option<String>, action2: Option<String> },
    Progress { id: u64, title: String, text: String, position: Option<f32>, cancel: Option<String> },
    UpdateProgress { id: u64, position: f32, text: String },
    Cancel { id: u64 },
}

#[derive(Debug, Clone, PartialEq)]
pub enum DialogReply {
    Login { username: String, password: String, store: bool },
    Action(i32),
    Dismiss,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EngineEvent {
    StateChanged(PlayerState),
    EndReached,
    Error(String),
    LengthChanged(i64),
    TracksChanged,
    MetaChanged,
    VideoOutput(u32),
    ChapterChanged(i32),
    TitleChanged(i32),
    Dialog(DialogRequest),
    RenderersChanged,
    JobProgress { id: u64, position: f32 },
    JobFinished { id: u64, ok: bool, message: String },
}

/// A network or device source VLC can discover (UPnP, SMB, SAP, Bonjour...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceInfo {
    pub name: String,
    pub long_name: String,
    pub category: ServiceCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceCategory {
    Devices,
    Lan,
    Podcasts,
    LocalDirs,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BrowseItem {
    pub name: String,
    pub source: MediaSource,
    pub is_container: bool,
}

/// Transcode or stream a source to a destination (VLC's Convert / Stream).
#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    pub source: MediaItem,
    /// A full `#transcode{...}:std{...}` chain.
    pub sout: String,
    pub play_locally: bool,
}

pub trait MediaEngine: Send + Sync {
    fn send(&self, cmd: EngineCommand);
    fn status(&self) -> EngineStatus;
    fn poll_events(&self) -> Vec<EngineEvent>;
    fn video(&self) -> Arc<VideoOutput>;
    fn version(&self) -> String;
    fn audio_devices(&self) -> Vec<AudioDevice>;
    fn renderers(&self) -> Vec<RendererInfo>;
    fn reply_dialog(&self, id: u64, reply: DialogReply);
    fn services(&self) -> Vec<ServiceInfo>;
    /// Starts a discovery service and returns what it has found so far.
    fn service_items(&self, name: &str) -> Vec<BrowseItem>;
    /// Lists a container MRL (an SMB share, a UPnP folder...). Blocks.
    fn browse(&self, source: &MediaSource) -> anyhow::Result<Vec<BrowseItem>>;
    fn start_job(&self, job: Job) -> u64;
    fn cancel_job(&self, id: u64);
}

/// Metadata and thumbnails, run on worker threads by the library.
pub trait MediaProbe: Send + Sync {
    fn probe(&self, source: &MediaSource) -> anyhow::Result<MediaMeta>;
    /// Writes a still frame at `fraction` of the duration to `out` (PNG).
    fn thumbnail(&self, source: &MediaSource, out: &std::path::Path, width: u32, fraction: f32) -> anyhow::Result<()>;
}

/// Key-value JSON persistence.
pub trait Store: Send + Sync {
    fn load(&self, key: &str) -> Option<String>;
    fn save(&self, key: &str, json: &str) -> anyhow::Result<()>;
    fn data_dir(&self) -> PathBuf;
    fn cache_dir(&self) -> PathBuf;
}

/// Operating-system integration that is not media: sleep prevention and
/// desktop notifications.
pub trait SystemHooks: Send + Sync {
    /// Keeps the display awake while `on` (video playing).
    fn inhibit_sleep(&self, on: bool);
    /// Shows a desktop notification.
    fn notify(&self, title: &str, body: &str);
}

/// Does nothing; the default until a real implementation is installed.
pub struct NoSystemHooks;

impl SystemHooks for NoSystemHooks {
    fn inhibit_sleep(&self, _on: bool) {}
    fn notify(&self, _title: &str, _body: &str) {}
}

/// What the system's "Now Playing" widget shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NowPlaying {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    /// A `file://` URL for cover art.
    pub artwork: Option<String>,
    pub duration_ms: i64,
    pub position_ms: i64,
    pub playing: bool,
}

/// Commands from media keys, headphones or the Now Playing widget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RemoteCommand {
    Play,
    Pause,
    Toggle,
    Next,
    Previous,
    Stop,
    SeekBy(i64),
    SeekTo(i64),
}

/// The platform's media controls. Lives on the UI thread.
pub trait MediaControls {
    fn update(&mut self, now: Option<&NowPlaying>);
    fn poll(&mut self) -> Vec<RemoteCommand>;
}
