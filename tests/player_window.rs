//! Player window modes driven by shortcuts, headlessly: fullscreen, hide UI,
//! picture in picture, and the Escape ladder that leaves them. The harness
//! does not keep viewport commands, so this checks the UI state that issues
//! them.

use std::sync::Arc;

use egui::{Event, Key, Modifiers};
use egui_kittest::Harness;
use notvlc::app::App;
use notvlc::app::fake::{FakeEngine, FakeProbe};
use notvlc::domain::media::MediaItem;
use notvlc::infra::store::JsonStore;
use notvlc::ui::NotVlcApp;
use notvlc::ui::state::View;

fn harness(dir: &std::path::Path) -> Harness<'static, NotVlcApp> {
    let store = Arc::new(JsonStore::at(dir.join("data"), dir.join("cache")));
    let mut app = App::new(Arc::new(FakeEngine::new()), Arc::new(FakeProbe), store);
    app.settings.general.onboarding_done = true;
    app.play_items(vec![MediaItem::file("/media/clip.mp4")], 0, None);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_eframe(|cc| NotVlcApp::new(&cc.egui_ctx, app, Vec::new()));
    h.state_mut().st.view = View::Player;
    h.run_steps(2);
    h
}

fn press(h: &mut Harness<'static, NotVlcApp>, key: Key, modifiers: Modifiers) {
    for pressed in [true, false] {
        h.event(Event::Key { key, physical_key: None, pressed, repeat: false, modifiers });
    }
    h.step();
}

#[test]
fn fullscreen_hide_ui_and_pip() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(dir.path());

    press(&mut h, Key::F, Modifiers::NONE);
    assert!(h.state().st.fullscreen);
    press(&mut h, Key::Escape, Modifiers::NONE);
    assert!(!h.state().st.fullscreen, "Esc leaves fullscreen");

    press(&mut h, Key::Backslash, Modifiers::COMMAND);
    assert!(h.state().st.hide_ui, "Ctrl+\\ hides the interface");
    press(&mut h, Key::Escape, Modifiers::NONE);
    assert!(!h.state().st.hide_ui, "Esc brings the interface back");

    press(&mut h, Key::P, Modifiers::COMMAND | Modifiers::SHIFT);
    assert!(h.state().st.pip, "Ctrl+Shift+P enters picture in picture");
    press(&mut h, Key::Escape, Modifiers::NONE);
    assert!(!h.state().st.pip);
}

#[test]
fn shortcuts_do_not_reach_the_player_under_a_modal() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(dir.path());
    h.state_mut().st.settings.open = true;
    h.step();
    press(&mut h, Key::F, Modifiers::NONE);
    assert!(!h.state().st.fullscreen, "F is ignored while settings are open");
    press(&mut h, Key::Escape, Modifiers::NONE);
    assert!(!h.state().st.settings.open, "Esc closes settings first");
}

#[test]
fn command_comma_opens_settings() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(dir.path());
    press(&mut h, Key::Comma, Modifiers::COMMAND);
    assert!(h.state().st.settings.open, "Cmd+, opens settings");
}

#[test]
fn going_home_leaves_fullscreen() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(dir.path());
    press(&mut h, Key::F, Modifiers::NONE);
    assert!(h.state().st.fullscreen);
    h.state_mut().st.navigate(notvlc::ui::pages::Page::Home);
    h.run_steps(2);
    let st = &h.state().st;
    assert_eq!(st.view, View::Library);
    assert!(!st.fullscreen && !st.hide_ui, "the library shows with its chrome and the mini player");
}

#[test]
fn closing_the_mini_player_keeps_it_closed_until_the_next_play() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(JsonStore::at(dir.path().join("data"), dir.path().join("cache")));
    let engine = Arc::new(FakeEngine::new());
    let mut app = App::new(engine.clone(), Arc::new(FakeProbe), store);
    app.settings.general.onboarding_done = true;
    app.play_items(vec![MediaItem::file("/media/a.mp3"), MediaItem::file("/media/b.mp3")], 0, None);
    app.play_index(1);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_eframe(|cc| NotVlcApp::new(&cc.egui_ctx, app, Vec::new()));
    h.run_steps(2);

    let s = h.state_mut();
    notvlc::ui::music::close_miniplayer(&mut s.app, &mut s.st);
    // libvlc reports the stop a frame or two later: the status still says playing.
    engine.status.lock().state = notvlc::app::ports::PlayerState::Playing;
    h.run_steps(3);
    assert!(h.state().st.music.mini_closed.is_some(), "a stale playing state must not reopen it");

    h.state_mut().app.play_index(0);
    h.run_steps(2);
    assert!(h.state().st.music.mini_closed.is_none(), "the next play brings it back");
}
