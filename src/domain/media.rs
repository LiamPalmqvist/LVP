//! Media descriptions shared by the library, playlists and the player.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// What kind of thing a media item is. Drives which library page shows it and
/// whether the player shows video or the music view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum MediaKind {
    Video,
    Audio,
    Image,
    Playlist,
    Subtitle,
    Disc,
    Stream,
    #[default]
    Unknown,
}

/// Extensions VLC opens, grouped the way VLC's own file filters group them.
pub const VIDEO_EXTENSIONS: &[&str] = &[
    "3g2", "3gp", "3gp2", "3gpp", "amv", "asf", "avi", "bik", "bin", "crf", "dav", "divx", "drc", "dv", "dvr-ms",
    "evo", "f4v", "flv", "gvi", "gxf", "iso", "m1v", "m2v", "m2t", "m2ts", "m4v", "mkv", "mov", "mp2", "mp2v", "mp4",
    "mp4v", "mpe", "mpeg", "mpeg1", "mpeg2", "mpeg4", "mpg", "mpv2", "mts", "mtv", "mxf", "mxg", "nsv", "nuv", "ogm",
    "ogv", "ogx", "ps", "rec", "rm", "rmvb", "rpl", "thp", "tod", "tp", "ts", "tts", "txd", "vob", "vro", "webm", "wm",
    "wmv", "wtv", "xesc",
];

pub const AUDIO_EXTENSIONS: &[&str] = &[
    "3ga", "669", "a52", "aac", "ac3", "adt", "adts", "aif", "aifc", "aiff", "amb", "amr", "aob", "ape", "au", "awb",
    "caf", "dts", "dsf", "dff", "flac", "it", "kar", "m4a", "m4b", "m4p", "m5p", "mid", "mka", "mlp", "mod", "mpa",
    "mp1", "mp2", "mp3", "mpc", "mpga", "mus", "oga", "ogg", "oma", "opus", "qcp", "ra", "rmi", "s3m", "sid", "spx",
    "tak", "thd", "tta", "voc", "vqf", "w64", "wav", "wma", "wv", "xa", "xm",
];

pub const SUBTITLE_EXTENSIONS: &[&str] = &[
    "cdg", "idx", "srt", "sub", "utf", "ass", "ssa", "aqt", "jss", "psb", "rt", "sami", "smi", "txt", "smil", "stl",
    "usf", "dks", "pjs", "mpl2", "mks", "vtt", "tt", "ttml", "dfxp", "scc",
];

pub const PLAYLIST_EXTENSIONS: &[&str] = &[
    "asx", "b4s", "cue", "ifo", "m3u", "m3u8", "pls", "ram", "rar", "sdp", "vlc", "xspf", "wax", "wvx", "zip", "conf",
];

pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp", "tif", "tiff"];

/// Classifies a path by extension.
pub fn kind_for_path(path: &Path) -> MediaKind {
    let ext = path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
    let ext = ext.as_str();
    // "mp2" is both; VLC treats it as audio unless it carries video, which the
    // parser corrects later.
    if AUDIO_EXTENSIONS.contains(&ext) {
        MediaKind::Audio
    } else if VIDEO_EXTENSIONS.contains(&ext) {
        MediaKind::Video
    } else if SUBTITLE_EXTENSIONS.contains(&ext) {
        MediaKind::Subtitle
    } else if PLAYLIST_EXTENSIONS.contains(&ext) {
        MediaKind::Playlist
    } else if IMAGE_EXTENSIONS.contains(&ext) {
        MediaKind::Image
    } else {
        MediaKind::Unknown
    }
}

/// True for files the player can open directly (not subtitles or images).
pub fn is_playable(path: &Path) -> bool {
    matches!(kind_for_path(path), MediaKind::Video | MediaKind::Audio | MediaKind::Playlist)
}

/// Where a media item comes from. Serialises as a single MRL string so stored
/// playlists stay readable.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "String", from = "String")]
pub enum MediaSource {
    File(PathBuf),
    /// Any MRL VLC understands: http(s), rtsp, rtmp, udp, smb, upnp, dvd://, ...
    Location(String),
}

impl MediaSource {
    pub fn from_user_input(input: &str) -> Self {
        let t = input.trim();
        if t.contains("://") {
            if let Some(path) = file_url_to_path(t) {
                return MediaSource::File(path);
            }
            return MediaSource::Location(t.to_string());
        }
        MediaSource::File(PathBuf::from(expand_tilde(t)))
    }

