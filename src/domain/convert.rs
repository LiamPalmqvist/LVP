//! Convert / Save and Stream: conversion profiles (VLC's standard list and
//! user-made ones) and the sout chains built from a profile and a
//! destination, e.g.
//! `#transcode{vcodec=h264,vb=2000,acodec=mp4a,ab=128,channels=2,samplerate=44100,scodec=none}:std{access=file,mux=mp4,dst='OUT'}`.

use serde::{Deserialize, Serialize};

/// Declares a fieldless enum with a label and a VLC name per variant.
macro_rules! vlc_enum {
    ($(#[$m:meta])* $name:ident { $($var:ident => ($label:expr, $vlc:expr)),+ $(,)? } default $def:ident) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name { $($var),+ }
        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$var),+];
            pub fn label(self) -> &'static str { match self { $($name::$var => $label),+ } }
            /// The name VLC's sout modules use.
            pub fn vlc(self) -> &'static str { match self { $($name::$var => $vlc),+ } }
        }
        impl Default for $name { fn default() -> Self { $name::$def } }
    };
}

vlc_enum!(
    /// Encapsulation (the `mux` of the output).
    Container {
        Mp4 => ("MP4/MOV", "mp4"),
        Webm => ("WebM", "webm"),
        Ogg => ("Ogg/OGM", "ogg"),
        Ts => ("MPEG-TS", "ts"),
        Ps => ("MPEG-PS", "ps"),
        Mkv => ("Matroska (MKV)", "mkv"),
        Asf => ("ASF/WMV", "asf"),
        Avi => ("AVI", "avi"),
        Flv => ("FLV", "ffmpeg{mux=flv}"),
        Wav => ("WAV", "wav"),
        Raw => ("Raw (audio only)", "raw"),
    } default Mp4
);

impl Container {
    /// File extension for this container; raw audio takes the codec's.
    pub fn extension(self, audio: AudioCodec) -> &'static str {
        match self {
            Container::Mp4 => "mp4",
            Container::Webm => "webm",
            Container::Ogg => "ogg",
            Container::Ts => "ts",
            Container::Ps => "mpg",
            Container::Mkv => "mkv",
            Container::Asf => "asf",
            Container::Avi => "avi",
            Container::Flv => "flv",
            Container::Wav => "wav",
            Container::Raw => match audio {
                AudioCodec::Mp3 | AudioCodec::Mpga => "mp3",
                AudioCodec::Flac => "flac",
                AudioCodec::Aac => "aac",
                AudioCodec::A52 => "ac3",
                AudioCodec::Opus => "opus",
                AudioCodec::Vorbis => "ogg",
                _ => "raw",
            },
        }
    }

    /// Whether the container can be written to a live network stream.
    pub fn streamable(self) -> bool {
        matches!(
            self,
            Container::Ts | Container::Ps | Container::Ogg | Container::Webm | Container::Asf | Container::Flv
        )
    }
}

vlc_enum!(
    VideoCodec {
        Keep => ("Keep original", ""),
        Disabled => ("No video", "none"),
        H264 => ("H.264", "h264"),
        H265 => ("H.265", "hevc"),
        Vp8 => ("VP8", "VP80"),
        Vp9 => ("VP9", "VP90"),
        Theora => ("Theora", "theo"),
        Mpeg2 => ("MPEG-2", "mp2v"),
        Mpeg4 => ("MPEG-4", "mp4v"),
        Div3 => ("DIV3", "DIV3"),
        Wmv2 => ("WMV2", "WMV2"),
        Mjpeg => ("M-JPEG", "MJPG"),
        Dirac => ("Dirac", "drac"),
    } default H264
);

vlc_enum!(
    AudioCodec {
        Keep => ("Keep original", ""),
        Disabled => ("No audio", "none"),
        Mp3 => ("MP3", "mp3"),
        Mpga => ("MPEG Audio", "mpga"),
        Aac => ("MPEG 4 Audio (AAC)", "mp4a"),
        Vorbis => ("Vorbis", "vorb"),
        Opus => ("Opus", "opus"),
        Flac => ("FLAC", "flac"),
        A52 => ("A52/AC-3", "a52"),
        Pcm => ("WAV (16-bit PCM)", "s16l"),
        Wma2 => ("WMA2", "wma2"),
        Speex => ("Speex", "spx"),
    } default Mpga
);

