//! The "App" group: General, Interface, Privacy and Controls.

use egui::Color32;

use super::kit::{Btn, Form, Pick, SliderSpec};
use super::{Confirm, Cx};
use crate::domain::settings::*;
use crate::ui::widgets;

pub fn general(f: &mut Form, s: &mut Settings, cx: &mut Cx) {
    let g = &mut s.general;
    f.section("Appearance");
    f.pick("Interface language", None, &mut g.language, LANGUAGES);
    f.row("Theme", Some("Dark or light colours, or follow the system appearance."), super::kit::ROW_FIELD, |ui| {
        let mut i = Theme::ALL.iter().position(|t| *t == g.theme).unwrap_or(0);
        let labels: Vec<&str> = Theme::ALL.iter().map(|t| t.label()).collect();
        if widgets::segmented(ui, &mut i, &labels, super::kit::CONTROL_W) {
            g.theme = Theme::ALL[i];
        }
    });
    f.row("Accent color", Some("The highlight colour used for sliders, toggles and selections."), 46.0, |ui| {
        let mut i = Accent::ALL.iter().position(|a| *a == g.accent).unwrap_or(0);
        let colors: Vec<Color32> = Accent::ALL
            .iter()
            .map(|a| {
                let h = a.hex();
                Color32::from_rgb((h >> 16) as u8, (h >> 8) as u8, h as u8)
            })
            .collect();
        if widgets::swatches(ui, &mut i, &colors) {
            g.accent = Accent::ALL[i];
        }
    });

    f.section("Startup");
    f.toggle(
        "Show confirmation and error dialogs",
        Some("Ask before risky actions and show a message when something goes wrong."),
        &mut g.show_dialogs,
    );
    f.toggle(
        "Check for updates automatically",
        Some("Look for a newer version when the app starts."),
        &mut g.check_updates,
    );

    f.section("Diagnostics");
    f.toggle(
        "Developer mode",
        Some("Shows extra technical options meant for troubleshooting and development."),
        &mut g.developer_mode,
    );
    f.toggle("Save a log", Some("Write VLC's messages to a file so problems can be investigated."), &mut g.save_log);
    f.path(
        "Log file",
        Some("Where the log is written. Leave empty to use the default location."),
        &mut g.log_file,
        Pick::File,
    );
    f.choice(
        "Log details",
        Some("How much detail goes into the log."),
        &mut g.log_level,
        LogLevel::ALL,
        LogLevel::label,
    );

    f.advanced(|f| {
        f.toggle(
            "Show minor warnings",
            Some("Also log harmless warnings, not just errors."),
            &mut g.show_minor_warnings,
        );
        f.text(
            "Additional command line options",
            Some("Extra VLC options passed to the engine, separated by spaces (for example --no-osd)."),
            &mut g.extra_args,
            "--option=value",
        );
        f.choice(
            "Log format",
            Some("Plain text or an HTML page."),
            &mut g.log_format,
            LogFormat::ALL,
            LogFormat::label,
        );
        f.toggle(
            "Collect playback statistics",
            Some("Count bitrate, dropped frames and similar numbers shown in Media Information."),
            &mut g.collect_stats,
        );
        if f.action(
            "Open configurations folder",
            Some("Shows the folder holding your settings, library and playlists."),
            "Open",
            Btn::Accent,
        ) {
            let dir = cx.app.data_dir();
            cx.later(move |app, _| {
                let _ = std::fs::create_dir_all(&dir);
                if let Err(e) = open::that(&dir) {
                    app.toast_error(format!("Could not open {}: {e}", dir.display()));
                }
            });
        }
        if f.action(
            "Reset all settings",
            Some("Puts every preference back to its default. Your library and playlists are kept."),
            "Reset",
            Btn::Danger,
        ) {
            cx.ss.confirm = Some(Confirm::ResetSettings);
        }

        f.section("Developer Options");
        f.toggle(
            "Write to the system log",
            Some("Also send VLC's messages to the operating system's log."),
            &mut g.syslog,
        );
        f.toggle(
            "Include debug messages in the system log",
            Some("Send the most detailed messages to the system log too."),
            &mut g.syslog_debug,
        );
        f.text(
            "System log identity",
            Some("The name the messages appear under in the system log."),
            &mut g.syslog_ident,
            "vlc",
        );
        f.choice(
            "System log facility",
            Some("The system log category the messages are filed under."),
            &mut g.syslog_facility,
            SyslogFacility::ALL,
            SyslogFacility::label,
        );
    });
}

