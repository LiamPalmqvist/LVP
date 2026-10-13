//! Named effect presets: the quick-settings "Default Preset" bar saves the
//! current audio and video effects under a name and applies them later.
//! Stored as JSON under the store key `effect_presets`.

use serde::{Deserialize, Serialize};

use super::App;
use crate::domain::effects::{AudioEffects, VideoEffects};

const KEY: &str = "effect_presets";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct EffectPreset {
    pub name: String,
    pub audio: AudioEffects,
    pub video: VideoEffects,
}

impl EffectPreset {
    /// Whether these are the effects currently applied.
    pub fn matches(&self, app: &App) -> bool {
        self.audio == app.settings.audio.effects && self.video == app.settings.video.effects
    }
}

impl App {
    /// Saved presets, in the order they were created.
    pub fn effect_presets(&self) -> Vec<EffectPreset> {
        self.store.load(KEY).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    fn store_effect_presets(&mut self, list: &[EffectPreset]) {
        match serde_json::to_string_pretty(list) {
            Ok(json) => {
                if let Err(e) = self.store.save(KEY, &json) {
                    self.toast_error(format!("Could not save the presets: {e:#}"));
                }
            }
            Err(e) => log::error!("serialising presets: {e}"),
        }
    }

    /// Saves the current effects under `name`, replacing a preset of that name.
    pub fn save_effect_preset(&mut self, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let preset = EffectPreset {
            name: name.to_string(),
            audio: self.settings.audio.effects.clone(),
            video: self.settings.video.effects.clone(),
        };
        let mut list = self.effect_presets();
        match list.iter_mut().find(|p| p.name == name) {
            Some(p) => *p = preset,
            None => list.push(preset),
        }
        self.store_effect_presets(&list);
        self.toast(format!("Saved preset \"{name}\""));
    }

    pub fn delete_effect_preset(&mut self, name: &str) {
        let mut list = self.effect_presets();
        list.retain(|p| p.name != name);
        self.store_effect_presets(&list);
    }

    pub fn rename_effect_preset(&mut self, from: &str, to: &str) {
        let to = to.trim();
        let mut list = self.effect_presets();
        if to.is_empty() || list.iter().any(|p| p.name == to) {
            return;
        }
        if let Some(p) = list.iter_mut().find(|p| p.name == from) {
            p.name = to.to_string();
        }
        self.store_effect_presets(&list);
    }

    /// Applies a preset's effects; `None` restores VLC's defaults.
    pub fn apply_effect_preset(&mut self, preset: Option<&EffectPreset>) {
        let (audio, video, name) = match preset {
            Some(p) => (p.audio.clone(), p.video.clone(), p.name.as_str()),
            None => (AudioEffects::default(), VideoEffects::default(), "Default Preset"),
        };
        self.set_audio_effects(audio);
        self.set_video_effects(video);
        self.show_osd(format!("Preset: {name}"));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::app::fake::{FakeEngine, FakeProbe};
    use crate::app::testing::MemStore;

    #[test]
    fn presets_round_trip_and_apply() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = App::new(Arc::new(FakeEngine::new()), Arc::new(FakeProbe), Arc::new(MemStore::new(dir.path())));
        assert!(app.effect_presets().is_empty());
        let mut fx = app.settings.audio.effects.clone();
        fx.equalizer.enabled = true;
        fx.equalizer.apply_preset(13);
        app.set_audio_effects(fx.clone());
        app.save_effect_preset("Rock night");
        let list = app.effect_presets();
        assert_eq!(list.len(), 1);
        assert!(list[0].matches(&app));
        app.apply_effect_preset(None);
        assert_eq!(app.settings.audio.effects, AudioEffects::default());
        assert!(!list[0].matches(&app));
        app.apply_effect_preset(Some(&list[0]));
        assert_eq!(app.settings.audio.effects, fx);
        app.rename_effect_preset("Rock night", "Loud");
        assert_eq!(app.effect_presets()[0].name, "Loud");
        app.save_effect_preset("Loud");
        assert_eq!(app.effect_presets().len(), 1);
        app.delete_effect_preset("Loud");
        assert!(app.effect_presets().is_empty());
    }
}
