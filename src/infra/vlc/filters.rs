//! Translates effect settings into VLC filter chains and module variables.

use super::core::Var;
use crate::domain::effects::{AudioEffects, SpatialMode, VideoEffects};

/// The `audio-filter` chain and the variables its modules read.
pub fn audio(fx: &AudioEffects) -> (String, Vec<(&'static str, Var)>) {
    let mut chain = Vec::new();
    let mut vars: Vec<(&'static str, Var)> = Vec::new();

    let eq = &fx.equalizer;
    if eq.enabled {
        chain.push("equalizer");
        let bands = eq.bands.iter().map(|b| format!("{b:.1}")).collect::<Vec<_>>().join(" ");
        vars.push(("equalizer-vlcfreqs", Var::Bool(true)));
        vars.push(("equalizer-2pass", Var::Bool(eq.two_pass)));
        vars.push(("equalizer-preamp", Var::Float(eq.preamp)));
        vars.push(("equalizer-bands", Var::Str(bands)));
    }
    let c = &fx.compressor;
    if c.enabled {
        chain.push("compressor");
        vars.extend([
            ("compressor-rms-peak", Var::Float(c.rms_peak)),
            ("compressor-attack", Var::Float(c.attack_ms)),
            ("compressor-release", Var::Float(c.release_ms)),
            ("compressor-threshold", Var::Float(c.threshold_db)),
            ("compressor-ratio", Var::Float(c.ratio)),
            ("compressor-knee", Var::Float(c.knee_db)),
            ("compressor-makeup-gain", Var::Float(c.makeup_gain_db)),
        ]);
    }
    let s = &fx.spatializer;
    if s.enabled {
        chain.push("spatializer");
        vars.extend([
            ("spatializer-roomsize", Var::Float(s.room_size)),
            ("spatializer-width", Var::Float(s.width)),
            ("spatializer-wet", Var::Float(s.wet)),
            ("spatializer-dry", Var::Float(s.dry)),
            ("spatializer-damp", Var::Float(s.damp)),
        ]);
    }
    let w = &fx.stereo_widener;
    if w.enabled {
        chain.push("stereo_widen");
        vars.extend([
            ("stereowiden-delay", Var::Float(w.delay_ms)),
            ("stereowiden-feedback", Var::Float(w.feedback)),
            ("stereowiden-crossfeed", Var::Float(w.crossfeed)),
            ("stereowiden-dry-mix", Var::Float(w.dry_mix)),
        ]);
    }
    if fx.spatial_audio {
        match fx.spatial_mode {
            SpatialMode::HeadphoneVirtualization => chain.push("headphone"),
            SpatialMode::Ambisonics => chain.push("spatialaudio"),
            SpatialMode::DolbySurroundDecoder => chain.push("dolby_surround_decoder"),
        }
    }
    if fx.karaoke {
        chain.push("karaoke");
    }
    if fx.chorus_flanger {
        chain.push("chorus_flanger");
    }
    if fx.normalize {
        chain.push("normvol");
        vars.push(("norm-max-level", Var::Float(fx.normalize_level)));
    }
    if fx.pitch_shift.abs() > 0.001 {
        chain.push("scaletempo_pitch");
        vars.push(("pitch-shift", Var::Float(fx.pitch_shift)));
    }
    (chain.join(":"), vars)
}

/// The `video-filter` chain, the `sub-source` chain and module variables.
pub fn video(fx: &VideoEffects) -> (String, Vec<(&'static str, Var)>) {
    let mut chain = Vec::new();
    let mut vars: Vec<(&'static str, Var)> = Vec::new();
    let e = &fx.essential;
    if e.adjust_enabled || !e.is_neutral_adjust() {
        chain.push("adjust");
        vars.extend([
            ("hue", Var::Float(e.hue)),
            ("brightness", Var::Float(e.brightness)),
            ("contrast", Var::Float(e.contrast)),
            ("saturation", Var::Float(e.saturation)),
            ("gamma", Var::Float(e.gamma)),
            ("brightness-threshold", Var::Bool(e.brightness_threshold)),
        ]);
    }
    if e.sharpen_enabled {
        chain.push("sharpen");
        vars.push(("sharpen-sigma", Var::Float(e.sharpen_sigma)));
    }
    if e.banding_enabled {
        chain.push("gradfun");
        vars.push(("gradfun-radius", Var::Int(e.banding_radius as i64)));
        vars.push(("gradfun-strength", Var::Float(e.banding_strength)));
    }
    if e.grain_enabled {
        chain.push("grain");
        vars.push(("grain-variance", Var::Float(e.grain_variance)));
    }
    let c = &fx.colors;
    if c.extract_enabled {
        chain.push("extract");
        vars.push(("extract-component", Var::Int(c.extract_color as i64)));
    }
    if c.threshold_enabled {
        chain.push("colorthres");
        vars.push(("colorthres-color", Var::Int(c.threshold_color as i64)));
        vars.push(("colorthres-saturationthres", Var::Int(c.threshold_saturation as i64)));
        vars.push(("colorthres-similaritythres", Var::Int(c.threshold_similarity as i64)));
    }
    if c.invert {
        chain.push("invert");
    }
    if c.posterize_enabled {
        chain.push("posterize");
        vars.push(("posterize-level", Var::Int(c.posterize_level as i64)));
    }
    if c.gradient_enabled {
        chain.push("gradient");
        vars.push(("gradient-mode", Var::Str(c.gradient_mode.vlc_name().into())));
        vars.push(("gradient-type", Var::Int(c.gradient_color as i64)));
        vars.push(("gradient-cartoon", Var::Bool(c.gradient_cartoon)));
    }
    if c.sepia_enabled {
        chain.push("sepia");
        vars.push(("sepia-intensity", Var::Int(c.sepia_intensity as i64)));
    }
    let x = &fx.extras;
    if let Some(t) = x.transform.vlc_type() {
        chain.push("transform");
        vars.push(("transform-type", Var::Str(t.into())));
    }
    if x.mirror {
        chain.push("mirror");
    }
    if x.psychedelic {
        chain.push("psychedelic");
    }
    if x.waves {
        chain.push("wave");
    }
    if x.ripple {
        chain.push("ripple");
    }
    if x.motion_blur {
        chain.push("motionblur");
        vars.push(("blur-factor", Var::Int(x.motion_blur_factor as i64)));
    }
    if x.anaglyph {
        chain.push("anaglyph");
    }
    if x.denoise {
        chain.push("hqdn3d");
    }
    if x.antiflicker {
        chain.push("antiflicker");
    }
    (chain.join(":"), vars)
}

/// The `sub-source` chain for overlays (marquee, logo).
pub fn sub_sources(fx: &VideoEffects) -> (String, Vec<(&'static str, Var)>) {
    let mut chain = Vec::new();
    let mut vars = Vec::new();
    let x = &fx.extras;
    if x.marquee_enabled && !x.marquee_text.is_empty() {
        chain.push("marq");
        vars.push(("marq-marquee", Var::Str(x.marquee_text.clone())));
    }
    if x.logo_enabled && !x.logo_file.is_empty() {
        chain.push("logo");
        vars.push(("logo-file", Var::Str(x.logo_file.clone())));
        vars.push(("logo-opacity", Var::Int(x.logo_opacity as i64)));
    }
    (chain.join(":"), vars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_by_default() {
        assert_eq!(audio(&AudioEffects::default()).0, "");
        assert_eq!(video(&VideoEffects::default()).0, "");
    }

    #[test]
    fn audio_chain_order_and_vars() {
        let mut fx = AudioEffects::default();
        fx.equalizer.enabled = true;
        fx.compressor.enabled = true;
        fx.pitch_shift = 2.0;
        let (chain, vars) = audio(&fx);
        assert_eq!(chain, "equalizer:compressor:scaletempo_pitch");
        assert!(
            vars.iter()
                .any(|(k, v)| *k == "equalizer-bands"
                    && *v == Var::Str("0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0".into()))
        );
        assert!(vars.iter().any(|(k, _)| *k == "compressor-knee"));
    }

    #[test]
    fn adjust_turns_on_when_values_move() {
        let mut fx = VideoEffects::default();
        fx.essential.contrast = 1.5;
        fx.colors.sepia_enabled = true;
        let (chain, _) = video(&fx);
        assert_eq!(chain, "adjust:sepia");
    }
}