pub fn interface(f: &mut Form, s: &mut Settings, _cx: &mut Cx) {
    let i = &mut s.interface;
    let seconds = SliderSpec::new().ticks(20).step(0.5).labels(&[(0.0, "0 s"), (5.0, "5 s"), (10.0, "10s")]);
    let percent = SliderSpec::new().ticks(18).step(0.01).labels(&[(0.1, "10%"), (1.0, "100%")]);

    f.section("Skin");
    f.toggle(
        "Use a custom skin",
        Some("Draw the window with a VLC skin file instead of the built-in look."),
        &mut i.custom_skin,
    );
    f.path("Skin file", Some("The .vlt skin to use when custom skins are on."), &mut i.skin_file, Pick::File);

    f.section("Window");
    f.toggle(
        "Play video inside main window",
        Some("Show video in the app window rather than in a separate one."),
        &mut i.video_in_main_window,
    );
    f.choice(
        "Resize window to fit video",
        Some("Change the window size to match the video's own size."),
        &mut i.resize_to_video,
        ResizeToVideo::ALL,
        ResizeToVideo::label,
    );
    f.toggle("Remember window size and position", None, &mut i.remember_window);
    f.toggle(
        "Keep window above other windows",
        Some("The window stays on top of every other app."),
        &mut i.always_on_top,
    );
    f.toggle("Show window decorations", Some("Show the title bar with the window buttons."), &mut i.show_decorations);
    f.toggle(
        "Show the file name in the title bar",
        Some("Use the playing file's name as the window title."),
        &mut i.filename_in_title,
    );
    f.choice(
        "Open fullscreen on",
        Some("Which display is used for fullscreen."),
        &mut i.fullscreen_screen,
        FullscreenScreen::ALL,
        FullscreenScreen::label,
    );

    f.section("Playback Controls");
    f.toggle("Start in compact mode", Some("Open with the small mini-player controls."), &mut i.start_compact);
    f.choice(
        "Auto-hide the controls",
        Some("When the controls fade out while a video plays."),
        &mut i.auto_hide_controls,
        AutoHideControls::ALL,
        AutoHideControls::label,
    );
    f.slider("Hide after", None, &mut i.hide_after_s, 0.0..=10.0, &seconds);
    f.toggle(
        "Show controls in fullscreen",
        Some("Show the control bar when the mouse moves in fullscreen."),
        &mut i.controls_in_fullscreen,
    );

    f.section("Notifications and System Tray");
    f.toggle("Show a system tray icon", Some("Keep an icon in the menu bar for quick control."), &mut i.tray_icon);
    f.toggle(
        "Start minimized to the tray",
        Some("Start with only the tray icon, without opening the window."),
        &mut i.start_minimized,
    );
    f.choice(
        "Show a popup when the track changes",
        Some("Show a small notification with the new track's name."),
        &mut i.track_popup,
        TrackPopup::ALL,
        TrackPopup::label,
    );
    f.slider("Popup duration", None, &mut i.popup_duration_s, 0.0..=10.0, &seconds);

    f.section("Playlist");
    f.toggle(
        "Show the playlist as a tree",
        Some("Group items inside folders and playlists instead of a flat list."),
        &mut i.playlist_tree,
    );

    f.advanced(|f| {
        f.slider("Window opacity", None, &mut i.window_opacity, 0.1..=1.0, &percent);
        f.toggle("Pause video when window is minimized", None, &mut i.pause_when_minimized);
        f.choice(
            "Bring VLC to the front when playback starts",
            Some("Raise the window above other apps when something starts playing."),
            &mut i.bring_to_front,
            BringToFront::ALL,
            BringToFront::label,
        );
        f.toggle("Show cover art in the window when playing audio", None, &mut i.cover_art_for_audio);
        f.toggle("Show the file browser inside the open dialog", None, &mut i.file_browser_in_open);
        f.toggle("Allow the app icon to change", None, &mut i.dynamic_app_icon);
        f.slider("Fullscreen controls opacity", None, &mut i.fs_controls_opacity, 0.1..=1.0, &percent);
        let mut sens = Sensitivity::ALL.iter().position(|x| *x == i.fs_controls_sensitivity).unwrap_or(1) as f32;
        let spec = SliderSpec::new().ticks(2).step(1.0).labels(&[(0.0, "Low"), (1.0, "Medium"), (2.0, "High")]);
        if f.slider("Fullscreen controls sensitivity", None, &mut sens, 0.0..=2.0, &spec) {
            i.fs_controls_sensitivity = Sensitivity::ALL[(sens.round() as usize).min(2)];
        }
    });
}

