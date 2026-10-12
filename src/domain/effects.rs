//! Audio and video effect settings, mirroring VLC's "Adjustments and Effects".
//!
//! Every field is a plain value in VLC's own units and range so the engine can
//! hand it straight to the matching VLC module. Ranges are documented on the
//! `RANGE_*` constants that the UI sliders use.

use serde::{Deserialize, Serialize};

pub const EQ_BANDS: usize = 10;
/// Band labels for VLC's classic equalizer frequencies, as shown in the design.
pub const EQ_BAND_LABELS: [&str; EQ_BANDS] =
    ["60 Hz", "170 Hz", "310 Hz", "600 Hz", "1 KHz", "3 KHz", "6 KHz", "12 KHz", "14 KHz", "16 KHz"];
pub const EQ_RANGE: (f32, f32) = (-20.0, 20.0);

/// VLC's built-in equalizer presets: name, preamp, ten bands.
pub const EQ_PRESETS: &[(&str, f32, [f32; EQ_BANDS])] = &[
    ("Flat", 12.0, [0.0; EQ_BANDS]),
    ("Classical", 12.0, [-1.11e-15, -1.11e-15, -1.11e-15, -1.11e-15, -1.11e-15, -1.11e-15, -7.2, -7.2, -7.2, -9.6]),
    ("Club", 6.0, [-1.11e-15, -1.11e-15, 8.0, 5.6, 5.6, 5.6, 3.2, -1.11e-15, -1.11e-15, -1.11e-15]),
    ("Dance", 5.0, [9.6, 7.2, 2.4, -1.11e-15, -1.11e-15, -5.6, -7.2, -7.2, -1.11e-15, -1.11e-15]),
    ("Full bass", 5.0, [-8.0, 9.6, 9.6, 5.6, 1.6, -4.0, -8.0, -10.4, -11.2, -11.2]),
    ("Full bass and treble", 4.0, [7.2, 5.6, -1.11e-15, -7.2, -4.8, 1.6, 8.0, 11.2, 12.0, 12.0]),
    ("Full treble", 3.0, [-9.6, -9.6, -9.6, -4.0, 2.4, 11.2, 16.0, 16.0, 16.0, 16.8]),
    ("Headphones", 4.0, [4.8, 11.2, 5.6, -3.2, -2.4, 1.6, 4.8, 9.6, 12.8, 14.4]),
    ("Large Hall", 5.0, [10.4, 10.4, 5.6, 5.6, -1.11e-15, -4.8, -4.8, -4.8, -1.11e-15, -1.11e-15]),
    ("Live", 7.0, [-4.8, -1.11e-15, 4.0, 5.6, 5.6, 5.6, 4.0, 2.4, 2.4, 2.4]),
    ("Party", 6.0, [7.2, 7.2, -1.11e-15, -1.11e-15, -1.11e-15, -1.11e-15, -1.11e-15, -1.11e-15, 7.2, 7.2]),
    ("Pop", 6.0, [-1.6, 4.8, 7.2, 8.0, 5.6, -1.11e-15, -2.4, -2.4, -1.6, -1.6]),
    ("Reggae", 8.0, [-1.11e-15, -1.11e-15, -1.11e-15, -5.6, -1.11e-15, 6.4, 6.4, -1.11e-15, -1.11e-15, -1.11e-15]),
    ("Rock", 5.0, [8.0, 4.8, -5.6, -8.0, -3.2, 4.0, 8.8, 11.2, 11.2, 11.2]),
    ("Ska", 6.0, [-2.4, -4.8, -4.0, -1.11e-15, 4.0, 5.6, 8.8, 9.6, 11.2, 9.6]),
    ("Soft", 5.0, [4.8, 1.6, -1.11e-15, -2.4, -1.11e-15, 4.0, 8.0, 9.6, 11.2, 12.0]),
    ("Soft rock", 7.0, [4.0, 4.0, 2.4, -1.11e-15, -4.0, -5.6, -3.2, -1.11e-15, 2.4, 8.8]),
    ("Techno", 5.0, [8.0, 5.6, -1.11e-15, -5.6, -4.8, -1.11e-15, 8.0, 9.6, 9.6, 8.8]),
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EqualizerSettings {
    pub enabled: bool,
    /// Index into [`EQ_PRESETS`], or `None` for custom values.
    pub preset: Option<usize>,
    pub preamp: f32,
    pub bands: [f32; EQ_BANDS],
    pub two_pass: bool,
}

impl Default for EqualizerSettings {
    fn default() -> Self {
        Self { enabled: false, preset: None, preamp: 12.0, bands: [0.0; EQ_BANDS], two_pass: false }
    }
}

impl EqualizerSettings {
    pub fn apply_preset(&mut self, index: usize) {
        if let Some((_, pre, bands)) = EQ_PRESETS.get(index) {
            self.preset = Some(index);
            self.preamp = *pre;
            self.bands = bands.map(|b| if b.abs() < 1e-6 { 0.0 } else { b });
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CompressorSettings {
    pub enabled: bool,
    /// RMS (0) to peak (1) detection.
    pub rms_peak: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub threshold_db: f32,
    pub ratio: f32,
    pub knee_db: f32,
    pub makeup_gain_db: f32,
}

impl Default for CompressorSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            rms_peak: 0.2,
            attack_ms: 25.0,
            release_ms: 100.0,
            threshold_db: -11.0,
            ratio: 4.0,
            knee_db: 5.0,
            makeup_gain_db: 7.0,
        }
    }
}

impl CompressorSettings {
    pub const RANGE_RMS_PEAK: (f32, f32) = (0.0, 1.0);
    pub const RANGE_ATTACK: (f32, f32) = (1.5, 400.0);
    pub const RANGE_RELEASE: (f32, f32) = (2.0, 800.0);
    pub const RANGE_THRESHOLD: (f32, f32) = (-30.0, 0.0);
    pub const RANGE_RATIO: (f32, f32) = (1.0, 20.0);
    pub const RANGE_KNEE: (f32, f32) = (1.0, 10.0);
    pub const RANGE_MAKEUP: (f32, f32) = (0.0, 24.0);
}

/// Freeverb-style room simulation (VLC's "spatializer").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SpatializerSettings {
    pub enabled: bool,
    pub room_size: f32,
    pub width: f32,
    pub wet: f32,
    pub dry: f32,
    pub damp: f32,
}