vlc_enum!(
    SubtitleMode {
        Keep => ("Keep", ""),
        Disabled => ("None", "none"),
        Encode => ("Encode", ""),
        Overlay => ("Burn in", "soverlay"),
    } default Disabled
);

vlc_enum!(
    SubtitleCodec {
        Tx3g => ("MP4 text (tx3g)", "tx3g"),
        Dvbs => ("DVB subtitles", "dvbs"),
        T140 => ("T.140", "t140"),
    } default Tx3g
);

/// One Convert profile. Zero in a numeric field means "leave it to VLC"
/// (same as the source, or the encoder's default).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub name: String,
    pub container: Container,
    pub video: VideoCodec,
    pub video_bitrate_kbps: u32,
    /// Scale factor; 0 or 1 keeps the size.
    pub scale: f32,
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub audio: AudioCodec,
    pub audio_bitrate_kbps: u32,
    pub channels: u32,
    pub sample_rate: u32,
    pub subtitles: SubtitleMode,
    pub subtitle_codec: SubtitleCodec,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "New profile".into(),
            container: Container::Mp4,
            video: VideoCodec::H264,
            video_bitrate_kbps: 0,
            scale: 0.0,
            width: 0,
            height: 0,
            fps: 0.0,
            audio: AudioCodec::Aac,
            audio_bitrate_kbps: 128,
            channels: 2,
            sample_rate: 44100,
            subtitles: SubtitleMode::Disabled,
            subtitle_codec: SubtitleCodec::Tx3g,
        }
    }
}

fn num(v: f32) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

impl Profile {
    fn video(name: &str, container: Container, video: VideoCodec, vb: u32, audio: AudioCodec, ab: u32) -> Self {
        Self {
            name: name.into(),
            container,
            video,
            video_bitrate_kbps: vb,
            audio,
            audio_bitrate_kbps: ab,
            ..Default::default()
        }
    }

    fn sized(mut self, width: u32) -> Self {
        self.width = width;
        self
    }

    fn audio_only(name: &str, container: Container, audio: AudioCodec, ab: u32) -> Self {
        Self::video(name, container, VideoCodec::Disabled, 0, audio, ab)
    }

    /// The `transcode{...}` part, or `None` when nothing is re-encoded
    /// (keep video, keep audio, keep subtitles: a remux).
    pub fn transcode(&self) -> Option<String> {
        let mut p: Vec<String> = Vec::new();
        match self.video {
            VideoCodec::Keep => {}
            VideoCodec::Disabled => p.push("vcodec=none".into()),
            c => {
                p.push(format!("vcodec={}", c.vlc()));
                if self.video_bitrate_kbps > 0 {
                    p.push(format!("vb={}", self.video_bitrate_kbps));
                }
                if self.scale > 0.0 && (self.scale - 1.0).abs() > f32::EPSILON {
                    p.push(format!("scale={}", num(self.scale)));
                }
                if self.width > 0 {
                    p.push(format!("width={}", self.width));
                }
                if self.height > 0 {
                    p.push(format!("height={}", self.height));
                }
                if self.fps > 0.0 {
                    p.push(format!("fps={}", num(self.fps)));
                }
            }
        }
        match self.audio {
            AudioCodec::Keep => {}
            AudioCodec::Disabled => p.push("acodec=none".into()),
            c => {
                p.push(format!("acodec={}", c.vlc()));
                if self.audio_bitrate_kbps > 0 && c != AudioCodec::Pcm && c != AudioCodec::Flac {
                    p.push(format!("ab={}", self.audio_bitrate_kbps));
                }
                if self.channels > 0 {
                    p.push(format!("channels={}", self.channels));
                }
                if self.sample_rate > 0 {
                    p.push(format!("samplerate={}", self.sample_rate));
                }
            }
        }
        match self.subtitles {
            SubtitleMode::Keep => {}
            SubtitleMode::Disabled => p.push("scodec=none".into()),
            SubtitleMode::Encode => p.push(format!("scodec={}", self.subtitle_codec.vlc())),
            SubtitleMode::Overlay => p.push("soverlay".into()),
        }
        if self.video == VideoCodec::Keep && self.audio == AudioCodec::Keep && self.subtitles == SubtitleMode::Keep {
            return None;
        }
        Some(format!("transcode{{{}}}", p.join(",")))
    }