    /// The MRL libvlc should open.
    pub fn mrl(&self) -> String {
        match self {
            MediaSource::File(p) => path_to_file_url(p),
            MediaSource::Location(l) => l.clone(),
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            MediaSource::File(p) => Some(p),
            MediaSource::Location(_) => None,
        }
    }

    /// A human title for the source when no metadata is known.
    pub fn display_name(&self) -> String {
        match self {
            MediaSource::File(p) => {
                p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string())
            }
            MediaSource::Location(l) => {
                let trimmed = l.trim_end_matches('/');
                let last = trimmed.rsplit('/').next().unwrap_or(trimmed);
                let decoded = percent_encoding::percent_decode_str(last).decode_utf8_lossy().into_owned();
                if decoded.is_empty() || decoded.ends_with(':') { l.clone() } else { decoded }
            }
        }
    }

    pub fn kind(&self) -> MediaKind {
        match self {
            MediaSource::File(p) => kind_for_path(p),
            MediaSource::Location(l) => {
                let lower = l.to_ascii_lowercase();
                if ["dvd://", "dvdsimple://", "bluray://", "vcd://", "cdda://"].iter().any(|s| lower.starts_with(s)) {
                    MediaKind::Disc
                } else {
                    let path_part = lower.split(['?', '#']).next().unwrap_or("");
                    match kind_for_path(Path::new(path_part)) {
                        MediaKind::Unknown => MediaKind::Stream,
                        k => k,
                    }
                }
            }
        }
    }
}

impl From<MediaSource> for String {
    fn from(s: MediaSource) -> String {
        s.mrl()
    }
}

impl From<String> for MediaSource {
    fn from(s: String) -> Self {
        MediaSource::from_user_input(&s)
    }
}

fn expand_tilde(p: &str) -> String {
    if let Some(rest) = p.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return Path::new(&home).join(rest).to_string_lossy().into_owned();
    }
    p.to_string()
}

pub fn path_to_file_url(p: &Path) -> String {
    url::Url::from_file_path(p).map(|u| u.to_string()).unwrap_or_else(|_| format!("file://{}", p.display()))
}

pub fn file_url_to_path(s: &str) -> Option<PathBuf> {
    if !s.starts_with("file:") {
        return None;
    }
    url::Url::parse(s).ok()?.to_file_path().ok()
}

/// Descriptive metadata, filled in by the parser as it becomes known.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MediaMeta {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub genre: Option<String>,
    pub date: Option<String>,
    pub track_number: Option<String>,
    pub disc_number: Option<String>,
    pub description: Option<String>,
    pub copyright: Option<String>,
    pub publisher: Option<String>,
    pub encoded_by: Option<String>,
    pub language: Option<String>,
    pub url: Option<String>,
    pub now_playing: Option<String>,
    pub show_name: Option<String>,
    pub season: Option<String>,
    pub episode: Option<String>,
    pub director: Option<String>,
    pub actors: Option<String>,
    pub artwork_url: Option<String>,
    pub duration_ms: Option<i64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub has_video: Option<bool>,
    pub codecs: Vec<CodecInfo>,
}

impl MediaMeta {
    pub fn merge(&mut self, other: MediaMeta) {
        macro_rules! take {
            ($($f:ident),*) => { $( if other.$f.is_some() { self.$f = other.$f; } )* };
        }
        take!(
            title,
            artist,
            album,
            album_artist,
            genre,
            date,
            track_number,
            disc_number,
            description,
            copyright,
            publisher,
            encoded_by,
            language,
            url,
            now_playing,
            show_name,
            season,
            episode,
            director,
            actors,
            artwork_url,
            duration_ms,
            width,
            height,
            has_video
        );
        if !other.codecs.is_empty() {
            self.codecs = other.codecs;
        }
    }
}

/// One elementary stream as reported by the demuxer, for the codec info view.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CodecInfo {
    pub kind: String,
    pub codec: String,
    pub language: Option<String>,
    pub description: Option<String>,
    pub bitrate: u32,
    pub channels: Option<u32>,
    pub sample_rate: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate: Option<f32>,
}

