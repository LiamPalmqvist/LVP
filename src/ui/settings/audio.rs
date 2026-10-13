//! The Audio page, with the startup volume choice and the equalizer.

use egui::{Align, Align2, CornerRadius, Rect, Sense, TextEdit, Ui, Vec2, pos2, vec2};

use super::Cx;
use super::kit::{self, Form, SliderSpec};
use super::media::track_languages;
use crate::domain::effects::{
    CompressorSettings, EQ_BAND_LABELS, EQ_PRESETS, EQ_RANGE, EqualizerSettings, SpatialMode,
};
use crate::domain::settings::*;
use crate::ui::icons::Icon;
use crate::ui::theme::{font, with_alpha};
use crate::ui::widgets::{self, paint_icon, pal};

/// The gentle compression night mode applies.
fn night_compressor() -> CompressorSettings {
    CompressorSettings {
        enabled: true,
        rms_peak: 0.0,
        attack_ms: 20.0,
        release_ms: 300.0,
        threshold_db: -22.0,
        ratio: 4.0,
        knee_db: 4.0,
        makeup_gain_db: 8.0,
    }
}

pub fn audio(f: &mut Form, s: &mut Settings, cx: &mut Cx) {
    let a = &mut s.audio;
    f.section("Output");
    f.toggle("Enable audio", Some("Turn off to play video without sound."), &mut a.enabled);
    let devices = if f.drawing() { cx.audio_devices() } else { Vec::new() };
    let mut names: Vec<String> = vec!["System default".into()];
    names.extend(devices.iter().map(|d| d.name.clone()));
    let cur = a
        .output_device
        .as_ref()
        .and_then(|id| devices.iter().find(|d| &d.id == id))
        .map(|d| d.name.clone())
        .unwrap_or_else(|| "System default".into());
    if let Some(i) = f.dropdown("Output device", Some("The speakers or headphones sound plays through."), &cur, &names)
    {
        let id = if i == 0 { None } else { devices.get(i - 1).map(|d| d.id.clone()) };
        cx.later(move |app, _| app.set_audio_device(id));
    }
    f.choice(
        "Speaker setup",
        Some("How channels are sent to your speakers. Automatic keeps the file's own layout."),
        &mut a.speaker_setup,
        SpeakerSetup::ALL,
        SpeakerSetup::label,
    );

    f.section("Volume");
    let mut max = a.max_volume as f32;
    let spec = SliderSpec::new().ticks(20).step(5.0).labels(&[(100.0, "100%"), (200.0, "200%"), (300.0, "300%")]);
    if f.slider("Maximum volume", None, &mut max, 100.0..=300.0, &spec) {
        a.max_volume = max.round() as i32;
    }
    f.row("Startup volume", Some("Start at the volume you used last, or always at the same level."), 88.0, |ui| {
        startup_volume(ui, &mut a.remember_volume, &mut a.startup_volume)
    });
    f.int(
        "Volume step",
        Some("How much the volume up and down shortcuts change the volume, in percent."),
        &mut a.volume_step,
        1..=50,
        "%",
    );

    f.section("Loudness");
    let fx = &mut a.effects;
    f.toggle(
        "Normalize loudness",
        Some("Evens out the volume so quiet and loud files sound alike."),
        &mut fx.normalize,
    );
    let mut level = fx.normalize_level as f64;
    if f.number(
        "Target level",
        Some("How loud normalised audio may get. Lower is quieter and more even."),
        &mut level,
        0.0..=10.0,
        1,
        "",
    ) {
        fx.normalize_level = level as f32;
    }
    if f.toggle(
        "Night mode",
        Some("Softens loud sounds and lifts quiet ones, for listening at low volume."),
        &mut a.night_mode,
    ) {
        fx.compressor = if a.night_mode { night_compressor() } else { CompressorSettings::default() };
    }
    f.choice(
        "Replay gain",
        Some("Use the loudness information stored in music files to play every track at the same volume."),
        &mut a.replay_gain,
        ReplayGain::ALL,
        ReplayGain::label,
    );
    let mut pre = a.replay_gain_preamp as f64;
    if f.number(
        "Replay gain preamp",
        Some("Extra gain applied on top of replay gain, in decibels."),
        &mut pre,
        -20.0..=20.0,
        1,
        "dB",
    ) {
        a.replay_gain_preamp = pre as f32;
    }

    f.section("Effects");
    let eq = &mut fx.equalizer;
    f.toggle("Equalizer", Some("Shape the sound by boosting or cutting bass, mid and treble."), &mut eq.enabled);
    let mut presets: Vec<String> = vec!["None".into()];
    presets.extend(EQ_PRESETS.iter().map(|(n, _, _)| n.to_string()));
    let cur = eq.preset.and_then(|i| EQ_PRESETS.get(i)).map(|(n, _, _)| n.to_string()).unwrap_or_else(|| "None".into());
    if let Some(i) = f.dropdown(
        "Equalizer preset",
        Some("Ready-made equalizer curves for different kinds of music."),
        &cur,
        &presets,
    ) {
        if i == 0 {
            eq.preset = None;
        } else {
            eq.apply_preset(i - 1);
            eq.enabled = true;
        }
    }
    let eq_open = &mut cx.ss.eq_open;
    f.block("equalizer bands preamp hz khz db", |ui| equalizer(ui, eq, eq_open));
    f.toggle("Two-pass", Some("Runs the equalizer twice for a stronger effect."), &mut eq.two_pass);
    f.toggle("Spatial audio", Some("Makes stereo and surround sound more spacious."), &mut fx.spatial_audio);
    let cur = fx.spatial_mode.label();
    let modes: Vec<String> = SpatialMode::ALL.iter().map(|m| m.label().to_string()).collect();
    if let Some(i) = f.dropdown("Spatial audio mode", Some("How the spatial effect is produced."), cur, &modes) {
        fx.spatial_mode = SpatialMode::ALL[i];
    }
    f.toggle(
        "Stereo widening",
        Some("Makes stereo sound wider than the speakers are apart."),
        &mut fx.stereo_widener.enabled,
    );
    f.toggle("Compressor", Some("Reduces the difference between loud and quiet passages."), &mut fx.compressor.enabled);
    let mut pitch = fx.pitch_shift as f64;
    if f.number(
        "Pitch shift",
        Some("Raises or lowers the pitch, in semitones, without changing the speed."),
        &mut pitch,
        -12.0..=12.0,
        2,
        "",
    ) {
        fx.pitch_shift = pitch as f32;
    }
    f.toggle("Chorus/flanger", Some("Adds a sweeping, doubled sound."), &mut fx.chorus_flanger);
    f.toggle("Karaoke", Some("Removes the vocals that are mixed in the centre of a stereo track."), &mut fx.karaoke);

    f.section("Tracks");
    let langs = track_languages();
    f.pick(
        "Preferred audio language",
        Some("Pick the audio track in this language when a file has several."),
        &mut a.preferred_language,
        &langs,
    );
    let mut delay = a.delay_ms as f64 / 1000.0;
    if f.number(
        "Audio delay",
        Some("Shifts the sound earlier (negative) or later (positive) to match the picture, in seconds."),
        &mut delay,
        -60.0..=60.0,
        2,
        "s",
    ) {
        a.delay_ms = (delay * 1000.0).round() as i64;
    }

    f.section("Visualization");
    f.choice(
        "Visualizer",
        Some("An animation that moves with the music when there is no video."),
        &mut a.visualizer,
        Visualizer::ALL,
        Visualizer::label,
    );

    f.advanced(|f| {
        f.toggle(
            "Pass compressed audio to the receiver",
            Some("Send Dolby and DTS sound untouched to an AV receiver over HDMI or optical cable."),
            &mut a.passthrough,
        );
        f.choice(
            "Detect Dolby Surround",
            Some("Whether Dolby Surround encoded stereo is decoded to surround."),
            &mut a.dolby_surround,
            DolbyDetect::ALL,
            DolbyDetect::label,
        );
        f.choice(
            "Resampling quality",
            Some("Quality of sample rate conversion. Best uses more processor time."),
            &mut a.resample_quality,
            ResampleQuality::ALL,
            ResampleQuality::label,
        );
        f.choice(
            "Media role",
            Some("Tells the system what kind of sound this is, so it can manage volume and interruptions."),
            &mut a.media_role,
            MediaRole::ALL,
            MediaRole::label,
        );
        f.choice(
            "Visualization size",
            Some("The size of the visualizer picture."),
            &mut a.visual_size,
            VisualSize::ALL,
            VisualSize::label,
        );
    });
}

