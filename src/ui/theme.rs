//! Colour tokens from the design's Dark/* and Light/* styles, and the egui
//! style built from them.

use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, Visuals};

use crate::domain::settings::{Accent, Theme};

/// Named after the Figma colour styles so screens can be checked against
/// the design token by token.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub dark: bool,
    /// FG/1: primary text on dark.
    pub fg1: Color32,
    /// FG/2: bright secondary text.
    pub fg2: Color32,
    /// FG/A1..A4: dimmer text and icons.
    pub fg_a1: Color32,
    pub fg_a2: Color32,
    pub fg_a3: Color32,
    pub fg_a4: Color32,
    pub fg_b1: Color32,
    /// BG/A1..A5: raised surfaces.
    pub bg_a1: Color32,
    pub bg_a2: Color32,
    pub bg_a3: Color32,
    pub bg_a4: Color32,
    pub bg_a5: Color32,
    /// BG/B1, B2: sunken surfaces (inputs, tracks).
    pub bg_b1: Color32,
    pub bg_b2: Color32,
    pub bg_a3_t: Color32,
    pub bg_b1_t: Color32,
    pub bg_b2_t: Color32,
    pub accent: Color32,
    pub accent_bright: Color32,
    pub accent_dim: Color32,
    pub accent_logo: Color32,
    /// BG/Accent Bright: the dark tint behind accent content (slider fill).
    pub bg_accent: Color32,
    pub bg_accent_dark: Color32,
    pub divider_bright: Color32,
    pub divider_dim: Color32,
    /// Title bar and window chrome.
    pub chrome: Color32,
    /// Area behind the video.
    pub stage: Color32,
    pub hover: Color32,
    pub danger: Color32,
}

fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

fn hexa(v: u32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied((v >> 16) as u8, (v >> 8) as u8, v as u8, (a * 255.0) as u8)
}

fn to_hsv(c: Color32) -> (f32, f32, f32) {
    let (r, g, b) = (c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, if max == 0.0 { 0.0 } else { d / max }, max)
}

fn from_hsv(h: f32, s: f32, v: f32) -> Color32 {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let f = |v: f32| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgb(f(r), f(g), f(b))
}

fn scale(c: Color32, k: f32) -> Color32 {
    let f = |v: u8| ((v as f32 * k).round().clamp(0.0, 255.0)) as u8;
    Color32::from_rgb(f(c.r()), f(c.g()), f(c.b()))
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()), f(a.a(), b.a()))
}

pub fn with_alpha(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (a.clamp(0.0, 1.0) * 255.0) as u8)
}

impl Palette {
    pub fn dark(accent: Accent) -> Self {
        let a = hex(accent.hex());
        let (h, s, v) = to_hsv(a);
        let (accent_bright, accent_dim, bg_accent, bg_accent_dark) = if accent == Accent::Orange {
            (hex(0xFC9729), hex(0xAB6314), hex(0x371F06), hex(0x1A1611))
        } else {
            (
                from_hsv(h, (s * 0.84).min(1.0), (v * 1.19).min(1.0)),
                scale(a, 0.81),
                scale(a, 0.26),
                mix(hex(0x161616), a, 0.04),
            )
        };
        Self {
            dark: true,
            fg1: hex(0xEAEAEA),
            fg2: hex(0xC6C6C6),
            fg_a1: hex(0x969696),
            fg_a2: hex(0x8F8F8F),
            fg_a3: hex(0x6B6B6B),
            fg_a4: hex(0x404040),
            fg_b1: hex(0x161616),
            bg_a1: hex(0x222222),
            bg_a2: hex(0x1F1F1F),
            bg_a3: hex(0x1B1B1B),
            bg_a4: hex(0x191919),
            bg_a5: hex(0x151515),
            bg_b1: hex(0x0E0E0E),
            bg_b2: hex(0x090909),
            bg_a3_t: hexa(0x191919, 0.95),
            bg_b1_t: hexa(0x0E0E0E, 0.95),
            bg_b2_t: hexa(0x000000, 0.8),
            accent: a,
            accent_bright,
            accent_dim,
            accent_logo: a,
            bg_accent,
            bg_accent_dark,
            divider_bright: hex(0x373737),
            divider_dim: hex(0x222222),
            chrome: hex(0x161616),
            stage: hex(0x2A2928),
            hover: hexa(0xFFFFFF, 0.06),
            danger: hex(0xFF3737),
        }
    }

