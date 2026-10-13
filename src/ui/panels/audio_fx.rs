//! Quick settings > Audio: tracks, stereo mode, delay and the audio filter
//! cards (Equalizer `6114:13610`, Compressor `536:1513`, Spatializer
//! `536:1703`, Stereo Widener `612:1223`).

use egui::{Sense, Ui, pos2, vec2};

use super::kit::{self, Sl};
use super::quick;
use crate::app::App;
use crate::domain::effects::{
    AudioEffects, CompressorSettings, EQ_BAND_LABELS, EQ_PRESETS, EQ_RANGE, EqualizerSettings, SpatializerSettings,
    StereoMode, StereoWidenerSettings,
};
use crate::ui::icons::Icon;
use crate::ui::state::UiState;
use crate::ui::widgets;

/// The icon the "EQ presets" frame (`2372:8487`) shows beside a preset.
pub fn preset_icon(name: &str) -> Icon {
    match name {
        "Classical" => Icon::EqClassical,
        "Club" => Icon::EqClub,
        "Dance" => Icon::EqDance,
        "Full bass" => Icon::EqFullBass,
        "Full bass and treble" => Icon::EqFullBassAndTreble,
        "Full treble" => Icon::EqFullTreble,
        "Headphones" => Icon::EqHeadphones,
        "Large Hall" => Icon::EqLargeHall,
        "Live" => Icon::EqLive,
        "Party" => Icon::EqParty,
        "Pop" => Icon::EqPop,
        "Reggae" => Icon::EqReggae,
        "Rock" => Icon::EqRock,
        "Ska" => Icon::EqSka,
        "Soft" => Icon::EqSoft,
        "Soft rock" => Icon::EqSoftRock,
        "Techno" => Icon::EqTechno,
        _ => Icon::Eq,
    }
}

pub fn show(ui: &mut Ui, app: &mut App, _st: &mut UiState) {
    kit::section(ui, "Defaults");
    ui.add_space(10.0);
    let tracks = app.status.audio_tracks.clone();
    if let Some(id) = kit::row(ui, "Audio Track", 48.0, |ui| {
        quick::track_number_box(ui, "audio-track", &tracks, app.status.audio_track)
    }) {
        app.set_audio_track(id);
    }
    ui.add_space(5.0);
    let lang = app.settings.audio.preferred_language.clone();
    if let Some(l) = kit::row(ui, "Audio Language", 48.0, |ui| quick::language_box(ui, "audio-lang", &lang)) {
        app.update_settings(|s| s.audio.preferred_language = l);
    }
    ui.add_space(5.0);
    kit::row(ui, "Stereo Mode", 52.0, |_| {});
    ui.add_space(10.0);
    let w = ui.available_width();
    if let Some(m) =
        kit::choice(ui, "stereo-mode", vec2(w, 48.0), app.stereo_mode, &StereoMode::ALL, |m| m.label().to_string())
    {
        app.set_stereo_mode(m);
    }
    ui.add_space(20.0);
    kit::rule(ui);

    kit::section(ui, "Synchronization");
    ui.add_space(10.0);
    if let Some(us) = quick::delay_editor(ui, "audio-delay", "Audio Track Delay", app.status.audio_delay_us) {
        app.set_audio_delay(us);
    }
    ui.add_space(10.0);
    kit::rule(ui);

    kit::section(ui, "Filters");
    let mut fx = app.settings.audio.effects.clone();
    equalizer(ui, &mut fx.equalizer);
    effect_card(ui, "Compressor", &mut fx.compressor, |c| &mut c.enabled, compressor);
    effect_card(ui, "Spatializer", &mut fx.spatializer, |c| &mut c.enabled, spatializer);
    effect_card(ui, "Stereo Widener", &mut fx.stereo_widener, |c| &mut c.enabled, widener);
    pitch(ui, &mut fx);
    if fx != app.settings.audio.effects {
        app.set_audio_effects(fx);
    }
}

fn caption_row(ui: &mut Ui, title: &str) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 37.0), Sense::hover());
    kit::caption(ui, pos2(rect.min.x + 4.0, rect.center().y), title);
}