/// The vertical "Values" control: remember the last volume, or always start
/// at a fixed level.
fn startup_volume(ui: &mut Ui, remember: &mut bool, level: &mut i32) {
    let p = pal(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(kit::CONTROL_W, 78.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(6), p.bg_b2);
    let r1 = Rect::from_min_size(rect.min + vec2(2.0, 2.0), vec2(rect.width() - 4.0, 36.0));
    let r2 = Rect::from_min_size(rect.min + vec2(2.0, 40.0), vec2(rect.width() - 4.0, 36.0));
    let field = Rect::from_min_size(pos2(r2.max.x - 184.0, r2.min.y + 2.0), vec2(182.0, 32.0));
    let a = ui.interact(r1, ui.id().with("remember"), Sense::click());
    let b = ui.interact(r2, ui.id().with("fixed"), Sense::click());
    if a.clicked() {
        *remember = true;
    }
    if b.clicked() {
        *remember = false;
    }
    for (r, resp, on, text) in [(r1, &a, *remember, "Remember last volume"), (r2, &b, !*remember, "Always starts at:")]
    {
        let bg = if on {
            p.bg_accent
        } else if resp.hovered() {
            p.bg_a2
        } else {
            p.bg_a4
        };
        ui.painter().rect_filled(r, CornerRadius::same(4), bg);
        ui.painter().text(
            pos2(r.min.x + 12.0, r.center().y),
            Align2::LEFT_CENTER,
            text,
            font::regular(14.0),
            if on { p.accent } else { p.fg_a3 },
        );
    }
    ui.painter().rect_filled(field, CornerRadius::same(2), p.bg_b2);
    let id = ui.id().with("startup-level");
    let buf_id = id.with("buf");
    let focused = ui.memory(|m| m.has_focus(id));
    let mut text = if focused {
        ui.data(|d| d.get_temp::<String>(buf_id)).unwrap_or_else(|| format!("{level}%"))
    } else {
        format!("{level}%")
    };
    let col = if *remember { with_alpha(p.fg_a2, 0.4) } else { p.fg_a2 };
    let inner = field.shrink2(vec2(12.0, 0.0));
    let er = ui.put(
        inner,
        TextEdit::singleline(&mut text)
            .id(id)
            .frame(false)
            .text_color(col)
            .font(font::regular(14.0))
            .vertical_align(Align::Center)
            .margin(egui::Margin::ZERO)
            .desired_width(inner.width()),
    );
    if er.gained_focus() {
        *remember = false;
    }
    if er.changed()
        && let Ok(n) = text.trim().trim_end_matches('%').trim().parse::<i32>()
    {
        *level = n.clamp(0, 300);
    }
    if er.has_focus() {
        ui.data_mut(|d| d.insert_temp(buf_id, text));
    } else {
        ui.data_mut(|d| d.remove::<String>(buf_id));
    }
    if er.hovered() || b.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll.abs() > 0.5 {
            *level = (*level + 5 * scroll.signum() as i32).clamp(0, 300);
            *remember = false;
        }
    }
}

