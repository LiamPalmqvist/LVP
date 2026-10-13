//! Quick settings > Video: picture geometry, output and the video filter
//! cards (Essential `536:1855`, Crop `543:990`, Colors `614:1430`, plus
//! Geometry, Overlay and Advanced).

use egui::{Align2, Rect, Sense, Ui, Vec2, pos2, vec2};

use super::kit::{self, Sl};
use crate::app::App;
use crate::domain::effects::{
    ASPECT_RATIOS, CROP_RATIOS, DEINTERLACE_METHODS, DeinterlaceMode, GradientMode, Transform, VideoEffects,
    VideoEssential, ZOOM_LEVELS,
};
use crate::domain::settings::VideoQuality;
use crate::ui::icons::Icon;
use crate::ui::state::UiState;
use crate::ui::theme::font;
use crate::ui::widgets::{self, pal};

const DROP: Vec2 = Vec2::new(220.0, 48.0);

/// HDR choices from the "HDR Options" frame (`2372:8733`). VLC tone-maps HDR
/// to the display on its own; the choice is kept for the session.
pub const HDR_OPTIONS: &[&str] = &["Auto", "Off", "HDR10", "HDR10+", "Dolby Vision", "HLG"];

pub fn show(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    defaults(ui, app, st);
    ui.add_space(10.0);
    kit::rule(ui);
    kit::section(ui, "Filters");
    let mut fx = app.settings.video.effects.clone();
    filter_card(ui, "Essential", |ui| essential(ui, &mut fx.essential));
    filter_card(ui, "Crop", |ui| crop(ui, &mut fx));
    filter_card(ui, "Colors", |ui| colors(ui, &mut fx));
    filter_card(ui, "Geometry", |ui| geometry(ui, &mut fx));
    filter_card(ui, "Overlay", |ui| overlay(ui, &mut fx));
    filter_card(ui, "Advanced", |ui| advanced(ui, &mut fx));
    if fx != app.settings.video.effects {
        app.set_video_effects(fx);
    }
}

/// A caption row and an effect card below it.
fn filter_card(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui)) {
    ui.add_space(10.0);
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 37.0), Sense::hover());
    kit::caption(ui, pos2(rect.min.x + 4.0, rect.center().y), title);
    ui.add_space(10.0);
    kit::card(ui, ("video-card", title), add);
    ui.add_space(5.0);
}

fn rotation(t: Transform) -> i32 {
    match t {
        Transform::Rotate90 => 90,
        Transform::Rotate180 => 180,
        Transform::Rotate270 => 270,
        _ => 0,
    }
}

fn from_rotation(deg: i32) -> Transform {
    match deg.rem_euclid(360) {
        90 => Transform::Rotate90,
        180 => Transform::Rotate180,
        270 => Transform::Rotate270,
        _ => Transform::None,
    }
}

fn ratio_dropdown(
    ui: &mut Ui,
    id: &str,
    value: &Option<String>,
    builtin: &[&str],
    custom: &str,
) -> Option<Option<String>> {
    let mut list: Vec<Option<String>> = vec![None];
    list.extend(builtin.iter().map(|s| Some(s.to_string())));
    list.extend(custom.split(',').map(str::trim).filter(|s| !s.is_empty()).map(|s| Some(s.to_string())));
    let labels: Vec<String> = list.iter().map(|r| r.clone().unwrap_or_else(|| "Default".into())).collect();
    let sel = list.iter().position(|r| r == value);
    let cur = value.clone().unwrap_or_else(|| "Default".into());
    kit::dropdown(ui, id, DROP, sel, &cur, &kit::opts(&labels)).map(|i| list[i].clone())
}