/// A media item as the rest of the app sees it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaItem {
    pub source: MediaSource,
    #[serde(default)]
    pub meta: MediaMeta,
    /// Extra per-item libvlc options (":start-time=...", ":sub-file=...").
    #[serde(default)]
    pub options: Vec<String>,
}

impl MediaItem {
    pub fn new(source: MediaSource) -> Self {
        Self { source, meta: MediaMeta::default(), options: Vec::new() }
    }

    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self::new(MediaSource::File(path.into()))
    }

    pub fn title(&self) -> String {
        self.meta.title.clone().filter(|t| !t.trim().is_empty()).unwrap_or_else(|| self.source.display_name())
    }

    pub fn kind(&self) -> MediaKind {
        match (self.source.kind(), self.meta.has_video) {
            (MediaKind::Audio, Some(true)) => MediaKind::Video,
            (MediaKind::Video, Some(false)) => MediaKind::Audio,
            (k, _) => k,
        }
    }

    pub fn is_audio(&self) -> bool {
        self.kind() == MediaKind::Audio
    }
}

/// Picks subtitle files that belong to a video, following VLC's
/// "sub-autodetect-fuzzy" rules: exact stem match first, then stems that
/// start with the video's stem ("Title with extra characters").
pub fn matching_subtitles(video: &Path, candidates: &[PathBuf], allow_extra: bool) -> Vec<PathBuf> {
    let Some(stem) = video.file_stem().map(|s| s.to_string_lossy().to_lowercase()) else {
        return Vec::new();
    };
    let mut exact = Vec::new();
    let mut fuzzy = Vec::new();
    for c in candidates {
        if kind_for_path(c) != MediaKind::Subtitle {
            continue;
        }
        let Some(cs) = c.file_stem().map(|s| s.to_string_lossy().to_lowercase()) else { continue };
        if cs == stem {
            exact.push(c.clone());
        } else if allow_extra && (cs.starts_with(&stem) || stem.starts_with(&cs)) {
            fuzzy.push(c.clone());
        }
    }
    exact.extend(fuzzy);
    exact
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_extensions() {
        assert_eq!(kind_for_path(Path::new("/a/b.MKV")), MediaKind::Video);
        assert_eq!(kind_for_path(Path::new("song.flac")), MediaKind::Audio);
        assert_eq!(kind_for_path(Path::new("x.srt")), MediaKind::Subtitle);
        assert_eq!(kind_for_path(Path::new("x.xspf")), MediaKind::Playlist);
        assert_eq!(kind_for_path(Path::new("noext")), MediaKind::Unknown);
    }

    #[test]
    fn source_from_input() {
        assert_eq!(
            MediaSource::from_user_input("https://example.com/live.m3u8"),
            MediaSource::Location("https://example.com/live.m3u8".into())
        );
        assert_eq!(MediaSource::from_user_input("file:///tmp/a%20b.mp4"), MediaSource::File("/tmp/a b.mp4".into()));
        assert_eq!(MediaSource::Location("dvd:///dev/disk2".into()).kind(), MediaKind::Disc);
        assert_eq!(MediaSource::Location("http://x/y.mp3?z=1".into()).kind(), MediaKind::Audio);
        assert_eq!(MediaSource::Location("rtsp://cam/stream".into()).kind(), MediaKind::Stream);
    }

    #[test]
    fn mrl_round_trips_through_serde() {
        let item = MediaItem::file("/tmp/a b.mp4");
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("file:///tmp/a%20b.mp4"));
        let back: MediaItem = serde_json::from_str(&json).unwrap();
        assert_eq!(back.source, item.source);
    }

    #[test]
    fn subtitle_matching_prefers_exact() {
        let subs = vec![
            PathBuf::from("/v/Show.S01E01.en.srt"),
            PathBuf::from("/v/Show.S01E01.srt"),
            PathBuf::from("/v/Other.srt"),
        ];
        let found = matching_subtitles(Path::new("/v/Show.S01E01.mkv"), &subs, true);
        assert_eq!(found, vec![PathBuf::from("/v/Show.S01E01.srt"), PathBuf::from("/v/Show.S01E01.en.srt")]);
        let exact_only = matching_subtitles(Path::new("/v/Show.S01E01.mkv"), &subs, false);
        assert_eq!(exact_only.len(), 1);
    }
}