/// "Open Equalizer" bar and, when open, the preamp and ten band sliders.
fn equalizer(ui: &mut Ui, eq: &mut EqualizerSettings, open: &mut bool) {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    ui.add_space(5.0);
    let (bar, resp) = ui.allocate_exact_size(vec2(w, 36.0), Sense::click());
    let col = if resp.hovered() { p.fg1 } else { p.fg_a2 };
    let label = if *open { "Close Equalizer" } else { "Open Equalizer" };
    let galley = ui.painter().layout_no_wrap(label.into(), font::regular(14.0), col);
    let total = galley.size().x + 10.0 + 24.0;
    let x = bar.center().x - total / 2.0;
    let gw = galley.size().x;
    ui.painter().galley(pos2(x, bar.center().y - galley.size().y / 2.0), galley, col);
    let chev = if *open { Icon::ChevronUp } else { Icon::ChevronDown };
    paint_icon(ui, Rect::from_min_size(pos2(x + gw + 10.0, bar.center().y - 12.0), Vec2::splat(24.0)), chev, col);
    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        *open = !*open;
    }
    if !*open {
        ui.add_space(5.0);
        return;
    }
    ui.add_space(24.0);
    let (area, _) = ui.allocate_exact_size(vec2(w, 415.0), Sense::hover());
    let n = EQ_BAND_LABELS.len() + 1;
    let col_w = 60.0;
    let gap = 8.0;
    let x0 = area.center().x - (n as f32 * col_w + (n - 1) as f32 * gap) / 2.0;
    let mut changed_band = false;
    for i in 0..n {
        let x = x0 + i as f32 * (col_w + gap);
        let slider_rect = Rect::from_min_size(pos2(x, area.min.y), vec2(col_w, 320.0));
        let (label, value): (&str, &mut f32) =
            if i == 0 { ("Preamp", &mut eq.preamp) } else { (EQ_BAND_LABELS[i - 1], &mut eq.bands[i - 1]) };
        let r = kit::place(ui, egui::UiBuilder::new().max_rect(slider_rect), |ui| {
            widgets::vslider(ui, value, EQ_RANGE.0..=EQ_RANGE.1, vec2(col_w, 320.0))
        });
        if r.changed() && i > 0 {
            changed_band = true;
        }
        let r = r.on_hover_text(format!("{label}: {:+.1} dB. Double-click to reset.", *value));
        if r.hovered() {
            let scroll = ui.input(|inp| inp.raw_scroll_delta.y);
            if scroll.abs() > 0.5 {
                *value = (*value + 0.5 * scroll.signum()).clamp(EQ_RANGE.0, EQ_RANGE.1);
                if i > 0 {
                    changed_band = true;
                }
            }
        }
        let lc = if i == 0 { p.fg1 } else { p.fg_a3 };
        ui.painter().text(
            pos2(x + col_w / 2.0, slider_rect.max.y + 33.0),
            Align2::CENTER_TOP,
            label,
            font::regular(14.0),
            lc,
        );
        let boxr = Rect::from_min_size(pos2(x, slider_rect.max.y + 62.0), vec2(col_w, 32.0));
        db_box(ui, boxr, value, i);
    }
    if changed_band {
        eq.preset = None;
    }
    ui.add_space(6.0);
}