fn defaults(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    let p = pal(ui.ctx());
    kit::section(ui, "Defaults");
    ui.add_space(26.0);
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 17.0), Sense::hover());
    ui.painter().text(pos2(r.min.x + 4.0, r.center().y), Align2::LEFT_CENTER, "Flip", font::regular(14.0), p.fg_a3);
    ui.add_space(20.0);
    let g = &mut app.geometry;
    let sel = if g.flip_h {
        1
    } else if g.flip_v {
        2
    } else {
        0
    };
    if let Some(i) = kit::segmented(ui, "flip", sel, &["None", "Horizontal", "Vertical"], vec2(w, 44.0)) {
        g.flip_h = i == 1;
        g.flip_v = i == 2;
    }
    ui.add_space(26.0);

    let mut fx = app.settings.video.effects.clone();
    let deg = rotation(fx.extras.transform);
    let r = kit::row(ui, "Rotate", 48.0, |ui| kit::value_box(ui, "rotate", &format!("{deg} deg"), 140.0));
    let new = match r {
        Some(kit::BoxEvent::Commit(t)) => kit::parse_number(&t).map(|v| ((v / 90.0).round() as i32) * 90),
        Some(kit::BoxEvent::Scroll(d)) => Some(deg + if d > 0.0 { 90 } else { -90 }),
        None => None,
    };
    if let Some(d) = new
        && d.rem_euclid(360) != deg
    {
        fx.extras.transform = from_rotation(d);
        app.set_video_effects(fx);
    }
    ui.add_space(15.0);

    kit::row(ui, "Zoom", 53.0, |ui| {
        let labels: Vec<&str> = ZOOM_LEVELS.iter().map(|z| z.0).collect();
        let sel = ZOOM_LEVELS.iter().position(|z| (z.1 - app.geometry.zoom).abs() < 0.001);
        let cur = sel.map(|i| ZOOM_LEVELS[i].0.to_string()).unwrap_or_else(|| format!("{:.2}x", app.geometry.zoom));
        if let Some(i) = kit::dropdown(ui, "zoom", DROP, sel, &cur, &kit::opts(&labels)) {
            app.geometry.zoom = ZOOM_LEVELS[i].1;
        }
    });
    ui.add_space(15.0);
    kit::row_with(ui, "HDR", Some("VLC tone-maps HDR video to your display automatically."), 68.0, |ui| {
        let sel = st.panels.hdr.min(HDR_OPTIONS.len() - 1);
        if let Some(i) = kit::dropdown(ui, "hdr", DROP, Some(sel), HDR_OPTIONS[sel], &kit::opts(HDR_OPTIONS)) {
            st.panels.hdr = i;
        }
    });
    kit::row(ui, "Resolution", 68.0, |ui| {
        if let Some(q) =
            kit::choice(ui, "resolution", DROP, app.settings.playback.preferred_quality, VideoQuality::ALL, |q| {
                q.label().to_string()
            })
        {
            app.update_settings(|s| s.playback.preferred_quality = q);
        }
    });
    let custom_aspect = app.settings.video.custom_aspect_ratios.clone();
    let aspect = app.geometry.aspect.clone();
    if let Some(a) =
        kit::row(ui, "Aspect Ratio", 68.0, |ui| ratio_dropdown(ui, "aspect", &aspect, ASPECT_RATIOS, &custom_aspect))
    {
        app.geometry.aspect = a;
    }
    let custom_crop = app.settings.video.custom_crop_ratios.clone();
    let crop = app.geometry.crop.clone();
    if let Some(c) = kit::row(ui, "Crop", 68.0, |ui| ratio_dropdown(ui, "crop", &crop, CROP_RATIOS, &custom_crop)) {
        app.geometry.crop = c;
    }
    let (mode, method) = (app.settings.video.deinterlace, app.settings.video.deinterlace_method.clone());
    if let Some(m) = kit::row(ui, "Deinterlace", 68.0, |ui| {
        kit::choice(ui, "deinterlace", DROP, mode, &DeinterlaceMode::ALL, |m| m.label().to_string())
    }) {
        app.set_deinterlace(m, method.clone());
    }
    if mode != DeinterlaceMode::Off {
        let labels: Vec<&str> = DEINTERLACE_METHODS.iter().map(|m| m.1).collect();
        let sel = DEINTERLACE_METHODS.iter().position(|m| m.0 == method);
        let cur = sel.map(|i| DEINTERLACE_METHODS[i].1).unwrap_or("Automatic");
        if let Some(i) =
            kit::row(ui, "Method", 68.0, |ui| kit::dropdown(ui, "deint-method", DROP, sel, cur, &kit::opts(&labels)))
        {
            app.set_deinterlace(mode, DEINTERLACE_METHODS[i].0.to_string());
        }
    }
    let renderers = app.engine.renderers();
    let current = app.status.renderer;
    let mut labels = vec!["Local".to_string()];
    labels.extend(renderers.iter().map(|r| r.name.clone()));
    let sel = match current {
        None => Some(0),
        Some(id) => renderers.iter().position(|r| r.id == id).map(|i| i + 1),
    };
    let cur = sel.map(|i| labels[i].clone()).unwrap_or_else(|| "Local".into());
    if let Some(i) =
        kit::row(ui, "Render", 68.0, |ui| kit::dropdown(ui, "render", DROP, sel, &cur, &kit::opts(&labels)))
        && sel != Some(i)
    {
        app.set_renderer(if i == 0 { None } else { Some(renderers[i - 1].id) });
    }
}

