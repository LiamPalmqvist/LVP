//! The first-run wizard (O1). Three steps, as in the Onboarding frames:
//! welcome with a media folder (`1899:3055`), theme and accent over a
//! preview of the player (`1902:3670` and its eight accent variants), and
//! the basic keyboard shortcuts (`1902:6770`). Laid out on the frame's
//! 1090x759 grid; choices are saved through `App::update_settings`.

use std::path::PathBuf;

use egui::{Align2, Color32, CornerRadius, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};

use super::frame::{self, Btn};
use crate::app::App;
use crate::domain::settings::{Accent, Theme};
use crate::domain::shortcuts::{Action, KeyChord};
use crate::ui::icons::Icon;
use crate::ui::theme::{Palette, font, with_alpha};
use crate::ui::widgets::{self, pal};

pub const SIZE: Vec2 = vec2(1090.0, 759.0);
const STEPS: usize = 3;

#[derive(Default)]
pub struct OnboardingState {
    pub step: usize,
    folder: String,
    primed: bool,
    /// The shortcut row last pressed, and when.
    lit: Option<(usize, f64)>,
}

/// The shortcut rows: label and the actions whose keys are shown.
const SHORTCUTS: [(&str, &[Action]); 12] = [
    ("Play / Pause", &[Action::PlayPause]),
    ("Seek Backward / Forward", &[Action::JumpBackwardVeryShort, Action::JumpForwardVeryShort]),
    ("Previous / Next Item", &[Action::Previous, Action::Next]),
    ("Next Frame", &[Action::NextFrame]),
    ("Cycle Subtitle Track", &[Action::CycleSubtitle]),
    ("Cycle Audio Track", &[Action::CycleAudioTrack]),
    ("Toggle Fullscreen", &[Action::ToggleFullscreen]),
    ("Increase / Decrease Volume", &[Action::VolumeUp, Action::VolumeDown]),
    ("Show / Hide Queue", &[Action::ToggleQueue]),
    ("Show / Hide File Browser", &[Action::ToggleSidebar]),
    ("Take a Snapshot", &[Action::Snapshot]),
    ("Record", &[Action::Record]),
];

fn tilde(p: &std::path::Path) -> String {
    if let Some(home) = std::env::var_os("HOME")
        && let Ok(rest) = p.strip_prefix(&home)
    {
        return format!("~/{}", rest.display());
    }
    p.display().to_string()
}

/// Runs `add` in a child Ui placed at `r`.
fn place<R>(ui: &mut Ui, r: Rect, add: impl FnOnce(&mut Ui) -> R) -> R {
    let mut c =
        ui.new_child(egui::UiBuilder::new().max_rect(r).layout(egui::Layout::left_to_right(egui::Align::Center)));
    add(&mut c)
}

fn button(ui: &mut Ui, at: Pos2, label: &str, kind: Btn) -> bool {
    place(ui, Rect::from_min_size(at, vec2(100.0, 32.0)), |ui| {
        frame::btn_sized(ui, label, kind, true, vec2(100.0, 32.0), font::semibold(16.0)).clicked()
    })
}