    pub fn light(accent: Accent) -> Self {
        let a = if accent == Accent::Orange { hex(0xE8851A) } else { hex(accent.hex()) };
        let (h, s, v) = to_hsv(a);
        let (accent_bright, accent_dim, accent_logo, bg_accent, bg_accent_dark) = if accent == Accent::Orange {
            (hex(0xFFA13B), hex(0x965916), hex(0xC2711A), hex(0xD8BA9A), hex(0xC6AB89))
        } else {
            (
                from_hsv(h, (s * 0.8).min(1.0), (v * 1.1).min(1.0)),
                scale(a, 0.65),
                scale(a, 0.84),
                mix(hex(0xEDEDED), a, 0.3),
                mix(hex(0xDDDDDD), a, 0.35),
            )
        };
        Self {
            dark: false,
            fg1: hex(0x161616),
            fg2: hex(0x2E2E2E),
            fg_a1: hex(0x3C3C3C),
            fg_a2: hex(0x2E2E2E),
            fg_a3: hex(0x6B6B6B),
            fg_a4: hex(0x9B9B9B),
            fg_b1: hex(0xEAEAEA),
            bg_a1: hex(0xEDEDED),
            bg_a2: hex(0xDDDDDD),
            bg_a3: hex(0xD5D5D5),
            bg_a4: hex(0xB3B3B3),
            bg_a5: hex(0xE2E2E2),
            bg_b1: hex(0xE7E7E7),
            bg_b2: hex(0xDEDEDE),
            bg_a3_t: hexa(0xD5D5D5, 0.75),
            bg_b1_t: hexa(0xE7E7E7, 0.8),
            bg_b2_t: hexa(0xFFFFFF, 0.8),
            accent: a,
            accent_bright,
            accent_dim,
            accent_logo,
            bg_accent,
            bg_accent_dark,
            divider_bright: hex(0x9B9B9B),
            divider_dim: hex(0xACACAC),
            chrome: hex(0xDDDDDD),
            stage: hex(0xCFCFCF),
            hover: hexa(0x000000, 0.06),
            danger: hex(0xD92D2D),
        }
    }

    pub fn for_settings(theme: Theme, accent: Accent, system_dark: bool) -> Self {
        match theme {
            Theme::Dark => Self::dark(accent),
            Theme::Light => Self::light(accent),
            Theme::System if system_dark => Self::dark(accent),
            Theme::System => Self::light(accent),
        }
    }

    /// The volume meter's gradient: green, yellow, accent orange, red.
    pub fn volume_stops(&self) -> [(f32, Color32); 4] {
        [(0.0, hex(0x95D318)), (0.579, hex(0xB2A918)), (0.792, hex(0xD37918)), (1.0, hex(0xFF3737))]
    }