fn essential(ui: &mut Ui, e: &mut VideoEssential) {
    let mut adjust = false;
    adjust |= kit::card_slider_f32(ui, "Hue", "", &mut e.hue, VideoEssential::RANGE_HUE, &Sl::default().reset(0.0));
    adjust |= kit::card_slider_f32(
        ui,
        "Brightness",
        "",
        &mut e.brightness,
        VideoEssential::RANGE_BRIGHTNESS,
        &Sl::default().reset(1.0),
    );
    adjust |= kit::card_slider_f32(
        ui,
        "Contrast",
        "",
        &mut e.contrast,
        VideoEssential::RANGE_CONTRAST,
        &Sl::default().reset(1.0),
    );
    adjust |= kit::card_slider_f32(
        ui,
        "Saturation",
        "",
        &mut e.saturation,
        VideoEssential::RANGE_SATURATION,
        &Sl::default().ticks(3).reset(1.0),
    );
    adjust |= kit::card_slider_f32(
        ui,
        "Gamma",
        "",
        &mut e.gamma,
        VideoEssential::RANGE_GAMMA,
        &Sl::default().ticks(10).reset(1.0),
    );
    if adjust {
        e.adjust_enabled = !e.is_neutral_adjust();
    }
    if kit::card_slider_f32(
        ui,
        "Sharpen Sigma",
        "",
        &mut e.sharpen_sigma,
        VideoEssential::RANGE_SHARPEN,
        &Sl::default().ticks(4).reset(0.0),
    ) {
        e.sharpen_enabled = e.sharpen_sigma > 0.0;
    }
    kit::row(ui, "Banding Removal Radius", 37.0, |ui| widgets::toggle(ui, &mut e.banding_enabled));
    let (lo, hi) = VideoEssential::RANGE_BANDING_RADIUS;
    if kit::slider_i32(ui, &mut e.banding_radius, lo..=hi, &Sl::default().ticks(7).disabled(!e.banding_enabled))
        .changed()
    {
        e.banding_enabled = true;
    }
    ui.add_space(5.0);
    if kit::card_slider_f32(
        ui,
        "Film Grain Variance",
        "",
        &mut e.grain_variance,
        VideoEssential::RANGE_GRAIN,
        &Sl::default().ticks(10).reset(0.0),
    ) {
        e.grain_enabled = e.grain_variance > 0.0;
    }
}