/// A caption with an enable switch, and the effect's card below. Moving a
/// slider in the card switches the effect on.
fn effect_card<T>(
    ui: &mut Ui,
    title: &str,
    s: &mut T,
    enabled: impl Fn(&mut T) -> &mut bool,
    add: impl FnOnce(&mut Ui, &mut T, bool) -> bool,
) {
    ui.add_space(10.0);
    kit::row(ui, title, 37.0, |ui| widgets::toggle(ui, enabled(s)));
    ui.add_space(10.0);
    let on = *enabled(s);
    let mut touched = false;
    kit::card(ui, ("audio-card", title), |ui| touched = add(ui, s, on));
    if touched {
        *enabled(s) = true;
    }
    ui.add_space(5.0);
}

fn db(v: f32) -> String {
    format!("{v:.1} dB")
}

fn equalizer(ui: &mut Ui, eq: &mut EqualizerSettings) {
    ui.add_space(10.0);
    caption_row(ui, "Equalizer");
    ui.add_space(10.0);
    let mut options = vec![kit::Opt::new("None")];
    options.extend(EQ_PRESETS.iter().map(|(n, ..)| kit::Opt::icon(preset_icon(n), *n)));
    let sel = if !eq.enabled { Some(0) } else { eq.preset.map(|i| i + 1) };
    let current = match (eq.enabled, eq.preset) {
        (false, _) => "None",
        (true, Some(i)) => EQ_PRESETS.get(i).map(|p| p.0).unwrap_or("Custom"),
        (true, None) => "Custom",
    };
    let w = ui.available_width();
    if let Some(i) = kit::dropdown(ui, "eq-preset", vec2(w, 48.0), sel, current, &options) {
        if i == 0 {
            eq.enabled = false;
        } else {
            eq.apply_preset(i - 1);
            eq.enabled = true;
        }
    }
    ui.add_space(10.0);
    let mut touched = false;
    kit::card(ui, "eq-card", |ui| {
        touched |= band(ui, "eq-pre", "Preamp", &mut eq.preamp);
        for (i, label) in EQ_BAND_LABELS.iter().enumerate() {
            touched |= band(ui, ("eq-band", i), label, &mut eq.bands[i]);
        }
    });
    if touched {
        eq.enabled = true;
        eq.preset = None;
    }
    ui.add_space(10.0);
    kit::toggle_row(ui, "2 Pass", &mut eq.two_pass, 48.0);
}

/// One equalizer band: label and a dB box, then the slider.
fn band(ui: &mut Ui, id: impl std::hash::Hash + Copy, label: &str, value: &mut f32) -> bool {
    let mut v = *value as f64;
    let range = EQ_RANGE.0 as f64..=EQ_RANGE.1 as f64;
    let mut changed =
        kit::row(ui, label, 56.0, |ui| kit::number_box(ui, id, &mut v, range.clone(), 0.5, |x| db(x as f32), 140.0));
    changed |= kit::slider(ui, &mut v, range, &Sl::default().step(0.1).reset(0.0)).changed();
    ui.add_space(10.0);
    if changed {
        *value = v as f32;
    }
    changed
}

