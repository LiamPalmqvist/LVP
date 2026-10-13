//! The Keyboard Shortcuts dialog (frame `2437:14832`): every bindable action
//! by category with its hotkey and global hotkey. Click a cell and press a
//! key to record it; Esc cancels, Backspace clears. Conflicts are flagged
//! with an offer to move the key. Nothing is stored until Save.

use egui::{
    Align2, Color32, CornerRadius, Event, EventFilter, Id, Key, Rect, RichText, Sense, Stroke, StrokeKind, Ui,
    UiBuilder, Vec2, pos2, vec2,
};

use super::kit::{self, Btn};
use crate::app::App;
use crate::domain::shortcuts::{Action, CATEGORIES, KeyChord, Shortcuts};
use crate::ui::icons::Icon;
use crate::ui::state::UiState;
use crate::ui::theme::{font, with_alpha};
use crate::ui::widgets::{self, paint_icon, pal};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Field {
    #[default]
    All,
    Action,
    Hotkey,
    Global,
}

impl Field {
    const ALL: [Field; 4] = [Field::All, Field::Action, Field::Hotkey, Field::Global];
    fn label(self) -> &'static str {
        match self {
            Field::All => "All Fields",
            Field::Action => "Action",
            Field::Hotkey => "Hotkey",
            Field::Global => "Global Hotkey",
        }
    }
}

/// A recorded chord that is already used elsewhere, waiting for a decision.
#[derive(Debug, Clone, PartialEq)]
pub struct Conflict {
    pub action: Action,
    pub global: bool,
    pub chord: KeyChord,
    pub others: Vec<Action>,
}

#[derive(Default)]
pub struct ShortcutsState {
    /// Edited bindings; `None` until the dialog opens.
    pub draft: Option<Shortcuts>,
    pub search: String,
    pub field: Field,
    /// The cell waiting for a key press: (action, global column).
    pub recording: Option<(Action, bool)>,
    pub conflict: Option<Conflict>,
}

fn keys_of(s: &Shortcuts, a: Action, global: bool) -> Vec<KeyChord> {
    if global { s.global.get(&a).cloned().unwrap_or_default() } else { s.keys(a) }
}

fn set_keys(s: &mut Shortcuts, a: Action, global: bool, keys: Vec<KeyChord>) {
    if global {
        if keys.is_empty() {
            s.global.remove(&a);
        } else {
            s.global.insert(a, keys);
        }
    } else {
        s.set(a, keys);
    }
}

fn conflicts(s: &Shortcuts, a: Action, global: bool, chord: &KeyChord) -> Vec<Action> {
    if global {
        s.global.iter().filter(|(x, keys)| **x != a && keys.contains(chord)).map(|(x, _)| *x).collect()
    } else {
        s.conflicts(a, chord)
    }
}

/// "Cmd + K, ]" style text for a cell.
pub fn chord_text(keys: &[KeyChord]) -> String {
    keys.iter().map(|k| k.to_string()).collect::<Vec<_>>().join(",  ")
}

/// Records `chord` for the cell, or raises a conflict.
pub fn assign(s: &mut Shortcuts, action: Action, global: bool, chord: KeyChord) -> Option<Conflict> {
    let others = conflicts(s, action, global, &chord);
    if !others.is_empty() {
        return Some(Conflict { action, global, chord, others });
    }
    set_keys(s, action, global, vec![chord]);
    None
}

/// Moves a conflicting chord: removes it from the other actions, then assigns it.
pub fn reassign(s: &mut Shortcuts, c: &Conflict) {
    for o in &c.others {
        let keys: Vec<KeyChord> = keys_of(s, *o, c.global).into_iter().filter(|k| *k != c.chord).collect();
        set_keys(s, *o, c.global, keys);
    }
    set_keys(s, c.action, c.global, vec![c.chord.clone()]);
}