pub fn show(ctx: &egui::Context, app: &mut App, open: &mut bool, st: &mut OnboardingState) {
    if !st.primed {
        st.folder = app.settings.library.video_folders.first().map(|p| tilde(p)).unwrap_or_else(|| "~/Movies".into());
        st.primed = true;
    }
    let p = pal(ctx);
    let mut finish = false;
    let resp = egui::Modal::new(egui::Id::new("lvp-onboarding"))
        .backdrop_color(Color32::from_black_alpha(if p.dark { 150 } else { 90 }))
        .frame(
            egui::Frame::new()
                .fill(p.bg_b1)
                .corner_radius(14)
                .stroke(Stroke::new(1.0, with_alpha(p.divider_dim, 0.6)))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 8],
                    blur: 30,
                    spread: 0,
                    color: Color32::from_black_alpha(118),
                }),
        )
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(SIZE, Sense::hover());
            let o = rect.min;
            match st.step {
                0 => welcome(ui, app, st, o),
                1 => theme_step(ui, app, o),
                _ => shortcuts_step(ui, app, st, o),
            }
            // Step dots under the buttons.
            for i in 0..STEPS {
                let c = o + vec2(SIZE.x / 2.0 + (i as f32 - 1.0) * 16.0, 736.0);
                let on = i == st.step;
                ui.painter().circle_filled(c, if on { 4.0 } else { 3.0 }, if on { p.accent } else { p.fg_a4 });
            }
            // Navigation.
            match st.step {
                0 => {
                    if button(ui, o + vec2(820.0, 671.0), "Skip", Btn::Plain) {
                        finish = true;
                    }
                    if button(ui, o + vec2(928.0, 671.0), "Next", Btn::Dim) {
                        st.step = 1;
                    }
                }
                1 => {
                    if button(ui, o + vec2(820.0, 671.0), "Back", Btn::Solid) {
                        st.step = 0;
                    }
                    if button(ui, o + vec2(928.0, 671.0), "Next", Btn::Solid) {
                        st.step = 2;
                    }
                }
                _ => {
                    if button(ui, o + vec2(820.0, 671.0), "Back", Btn::Plain) {
                        st.step = 1;
                    }
                    if button(ui, o + vec2(928.0, 671.0), "Finish", Btn::Solid) {
                        finish = true;
                    }
                }
            }
        });
    // Esc skips the wizard, as Skip does.
    if resp.is_top_modal
        && !resp.any_popup_open
        && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        finish = true;
    }
    if finish {
        let folder = PathBuf::from(crate::app::expand_home(st.folder.trim()));
        app.update_settings(|s| {
            if !st.folder.trim().is_empty() && s.library.video_folders.first() != Some(&folder) {
                s.library.video_folders.retain(|f| f != &folder);
                s.library.video_folders.insert(0, folder.clone());
            }
            s.general.onboarding_done = true;
        });
        app.save_settings_now();
        *st = OnboardingState::default();
        *open = false;
    }
}

fn welcome(ui: &mut Ui, app: &App, st: &mut OnboardingState, o: Pos2) {
    let p = pal(ui.ctx());
    let _ = app;
    widgets::paint_icon(ui, Rect::from_min_size(o + vec2(475.0, 116.0), vec2(141.0, 141.0)), Icon::Logo, p.accent_logo);
    ui.painter().text(
        o + vec2(SIZE.x / 2.0, 286.0 + 19.5),
        Align2::CENTER_CENTER,
        "Welcome to LVP!",
        font::light(32.0),
        p.fg1,
    );
    let text = "VLC is a free and open source cross-platform multimedia player and framework that plays most \
                multimedia files as well as DVDs, Audio CDs, VCDs, and various streaming protocols.";
    let mut job = egui::text::LayoutJob::single_section(
        text.to_string(),
        egui::TextFormat { font_id: font::light(16.0), color: p.fg_a1, line_height: Some(28.0), ..Default::default() },
    );
    job.wrap.max_width = 612.0;
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    ui.painter().galley(o + vec2(248.0, 362.0), galley, p.fg_a1);
    ui.painter().text(
        o + vec2(248.0, 500.0),
        Align2::LEFT_TOP,
        "Choose a Media Folder to get started with",
        font::regular(14.0),
        p.fg_a2,
    );
    place(ui, Rect::from_min_size(o + vec2(248.0, 533.0), vec2(471.0, 36.0)), |ui| {
        egui::Frame::new()
            .fill(p.bg_b2)
            .stroke(Stroke::new(2.0, p.bg_a4))
            .corner_radius(6)
            .inner_margin(egui::Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut st.folder)
                        .frame(false)
                        .margin(egui::Margin::ZERO)
                        .text_color(p.fg1)
                        .font(font::regular(16.0))
                        .desired_width(471.0 - 28.0),
                );
            });
    });
    let browse = place(ui, Rect::from_min_size(o + vec2(729.0, 534.0), vec2(114.0, 35.0)), |ui| {
        let (r, resp) = ui.allocate_exact_size(vec2(114.0, 35.0), Sense::click());
        ui.painter().rect_filled(r, CornerRadius::same(6), if resp.hovered() { p.bg_a2 } else { p.bg_a4 });
        ui.painter().text(
            r.center(),
            Align2::CENTER_CENTER,
            "Browse",
            font::regular(16.0),
            if resp.hovered() { p.fg1 } else { p.fg_a2 },
        );
        resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
    });
    if browse {
        let start = PathBuf::from(crate::app::expand_home(st.folder.trim()));
        let mut dlg = rfd::FileDialog::new().set_title("Media Folder");
        if start.is_dir() {
            dlg = dlg.set_directory(&start);
        }
        if let Some(d) = dlg.pick_folder() {
            st.folder = tilde(&d);
        }
    }
}