    pub fn extension(&self) -> &'static str {
        self.container.extension(self.audio)
    }

    /// A one-line description for the profile list.
    pub fn summary(&self) -> String {
        let v = match self.video {
            VideoCodec::Keep => "video kept".to_string(),
            VideoCodec::Disabled => "no video".to_string(),
            c if self.video_bitrate_kbps > 0 => format!("{} {} kb/s", c.label(), self.video_bitrate_kbps),
            c => c.label().to_string(),
        };
        let a = match self.audio {
            AudioCodec::Keep => "audio kept".to_string(),
            AudioCodec::Disabled => "no audio".to_string(),
            c if self.audio_bitrate_kbps > 0 => format!("{} {} kb/s", c.label(), self.audio_bitrate_kbps),
            c => c.label().to_string(),
        };
        format!("{}, {v}, {a}", self.container.label())
    }
}

/// VLC 3's standard Convert profiles, in VLC's order.
pub fn standard_profiles() -> Vec<Profile> {
    use AudioCodec as A;
    use Container as C;
    use VideoCodec as V;
    let mut cd = Profile::audio_only("Audio - CD", C::Wav, A::Pcm, 0);
    cd.sample_rate = 44100;
    vec![
        Profile::video("Video - H.264 + MP3 (MP4)", C::Mp4, V::H264, 0, A::Mpga, 128),
        Profile::video("Video - VP80 + Vorbis (Webm)", C::Webm, V::Vp8, 2000, A::Vorbis, 128),
        Profile::video("Video - H.265 + MP3 (MP4)", C::Mp4, V::H265, 0, A::Mpga, 128),
        Profile::video("Video - Theora + Vorbis (OGG)", C::Ogg, V::Theora, 800, A::Vorbis, 128),
        Profile::video("Video - MPEG-2 + MPGA (TS)", C::Ts, V::Mpeg2, 800, A::Mpga, 128),
        Profile::video("Video - Dirac + AAC (TS)", C::Ts, V::Dirac, 800, A::Aac, 128),
        Profile::video("Video - H.264 + AAC (TS)", C::Ts, V::H264, 800, A::Aac, 128),
        Profile::video("Video - WMV + WMA (ASF)", C::Asf, V::Wmv2, 800, A::Wma2, 128),
        Profile::video("Video - DIV3 + MP3 (ASF)", C::Asf, V::Div3, 800, A::Mp3, 128),
        Profile::audio_only("Audio - Vorbis (OGG)", C::Ogg, A::Vorbis, 128),
        Profile::audio_only("Audio - MP3", C::Raw, A::Mp3, 128),
        Profile::audio_only("Audio - MP3 (MP4)", C::Mp4, A::Mpga, 128),
        Profile::audio_only("Audio - FLAC", C::Raw, A::Flac, 0),
        cd,
        Profile::video("Video for MPEG4 720p TV/device", C::Mp4, V::H264, 3500, A::Aac, 160).sized(1280),
        Profile::video("Video for MPEG4 1080p TV/device", C::Mp4, V::H264, 6000, A::Aac, 192).sized(1920),
        Profile::video("Video for DivX compatible player", C::Avi, V::Mpeg4, 1500, A::Mpga, 128).sized(720),
        Profile::video("Video for iPod SD", C::Mp4, V::H264, 700, A::Aac, 128).sized(320),
        Profile::video("Video for iPod HD/iPhone/PSP", C::Mp4, V::H264, 1500, A::Aac, 128).sized(480),
        Profile::video("Video for Android SD Low", C::Mp4, V::H264, 200, A::Aac, 64).sized(176),
        Profile::video("Video for Android SD High", C::Mp4, V::H264, 500, A::Aac, 128).sized(480),
        Profile::video("Video for Android HD", C::Mp4, V::H264, 2000, A::Aac, 192).sized(1280),
        Profile::video("Video for Youtube SD", C::Mp4, V::H264, 1500, A::Aac, 128).sized(640),
        Profile::video("Video for Youtube HD", C::Mp4, V::H264, 5000, A::Aac, 192).sized(1280),
    ]
}