pub fn show(ctx: &egui::Context, app: &mut App, st: &mut UiState) {
    if !st.dialogs.shortcuts {
        if st.settings.shortcuts.draft.is_some() {
            st.settings.shortcuts = Default::default();
        }
        return;
    }
    let ss = &mut st.settings.shortcuts;
    if ss.draft.is_none() {
        ss.draft = Some(app.settings.controls.shortcuts.clone());
    }
    capture_keys(ctx, ss);

    let p = pal(ctx);
    let screen = ctx.content_rect();
    let size = vec2(1048f32.min(screen.width() - 24.0), 800f32.min(screen.height() - 24.0)).max(vec2(600.0, 400.0));
    let mut close = false;
    let mut save = false;
    egui::Modal::new(Id::new("shortcuts-dialog"))
        .frame(egui::Frame::NONE)
        .backdrop_color(Color32::from_black_alpha(if st.settings.open { 60 } else { 128 }))
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            let (c, s) = dialog(ui, rect, ss, &app.settings.controls.shortcuts);
            close = c;
            save = s;
        });
    let escape = ss.recording.is_none()
        && ss.conflict.is_none()
        && !egui::Popup::is_any_open(ctx)
        && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape));
    if save {
        if let Some(d) = ss.draft.take() {
            app.update_settings(|s| s.controls.shortcuts = d);
            app.toast("Keyboard shortcuts saved");
        }
        close = true;
    }
    let _ = p;
    if close || escape {
        st.dialogs.shortcuts = false;
        st.settings.shortcuts = Default::default();
        ctx.memory_mut(|m| {
            if let Some(id) = m.focused() {
                m.surrender_focus(id);
            }
        });
    }
}

fn cell_id(a: Action, global: bool) -> Id {
    Id::new(("shortcut-cell", a, global))
}

/// While a cell is recording, turns the next key press into its chord.
fn capture_keys(ctx: &egui::Context, ss: &mut ShortcutsState) {
    let Some((action, global)) = ss.recording else { return };
    let keys: Vec<(Key, egui::Modifiers)> = ctx.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                Event::Key { key, pressed: true, repeat: false, modifiers, .. } => Some((*key, *modifiers)),
                _ => None,
            })
            .collect()
    });
    let Some((key, m)) = keys.first().copied() else { return };
    let id = cell_id(action, global);
    let stop = |ctx: &egui::Context| ctx.memory_mut(|mem| mem.surrender_focus(id));
    let Some(draft) = ss.draft.as_mut() else { return };
    match key {
        Key::Escape if !m.any() => {
            ss.recording = None;
            stop(ctx);
        }
        Key::Backspace | Key::Delete if !m.any() => {
            set_keys(draft, action, global, Vec::new());
            ss.recording = None;
            stop(ctx);
        }
        _ => {
            if let Some(name) = crate::ui::key_name(key) {
                let chord = KeyChord { ctrl: m.command, alt: m.alt, shift: m.shift, key: name.to_string() };
                ss.conflict = assign(draft, action, global, chord);
                ss.recording = None;
                stop(ctx);
            }
        }
    }
}