/// "Top  0 px": a 141x33 field with its label inside.
fn crop_field(ui: &mut Ui, id: &str, label: &str, value: &mut u32) -> bool {
    let p = pal(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(141.0, 33.0), Sense::hover());
    let text = Rect::from_min_max(pos2(rect.min.x + 72.0, rect.min.y), pos2(rect.max.x - 28.0, rect.max.y));
    let ev = kit::value_box_in(ui, rect, text, resp.hovered(), id, &value.to_string());
    ui.painter().text(
        pos2(rect.min.x + 12.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font::regular(14.0),
        p.fg_a3,
    );
    ui.painter().text(
        pos2(rect.max.x - 12.0, rect.center().y),
        Align2::RIGHT_CENTER,
        "px",
        font::regular(14.0),
        p.fg_a4,
    );
    let new = match ev {
        Some(kit::BoxEvent::Commit(t)) => kit::parse_number(&t).map(|v| v.max(0.0).round() as u32),
        Some(kit::BoxEvent::Scroll(d)) => Some(if d > 0.0 { *value + 1 } else { value.saturating_sub(1) }),
        None => None,
    };
    match new {
        Some(v) if v != *value => {
            *value = v.min(4096);
            true
        }
        _ => false,
    }
}

fn link_button(ui: &mut Ui, id: &str, on: &mut bool, tip: &str) {
    let p = pal(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(30.0, 32.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 4, p.hover);
    }
    let icon = if *on { Icon::LinkActive } else { Icon::Link };
    widgets::paint_icon(
        ui,
        Rect::from_center_size(rect.center(), Vec2::splat(24.0)),
        icon,
        if *on { p.accent } else { p.fg_a2 },
    );
    if resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        *on = !*on;
    }
    let _ = id;
}

fn crop(ui: &mut Ui, fx: &mut VideoEffects) {
    let c = &mut fx.crop;
    for (a, b, vertical) in [("Top", "Bottom", true), ("Left", "Right", false)] {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let (va, vb, link) = if vertical {
                (&mut c.top, &mut c.bottom, &mut c.sync_vertical)
            } else {
                (&mut c.left, &mut c.right, &mut c.sync_horizontal)
            };
            if crop_field(ui, &format!("crop-{a}"), a, va) && *link {
                *vb = *va;
            }
            if crop_field(ui, &format!("crop-{b}"), b, vb) && *link {
                *va = *vb;
            }
            let tip = if vertical { "Crop top and bottom together" } else { "Crop left and right together" };
            let was = *link;
            link_button(ui, a, link, tip);
            if *link && !was {
                *vb = *va;
            }
        });
    }
    ui.add_space(4.0);
}

fn color_row(ui: &mut Ui, label: &str, id: &str, color: &mut u32, on: &mut bool) {
    kit::row(ui, label, 46.0, |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        widgets::toggle(ui, on);
        if kit::color_box(ui, id, color, vec2(116.0, 32.0)) {
            *on = true;
        }
    });
}

fn colors(ui: &mut Ui, fx: &mut VideoEffects) {
    let c = &mut fx.colors;
    color_row(ui, "Color Extraction", "extract-color", &mut c.extract_color, &mut c.extract_enabled);
    kit::card_rule(ui);
    color_row(ui, "Color Threshold", "threshold-color", &mut c.threshold_color, &mut c.threshold_enabled);
    let off = !c.threshold_enabled;
    kit::label_value(ui, "Saturation", "");
    kit::slider_i32(ui, &mut c.threshold_saturation, 0..=255, &Sl::default().disabled(off));
    ui.add_space(5.0);
    kit::label_value(ui, "Similarity", "");
    kit::slider_i32(ui, &mut c.threshold_similarity, 0..=255, &Sl::default().disabled(off));
    ui.add_space(10.0);
    kit::card_rule(ui);
    kit::toggle_row(ui, "Negate Colors", &mut c.invert, 52.0);
    kit::card_rule(ui);
    kit::toggle_row(ui, "Posterize", &mut c.posterize_enabled, 52.0);
    if c.posterize_enabled {
        kit::label_value(ui, "Levels", &c.posterize_level.to_string());
        kit::slider_i32(ui, &mut c.posterize_level, 2..=16, &Sl::default().ticks(14));
        ui.add_space(10.0);
    }
    kit::card_rule(ui);
    kit::toggle_row(ui, "Gradient", &mut c.gradient_enabled, 48.0);
    ui.add_space(6.0);
    let w = ui.available_width();
    if let Some(m) =
        kit::choice(ui, "gradient-mode", vec2(w, 48.0), c.gradient_mode, &GradientMode::ALL, |m| m.label().to_string())
    {
        c.gradient_mode = m;
        c.gradient_enabled = true;
    }
    ui.add_space(6.0);
    kit::toggle_row(ui, "Color", &mut c.gradient_color, 46.0);
    kit::toggle_row(ui, "Cartoon", &mut c.gradient_cartoon, 46.0);
    kit::card_rule(ui);
    kit::row(ui, "Sepia Intensity", 46.0, |ui| widgets::toggle(ui, &mut c.sepia_enabled));
    if kit::slider_i32(ui, &mut c.sepia_intensity, 0..=255, &Sl::default().ticks(3).disabled(!c.sepia_enabled))
        .changed()
    {
        c.sepia_enabled = true;
    }
    ui.add_space(5.0);
}