fn compressor(ui: &mut Ui, c: &mut CompressorSettings, _on: bool) -> bool {
    let s = Sl::default();
    let mut t = false;
    let r = CompressorSettings::RANGE_RMS_PEAK;
    t |=
        kit::card_slider_f32(ui, "RMS / Peak", &format!("{:.1}", c.rms_peak), &mut c.rms_peak, r, &s.clone().step(0.1));
    t |= kit::card_slider_f32(
        ui,
        "Attack",
        &format!("{:.1} ms", c.attack_ms),
        &mut c.attack_ms,
        CompressorSettings::RANGE_ATTACK,
        &s.clone().step(0.5),
    );
    t |= kit::card_slider_f32(
        ui,
        "Release",
        &format!("{:.1} ms", c.release_ms),
        &mut c.release_ms,
        CompressorSettings::RANGE_RELEASE,
        &s.clone().step(1.0),
    );
    t |= kit::card_slider_f32(
        ui,
        "Threshold",
        &db(c.threshold_db),
        &mut c.threshold_db,
        CompressorSettings::RANGE_THRESHOLD,
        &s.clone().step(0.5),
    );
    t |= kit::card_slider_f32(
        ui,
        "Ratio",
        &format!("{:.1} : 1", c.ratio),
        &mut c.ratio,
        CompressorSettings::RANGE_RATIO,
        &s.clone().step(0.1),
    );
    t |= kit::card_slider_f32(
        ui,
        "Knee Radius",
        &db(c.knee_db),
        &mut c.knee_db,
        CompressorSettings::RANGE_KNEE,
        &s.clone().step(0.1),
    );
    t |= kit::card_slider_f32(
        ui,
        "Makeup Gain",
        &db(c.makeup_gain_db),
        &mut c.makeup_gain_db,
        CompressorSettings::RANGE_MAKEUP,
        &s.step(0.1),
    );
    t
}

fn tenths(v: f32) -> String {
    format!("{:.1}", v * 10.0)
}

fn spatializer(ui: &mut Ui, sp: &mut SpatializerSettings, _on: bool) -> bool {
    let s = Sl::default().ticks(10).step(0.01);
    let mut t = false;
    t |=
        kit::card_slider_f32(ui, "Size", &tenths(sp.room_size), &mut sp.room_size, SpatializerSettings::RANGE_ROOM, &s);
    t |= kit::card_slider_f32(ui, "Width", &tenths(sp.width), &mut sp.width, SpatializerSettings::RANGE, &s);
    t |= kit::card_slider_f32(ui, "Wet", &tenths(sp.wet), &mut sp.wet, SpatializerSettings::RANGE, &s);
    t |= kit::card_slider_f32(ui, "Dry", &tenths(sp.dry), &mut sp.dry, SpatializerSettings::RANGE, &s);
    t |= kit::card_slider_f32(ui, "Damp", &tenths(sp.damp), &mut sp.damp, SpatializerSettings::RANGE, &s);
    t
}

fn percent(v: f32) -> String {
    format!("{:.0} %", v * 100.0)
}

fn widener(ui: &mut Ui, w: &mut StereoWidenerSettings, _on: bool) -> bool {
    let s = Sl::default();
    let mut t = false;
    t |= kit::card_slider_f32(
        ui,
        "Delay Time",
        &format!("{:.1} ms", w.delay_ms),
        &mut w.delay_ms,
        StereoWidenerSettings::RANGE_DELAY,
        &s.clone().step(0.5),
    );
    t |= kit::card_slider_f32(
        ui,
        "Feedback Gain",
        &percent(w.feedback),
        &mut w.feedback,
        StereoWidenerSettings::RANGE_FEEDBACK,
        &s.clone().ticks(9).step(0.01),
    );
    t |= kit::card_slider_f32(
        ui,
        "Crossfeed",
        &percent(w.crossfeed),
        &mut w.crossfeed,
        StereoWidenerSettings::RANGE_CROSSFEED,
        &s.clone().ticks(8).step(0.01),
    );
    t |= kit::card_slider_f32(
        ui,
        "Dry Mix",
        &percent(w.dry_mix),
        &mut w.dry_mix,
        StereoWidenerSettings::RANGE_DRY,
        &s.ticks(10).step(0.01),
    );
    t
}

fn pitch(ui: &mut Ui, fx: &mut AudioEffects) {
    ui.add_space(15.0);
    let mut v = fx.pitch_shift as f64;
    let text = format!("{:.1} semitones", fx.pitch_shift);
    egui::Frame::new().inner_margin(egui::Margin::symmetric(4, 0)).show(ui, |ui| {
        kit::label_value(ui, "Adjust Pitch", &text);
    });
    ui.add_space(5.0);
    if kit::slider(ui, &mut v, -12.0..=12.0, &Sl::default().origin(0.0).ticks(24).step(0.5).reset(0.0)).changed() {
        fx.pitch_shift = v as f32;
    }
    ui.add_space(10.0);
}