impl Default for SpatializerSettings {
    fn default() -> Self {
        Self { enabled: false, room_size: 0.85, width: 1.0, wet: 0.4, dry: 0.5, damp: 0.5 }
    }
}

impl SpatializerSettings {
    pub const RANGE: (f32, f32) = (0.0, 1.0);
    pub const RANGE_ROOM: (f32, f32) = (0.0, 1.1);
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StereoWidenerSettings {
    pub enabled: bool,
    pub delay_ms: f32,
    pub feedback: f32,
    pub crossfeed: f32,
    pub dry_mix: f32,
}

impl Default for StereoWidenerSettings {
    fn default() -> Self {
        Self { enabled: false, delay_ms: 20.0, feedback: 0.3, crossfeed: 0.3, dry_mix: 0.8 }
    }
}

impl StereoWidenerSettings {
    pub const RANGE_DELAY: (f32, f32) = (1.0, 100.0);
    pub const RANGE_FEEDBACK: (f32, f32) = (0.0, 0.9);
    pub const RANGE_CROSSFEED: (f32, f32) = (0.0, 0.8);
    pub const RANGE_DRY: (f32, f32) = (0.0, 1.0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SpatialMode {
    #[default]
    HeadphoneVirtualization,
    Ambisonics,
    DolbySurroundDecoder,
}

impl SpatialMode {
    pub const ALL: [SpatialMode; 3] =
        [SpatialMode::HeadphoneVirtualization, SpatialMode::Ambisonics, SpatialMode::DolbySurroundDecoder];
    pub fn label(self) -> &'static str {
        match self {
            SpatialMode::HeadphoneVirtualization => "Headphone virtualization",
            SpatialMode::Ambisonics => "Binaural (ambisonics)",
            SpatialMode::DolbySurroundDecoder => "Dolby Surround decoder",
        }
    }
}

/// Stereo channel routing (VLC's Audio > Stereo Mode).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum StereoMode {
    #[default]
    Stereo,
    Reverse,
    Left,
    Right,
    DolbySurround,
}

impl StereoMode {
    pub const ALL: [StereoMode; 5] =
        [StereoMode::Stereo, StereoMode::Reverse, StereoMode::Left, StereoMode::Right, StereoMode::DolbySurround];
    pub fn label(self) -> &'static str {
        match self {
            StereoMode::Stereo => "Stereo",
            StereoMode::Reverse => "Reverse stereo",
            StereoMode::Left => "Left",
            StereoMode::Right => "Right",
            StereoMode::DolbySurround => "Dolby Surround",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioEffects {
    pub equalizer: EqualizerSettings,
    pub compressor: CompressorSettings,
    pub spatializer: SpatializerSettings,
    pub stereo_widener: StereoWidenerSettings,
    pub spatial_audio: bool,
    pub spatial_mode: SpatialMode,
    /// Semitones, -12..12, through scaletempo_pitch.
    pub pitch_shift: f32,
    pub chorus_flanger: bool,
    pub karaoke: bool,
    /// Volume normaliser.
    pub normalize: bool,
    pub normalize_level: f32,
    /// Keep pitch when the speed changes.
    pub time_stretch: bool,
}

impl Default for AudioEffects {
    fn default() -> Self {
        Self {
            equalizer: EqualizerSettings::default(),
            compressor: CompressorSettings::default(),
            spatializer: SpatializerSettings::default(),
            stereo_widener: StereoWidenerSettings::default(),
            spatial_audio: false,
            spatial_mode: SpatialMode::default(),
            pitch_shift: 0.0,
            chorus_flanger: false,
            karaoke: false,
            normalize: false,
            normalize_level: 2.0,
            time_stretch: true,
        }
    }
}

/// "Essential" picture controls: VLC's adjust, sharpen, gradfun and grain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VideoEssential {
    pub adjust_enabled: bool,
    pub hue: f32,
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub gamma: f32,
    pub brightness_threshold: bool,
    pub sharpen_enabled: bool,
    pub sharpen_sigma: f32,
    pub banding_enabled: bool,
    pub banding_radius: i32,
    pub banding_strength: f32,
    pub grain_enabled: bool,
    pub grain_variance: f32,
}

impl Default for VideoEssential {
    fn default() -> Self {
        Self {
            adjust_enabled: false,
            hue: 0.0,
            brightness: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            gamma: 1.0,
            brightness_threshold: false,
            sharpen_enabled: false,
            sharpen_sigma: 0.05,
            banding_enabled: false,
            banding_radius: 16,
            banding_strength: 1.2,
            grain_enabled: false,
            grain_variance: 2.0,
        }
    }
}

impl VideoEssential {
    pub const RANGE_HUE: (f32, f32) = (-180.0, 180.0);
    pub const RANGE_BRIGHTNESS: (f32, f32) = (0.0, 2.0);
    pub const RANGE_CONTRAST: (f32, f32) = (0.0, 2.0);
    pub const RANGE_SATURATION: (f32, f32) = (0.0, 3.0);
    pub const RANGE_GAMMA: (f32, f32) = (0.01, 10.0);
    pub const RANGE_SHARPEN: (f32, f32) = (0.0, 2.0);
    pub const RANGE_BANDING_RADIUS: (i32, i32) = (4, 32);
    pub const RANGE_GRAIN: (f32, f32) = (0.0, 10.0);