/// Network protocols for Stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum StreamProtocol {
    #[default]
    Http,
    Rtsp,
    Rtp,
    Udp,
}

impl StreamProtocol {
    pub const ALL: [StreamProtocol; 4] =
        [StreamProtocol::Http, StreamProtocol::Rtsp, StreamProtocol::Rtp, StreamProtocol::Udp];

    pub fn label(self) -> &'static str {
        match self {
            StreamProtocol::Http => "HTTP",
            StreamProtocol::Rtsp => "RTSP",
            StreamProtocol::Rtp => "RTP / MPEG-TS",
            StreamProtocol::Udp => "UDP (legacy)",
        }
    }

    pub fn default_port(self) -> u16 {
        match self {
            StreamProtocol::Http => 8080,
            StreamProtocol::Rtsp => 8554,
            StreamProtocol::Rtp => 5004,
            StreamProtocol::Udp => 1234,
        }
    }

    /// Whether the destination is an address to send to (RTP, UDP) rather
    /// than a server path others connect to (HTTP, RTSP).
    pub fn sends(self) -> bool {
        matches!(self, StreamProtocol::Rtp | StreamProtocol::Udp)
    }
}

/// Where the converted output goes.
#[derive(Debug, Clone, PartialEq)]
pub enum Destination {
    File(String),
    Stream { protocol: StreamProtocol, address: String, port: u16, path: String },
}

/// Quotes a value for a sout chain: VLC's config chains take '...' with
/// backslash escapes.
pub fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"))
}

fn clean_path(p: &str) -> String {
    let t = p.trim().trim_start_matches('/');
    format!("/{t}")
}

/// The full `#...` sout chain for a profile and destination.
pub fn sout(profile: &Profile, dest: &Destination) -> String {
    let out = match dest {
        Destination::File(path) => {
            format!("std{{access=file,mux={},dst={}}}", profile.container.vlc(), quote(path))
        }
        Destination::Stream { protocol, address, port, path } => {
            // Live streams need a container that can be read from the
            // middle; MP4, MKV, AVI and WAV fall back to MPEG-TS.
            let mux = if profile.container.streamable() { profile.container.vlc() } else { "ts" };
            match protocol {
                StreamProtocol::Http => format!("http{{mux={mux},dst=:{port}{}}}", clean_path(path)),
                StreamProtocol::Rtsp => format!("rtp{{sdp=rtsp://:{port}{}}}", clean_path(path)),
                StreamProtocol::Rtp => format!("rtp{{dst={},port={port},mux=ts}}", address.trim()),
                StreamProtocol::Udp => format!("udp{{mux=ts,dst={}:{port}}}", address.trim()),
            }
        }
    };
    match profile.transcode() {
        Some(t) => format!("#{t}:{out}"),
        None => format!("#{out}"),
    }
}

