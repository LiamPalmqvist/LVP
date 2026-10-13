//! Renders the real UI headlessly to a PNG, with demo content taken from the
//! design file, so screens can be compared with the Figma renders.
//!
//! usage: cargo run --example snapshot -- [options] [out.png]
//!   --size WxH            window size in points (default 1920x1080)
//!   --scale N             pixels per point (default 1)
//!   --view library|player
//!   --page NAME           home, videos, music, network, playlists, recent, folder:PATH, playlist:ID
//!   --panel KIND:N        queue:N, tracks:N, quick:N
//!   --settings CATEGORY   open settings at a category (general, audio, ...)
//!   --dialog NAME         open a dialog by name (see ui::dialogs::DialogsState::open_by_name)
//!   --side-panel          show the library column beside the video
//!   --collapsed           collapse the library sidebar to icons
//!   --light               light theme
//!   --accent NAME         accent colour (orange, red, olive, green, teal, blue, purple, pink, gray)
//!   --audio               play a music track instead of a video
//!   --idle                nothing playing
//!   --onboarding          show the first-run wizard
//!   --steps N             frames to run before capturing (default 6)

use std::path::{Path, PathBuf};
use std::sync::Arc;

use notvlc::app::App;
use notvlc::app::fake::{FakeEngine, FakeProbe};
use notvlc::app::ports::{Chapter, MediaEngine, PlayerState, Track};
use notvlc::domain::media::MediaItem;
use notvlc::domain::settings::{Accent, Theme};
use notvlc::infra::store::JsonStore;
use notvlc::ui::NotVlcApp;
use notvlc::ui::state::{PlayerPanel, View};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/demo");

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned()
}

fn has(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

fn touch(p: &Path) {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).unwrap();
    }
    std::fs::write(p, b"").unwrap();
}

