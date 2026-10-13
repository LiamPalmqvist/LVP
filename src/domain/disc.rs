//! MRLs and options for discs and capture devices, built the way VLC's own
//! Open Disc and Open Capture Device panels build them.

/// The disc types VLC's Open Disc panel offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiscKind {
    #[default]
    Dvd,
    Bluray,
    AudioCd,
    Vcd,
}

impl DiscKind {
    pub const ALL: [DiscKind; 4] = [DiscKind::Dvd, DiscKind::Bluray, DiscKind::AudioCd, DiscKind::Vcd];

    pub fn label(self) -> &'static str {
        match self {
            DiscKind::Dvd => "DVD",
            DiscKind::Bluray => "Blu-ray",
            DiscKind::AudioCd => "Audio CD",
            DiscKind::Vcd => "SVCD/VCD",
        }
    }

    /// Whether the disc has menus that can be skipped.
    pub fn has_menus(self) -> bool {
        matches!(self, DiscKind::Dvd | DiscKind::Bluray)
    }
}

/// Everything the Open Disc panel asks for. Zero means "not set".
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiscRequest {
    pub kind: DiscKind,
    /// Device, mounted volume, VIDEO_TS/BDMV folder or ISO file. Empty lets
    /// VLC pick the default drive.
    pub path: String,
    pub no_menus: bool,
    /// Title (DVD, Blu-ray, VCD) or track (Audio CD).
    pub title: u32,
    pub chapter: u32,
    pub audio_track: u32,
    pub subtitle_track: u32,
}

impl DiscRequest {
    /// `dvd:///path#title:chapter`, `dvdsimple://`, `bluray://`, `cdda://`, `vcd://`.
    pub fn mrl(&self) -> String {
        let scheme = match self.kind {
            DiscKind::Dvd if self.no_menus => "dvdsimple://",
            DiscKind::Dvd => "dvd://",
            DiscKind::Bluray => "bluray://",
            DiscKind::AudioCd => "cdda://",
            DiscKind::Vcd => "vcd://",
        };
        let mut mrl = format!("{scheme}{}", self.path.trim());
        match self.kind {
            DiscKind::Dvd | DiscKind::Vcd if self.title > 0 => {
                mrl.push_str(&format!("#{}", self.title));
                if self.chapter > 0 {
                    mrl.push_str(&format!(":{}", self.chapter));
                }
            }
            DiscKind::Bluray if self.title > 0 => mrl.push_str(&format!("#{}", self.title)),
            _ => {}
        }
        mrl
    }

    /// Per-input options to go with [`DiscRequest::mrl`].
    pub fn options(&self) -> Vec<String> {
        let mut o = Vec::new();
        if self.kind == DiscKind::Bluray && self.no_menus {
            o.push(":no-bluray-menu".to_string());
        }
        if self.kind == DiscKind::AudioCd && self.title > 0 {
            o.push(format!(":cdda-track={}", self.title));
        }
        if self.kind == DiscKind::Bluray && self.chapter > 0 {
            o.push(format!(":chapter={}", self.chapter));
        }
        if self.kind != DiscKind::AudioCd {
            if self.audio_track > 0 {
                o.push(format!(":audio-track={}", self.audio_track - 1));
            }
            if self.subtitle_track > 0 {
                o.push(format!(":sub-track={}", self.subtitle_track - 1));
            }
        }
        o
    }
}

/// What the Open Capture Device panel can open on macOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaptureKind {
    /// The default camera through AVFoundation (`avcapture://`).
    #[default]
    Camera,
    /// The default microphone (`avaudiocapture://`).
    Microphone,
    /// The desktop (`screen://`).
    Screen,
}

impl CaptureKind {
    pub const ALL: [CaptureKind; 3] = [CaptureKind::Camera, CaptureKind::Microphone, CaptureKind::Screen];

    pub fn label(self) -> &'static str {
        match self {
            CaptureKind::Camera => "Camera",
            CaptureKind::Microphone => "Microphone",
            CaptureKind::Screen => "Screen",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptureRequest {
    pub kind: CaptureKind,
    /// An AVFoundation device unique ID; empty uses the default device.
    pub device: String,
    /// Also record the microphone with the camera.
    pub with_audio: bool,
    pub fps: f32,
    /// Screen region; zero width or height captures the whole screen.
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
    pub caching_ms: u32,
}

impl Default for CaptureRequest {
    fn default() -> Self {
        Self {
            kind: CaptureKind::Camera,
            device: String::new(),
            with_audio: false,
            fps: 15.0,
            left: 0,
            top: 0,
            width: 0,
            height: 0,
            caching_ms: 300,
        }
    }
}

impl CaptureRequest {
    pub fn mrl(&self) -> String {
        let device = self.device.trim();
        match self.kind {
            CaptureKind::Camera => format!("avcapture://{device}"),
            CaptureKind::Microphone => format!("avaudiocapture://{device}"),
            CaptureKind::Screen => "screen://".to_string(),
        }
    }

    pub fn options(&self) -> Vec<String> {
        let mut o = vec![format!(":live-caching={}", self.caching_ms)];
        match self.kind {
            CaptureKind::Camera if self.with_audio => o.push(":input-slave=avaudiocapture://".to_string()),
            CaptureKind::Screen => {
                o.push(format!(":screen-fps={}", trim_float(self.fps)));
                if self.width > 0 && self.height > 0 {
                    o.push(format!(":screen-left={}", self.left));
                    o.push(format!(":screen-top={}", self.top));
                    o.push(format!(":screen-width={}", self.width));
                    o.push(format!(":screen-height={}", self.height));
                }
            }
            _ => {}
        }
        o
    }
}

fn trim_float(v: f32) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dvd_with_title_and_chapter() {
        let r = DiscRequest { path: "/Volumes/MOVIE".into(), title: 2, chapter: 5, ..Default::default() };
        assert_eq!(r.mrl(), "dvd:///Volumes/MOVIE#2:5");
        let r = DiscRequest { no_menus: true, ..r };
        assert_eq!(r.mrl(), "dvdsimple:///Volumes/MOVIE#2:5");
    }

    #[test]
    fn bluray_without_menus() {
        let r = DiscRequest {
            kind: DiscKind::Bluray,
            path: "/Volumes/BD".into(),
            no_menus: true,
            title: 1,
            audio_track: 2,
            ..Default::default()
        };
        assert_eq!(r.mrl(), "bluray:///Volumes/BD#1");
        assert_eq!(r.options(), vec![":no-bluray-menu", ":audio-track=1"]);
    }

    #[test]
    fn audio_cd_track() {
        let r = DiscRequest { kind: DiscKind::AudioCd, title: 3, ..Default::default() };
        assert_eq!(r.mrl(), "cdda://");
        assert_eq!(r.options(), vec![":cdda-track=3"]);
    }

    #[test]
    fn screen_capture_options() {
        let r = CaptureRequest { kind: CaptureKind::Screen, fps: 30.0, ..Default::default() };
        assert_eq!(r.mrl(), "screen://");
        assert_eq!(r.options(), vec![":live-caching=300", ":screen-fps=30"]);
        let cam = CaptureRequest { with_audio: true, ..Default::default() };
        assert_eq!(cam.options()[1], ":input-slave=avaudiocapture://");
        let named = CaptureRequest { device: "0x1420000046d0825".into(), ..Default::default() };
        assert_eq!(named.mrl(), "avcapture://0x1420000046d0825");
    }
}