    pub fn apply(&self, ctx: &egui::Context) {
        let mut v = if self.dark { Visuals::dark() } else { Visuals::light() };
        v.panel_fill = self.bg_b1;
        v.window_fill = self.bg_a4;
        v.extreme_bg_color = self.bg_b2;
        v.faint_bg_color = self.bg_a5;
        v.code_bg_color = self.bg_b2;
        v.window_stroke = Stroke::new(1.0, self.divider_dim);
        v.window_corner_radius = CornerRadius::same(14);
        v.menu_corner_radius = CornerRadius::same(12);
        v.selection.bg_fill = with_alpha(self.accent, 0.45);
        v.selection.stroke = Stroke::new(1.0, self.accent);
        v.hyperlink_color = self.accent_bright;
        v.override_text_color = Some(self.fg_a1);
        v.popup_shadow =
            egui::epaint::Shadow { offset: [0, 8], blur: 30, spread: 0, color: Color32::from_black_alpha(118) };
        v.window_shadow = v.popup_shadow;
        v.text_cursor.stroke = Stroke::new(1.5, self.accent);
        for (w, fill) in [
            (&mut v.widgets.noninteractive, self.bg_a4),
            (&mut v.widgets.inactive, self.bg_b2),
            (&mut v.widgets.hovered, self.bg_a1),
            (&mut v.widgets.active, self.bg_a1),
            (&mut v.widgets.open, self.bg_a1),
        ] {
            w.bg_fill = fill;
            w.weak_bg_fill = fill;
            w.corner_radius = CornerRadius::same(6);
            w.bg_stroke = Stroke::NONE;
            w.fg_stroke = Stroke::new(1.0, self.fg_a1);
            w.expansion = 0.0;
        }
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, self.divider_dim);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.fg_a2);
        v.widgets.hovered.fg_stroke = Stroke::new(1.0, self.fg1);
        v.widgets.active.fg_stroke = Stroke::new(1.0, self.fg1);
        v.interact_cursor = Some(egui::CursorIcon::PointingHand);
        ctx.set_visuals(v);
        ctx.style_mut(|s| {
            s.spacing.item_spacing = egui::vec2(8.0, 8.0);
            s.spacing.button_padding = egui::vec2(10.0, 6.0);
            s.spacing.interact_size = egui::vec2(24.0, 24.0);
            s.spacing.scroll = egui::style::ScrollStyle {
                floating: true,
                bar_width: 6.0,
                floating_width: 2.0,
                floating_allocated_width: 0.0,
                handle_min_length: 24.0,
                bar_inner_margin: 2.0,
                bar_outer_margin: 2.0,
                ..egui::style::ScrollStyle::floating()
            };
            s.text_styles.insert(egui::TextStyle::Body, FontId::new(14.0, FontFamily::Proportional));
            s.text_styles.insert(egui::TextStyle::Button, FontId::new(14.0, FontFamily::Proportional));
            s.text_styles.insert(egui::TextStyle::Small, FontId::new(12.0, FontFamily::Proportional));
            s.text_styles.insert(egui::TextStyle::Heading, FontId::new(16.0, FontFamily::Name("medium".into())));
            s.text_styles.insert(egui::TextStyle::Monospace, FontId::new(14.0, FontFamily::Monospace));
            s.animation_time = 0.12;
        });
    }
}

/// Font helpers matching the design's text styles.
pub mod font {
    use egui::{FontFamily, FontId};

    pub fn regular(size: f32) -> FontId {
        FontId::new(size, FontFamily::Proportional)
    }
    pub fn light(size: f32) -> FontId {
        FontId::new(size, FontFamily::Name("light".into()))
    }
    pub fn medium(size: f32) -> FontId {
        FontId::new(size, FontFamily::Name("medium".into()))
    }
    pub fn semibold(size: f32) -> FontId {
        FontId::new(size, FontFamily::Name("semibold".into()))
    }
    pub fn bold(size: f32) -> FontId {
        FontId::new(size, FontFamily::Name("bold".into()))
    }
    pub fn mono(size: f32) -> FontId {
        FontId::new(size, FontFamily::Monospace)
    }
}

/// Loads Inter (Light, Regular, Medium, SemiBold, Bold) and DM Mono.
/// System fonts that cover scripts Inter lacks (Japanese, Chinese, Korean
/// and everything else), as (name, regular-weight path, bold-weight path),
/// in fallback order. Missing files are skipped.
fn system_fallbacks() -> Vec<(&'static str, &'static str, &'static str)> {
    if cfg!(target_os = "macos") {
        vec![
            // Hiragino Sans: Japanese kana and kanji, also good for Chinese.
            (
                "jp",
                "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
                "/System/Library/Fonts/ヒラギノ角ゴシック W6.ttc",
            ),
            (
                "kr",
                "/System/Library/Fonts/Supplemental/AppleGothic.ttf",
                "/System/Library/Fonts/Supplemental/AppleGothic.ttf",
            ),
            ("zh", "/System/Library/Fonts/Hiragino Sans GB.ttc", "/System/Library/Fonts/Hiragino Sans GB.ttc"),
            (
                "uni",
                "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
                "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
            ),
        ]
    } else if cfg!(windows) {
        vec![
            ("jp", "C:\\Windows\\Fonts\\YuGothR.ttc", "C:\\Windows\\Fonts\\YuGothB.ttc"),
            ("kr", "C:\\Windows\\Fonts\\malgun.ttf", "C:\\Windows\\Fonts\\malgunbd.ttf"),
            ("zh", "C:\\Windows\\Fonts\\msyh.ttc", "C:\\Windows\\Fonts\\msyhbd.ttc"),
            ("uni", "C:\\Windows\\Fonts\\seguisym.ttf", "C:\\Windows\\Fonts\\seguisym.ttf"),
        ]
    } else {
        vec![
            (
                "cjk",
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc",
            ),
            (
                "uni",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
            ),
        ]
    }
}