/// Lays out a fake media folder matching the design's library screens.
fn demo_files(home: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let season = home.join("Videos/Shows/The Wand/S01");
    let eps: Vec<PathBuf> = (1..=15).map(|n| season.join(format!("The_Wand_S01E{n:02}.mp4"))).collect();
    for e in &eps {
        touch(e);
    }
    touch(&home.join("Videos/Movies/DON'T BLINK.mp4"));
    touch(&home.join("Videos/Anime/Frieren/Frieren_01.mkv"));
    let songs: Vec<PathBuf> =
        ["untangle - juxtopposed.mp3", "moonlight - juxtopposed.mp3", "black bird - juxtopposed.mp3"]
            .iter()
            .map(|s| home.join("Music/juxtopposed").join(s))
            .collect();
    for s in &songs {
        touch(s);
    }
    (eps, songs)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = args.iter().rev().find(|a| a.ends_with(".png")).cloned().unwrap_or_else(|| "snapshot.png".into());
    let (w, h) = arg_value(&args, "--size")
        .and_then(|s| s.split_once('x').map(|(a, b)| (a.parse().unwrap_or(1920.0), b.parse().unwrap_or(1080.0))))
        .unwrap_or((1920.0f32, 1080.0f32));
    let scale: f32 = arg_value(&args, "--scale").and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let steps: usize = arg_value(&args, "--steps").and_then(|s| s.parse().ok()).unwrap_or(6);

    let home = std::env::temp_dir().join(format!("notvlc-snapshot-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let (eps, songs) = demo_files(&home);
    let store = Arc::new(JsonStore::at(home.join("data"), home.join("cache")));
    let fake = Arc::new(FakeEngine::new());
    let engine: Arc<dyn MediaEngine> = fake.clone();
    let mut app = App::new(engine, Arc::new(FakeProbe), store);

    app.settings.general.onboarding_done = !has(&args, "--onboarding");
    app.settings.library.video_folders = vec![home.join("Videos")];
    app.settings.library.music_folders = vec![home.join("Music")];
    if has(&args, "--light") {
        app.settings.general.theme = Theme::Light;
    }
    if let Some(a) = arg_value(&args, "--accent") {
        app.settings.general.accent =
            Accent::ALL.iter().copied().find(|x| x.label().eq_ignore_ascii_case(&a)).unwrap_or(Accent::Orange);
    }

    // Thumbnails from the design, placed where the library expects them.
    for (i, e) in eps.iter().enumerate() {
        let src = Path::new(FIXTURES).join(format!("thumbs/ep{:02}.png", i % 13 + 1));
        let dst = app.library.thumbnail_path(&notvlc::domain::media::MediaSource::File(e.clone()));
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::copy(src, dst).unwrap();
    }
    let covers: Vec<PathBuf> =
        std::fs::read_dir(Path::new(FIXTURES).join("covers")).unwrap().flatten().map(|e| e.path()).collect();

    // History for Home: a few watched episodes and songs.
    for (i, e) in eps.iter().enumerate().take(4) {
        let item = MediaItem::file(e);
        app.library.mark_played(&item, true);
        app.library.save_position(&item.source, 600_000 + i as i64 * 60_000, 1_540_000);
    }
    for (i, s) in songs.iter().enumerate() {
        let mut item = MediaItem::file(s);
        item.meta.artist = Some("juxtopposed".into());
        item.meta.title = Some(s.file_stem().unwrap().to_string_lossy().split(" - ").next().unwrap().to_string());
        item.meta.artwork_url = covers.get(i).map(|c| notvlc::domain::media::path_to_file_url(c));
        item.meta.duration_ms = Some(201_000 + i as i64 * 17_000);
        app.library.set_meta(&item.source, item.meta.clone());
        app.library.mark_played(&item, true);
    }
    app.library.create_playlist("Cool Songs", songs.iter().map(MediaItem::file).collect());
    app.library.create_playlist("Anime to Watch", eps.iter().take(5).map(MediaItem::file).collect());

    if !has(&args, "--idle") {
        if has(&args, "--audio") {
            let items: Vec<MediaItem> = songs
                .iter()
                .map(|s| {
                    let mut i = MediaItem::file(s);
                    app.library.enrich(&mut i);
                    i
                })
                .collect();
            app.play_items(items, 0, Some("juxtopposed".into()));
        } else {
            let items: Vec<MediaItem> = eps.iter().map(MediaItem::file).collect();
            app.play_items(items, 10, Some("S01".into()));
            let img = image::open(Path::new(FIXTURES).join("frame.png")).unwrap().to_rgba8();
            fake.show_picture(img.width(), img.height(), img.as_raw());
        }
        let mut st = fake.status.lock();
        st.state = PlayerState::Playing;
        st.time_ms = 130_000;
        st.length_ms = 1_540_000;
        st.seekable = true;
        st.pausable = true;
        st.has_video = !has(&args, "--audio");
        st.volume = 40;
        st.audio_tracks = vec![
            Track { id: -1, name: "Disable".into() },
            Track { id: 1, name: "English".into() },
            Track { id: 2, name: "Japanese".into() },
        ];
        st.audio_track = 1;
        st.subtitle_tracks = vec![
            Track { id: -1, name: "Disable".into() },
            Track { id: 3, name: "English".into() },
            Track { id: 4, name: "Japanese".into() },
        ];
        st.subtitle_track = 3;
        st.video_tracks = vec![Track { id: -1, name: "Disable".into() }, Track { id: 0, name: "Track 1".into() }];
        st.video_track = 0;
        st.chapters = ["Winter", "Spring", "Summer", "Fall", "Winter (5 Years Later)"]
            .iter()
            .enumerate()
            .map(|(i, n)| Chapter { name: n.to_string(), start_ms: i as i64 * 300_000, duration_ms: 300_000 })
            .collect();
    }

    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(w, h))
        .with_pixels_per_point(scale)
        .wgpu()
        .build_eframe(|cc| NotVlcApp::new(&cc.egui_ctx, app, Vec::new()));

    {
        let ui = harness.state_mut();
        let st = &mut ui.st;
        st.view = if has(&args, "--idle") { View::Library } else { View::Player };
        if let Some(v) = arg_value(&args, "--view") {
            st.view = if v == "library" { View::Library } else { View::Player };
        }
        if let Some(page) = arg_value(&args, "--page") {
            st.page = match page.as_str() {
                "videos" => notvlc::ui::pages::Page::Videos,
                "music" => notvlc::ui::pages::Page::Music,
                "network" => notvlc::ui::pages::Page::Network,
                "playlists" => notvlc::ui::pages::Page::Playlists,
                "recent" => notvlc::ui::pages::Page::Recent,
                p if p.starts_with("folder:") => notvlc::ui::pages::Page::Folder(PathBuf::from(&p[7..])),
                p if p.starts_with("playlist:") => notvlc::ui::pages::Page::Playlist(p[9..].parse().unwrap_or(1)),
                "season" => notvlc::ui::pages::Page::Folder(home.join("Videos/Shows/The Wand/S01")),
                _ => notvlc::ui::pages::Page::Home,
            };
        }
        if let Some(panel) = arg_value(&args, "--panel") {
            let (kind, n) = panel.split_once(':').unwrap_or((panel.as_str(), "0"));
            let n: usize = n.parse().unwrap_or(0);
            st.panel = Some(match kind {
                "queue" => PlayerPanel::Queue(n),
                "tracks" => PlayerPanel::Tracks(n),
                _ => PlayerPanel::QuickSettings(n),
            });
        }
        if let Some(cat) = arg_value(&args, "--settings") {
            st.settings.open = true;
            if let Some(c) = notvlc::ui::settings::Category::from_name(&cat) {
                st.settings.category = c;
            }
        }
        if let Some(d) = arg_value(&args, "--dialog")
            && !st.dialogs.open_by_name(&d)
        {
            eprintln!("unknown dialog {d}");
        }
        st.side_panel = has(&args, "--side-panel");
        if has(&args, "--collapsed") {
            st.sidebar_expanded = false;
        }

        // ---- dialogs ----
        // --vlc-dialog login|question|progress|error   queue one of VLC's prompts
        if let Some(kind) = arg_value(&args, "--vlc-dialog") {
            use notvlc::app::ports::DialogRequest;
            let (title, text) = ("Authentication required".to_string(), "smb://nas/Movies needs a login.".to_string());
            ui.app.dialogs.push_back(match kind.as_str() {
                "question" => DialogRequest::Question {
                    id: 7,
                    title: "Insecure site".into(),
                    text: "The certificate of this server could not be verified. Do you want to continue anyway?"
                        .into(),
                    cancel: "Cancel".into(),
                    action1: Some("Accept permanently".into()),
                    action2: Some("Accept once".into()),
                },
                "progress" => DialogRequest::Progress {
                    id: 8,
                    title: "Downloading".into(),
                    text: "Fetching the playlist...".into(),
                    position: Some(0.42),
                    cancel: Some("Cancel".into()),
                },
                "error" => DialogRequest::Error {
                    title: "Your input can't be opened".into(),
                    text: "VLC is unable to open the MRL 'http://example.com/missing.mp4'.".into(),
                },
                _ => DialogRequest::Login { id: 6, title, text, default_username: "liam".into(), ask_store: true },
            });
        }
    }

    for _ in 0..steps {
        harness.step();
        std::thread::sleep(std::time::Duration::from_millis(60));
    }
    let img = harness.render().expect("render");
    img.save(&out).expect("save");
    println!("{out} ({}x{})", img.width(), img.height());
    let _ = std::fs::remove_dir_all(&home);
}
