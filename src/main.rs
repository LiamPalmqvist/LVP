//! Entry point: wires libvlc and the JSON store into the app and opens the
//! window.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Arc;

use lvp::app::App;
use lvp::app::ports::Store;
use lvp::domain::settings::Settings;
use lvp::infra::store::JsonStore;
use lvp::infra::vlc::{VlcEngine, VlcProbe};
use lvp::ui::LvpApp;

fn window_icon() -> Option<egui::IconData> {
    let svg = lvp::ui::icons::Icon::Logo.svg().replace("rgb(211,121,24)", "rgb(255,136,0)");
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).ok()?;
    let mut pm = resvg::tiny_skia::Pixmap::new(256, 256)?;
    let k = 256.0 / tree.size().width();
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(k, k), &mut pm.as_mut());
    // Un-premultiply for the window icon.
    let rgba = pm
        .data()
        .chunks_exact(4)
        .flat_map(|p| {
            let a = p[3] as f32 / 255.0;
            let f = |c: u8| if a > 0.0 { ((c as f32 / a).round().min(255.0)) as u8 } else { 0 };
            [f(p[0]), f(p[1]), f(p[2]), p[3]]
        })
        .collect();
    Some(egui::IconData { rgba, width: 256, height: 256 })
}

fn main() -> eframe::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    // Packaging helper: `lvp --export-icon out.png` writes the app icon.
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--export-icon") {
        let out = args.get(2).cloned().unwrap_or_else(|| "lvp.png".into());
        let icon = window_icon().expect("icon renders");
        image::RgbaImage::from_raw(icon.width, icon.height, icon.rgba)
            .expect("icon buffer")
            .save(&out)
            .expect("save icon");
        return Ok(());
    }
    let files: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).filter(|p| p.exists()).collect();

    let store = Arc::new(JsonStore::new());
    // Files from a second launch or from Finder arrive through this channel.
    let (inbox_tx, inbox_rx) = crossbeam_channel::unbounded();
    #[cfg(not(unix))]
    drop(inbox_tx);
    #[cfg(unix)]
    {
        let one = store
            .load("settings")
            .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
            .is_some_and(|s| s.library.one_instance || (s.library.one_instance_from_file_manager && !files.is_empty()));
        if one && lvp::infra::single_instance::forward(&store.data_dir(), &files) {
            return Ok(());
        }
        let _ = lvp::infra::single_instance::listen(&store.data_dir(), inbox_tx.clone());
    }
    #[cfg(target_os = "macos")]
    lvp::infra::macos_open::install(inbox_tx);
    let settings: Settings = store.load("settings").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let engine = match VlcEngine::new(&settings) {
        Ok(e) => Arc::new(e),
        Err(e) => {
            rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Error)
                .set_title("LVP needs VLC")
                .set_description(format!(
                    "LVP plays media through VLC's engine (libvlc 3), which could not be loaded.\n\n{e:#}\n\nInstall VLC from videolan.org, or set LVP_LIBVLC to the folder that contains libvlc."
                ))
                .show();
            std::process::exit(1);
        }
    };
    let probe = Arc::new(VlcProbe::new(engine.helper()));
    let mut app = App::new(engine, probe, store);
    app.set_system_hooks(Arc::new(lvp::infra::platform::SystemIntegration::new()));
    app.set_inbox(inbox_rx);

    let size = app
        .settings
        .interface
        .window_size
        .filter(|_| app.settings.interface.remember_window)
        .unwrap_or([1280.0, 800.0]);
    let remember = app.settings.interface.remember_window;
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("VLC Media Player")
        .with_app_id("org.lvp.LVP")
        .with_inner_size(size)
        .with_min_inner_size([640.0, 400.0])
        // macOS keeps its native frame (corners, shadow, traffic lights) with the
        // app's bar under a transparent title bar; elsewhere the app draws it all.
        .with_decorations(cfg!(target_os = "macos"))
        .with_fullsize_content_view(true)
        .with_titlebar_shown(false)
        .with_title_shown(false)
        .with_transparent(false);
    if remember && let Some([x, y]) = app.settings.interface.window_pos {
        viewport = viewport.with_position([x, y]);
    }
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(Arc::new(icon));
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        persist_window: false,
        ..Default::default()
    };
    if let Some(c) = lvp::infra::media_controls::SystemMediaControls::new() {
        app.set_media_controls(Box::new(c));
    }
    eframe::run_native(
        "LVP",
        options,
        Box::new(move |cc| {
            #[allow(unused_mut)]
            let mut ui = LvpApp::new(&cc.egui_ctx, app, files);
            #[cfg(target_os = "macos")]
            if let Some(mut lights) = lvp::infra::macos_window::TrafficLights::new(cc) {
                ui.set_native_titlebar(Box::new(move |height, shown| lights.place(height as f64, shown)));
            }
            Ok(Box::new(ui))
        }),
    )
}