pub fn privacy(f: &mut Form, s: &mut Settings, cx: &mut Cx) {
    let pr = &mut s.privacy;
    f.section("History");
    f.toggle(
        "Save recently played items",
        Some("Keep a list of what you played on the Recent page."),
        &mut pr.save_recent,
    );
    f.toggle(
        "Remember the playback position",
        Some("Resume files where you stopped watching or listening."),
        &mut pr.remember_position,
    );
    f.text(
        "Never remember items matching",
        Some("Files whose name or path contains any of these words (separated by commas) are never added to the history."),
        &mut pr.never_remember_pattern,
        "private, tmp",
    );
    if f.action(
        "Clear history now",
        Some("Forget every recently played item and saved position."),
        "Clear",
        Btn::Normal,
    ) {
        cx.later(|app, _| app.clear_history());
    }

    f.section("Network");
    f.toggle(
        "Fetch titles and cover art from the internet",
        Some("Look up missing titles, artists and artwork online."),
        &mut pr.fetch_online_meta,
    );
    f.toggle(
        "Submit played tracks to last.fm",
        Some("Send the songs you listen to to your last.fm profile (scrobbling)."),
        &mut pr.lastfm_submit,
    );
    f.text("Last.fm username", Some("Your last.fm account name."), &mut pr.lastfm_username, "");
    f.password(
        "Last.fm password",
        Some("Your last.fm password. It is stored in the settings file."),
        &mut pr.lastfm_password,
        "",
    );
    f.text(
        "Last.fm submission URL",
        Some("The scrobbling server. Only change this for a last.fm compatible service."),
        &mut pr.lastfm_url,
        "post.audioscrobbler.com",
    );
}

pub fn controls(f: &mut Form, s: &mut Settings, cx: &mut Cx) {
    let c = &mut s.controls;
    f.section("Keyboard Shortcuts");
    if f.action_wide("Defaults", None, "Edit Keyboard Shortcuts", Btn::Normal) {
        cx.later(|_, st| st.dialogs.shortcuts = true);
    }
    if f.action_wide("Reset all shortcuts", None, "Reset", Btn::Danger) {
        cx.ss.confirm = Some(Confirm::ResetShortcuts);
    }

    f.section("Skip Amounts");
    f.int("Very short skip", None, &mut c.skip_very_short_s, 1..=3600, "s");
    f.int("Short skip", None, &mut c.skip_short_s, 1..=3600, "s");
    f.int("Medium skip", None, &mut c.skip_medium_s, 1..=3600, "s");
    f.int("Long skip", None, &mut c.skip_long_s, 1..=36000, "s");

    f.section("Mouse");
    f.choice("Single click on the video", None, &mut c.single_click, ClickAction::ALL, ClickAction::label);
    f.choice("Double click on the video", None, &mut c.double_click, ClickAction::ALL, ClickAction::label);
    f.choice("Scroll wheel", None, &mut c.scroll_wheel, WheelAction::ALL, WheelAction::label);
    f.choice("Horizontal scroll", None, &mut c.horizontal_scroll, WheelAction::ALL, WheelAction::label);
    f.toggle("Reverse scroll direction", None, &mut c.reverse_scroll);

    f.section("Remotes");
    f.toggle("Web remote control", None, &mut c.web_remote);
    let mut port = c.web_remote_port as i64;
    if f.int("Port", None, &mut port, 1..=65535, "") {
        c.web_remote_port = port as u16;
    }
    f.password("Password", None, &mut c.web_remote_password, "Required to connect");
}