/// A destination file name for `stem` under this profile. When the source
/// already has that extension, " (converted)" keeps it from being
/// overwritten.
pub fn output_name(stem: &str, source_ext: &str, profile: &Profile) -> String {
    let ext = profile.extension();
    if source_ext.eq_ignore_ascii_case(ext) { format!("{stem} (converted).{ext}") } else { format!("{stem}.{ext}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h264() -> Profile {
        Profile {
            name: "test".into(),
            container: Container::Mp4,
            video: VideoCodec::H264,
            video_bitrate_kbps: 2000,
            audio: AudioCodec::Aac,
            audio_bitrate_kbps: 128,
            channels: 2,
            sample_rate: 44100,
            subtitles: SubtitleMode::Disabled,
            ..Default::default()
        }
    }

    #[test]
    fn file_chain_matches_vlc() {
        assert_eq!(
            sout(&h264(), &Destination::File("/tmp/out.mp4".into())),
            "#transcode{vcodec=h264,vb=2000,acodec=mp4a,ab=128,channels=2,samplerate=44100,scodec=none}\
             :std{access=file,mux=mp4,dst='/tmp/out.mp4'}"
        );
    }

    #[test]
    fn quotes_are_escaped() {
        assert_eq!(quote("/tmp/DON'T BLINK.mp4"), r"'/tmp/DON\'T BLINK.mp4'");
        assert_eq!(quote(r"C:\a"), r"'C:\\a'");
    }

    #[test]
    fn http_stream_falls_back_to_ts() {
        let d = Destination::Stream {
            protocol: StreamProtocol::Http,
            address: String::new(),
            port: 8080,
            path: "stream".into(),
        };
        assert!(sout(&h264(), &d).ends_with(":http{mux=ts,dst=:8080/stream}"));
        let mut webm = h264();
        webm.container = Container::Webm;
        assert!(sout(&webm, &d).ends_with(":http{mux=webm,dst=:8080/stream}"));
    }

    #[test]
    fn rtsp_rtp_and_udp() {
        let p = h264();
        let mk = |protocol, address: &str, port| Destination::Stream {
            protocol,
            address: address.into(),
            port,
            path: "/live".into(),
        };
        assert!(sout(&p, &mk(StreamProtocol::Rtsp, "", 8554)).ends_with(":rtp{sdp=rtsp://:8554/live}"));
        assert!(
            sout(&p, &mk(StreamProtocol::Rtp, "239.0.0.1", 5004)).ends_with(":rtp{dst=239.0.0.1,port=5004,mux=ts}")
        );
        assert!(sout(&p, &mk(StreamProtocol::Udp, "239.0.0.1", 1234)).ends_with(":udp{mux=ts,dst=239.0.0.1:1234}"));
    }

    #[test]
    fn remux_has_no_transcode() {
        let p = Profile {
            video: VideoCodec::Keep,
            audio: AudioCodec::Keep,
            subtitles: SubtitleMode::Keep,
            container: Container::Mkv,
            ..Default::default()
        };
        assert_eq!(p.transcode(), None);
        assert_eq!(sout(&p, &Destination::File("a.mkv".into())), "#std{access=file,mux=mkv,dst='a.mkv'}");
    }

    #[test]
    fn audio_only_and_overlay() {
        let mp3 = standard_profiles().into_iter().find(|p| p.name == "Audio - MP3").unwrap();
        assert_eq!(mp3.extension(), "mp3");
        assert_eq!(
            mp3.transcode().unwrap(),
            "transcode{vcodec=none,acodec=mp3,ab=128,channels=2,samplerate=44100,scodec=none}"
        );
        let mut burn = h264();
        burn.subtitles = SubtitleMode::Overlay;
        burn.width = 1280;
        burn.fps = 23.976;
        assert!(burn.transcode().unwrap().contains("width=1280,fps=23.976,acodec"));
        assert!(burn.transcode().unwrap().ends_with(",soverlay}"));
    }

    #[test]
    fn standard_list_and_names() {
        let all = standard_profiles();
        assert!(all.len() >= 20);
        assert_eq!(all[0].name, "Video - H.264 + MP3 (MP4)");
        assert_eq!(output_name("ep01", "mkv", &all[0]), "ep01.mp4");
        assert_eq!(output_name("ep01", "MP4", &all[0]), "ep01 (converted).mp4");
        let json = serde_json::to_string(&all).unwrap();
        let back: Vec<Profile> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, all);
    }
}