    pub fn is_neutral_adjust(&self) -> bool {
        self.hue == 0.0 && self.brightness == 1.0 && self.contrast == 1.0 && self.saturation == 1.0 && self.gamma == 1.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GradientMode {
    #[default]
    Gradient,
    Edge,
    Hough,
}

impl GradientMode {
    pub const ALL: [GradientMode; 3] = [GradientMode::Gradient, GradientMode::Edge, GradientMode::Hough];
    pub fn label(self) -> &'static str {
        match self {
            GradientMode::Gradient => "Gradient",
            GradientMode::Edge => "Edge",
            GradientMode::Hough => "Hough",
        }
    }
    pub fn vlc_name(self) -> &'static str {
        match self {
            GradientMode::Gradient => "gradient",
            GradientMode::Edge => "edge",
            GradientMode::Hough => "hough",
        }
    }
}

/// VLC's "Colors" effects tab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VideoColors {
    pub extract_enabled: bool,
    pub extract_color: u32,
    pub threshold_enabled: bool,
    pub threshold_color: u32,
    pub threshold_saturation: i32,
    pub threshold_similarity: i32,
    pub invert: bool,
    pub posterize_enabled: bool,
    pub posterize_level: i32,
    pub gradient_enabled: bool,
    pub gradient_mode: GradientMode,
    pub gradient_color: bool,
    pub gradient_cartoon: bool,
    pub sepia_enabled: bool,
    pub sepia_intensity: i32,
}

impl Default for VideoColors {
    fn default() -> Self {
        Self {
            extract_enabled: false,
            extract_color: 0xFF0000,
            threshold_enabled: false,
            threshold_color: 0xFF0000,
            threshold_saturation: 20,
            threshold_similarity: 15,
            invert: false,
            posterize_enabled: false,
            posterize_level: 6,
            gradient_enabled: false,
            gradient_mode: GradientMode::Gradient,
            gradient_color: false,
            gradient_cartoon: true,
            sepia_enabled: false,
            sepia_intensity: 120,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CropPixels {
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
    pub sync_vertical: bool,
    pub sync_horizontal: bool,
}

impl CropPixels {
    pub fn is_zero(&self) -> bool {
        self.top == 0 && self.bottom == 0 && self.left == 0 && self.right == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Transform {
    #[default]
    None,
    Rotate90,
    Rotate180,
    Rotate270,
    FlipHorizontal,
    FlipVertical,
    Transpose,
    AntiTranspose,
}

impl Transform {
    pub const ALL: [Transform; 8] = [
        Transform::None,
        Transform::Rotate90,
        Transform::Rotate180,
        Transform::Rotate270,
        Transform::FlipHorizontal,
        Transform::FlipVertical,
        Transform::Transpose,
        Transform::AntiTranspose,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Transform::None => "None",
            Transform::Rotate90 => "Rotate by 90 degrees",
            Transform::Rotate180 => "Rotate by 180 degrees",
            Transform::Rotate270 => "Rotate by 270 degrees",
            Transform::FlipHorizontal => "Flip horizontally",
            Transform::FlipVertical => "Flip vertically",
            Transform::Transpose => "Transpose",
            Transform::AntiTranspose => "Anti-transpose",
        }
    }
    pub fn vlc_type(self) -> Option<&'static str> {
        Some(match self {
            Transform::None => return None,
            Transform::Rotate90 => "90",
            Transform::Rotate180 => "180",
            Transform::Rotate270 => "270",
            Transform::FlipHorizontal => "hflip",
            Transform::FlipVertical => "vflip",
            Transform::Transpose => "transpose",
            Transform::AntiTranspose => "antitranspose",
        })
    }
}

/// Less common filters from VLC's "Geometry", "Overlay" and "Advanced" tabs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VideoExtras {
    pub transform: Transform,
    pub mirror: bool,
    pub psychedelic: bool,
    pub waves: bool,
    pub ripple: bool,
    pub motion_blur: bool,
    pub motion_blur_factor: i32,
    pub anaglyph: bool,
    pub denoise: bool,
    pub antiflicker: bool,
    pub marquee_enabled: bool,
    pub marquee_text: String,
    pub logo_enabled: bool,
    pub logo_file: String,
    pub logo_opacity: i32,
}

impl Default for VideoExtras {
    fn default() -> Self {
        Self {
            transform: Transform::None,
            mirror: false,
            psychedelic: false,
            waves: false,
            ripple: false,
            motion_blur: false,
            motion_blur_factor: 80,
            anaglyph: false,
            denoise: false,
            antiflicker: false,
            marquee_enabled: false,
            marquee_text: String::new(),
            logo_enabled: false,
            logo_file: String::new(),
            logo_opacity: 255,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VideoEffects {
    pub essential: VideoEssential,
    pub colors: VideoColors,
    pub crop: CropPixels,
    pub extras: VideoExtras,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DeinterlaceMode {
    Off,
    On,
    #[default]
    Automatic,
}

impl DeinterlaceMode {
    pub const ALL: [DeinterlaceMode; 3] = [DeinterlaceMode::Automatic, DeinterlaceMode::On, DeinterlaceMode::Off];
    pub fn label(self) -> &'static str {
        match self {
            DeinterlaceMode::Off => "Off",
            DeinterlaceMode::On => "On",
            DeinterlaceMode::Automatic => "Automatic",
        }
    }
}

/// VLC's deinterlace algorithms. `Automatic` lets VLC choose.
pub const DEINTERLACE_METHODS: &[(&str, &str)] = &[
    ("auto", "Automatic"),
    ("discard", "Discard"),
    ("blend", "Blend"),
    ("mean", "Mean"),
    ("bob", "Bob"),
    ("linear", "Linear"),
    ("x", "X"),
    ("yadif", "Yadif"),
    ("yadif2x", "Yadif (2x)"),
    ("phosphor", "Phosphor"),
    ("ivtc", "Film NTSC (IVTC)"),
];

/// Aspect ratios VLC offers by default; `None` means the source's own.
pub const ASPECT_RATIOS: &[&str] = &["1:1", "4:3", "5:4", "16:9", "16:10", "2.21:1", "2.35:1", "2.39:1"];
pub const CROP_RATIOS: &[&str] = &["16:10", "16:9", "4:3", "1.85:1", "2.21:1", "2.35:1", "2.39:1", "5:3", "5:4", "1:1"];

/// Zoom presets: label and scale (0 = fit to window).
pub const ZOOM_LEVELS: &[(&str, f32)] =
    &[("Fit", 0.0), ("1:4 Quarter", 0.25), ("1:2 Half", 0.5), ("1:1 Original", 1.0), ("2:1 Double", 2.0)];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_load_and_clean_negative_zero() {
        let mut eq = EqualizerSettings::default();
        let rock = EQ_PRESETS.iter().position(|p| p.0 == "Rock").unwrap();
        eq.apply_preset(rock);
        assert_eq!(eq.preset, Some(rock));
        assert_eq!(eq.preamp, 5.0);
        assert_eq!(eq.bands[0], 8.0);
        let classical = EQ_PRESETS.iter().position(|p| p.0 == "Classical").unwrap();
        eq.apply_preset(classical);
        assert_eq!(eq.bands[0], 0.0);
    }

    #[test]
    fn defaults_match_design_values() {
        let c = CompressorSettings::default();
        assert_eq!((c.rms_peak, c.attack_ms, c.release_ms, c.threshold_db), (0.2, 25.0, 100.0, -11.0));
        assert_eq!((c.ratio, c.knee_db, c.makeup_gain_db), (4.0, 5.0, 7.0));
        assert_eq!(StereoWidenerSettings::default().delay_ms, 20.0);
    }
}