/// Draws the dialog; returns (close, save).
fn dialog(ui: &mut Ui, rect: Rect, ss: &mut ShortcutsState, saved: &Shortcuts) -> (bool, bool) {
    let p = pal(ui.ctx());
    ui.painter().add(
        egui::epaint::Shadow { offset: [0, 12], blur: 40, spread: 0, color: Color32::from_black_alpha(110) }
            .as_shape(rect, CornerRadius::same(14)),
    );
    let fill = if p.dark { p.bg_b1_t } else { with_alpha(p.bg_b1, 0.98) };
    ui.painter().rect_filled(rect, CornerRadius::same(14), fill);
    let header = Rect::from_min_size(rect.min, vec2(rect.width(), 96.0));
    ui.painter().rect_filled(header, CornerRadius::same(14), if p.dark { Color32::BLACK } else { p.chrome });
    ui.painter().text(
        pos2(rect.min.x + 19.0, rect.min.y + 25.0),
        Align2::LEFT_CENTER,
        "Keyboard Shortcuts",
        font::regular(14.0),
        p.fg_a1,
    );
    let close_r = Rect::from_min_size(pos2(rect.max.x - 41.0, rect.min.y + 9.0), vec2(32.0, 32.0));
    let mut close = kit::place(ui, UiBuilder::new().max_rect(close_r), |ui| {
        widgets::icon_button_sized(ui, Icon::Close, vec2(32.0, 32.0), 24.0, false, "Close without saving").clicked()
    });

    // Search and field filter.
    let search_r =
        Rect::from_min_max(pos2(rect.min.x + 9.0, rect.min.y + 53.0), pos2(rect.max.x - 219.0, rect.min.y + 87.0));
    kit::place(ui, UiBuilder::new().max_rect(search_r), |ui| {
        widgets::search_field(ui, &mut ss.search, "Search Keyboard Shortcuts...", search_r.width());
    });
    let field_r = Rect::from_min_size(pos2(rect.max.x - 209.0, rect.min.y + 53.0), vec2(200.0, 34.0));
    let fresp = ui.interact(field_r, Id::new("shortcut-field"), Sense::click());
    ui.painter().rect_filled(field_r, CornerRadius::same(6), if fresp.hovered() { p.bg_a3 } else { p.bg_a5 });
    ui.painter().text(
        pos2(field_r.min.x + 12.0, field_r.center().y),
        Align2::LEFT_CENTER,
        ss.field.label(),
        font::regular(14.0),
        p.fg_a1,
    );
    paint_icon(
        ui,
        Rect::from_min_size(pos2(field_r.max.x - 32.0, field_r.center().y - 12.0), Vec2::splat(24.0)),
        Icon::ChevronDown,
        p.fg_a1,
    );
    egui::Popup::menu(&fresp).id(Id::new("shortcut-field-menu")).width(200.0).frame(widgets::menu_frame(&p)).show(
        |ui| {
            ui.set_min_width(188.0);
            for f in Field::ALL {
                if widgets::menu_row(ui, None, f.label(), "", f == ss.field).clicked() {
                    ss.field = f;
                }
            }
        },
    );

    // Footer.
    let footer = Rect::from_min_max(pos2(rect.min.x, rect.max.y - 72.0), rect.max);
    let dirty = ss.draft.as_ref().is_some_and(|d| d != saved);
    let cancel_r = Rect::from_min_size(pos2(footer.min.x + 20.0, footer.min.y + 20.0), vec2(100.0, 32.0));
    let cancel = ui.interact(cancel_r, Id::new("shortcut-cancel"), Sense::click());
    ui.painter().rect_filled(cancel_r, CornerRadius::same(6), if cancel.hovered() { p.bg_a1 } else { p.bg_a3 });
    ui.painter().text(cancel_r.center(), Align2::CENTER_CENTER, "Cancel", font::semibold(14.0), p.fg_a2);
    if cancel.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        close = true;
    }
    let save_r = Rect::from_min_size(pos2(footer.max.x - 120.0, footer.min.y + 20.0), vec2(100.0, 32.0));
    let save = ui.interact(save_r, Id::new("shortcut-save"), if dirty { Sense::click() } else { Sense::hover() });
    let a = if dirty { 1.0 } else { 0.5 };
    let bg = if save.hovered() && dirty { p.accent_dim } else { p.bg_accent };
    ui.painter().rect_filled(save_r, CornerRadius::same(6), with_alpha(bg, a));
    ui.painter().text(save_r.center(), Align2::CENTER_CENTER, "Save", font::semibold(14.0), with_alpha(p.accent, a));
    let save = save.clicked() && dirty;

    // Conflict bar above the footer.
    let mut body_bottom = footer.min.y;
    if let Some(c) = ss.conflict.clone() {
        let bar = Rect::from_min_max(
            pos2(rect.min.x + 20.0, footer.min.y - 52.0),
            pos2(rect.max.x - 20.0, footer.min.y - 4.0),
        );
        body_bottom = bar.min.y - 4.0;
        ui.painter().rect_filled(bar, CornerRadius::same(6), p.bg_accent_dark);
        ui.painter().rect_stroke(bar, CornerRadius::same(6), Stroke::new(1.0, p.bg_accent), StrokeKind::Inside);
        paint_icon(
            ui,
            Rect::from_min_size(pos2(bar.min.x + 12.0, bar.center().y - 12.0), Vec2::splat(24.0)),
            Icon::Info,
            p.accent,
        );
        let names: Vec<String> = c.others.iter().map(|o| format!("\u{201c}{}\u{201d}", o.label())).collect();
        let msg = format!(
            "{} is already used by {}. Move it to \u{201c}{}\u{201d}?",
            c.chord,
            names.join(", "),
            c.action.label()
        );
        widgets::text_clipped(
            ui,
            pos2(bar.min.x + 46.0, bar.center().y),
            Align2::LEFT_CENTER,
            &msg,
            font::regular(14.0),
            p.fg1,
            bar.width() - 290.0,
        );
        let btns = Rect::from_min_max(pos2(bar.max.x - 230.0, bar.min.y + 8.0), pos2(bar.max.x - 8.0, bar.max.y - 8.0));
        kit::place(
            ui,
            UiBuilder::new().max_rect(btns).layout(egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if kit::action_button(ui, "Reassign", Btn::Accent, 100.0).clicked() {
                    if let Some(d) = ss.draft.as_mut() {
                        reassign(d, &c);
                    }
                    ss.conflict = None;
                }
                if kit::action_button(ui, "Keep", Btn::Normal, 100.0).clicked() {
                    ss.conflict = None;
                }
            },
        );
    }

    // Table.
    let body = Rect::from_min_max(pos2(rect.min.x, rect.min.y + 96.0), pos2(rect.max.x, body_bottom));
    kit::place(ui, UiBuilder::new().max_rect(body), |ui| {
        ui.set_clip_rect(body.intersect(ui.clip_rect()));
        egui::ScrollArea::vertical().id_salt("shortcuts-table").auto_shrink([false, false]).show(ui, |ui| {
            egui::Frame::new().inner_margin(egui::Margin { left: 20, right: 20, top: 20, bottom: 12 }).show(ui, |ui| {
                ui.set_width(body.width() - 40.0);
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                table(ui, ss);
            });
        });
    });
    (close, save)
}