fn geometry(ui: &mut Ui, fx: &mut VideoEffects) {
    let x = &mut fx.extras;
    kit::label_value(ui, "Transform", "");
    let w = ui.available_width();
    if let Some(t) =
        kit::choice(ui, "transform", vec2(w, 48.0), x.transform, &Transform::ALL, |t| t.label().to_string())
    {
        x.transform = t;
    }
    ui.add_space(6.0);
    kit::toggle_row(ui, "Mirror", &mut x.mirror, 46.0);
    kit::toggle_row(ui, "Psychedelic", &mut x.psychedelic, 46.0);
    kit::toggle_row(ui, "Waves", &mut x.waves, 46.0);
    kit::toggle_row(ui, "Water Ripple", &mut x.ripple, 46.0);
    kit::toggle_row(ui, "Motion Blur", &mut x.motion_blur, 46.0);
    if x.motion_blur {
        kit::label_value(ui, "Blur Factor", &x.motion_blur_factor.to_string());
        kit::slider_i32(ui, &mut x.motion_blur_factor, 1..=127, &Sl::default());
        ui.add_space(5.0);
    }
}

fn overlay(ui: &mut Ui, fx: &mut VideoEffects) {
    let x = &mut fx.extras;
    let w = ui.available_width();
    kit::toggle_row(ui, "Marquee", &mut x.marquee_enabled, 46.0);
    if let Some(kit::BoxEvent::Commit(t)) = kit::value_box_sized(ui, "marquee-text", &x.marquee_text, vec2(w, 36.0)) {
        x.marquee_text = t;
        x.marquee_enabled = !x.marquee_text.is_empty();
    }
    ui.add_space(10.0);
    kit::card_rule(ui);
    kit::toggle_row(ui, "Logo", &mut x.logo_enabled, 46.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let shown = if x.logo_file.is_empty() { "No file".to_string() } else { x.logo_file.clone() };
        if let Some(kit::BoxEvent::Commit(t)) = kit::value_box_sized(ui, "logo-file", &shown, vec2(w - 90.0, 36.0)) {
            x.logo_file = t;
        }
        if kit::chip(ui, "Browse", "Choose an image").clicked()
            && let Some(f) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg"]).pick_file()
        {
            x.logo_file = f.display().to_string();
            x.logo_enabled = true;
        }
    });
    ui.add_space(5.0);
    kit::label_value(ui, "Logo Opacity", &format!("{} %", (x.logo_opacity as f32 / 2.55).round()));
    kit::slider_i32(ui, &mut x.logo_opacity, 0..=255, &Sl::default().disabled(!x.logo_enabled));
    ui.add_space(5.0);
}

fn advanced(ui: &mut Ui, fx: &mut VideoEffects) {
    let x = &mut fx.extras;
    kit::toggle_row(ui, "Anaglyph 3D", &mut x.anaglyph, 46.0);
    kit::toggle_row(ui, "Denoise", &mut x.denoise, 46.0);
    kit::toggle_row(ui, "Anti-flicker", &mut x.antiflicker, 46.0);
}