/// The light, dark and "system" theme cards' colours, taken from the two
/// palettes they stand for.
fn card_colors(accent: Accent) -> (Color32, Color32) {
    (Palette::light(accent).divider_dim, Palette::dark(accent).bg_a1)
}

fn theme_step(ui: &mut Ui, app: &mut App, o: Pos2) {
    let p = pal(ui.ctx());
    player_preview(ui, o);
    ui.painter().text(
        o + vec2(SIZE.x / 2.0, 173.0 + 21.0),
        Align2::CENTER_CENTER,
        "Select your preferred theme and color.",
        font::light(24.0),
        p.fg_a1,
    );
    let (light, dark) = card_colors(app.settings.general.accent);
    let themes = [(Theme::Light, 331.0), (Theme::Dark, 487.0), (Theme::System, 643.0)];
    for (theme, x) in themes {
        let outer = Rect::from_min_size(o + vec2(x, 286.0), vec2(116.0, 116.0));
        let resp = ui.interact(outer, ui.id().with(("theme", theme.label())), Sense::click());
        let selected = app.settings.general.theme == theme;
        if selected {
            ui.painter().rect_stroke(outer, CornerRadius::same(19), Stroke::new(4.0, p.accent), StrokeKind::Inside);
        } else if resp.hovered() {
            ui.painter().rect_stroke(
                outer,
                CornerRadius::same(19),
                Stroke::new(4.0, with_alpha(p.accent, 0.35)),
                StrokeKind::Inside,
            );
        }
        let inner = outer.shrink(8.0);
        match theme {
            Theme::Light => {
                ui.painter().rect_filled(inner, CornerRadius::same(12), light);
            }
            Theme::Dark => {
                ui.painter().rect_filled(inner, CornerRadius::same(12), dark);
            }
            Theme::System => {
                ui.painter().rect_filled(inner, CornerRadius::same(12), dark);
                // The light half: the top right, cut by a diagonal.
                let a = inner.min + vec2(inner.width() * 0.38, 0.0);
                let b = inner.min + vec2(inner.width() * 0.72, inner.height());
                let pts = clip_right_of(&rounded_outline(inner, 12.0), a, b);
                if pts.len() >= 3 {
                    ui.painter().add(Shape::convex_polygon(pts, light, Stroke::NONE));
                }
            }
        }
        ui.painter().text(
            o + vec2(x + 58.0, 424.0 + 9.5),
            Align2::CENTER_CENTER,
            theme.label(),
            font::light(16.0),
            p.fg_a2,
        );
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !selected {
            app.update_settings(|s| s.general.theme = theme);
        }
    }
    // Accent swatches.
    for (i, accent) in Accent::ALL.iter().enumerate() {
        let cell = Rect::from_min_size(o + vec2(41.0 + 80.0 * i as f32, 655.0), vec2(64.0, 64.0));
        let resp = ui.interact(cell, ui.id().with(("accent", i)), Sense::click());
        let selected = app.settings.general.accent == *accent;
        let c = Color32::from_rgb((accent.hex() >> 16) as u8, (accent.hex() >> 8) as u8, accent.hex() as u8);
        if selected {
            ui.painter().circle_stroke(cell.center(), 30.0, Stroke::new(4.0, with_alpha(c, 0.3)));
        }
        let a = if selected || resp.hovered() { 1.0 } else { 0.5 };
        ui.painter().circle_filled(cell.center(), 20.0, with_alpha(c, a));
        if resp.on_hover_text(accent.label()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !selected {
            let accent = *accent;
            app.update_settings(|s| s.general.accent = accent);
        }
    }
}

/// Points along a rounded rectangle's outline, clockwise from the top left.
fn rounded_outline(r: Rect, radius: f32) -> Vec<Pos2> {
    let mut pts = Vec::new();
    let corners = [
        (pos2(r.max.x - radius, r.min.y + radius), -90.0f32),
        (pos2(r.max.x - radius, r.max.y - radius), 0.0),
        (pos2(r.min.x + radius, r.max.y - radius), 90.0),
        (pos2(r.min.x + radius, r.min.y + radius), 180.0),
    ];
    for (c, start) in corners {
        for k in 0..=6 {
            let t = (start + 15.0 * k as f32).to_radians();
            pts.push(c + vec2(t.cos(), t.sin()) * radius);
        }
    }
    pts
}

/// Keeps the part of a convex polygon to the right of the line a->b.
fn clip_right_of(poly: &[Pos2], a: Pos2, b: Pos2) -> Vec<Pos2> {
    let side = |p: Pos2| (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
    let mut out = Vec::new();
    for i in 0..poly.len() {
        let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
        let (sp, sq) = (side(p), side(q));
        if sp <= 0.0 {
            out.push(p);
        }
        if (sp < 0.0) != (sq < 0.0) && sp != sq {
            let t = sp / (sp - sq);
            out.push(p + (q - p) * t);
        }
    }
    out
}

/// A small drawing of the player behind the theme step: title bar, black
/// video, a library strip and the control bar, in the chosen accent.
fn player_preview(ui: &Ui, o: Pos2) {
    let p = pal(ui.ctx());
    let painter = ui.painter();
    let win = Rect::from_min_size(o + vec2(17.0, 19.0), vec2(1056.0, 594.0));
    painter.rect_filled(win, 0, if p.dark { Color32::BLACK } else { p.stage });
    let bar = Rect::from_min_size(win.min, vec2(win.width(), 22.0));
    painter.rect_filled(bar, 0, p.bg_a4);
    widgets::paint_icon(ui, Rect::from_min_size(bar.min + vec2(9.0, 3.0), vec2(16.0, 16.0)), Icon::Logo, p.accent_logo);
    for (k, icon) in [Icon::Close, Icon::Fullscreen, Icon::Maximize, Icon::Minimize].iter().enumerate() {
        let x = bar.max.x - 18.0 - 19.0 * k as f32;
        widgets::paint_icon(ui, Rect::from_center_size(pos2(x, bar.center().y), vec2(16.0, 16.0)), *icon, p.fg_a2);
    }
    // The library column, mostly off to the left as in the frame.
    let side = Rect::from_min_max(pos2(win.min.x, bar.max.y), pos2(win.min.x + 23.0, win.max.y - 30.0));
    painter.rect_filled(side, 0, p.bg_a4);
    for k in 0..12 {
        let y = side.min.y + 26.0 + 42.0 * k as f32;
        let col = if k == 0 { p.accent } else { p.fg_a3 };
        painter.rect_filled(Rect::from_min_size(pos2(side.min.x + 2.0, y), vec2(14.0, 3.0)), 1, col);
    }
    // Control bar.
    let ctl = Rect::from_min_max(pos2(win.min.x, win.max.y - 30.0), win.max);
    painter.rect_filled(ctl, 0, with_alpha(p.bg_a4, 0.93));
    let cy = ctl.center().y;
    widgets::paint_icon(
        ui,
        Rect::from_center_size(pos2(ctl.min.x + 35.0, cy), vec2(14.0, 14.0)),
        Icon::Previous,
        p.fg_a2,
    );
    let play = Rect::from_center_size(pos2(ctl.min.x + 85.0, cy), vec2(54.0, 22.0));
    painter.rect_filled(play, 4, p.accent);
    widgets::paint_icon(ui, Rect::from_center_size(play.center(), vec2(14.0, 14.0)), Icon::Play, p.fg_b1);
    widgets::paint_icon(ui, Rect::from_center_size(pos2(ctl.min.x + 135.0, cy), vec2(14.0, 14.0)), Icon::Next, p.fg_a2);
    painter.text(pos2(ctl.min.x + 177.0, cy), Align2::LEFT_CENTER, "00:02:10", font::regular(11.0), p.fg_a2);
    let rail = Rect::from_min_max(pos2(ctl.min.x + 255.0, cy - 9.0), pos2(ctl.max.x - 260.0, cy + 9.0));
    painter.rect_filled(rail, 2, p.bg_b2);
    let filled = Rect::from_min_max(rail.min, pos2(rail.min.x + 96.0, rail.max.y));
    painter.rect_filled(filled, 2, p.bg_accent);
    painter.rect_filled(
        Rect::from_min_size(pos2(filled.max.x - 3.0, rail.min.y), vec2(4.0, rail.height())),
        1,
        p.accent,
    );
    painter.text(pos2(rail.max.x + 10.0, cy), Align2::LEFT_CENTER, "00:25:40", font::regular(11.0), p.fg_a2);
    let vol = Rect::from_min_size(pos2(ctl.max.x - 170.0, cy - 4.0), vec2(60.0, 8.0));
    painter.rect_filled(vol, 2, p.bg_b2);
    painter.rect_filled(Rect::from_min_size(vol.min, vec2(36.0, 8.0)), 2, p.accent_dim);
    for (k, icon) in [Icon::Queue, Icon::Settings, Icon::Subtitles].iter().enumerate() {
        let x = ctl.max.x - 20.0 - 22.0 * k as f32;
        widgets::paint_icon(ui, Rect::from_center_size(pos2(x, cy), vec2(14.0, 14.0)), *icon, p.fg_a2);
    }
}

fn same_mods(a: &KeyChord, b: &KeyChord) -> bool {
    (a.ctrl || a.alt || a.shift) && (a.ctrl, a.alt, a.shift) == (b.ctrl, b.alt, b.shift)
}

fn cap_label(chord: &KeyChord) -> Vec<String> {
    let mut out = Vec::new();
    if chord.ctrl {
        out.push(if cfg!(target_os = "macos") { "CMD" } else { "CTRL" }.to_string());
    }
    if chord.alt {
        out.push(if cfg!(target_os = "macos") { "OPTION" } else { "ALT" }.to_string());
    }
    if chord.shift {
        out.push("SHIFT".to_string());
    }
    out.push(
        match chord.key.as_str() {
            "Left" => "←",
            "Right" => "→",
            "Up" => "↑",
            "Down" => "↓",
            k => k,
        }
        .to_string(),
    );
    out
}

/// Draws one key cap; `pressed` uses the design's pressed state.
fn key_cap(ui: &Ui, x: f32, y: f32, text: &str, pressed: bool) -> f32 {
    let p = pal(ui.ctx());
    let galley = ui.painter().layout_no_wrap(text.to_string(), font::regular(15.0), Color32::PLACEHOLDER);
    let w = (galley.size().x + 40.0).max(42.0);
    let r = Rect::from_min_size(pos2(x, y), vec2(w, 42.0));
    if pressed {
        ui.painter().rect_filled(r, CornerRadius::same(12), p.bg_a5);
        ui.painter().rect_stroke(r, CornerRadius::same(12), Stroke::new(3.0, p.bg_b2), StrokeKind::Inside);
    } else {
        // A deeper bottom edge gives the cap its raised look.
        ui.painter().rect_filled(r.translate(vec2(0.0, 3.0)), CornerRadius::same(12), p.bg_b2);
        ui.painter().rect_filled(r, CornerRadius::same(12), p.bg_a3);
        ui.painter().rect_stroke(r, CornerRadius::same(12), Stroke::new(3.0, p.divider_bright), StrokeKind::Inside);
    }
    ui.painter().galley(r.center() - galley.size() / 2.0, galley, if pressed { p.fg1 } else { p.fg2 });
    w
}

fn shortcuts_step(ui: &mut Ui, app: &App, st: &mut OnboardingState, o: Pos2) {
    let p = pal(ui.ctx());
    let now = ui.input(|i| i.time);
    let shortcuts = &app.settings.controls.shortcuts;
    // Light up the row whose key was just pressed.
    let pressed: Vec<KeyChord> = ui.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                egui::Event::Key { key, pressed: true, modifiers, .. } => crate::ui::key_name(*key).map(|n| KeyChord {
                    ctrl: modifiers.command,
                    alt: modifiers.alt,
                    shift: modifiers.shift,
                    key: n.to_string(),
                }),
                _ => None,
            })
            .collect()
    });
    for chord in &pressed {
        if let Some(row) =
            SHORTCUTS.iter().position(|(_, acts)| acts.iter().any(|a| shortcuts.keys(*a).contains(chord)))
        {
            st.lit = Some((row, now));
        }
    }
    let lit = st.lit.filter(|(_, t)| now - t < 0.8).map(|(r, _)| r);
    if lit.is_some() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
    }
    ui.painter().text(
        o + vec2(SIZE.x / 2.0, 60.0 + 21.0),
        Align2::CENTER_CENTER,
        "Learn the basic keyboard shortcuts.",
        font::regular(24.0),
        p.fg_a1,
    );
    for (i, (label, actions)) in SHORTCUTS.iter().enumerate() {
        let (col, row) = (i / 6, i % 6);
        let (lx, kx) = if col == 0 { (62.0, 302.0) } else { (605.0, 845.0) };
        let y = 154.0 + 82.0 * row as f32;
        let on = lit == Some(i);
        ui.painter().text(
            o + vec2(lx, y + 21.0),
            Align2::LEFT_CENTER,
            *label,
            font::light(16.0),
            if on { p.fg1 } else { p.fg_a1 },
        );
        let mut x = o.x + kx;
        let chords: Vec<Option<KeyChord>> = actions.iter().map(|a| shortcuts.keys(*a).first().cloned()).collect();
        // "[SHIFT] [<-] / [->]": shared modifiers are drawn once.
        let shared = match chords.as_slice() {
            [Some(a), rest @ ..]
                if !rest.is_empty() && rest.iter().all(|c| c.as_ref().is_some_and(|c| same_mods(a, c))) =>
            {
                Some(a.clone())
            }
            _ => None,
        };
        if let Some(m) = &shared {
            let caps = cap_label(m);
            for cap in &caps[..caps.len() - 1] {
                x += key_cap(ui, x, o.y + y, cap, on) + 4.0;
            }
        }
        for (k, chord) in chords.iter().enumerate() {
            if k > 0 {
                ui.painter().text(
                    pos2(x + 9.0, o.y + y + 21.0),
                    Align2::CENTER_CENTER,
                    "/",
                    font::light(16.0),
                    p.fg_a2,
                );
                x += 18.0;
            }
            match chord {
                Some(chord) => {
                    let caps = cap_label(chord);
                    let caps = if shared.is_some() { &caps[caps.len() - 1..] } else { &caps[..] };
                    for (n, cap) in caps.iter().enumerate() {
                        if n > 0 {
                            x += 4.0;
                        }
                        x += key_cap(ui, x, o.y + y, cap, on);
                    }
                }
                None => {
                    ui.painter().text(
                        pos2(x, o.y + y + 21.0),
                        Align2::LEFT_CENTER,
                        "Not set",
                        font::light(14.0),
                        p.fg_a3,
                    );
                    x += 60.0;
                }
            }
            x += 6.0;
        }
    }
    ui.painter().text(
        o + vec2(62.0, 671.0 + 14.0),
        Align2::LEFT_CENTER,
        "Find the full list in the menu.",
        font::regular(16.0),
        p.fg_a1,
    );
}