/// Cell columns relative to the table's left edge (from the design).
const HOTKEY_X: f32 = 379.0;
const GLOBAL_X: f32 = 738.0;
const CELL_W: f32 = 270.0;

fn table(ui: &mut Ui, ss: &mut ShortcutsState) {
    let p = pal(ui.ctx());
    let w = ui.available_width();
    let (hr, _) = ui.allocate_exact_size(vec2(w, 25.0), Sense::hover());
    let scale = (w / 1008.0).min(1.0);
    let hx = hr.min.x + HOTKEY_X * scale;
    let gx = hr.min.x + (GLOBAL_X * scale).max(w - CELL_W);
    let cw = CELL_W.min(gx - hx - 20.0);
    let head = |x: f32, t: &str, align: Align2| {
        ui.painter().text(pos2(x, hr.center().y), align, t, font::regular(14.0), p.fg2);
    };
    head(hr.min.x, "Action", Align2::LEFT_CENTER);
    head(hx + cw / 2.0, "Hotkey", Align2::CENTER_CENTER);
    head(gx + cw / 2.0, "Global Hotkey", Align2::CENTER_CENTER);

    let Some(draft) = ss.draft.clone() else { return };
    let q = ss.search.trim().to_lowercase();
    let field = ss.field;
    let matches = |a: Action| {
        if q.is_empty() {
            return true;
        }
        let in_action = || a.label().to_lowercase().contains(&q) || a.category().to_lowercase().contains(&q);
        let in_keys = |global: bool| chord_text(&keys_of(&draft, a, global)).to_lowercase().contains(&q);
        match field {
            Field::All => in_action() || in_keys(false) || in_keys(true),
            Field::Action => in_action(),
            Field::Hotkey => in_keys(false),
            Field::Global => in_keys(true),
        }
    };
    let groups: Vec<(&str, Vec<Action>)> = CATEGORIES
        .iter()
        .map(|cat| (*cat, Action::ALL.iter().copied().filter(|a| a.category() == *cat && matches(*a)).collect()))
        .collect();
    let mut any = false;
    for (cat, actions) in groups {
        if actions.is_empty() {
            continue;
        }
        any = true;
        ui.add_space(20.0);
        let (dr, _) = ui.allocate_exact_size(vec2(w, 1.0), Sense::hover());
        ui.painter().rect_filled(dr, 0, p.divider_dim);
        ui.add_space(20.0);
        let (cr, _) = ui.allocate_exact_size(vec2(w, 28.0), Sense::hover());
        ui.painter().text(pos2(cr.min.x, cr.center().y), Align2::LEFT_CENTER, cat, font::regular(16.0), p.fg2);
        ui.add_space(20.0);
        for (i, a) in actions.iter().enumerate() {
            if i > 0 {
                ui.add_space(14.0);
            }
            let (r, _) = ui.allocate_exact_size(vec2(w, 33.0), Sense::hover());
            widgets::text_clipped(
                ui,
                pos2(r.min.x, r.center().y),
                Align2::LEFT_CENTER,
                a.label(),
                font::regular(14.0),
                p.fg_a1,
                hx - r.min.x - 16.0,
            );
            for (global, x) in [(false, hx), (true, gx)] {
                let cell = Rect::from_min_size(pos2(x, r.min.y), vec2(cw, 33.0));
                key_cell(ui, ss, &draft, *a, global, cell);
            }
        }
    }
    if !any {
        ui.add_space(60.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("No shortcuts match your search").font(font::medium(16.0)).color(p.fg_a1));
        });
    }
}

