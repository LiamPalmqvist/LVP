//! Use-case tests for `App` against the fake engine.

use std::sync::Arc;

use super::App;
use super::fake::{FakeEngine, FakeProbe};
use super::ports::{EngineCommand, EngineEvent, MediaEngine, PlayerState};
use super::testing::MemStore;
use crate::domain::media::MediaItem;
use crate::domain::settings::RepeatMode;
use crate::domain::shortcuts::Action;

fn setup() -> (tempfile::TempDir, Arc<FakeEngine>, App) {
    let dir = tempfile::tempdir().unwrap();
    let fake = Arc::new(FakeEngine::new());
    let engine: Arc<dyn MediaEngine> = fake.clone();
    let app = App::new(engine, Arc::new(FakeProbe), Arc::new(MemStore::new(dir.path())));
    (dir, fake, app)
}

fn items(n: usize) -> Vec<MediaItem> {
    (0..n).map(|i| MediaItem::file(format!("/media/clip{i}.mp4"))).collect()
}

fn opened(fake: &FakeEngine) -> Vec<String> {
    fake.sent
        .lock()
        .iter()
        .filter_map(|c| match c {
            EngineCommand::Open(r) => Some(r.item.source.display_name()),
            _ => None,
        })
        .collect()
}

#[test]
fn play_items_opens_the_chosen_item() {
    let (_d, fake, mut app) = setup();
    app.play_items(items(3), 1, Some("Folder".into()));
    assert_eq!(opened(&fake), vec!["clip1.mp4"]);
    assert_eq!(app.queue.current, Some(1));
    assert_eq!(app.queue.origin.as_deref(), Some("Folder"));
}

#[test]
fn end_of_item_advances_and_repeat_all_wraps() {
    let (_d, fake, mut app) = setup();
    app.play_items(items(2), 0, None);
    fake.events.lock().push(EngineEvent::EndReached);
    app.tick();
    assert_eq!(app.queue.current, Some(1));
    app.set_repeat(RepeatMode::All);
    fake.events.lock().push(EngineEvent::EndReached);
    app.tick();
    assert_eq!(app.queue.current, Some(0));
    assert_eq!(opened(&fake), vec!["clip0.mp4", "clip1.mp4", "clip0.mp4"]);
}

#[test]
fn previous_restarts_after_three_seconds() {
    let (_d, fake, mut app) = setup();
    app.play_items(items(3), 2, None);
    fake.status.lock().time_ms = 10_000;
    fake.status.lock().seekable = true;
    app.tick();
    app.previous();
    assert_eq!(app.queue.current, Some(2), "restarts instead of going back");
    assert!(matches!(fake.sent.lock().last(), Some(EngineCommand::SeekMs { ms: 0, .. })));
    fake.status.lock().time_ms = 1_000;
    app.tick();
    app.previous();
    assert_eq!(app.queue.current, Some(1));
}

#[test]
fn ab_loop_jumps_back_to_a() {
    let (_d, fake, mut app) = setup();
    app.play_items(items(1), 0, None);
    fake.status.lock().time_ms = 5_000;
    app.tick();
    app.cycle_ab_loop();
    fake.status.lock().time_ms = 9_000;
    app.tick();
    app.cycle_ab_loop();
    assert_eq!((app.ab_loop.a, app.ab_loop.b), (Some(5_000), Some(9_000)));
    {
        let mut st = fake.status.lock();
        st.time_ms = 9_100;
        st.state = PlayerState::Playing;
    }
    app.tick();
    assert!(matches!(fake.sent.lock().last(), Some(EngineCommand::SeekMs { ms: 5_000, .. })));
}

#[test]
fn volume_is_clamped_to_the_maximum() {
    let (_d, _fake, mut app) = setup();
    app.settings.audio.max_volume = 125;
    app.set_volume(400);
    assert_eq!(app.status.volume, 125);
    assert_eq!(app.settings.audio.volume, 125);
}

#[test]
fn shortcut_jump_uses_the_skip_setting() {
    let (_d, fake, mut app) = setup();
    app.play_items(items(1), 0, None);
    {
        let mut st = fake.status.lock();
        st.time_ms = 60_000;
        st.length_ms = 600_000;
        st.seekable = true;
    }
    app.tick();
    app.settings.controls.skip_short_s = 15;
    assert!(app.perform(Action::JumpForwardShort).is_none());
    assert!(matches!(fake.sent.lock().last(), Some(EngineCommand::SeekMs { ms: 75_000, .. })));
    assert_eq!(app.perform(Action::ToggleFullscreen), Some(super::UiCommand::ToggleFullscreen));
}

#[test]
fn settings_changes_reconfigure_and_persist() {
    let (_d, fake, mut app) = setup();
    app.update_settings(|s| s.network.network_caching_ms = 3000);
    assert!(fake.sent.lock().iter().any(|c| matches!(c, EngineCommand::Reconfigure(_))));
    app.save_settings_now();
    let saved = app.store.load("settings").expect("settings saved");
    assert!(saved.contains("\"network_caching_ms\": 3000"));
}

#[test]
fn resume_point_is_used_when_reopening() {
    let (_d, fake, mut app) = setup();
    let item = MediaItem::file("/media/movie.mkv");
    app.library.save_position(&item.source, 120_000, 3_600_000);
    app.play_items(vec![item], 0, None);
    let start = fake.sent.lock().iter().find_map(|c| match c {
        EngineCommand::Open(r) => Some(r.start_ms),
        _ => None,
    });
    assert_eq!(start, Some(Some(120_000)));
}

#[test]
fn reopening_a_file_restores_its_tracks_and_delays() {
    let (_d, fake, mut app) = setup();
    app.play_items(items(2), 0, None);
    app.set_subtitle_track(4);
    app.set_audio_track(2);
    app.set_subtitle_delay(-4_000_000);
    app.play_index(1);
    app.play_index(0);
    let last = fake.sent.lock().iter().rev().find_map(|c| match c {
        EngineCommand::Open(r) => Some((**r).clone()),
        _ => None,
    });
    let r = last.unwrap();
    assert_eq!(r.item.source.display_name(), "clip0.mp4");
    assert_eq!((r.subtitle_track, r.audio_track, r.subtitle_delay_us), (Some(4), Some(2), -4_000_000));
}