/// Loads Inter (Light, Regular, Medium, SemiBold, Bold) and DM Mono, with
/// system fonts behind them for Japanese, Chinese, Korean and other scripts.
pub fn install_fonts(ctx: &egui::Context) {
    use std::sync::Arc;
    let mut fonts = egui::FontDefinitions::default();
    let data: [(&str, &'static [u8]); 6] = [
        ("Inter-Light", include_bytes!("../../assets/fonts/Inter-Light.ttf")),
        ("Inter-Regular", include_bytes!("../../assets/fonts/Inter-Regular.ttf")),
        ("Inter-Medium", include_bytes!("../../assets/fonts/Inter-Medium.ttf")),
        ("Inter-SemiBold", include_bytes!("../../assets/fonts/Inter-SemiBold.ttf")),
        ("Inter-Bold", include_bytes!("../../assets/fonts/Inter-Bold.ttf")),
        ("DMMono", include_bytes!("../../assets/fonts/DMMono-Regular.ttf")),
    ];
    for (name, bytes) in data {
        fonts.font_data.insert(name.into(), Arc::new(egui::FontData::from_static(bytes)));
    }
    // System fallbacks, regular and bold weights. A file shared by both
    // weights is loaded once.
    let mut regular_fallbacks = Vec::new();
    let mut bold_fallbacks = Vec::new();
    let mut loaded: std::collections::HashMap<&str, String> = std::collections::HashMap::new();
    let mut load = |key: String, path: &'static str, fonts: &mut egui::FontDefinitions| -> Option<String> {
        if let Some(k) = loaded.get(path) {
            return Some(k.clone());
        }
        let bytes = std::fs::read(path).ok()?;
        // Hiragino and friends sit a little high next to Inter.
        let tweak = egui::FontTweak { y_offset_factor: 0.12, ..Default::default() };
        fonts.font_data.insert(key.clone(), Arc::new(egui::FontData::from_owned(bytes).tweak(tweak)));
        loaded.insert(path, key.clone());
        Some(key)
    };
    for (name, regular, bold) in system_fallbacks() {
        if let Some(k) = load(format!("sys-{name}"), regular, &mut fonts) {
            regular_fallbacks.push(k);
        }
        if let Some(k) = load(format!("sys-{name}-bold"), bold, &mut fonts) {
            bold_fallbacks.push(k);
        }
    }
    // egui's own defaults stay last, for emoji and symbols.
    let defaults: Vec<String> = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let family = |main: &str, bold: bool| {
        let mut v = vec![main.to_string()];
        v.extend(if bold { bold_fallbacks.iter() } else { regular_fallbacks.iter() }.cloned());
        v.extend(defaults.iter().cloned());
        v
    };
    fonts.families.insert(FontFamily::Proportional, family("Inter-Regular", false));
    fonts.families.insert(FontFamily::Name("light".into()), family("Inter-Light", false));
    fonts.families.insert(FontFamily::Name("medium".into()), family("Inter-Medium", false));
    fonts.families.insert(FontFamily::Name("semibold".into()), family("Inter-SemiBold", true));
    fonts.families.insert(FontFamily::Name("bold".into()), family("Inter-Bold", true));
    let mut mono = vec!["DMMono".to_string()];
    mono.extend(regular_fallbacks.iter().cloned());
    mono.extend(fonts.families.get(&FontFamily::Monospace).cloned().unwrap_or_default());
    fonts.families.insert(FontFamily::Monospace, mono);
    ctx.set_fonts(fonts);
}
