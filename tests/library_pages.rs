//! Drives the library pages headlessly: clicking a folder card opens it,
//! Backspace goes back, arrows and Enter play a video.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use egui::{Event, Key, Modifiers, PointerButton, Pos2, pos2};
use egui_kittest::Harness;
use notvlc::app::App;
use notvlc::app::fake::{FakeEngine, FakeProbe};
use notvlc::infra::store::JsonStore;
use notvlc::ui::NotVlcApp;
use notvlc::ui::pages::Page;
use notvlc::ui::state::View;

fn harness(home: &Path) -> Harness<'static, NotVlcApp> {
    let season = home.join("Videos/Shows/S01");
    std::fs::create_dir_all(&season).unwrap();
    for n in ["E1.mp4", "E2.mp4"] {
        std::fs::write(season.join(n), b"").unwrap();
    }
    let store = Arc::new(JsonStore::at(home.join("data"), home.join("cache")));
    let mut app = App::new(Arc::new(FakeEngine::new()), Arc::new(FakeProbe), store);
    app.settings.general.onboarding_done = true;
    app.settings.library.video_folders = vec![home.join("Videos")];
    let mut h = Harness::builder()
        .with_size(egui::vec2(1920.0, 1080.0))
        .build_eframe(|cc| NotVlcApp::new(&cc.egui_ctx, app, Vec::new()));
    h.state_mut().st.view = View::Library;
    h.state_mut().st.page = Page::Videos;
    h.run_steps(3);
    h
}

fn click(h: &mut Harness<'static, NotVlcApp>, pos: Pos2) {
    h.event(Event::PointerMoved(pos));
    h.step();
    let press =
        |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    h.event(press(true));
    h.step();
    h.event(press(false));
    h.run_steps(2);
}

/// Past the guard that ignores clicks right after a page opens.
fn settle(h: &mut Harness<'static, NotVlcApp>) {
    std::thread::sleep(Duration::from_millis(450));
    h.run_steps(1);
}

#[test]
fn folder_card_opens_and_backspace_returns() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(dir.path());
    settle(&mut h);
    // First card of the grid: content starts at (237, 131).
    click(&mut h, pos2(237.0 + 150.0, 131.0 + 80.0));
    assert_eq!(h.state_mut().st.page, Page::Folder(dir.path().join("Videos/Shows")));
    settle(&mut h);
    h.key_press(Key::Backspace);
    h.run_steps(2);
    assert_eq!(h.state_mut().st.page, Page::Videos);
}

#[test]
fn arrows_and_enter_play_a_video() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(dir.path());
    h.state_mut().st.page = Page::Folder(dir.path().join("Videos/Shows/S01"));
    h.run_steps(2);
    settle(&mut h);
    h.key_press(Key::ArrowRight);
    h.run_steps(1);
    h.key_press(Key::ArrowRight);
    h.run_steps(1);
    assert_eq!(h.state_mut().st.library.selected, Some(1));
    h.key_press(Key::Enter);
    h.run_steps(2);
    let ui = h.state_mut();
    assert_eq!(ui.app.queue.len(), 2);
    assert_eq!(ui.app.queue.current, Some(1));
    assert_eq!(ui.st.view, View::Player);
}

#[test]
fn right_click_opens_the_file_menu() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(dir.path());
    h.state_mut().st.page = Page::Folder(dir.path().join("Videos/Shows/S01"));
    h.run_steps(2);
    settle(&mut h);
    let pos = pos2(237.0 + 150.0, 131.0 + 80.0);
    h.event(Event::PointerMoved(pos));
    h.step();
    for pressed in [true, false] {
        h.event(Event::PointerButton { pos, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE });
        h.step();
    }
    h.run_steps(2);
    assert!(egui::Popup::is_any_open(&h.ctx));
}
