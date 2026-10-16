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