/// The "0.0 dB" box under each equalizer slider; type a value to set it.
fn db_box(ui: &mut Ui, rect: Rect, value: &mut f32, i: usize) {
    let p = pal(ui.ctx());
    ui.painter().rect_filled(rect, CornerRadius::same(6), p.bg_b2);
    let id = ui.id().with(("eq-db", i));
    let buf_id = id.with("buf");
    let focused = ui.memory(|m| m.has_focus(id));
    let fmt = format!("{:.1} dB", *value);
    let mut text = if focused { ui.data(|d| d.get_temp::<String>(buf_id)).unwrap_or(fmt) } else { fmt };
    let er = ui.put(
        rect.shrink2(vec2(4.0, 0.0)),
        TextEdit::singleline(&mut text)
            .id(id)
            .frame(false)
            .horizontal_align(Align::Center)
            .vertical_align(Align::Center)
            .text_color(if i == 0 { p.fg1 } else { p.fg_a2 })
            .font(font::regular(12.0))
            .margin(egui::Margin::ZERO)
            .desired_width(rect.width() - 8.0),
    );
    if er.changed()
        && let Ok(n) = text.trim().trim_end_matches("dB").trim().replace(',', ".").parse::<f32>()
    {
        *value = n.clamp(EQ_RANGE.0, EQ_RANGE.1);
    }
    if er.has_focus() {
        ui.data_mut(|d| d.insert_temp(buf_id, text));
    } else {
        ui.data_mut(|d| d.remove::<String>(buf_id));
    }
}