fn key_cell(ui: &mut Ui, ss: &mut ShortcutsState, draft: &Shortcuts, a: Action, global: bool, cell: Rect) {
    let p = pal(ui.ctx());
    let id = cell_id(a, global);
    let resp = ui.interact(cell, id, Sense::click());
    let recording = ss.recording == Some((a, global));
    let conflicted = ss.conflict.as_ref().is_some_and(|c| c.action == a && c.global == global);
    let bg = if recording || resp.hovered() { p.bg_a5 } else { p.bg_b2 };
    ui.painter().rect_filled(cell, CornerRadius::same(6), bg);
    if recording || conflicted {
        ui.painter().rect_stroke(cell, CornerRadius::same(6), Stroke::new(1.5, p.accent), StrokeKind::Inside);
    }
    let keys = keys_of(draft, a, global);
    let (text, col, f) = if recording {
        ("Press a key...".to_string(), p.accent, font::regular(13.0))
    } else {
        (chord_text(&keys), p.fg2, font::mono(14.0))
    };
    widgets::text_clipped(ui, cell.center(), Align2::CENTER_CENTER, &text, f, col, cell.width() - 70.0);
    let icon_col = if resp.hovered() || recording { p.fg_a1 } else { p.fg_a3 };
    paint_icon(
        ui,
        Rect::from_min_size(pos2(cell.max.x - 29.0, cell.center().y - 12.0), Vec2::splat(24.0)),
        Icon::Edit,
        icon_col,
    );
    let tip = if global {
        "Click, then press the keys to use even when the app is in the background. Backspace clears."
    } else {
        "Click, then press the new keys. Esc cancels, Backspace clears."
    };
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip);
    if resp.clicked() {
        if recording {
            ss.recording = None;
            resp.surrender_focus();
        } else {
            ss.recording = Some((a, global));
            ss.conflict = None;
            resp.request_focus();
        }
    }
    if recording {
        ui.memory_mut(|m| {
            m.request_focus(id);
            m.set_focus_lock_filter(
                id,
                EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true },
            );
        });
    }
    resp.context_menu(|ui| {
        if widgets::menu_row(ui, None, "Reset to default", "", false).clicked()
            && let Some(d) = ss.draft.as_mut()
        {
            if global {
                d.global.remove(&a);
            } else {
                d.set(a, a.default_keys());
            }
            ui.close();
        }
        if widgets::menu_row(ui, None, "Remove shortcut", "", false).clicked()
            && let Some(d) = ss.draft.as_mut()
        {
            set_keys(d, a, global, Vec::new());
            ui.close();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assign_flags_conflicts_and_reassign_moves_the_key() {
        let mut s = Shortcuts::default();
        let space = KeyChord::parse("Space").unwrap();
        let c = assign(&mut s, Action::Stop, false, space.clone()).expect("Space is Play / pause");
        assert_eq!(c.others, vec![Action::PlayPause]);
        assert_eq!(s.keys(Action::Stop), Action::Stop.default_keys());
        reassign(&mut s, &c);
        assert_eq!(s.keys(Action::Stop), vec![space.clone()]);
        assert!(s.keys(Action::PlayPause).is_empty());
        assert_eq!(s.action_for(&space), Some(Action::Stop));
    }

    #[test]
    fn global_keys_are_separate() {
        let mut s = Shortcuts::default();
        let k = KeyChord::parse("Ctrl+Alt+P").unwrap();
        assert!(assign(&mut s, Action::PlayPause, true, k.clone()).is_none());
        assert_eq!(s.global.get(&Action::PlayPause), Some(&vec![k.clone()]));
        let c = assign(&mut s, Action::Stop, true, k).expect("conflict among globals");
        assert_eq!(c.others, vec![Action::PlayPause]);
    }
}
