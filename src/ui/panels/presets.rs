//! The "Default Preset" bar at the foot of quick settings (`2372:8834`):
//! saves and applies named sets of audio and video effects.

use egui::{Ui, vec2};

use super::kit;
use crate::app::App;
use crate::app::presets::EffectPreset;
use crate::ui::icons::Icon;
use crate::ui::state::UiState;
use crate::ui::widgets::{self, pal};

#[derive(Default)]
pub struct PresetUi {
    /// Saved presets, reloaded after every change.
    cache: Option<Vec<EffectPreset>>,
    /// The name typed for a new preset.
    new_name: String,
}

pub fn bar(ui: &mut Ui, app: &mut App, st: &mut UiState) {
    let p = pal(ui.ctx());
    let pu = &mut st.panels.presets;
    let list = pu.cache.get_or_insert_with(|| app.effect_presets()).clone();
    let is_default =
        app.settings.audio.effects == Default::default() && app.settings.video.effects == Default::default();
    let current = list.iter().find(|x| x.matches(app)).map(|x| x.name.clone());
    let text = match (&current, is_default) {
        (Some(n), _) => n.clone(),
        (None, true) => "Default Preset".to_string(),
        (None, false) => "Custom Effects".to_string(),
    };
    let resp = kit::preset_box(ui, &text);
    let w = resp.rect.width();
    let mut changed = false;
    egui::Popup::menu(&resp)
        .id(ui.make_persistent_id("preset-list"))
        .width(w)
        .gap(8.0)
        .align(egui::RectAlign::TOP_START)
        .frame(kit::list_frame(&p))
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_min_width(w - 4.0);
            ui.set_max_width(w - 4.0);
            ui.spacing_mut().item_spacing = vec2(6.0, 2.0);
            // New preset: name it, then save the current effects under it.
            ui.horizontal(|ui| {
                ui.set_min_height(44.0);
                ui.add_space(8.0);
                let (r, _) = ui.allocate_exact_size(vec2(24.0, 24.0), egui::Sense::hover());
                kit::paint_plus(ui, r, p.fg_a3);
                let field_w = ui.available_width() - 84.0;
                let te = egui::TextEdit::singleline(&mut pu.new_name)
                    .hint_text(egui::RichText::new("New Preset").color(p.fg_a3))
                    .frame(false)
                    .font(crate::ui::theme::font::regular(14.0))
                    .text_color(p.fg_a1)
                    .desired_width(field_w);
                let r = ui.add(te);
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let save = kit::chip(ui, "Save", "Save the current effects as a preset").clicked();
                if (enter || save) && !pu.new_name.trim().is_empty() {
                    app.save_effect_preset(&pu.new_name);
                    pu.new_name.clear();
                    changed = true;
                }
            });
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for preset in &list {
                    let on = current.as_deref() == Some(preset.name.as_str());
                    let r = kit::option_row(ui, Some(Icon::More), &preset.name, on, false);
                    if r.clicked() {
                        app.apply_effect_preset(Some(preset));
                        ui.close();
                    }
                    egui::Popup::context_menu(&r).frame(widgets::menu_frame(&p)).show(|ui| {
                        ui.set_min_width(220.0);
                        if widgets::menu_row(ui, Some(Icon::Save), "Update with Current Effects", "", false).clicked() {
                            app.save_effect_preset(&preset.name);
                            changed = true;
                        }
                        if widgets::menu_row(ui, Some(Icon::Remove), "Delete Preset", "", false).clicked() {
                            app.delete_effect_preset(&preset.name);
                            changed = true;
                        }
                    });
                }
            });
            if kit::option_row(ui, None, "Default Preset", current.is_none() && is_default, false).clicked() {
                app.apply_effect_preset(None);
                ui.close();
            }
        });
    if changed {
        pu.cache = None;
    }
}
