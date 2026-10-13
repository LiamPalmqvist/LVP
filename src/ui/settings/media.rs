//! The "Media" group: Playback, Video and Subtitles (Audio and Library have
//! their own files).

use super::Cx;
use super::kit::{Btn, Form, Pick, SliderSpec};
use crate::domain::effects::{
    ASPECT_RATIOS, CROP_RATIOS, DEINTERLACE_METHODS, DeinterlaceMode, VideoEssential, ZOOM_LEVELS,
};
use crate::domain::settings::*;

/// "System language" followed by the interface languages.
pub fn track_languages() -> Vec<&'static str> {
    let mut v = vec!["System language"];
    v.extend_from_slice(LANGUAGES);
    v
}

pub fn playback(f: &mut Form, s: &mut Settings, _cx: &mut Cx) {
    let p = &mut s.playback;
    f.section("Startup");
    f.choice(
        "Continue where you left off",
        Some("Resume a file at the point where you stopped it last time."),
        &mut p.continue_playback,
        ContinuePlayback::ALL,
        ContinuePlayback::label,
    );
    f.toggle(
        "Start playing as soon as a file opens",
        Some("Begin playback right away instead of waiting for Play."),
        &mut p.autoplay,
    );
    let mut idx = SPEED_STEPS
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1 - p.default_speed).abs().total_cmp(&(b.1 - p.default_speed).abs()))
        .map(|(i, _)| i)
        .unwrap_or(3) as f32;
    let spec = SliderSpec {
        ticks: SPEED_STEPS.len() - 1,
        step: 1.0,
        labels: SPEED_STEPS.iter().enumerate().map(|(i, v)| (i as f32, format!("{v}"))).collect(),
        origin: None,
    };
    if f.slider("Default playback speed", None, &mut idx, 0.0..=(SPEED_STEPS.len() - 1) as f32, &spec) {
        p.default_speed = SPEED_STEPS[(idx.round() as usize).min(SPEED_STEPS.len() - 1)];
    }
    f.toggle(
        "Pause when another app uses audio",
        Some("Pause automatically when a call or another app starts playing sound."),
        &mut p.pause_on_interrupt,
    );

    f.section("Tracks");
    f.choice(
        "Preferred video quality",
        Some("For streams offered in several resolutions, pick this one when available."),
        &mut p.preferred_quality,
        VideoQuality::ALL,
        VideoQuality::label,
    );
    f.pick(
        "Preferred menu language",
        Some("The language used for DVD and Blu-ray menus when the disc offers it."),
        &mut p.menu_language,
        LANGUAGES,
    );
    f.int(
        "Default audio track",
        Some("Which audio track to start with, counting from 1."),
        &mut p.default_audio_track,
        1..=99,
        "",
    );
    f.int(
        "Default subtitle track",
        Some("Which subtitle track to start with, counting from 1."),
        &mut p.default_subtitle_track,
        1..=99,
        "",
    );

    f.section("Order");
    f.choice(
        "Repeat",
        Some("Repeat nothing, the current item, or the whole queue."),
        &mut p.repeat,
        RepeatMode::ALL,
        RepeatMode::label,
    );
    f.toggle("Shuffle", Some("Play the queue in a random order."), &mut p.shuffle);
    f.toggle(
        "Stop after each item",
        Some("Stop when an item ends instead of playing the next one."),
        &mut p.stop_after_each,
    );
    f.choice(
        "When playback finishes",
        Some("What happens when the end of the queue is reached."),
        &mut p.when_finished,
        WhenFinished::ALL,
        WhenFinished::label,
    );

    f.section("Seeking");
    f.toggle(
        "Seek precisely",
        Some("Jump to the exact moment you pick. Turn off for faster but less exact seeking."),
        &mut p.seek_precisely,
    );
    f.toggle(
        "Show thumbnail while scrubbing",
        Some("Show a preview picture above the seek bar while you hover or drag."),
        &mut p.scrub_thumbnails,
    );

    f.section("Files & Discs");
    let mut drives: Vec<String> = vec!["-".into()];
    if let Ok(rd) = std::fs::read_dir("/Volumes") {
        let mut v: Vec<String> = rd.flatten().map(|e| e.path().to_string_lossy().into_owned()).collect();
        v.sort();
        drives.extend(v);
    }
    if !p.optical_drive.is_empty() && !drives.contains(&p.optical_drive) {
        drives.push(p.optical_drive.clone());
    }
    let current = if p.optical_drive.is_empty() { "-".to_string() } else { p.optical_drive.clone() };
    if let Some(i) = f.dropdown(
        "Default optical drive",
        Some("The disc drive opened by Open Disc when none is chosen."),
        &current,
        &drives,
    ) {
        p.optical_drive = if i == 0 { String::new() } else { drives[i].clone() };
    }
    f.choice(
        "Repair damaged AVI files",
        Some("AVI files with a broken index cannot be seeked. VLC can rebuild the index when they open."),
        &mut p.repair_avi,
        AskAlways::ALL,
        AskAlways::label,
    );
    f.toggle(
        "Load matching .mkv files from the same folder",
        Some("Open the other parts of a split Matroska file automatically."),
        &mut p.load_matching_mkv,
    );

    f.advanced(|f| {
        f.path(
            "Time-shift buffer folder",
            Some("Where live streams are buffered so they can be paused and rewound."),
            &mut p.timeshift_dir,
            Pick::Folder,
        );
        f.choice(
            "Time-shift buffer size",
            Some("How much of a live stream is kept for rewinding."),
            &mut p.timeshift_size,
            BufferSize::ALL,
            BufferSize::label,
        );
    });
}

const DEINTERLACE: [(DeinterlaceMode, &str); 3] =
    [(DeinterlaceMode::Off, "Off"), (DeinterlaceMode::On, "On"), (DeinterlaceMode::Automatic, "Automatic")];

pub fn video(f: &mut Form, s: &mut Settings, _cx: &mut Cx) {
    let v = &mut s.video;
    f.section("Output");
    f.toggle("Enable video", Some("Turn off to play only the sound of video files."), &mut v.enabled);
    f.toggle(
        "Start videos in fullscreen",
        Some("Switch to fullscreen whenever a video starts."),
        &mut v.start_fullscreen,
    );
    f.toggle(
        "Stop the screensaver during playback",
        Some("Keep the display awake while a video plays."),
        &mut v.stop_screensaver,
    );
    f.toggle(
        "Play video as desktop wallpaper",
        Some("Draw the video behind the desktop icons (Windows only)."),
        &mut v.wallpaper,
    );

    f.section("Picture");
    let e = &mut v.effects.essential;
    let mut changed = false;
    let r = VideoEssential::RANGE_HUE;
    changed |= f.slider("Hue", None, &mut e.hue, r.0..=r.1, &SliderSpec::new().origin(0.0));
    let r = VideoEssential::RANGE_BRIGHTNESS;
    changed |= f.slider("Brightness", None, &mut e.brightness, r.0..=r.1, &SliderSpec::new().ticks(2));
    let r = VideoEssential::RANGE_CONTRAST;
    changed |= f.slider("Contrast", None, &mut e.contrast, r.0..=r.1, &SliderSpec::new().ticks(2));
    let r = VideoEssential::RANGE_SATURATION;
    changed |= f.slider("Saturation", None, &mut e.saturation, r.0..=r.1, &SliderSpec::new().ticks(3));
    let r = VideoEssential::RANGE_GAMMA;
    changed |= f.slider("Gamma", None, &mut e.gamma, r.0..=r.1, &SliderSpec::new().ticks(10));
    if changed {
        e.adjust_enabled = !e.is_neutral_adjust();
    }
    if f.action("Reset picture", None, "Reset", Btn::Normal) {
        let d = VideoEssential::default();
        e.hue = d.hue;
        e.brightness = d.brightness;
        e.contrast = d.contrast;
        e.saturation = d.saturation;
        e.gamma = d.gamma;
        e.adjust_enabled = false;
    }

    f.section("Deinterlacing");
    let cur = DEINTERLACE.iter().find(|(m, _)| *m == v.deinterlace).map(|(_, l)| *l).unwrap_or("Automatic");
    let opts: Vec<String> = DEINTERLACE.iter().map(|(_, l)| l.to_string()).collect();
    if let Some(i) = f.dropdown(
        "Deinterlacing",
        Some("Removes the comb-like lines of interlaced video from TV and DVDs."),
        cur,
        &opts,
    ) {
        v.deinterlace = DEINTERLACE[i].0;
    }
    let cur =
        DEINTERLACE_METHODS.iter().find(|(k, _)| *k == v.deinterlace_method).map(|(_, l)| *l).unwrap_or("Automatic");
    let opts: Vec<String> = DEINTERLACE_METHODS.iter().map(|(_, l)| l.to_string()).collect();
    if let Some(i) =
        f.dropdown("Method", Some("How interlaced lines are combined. Automatic picks a good one."), cur, &opts)
    {
        v.deinterlace_method = DEINTERLACE_METHODS[i].0.to_string();
    }

    f.section("Framing");
    let mut aspects: Vec<String> = vec!["Automatic".into()];
    aspects.extend(ASPECT_RATIOS.iter().map(|a| a.to_string()));
    aspects.extend(custom_list(&v.custom_aspect_ratios));
    let cur = v.aspect_ratio.clone().unwrap_or_else(|| "Automatic".into());
    if let Some(i) =
        f.dropdown("Aspect ratio", Some("Force the picture's shape when a file reports the wrong one."), &cur, &aspects)
    {
        v.aspect_ratio = if i == 0 { None } else { Some(aspects[i].clone()) };
    }
    let mut crops: Vec<String> = vec!["None".into()];
    crops.extend(CROP_RATIOS.iter().map(|a| a.to_string()));
    crops.extend(custom_list(&v.custom_crop_ratios));
    let cur = v.crop.clone().unwrap_or_else(|| "None".into());
    if let Some(i) =
        f.dropdown("Crop", Some("Cut the picture to this shape, for example to hide black bars."), &cur, &crops)
    {
        v.crop = if i == 0 { None } else { Some(crops[i].clone()) };
    }
    f.toggle(
        "Scale video to the window",
        Some("Grow or shrink the picture to fill the window."),
        &mut v.scale_to_window,
    );
    let zooms: Vec<String> = ZOOM_LEVELS.iter().map(|(l, _)| l.to_string()).collect();
    let cur = ZOOM_LEVELS.iter().find(|(_, z)| (*z - v.zoom).abs() < 1e-3).map(|(l, _)| *l).unwrap_or("Fit");
    if let Some(i) =
        f.dropdown("Zoom", Some("Show the video at a fixed size instead of fitting the window."), cur, &zooms)
    {
        v.zoom = ZOOM_LEVELS[i].1;
    }
    f.choice(
        "Position in the window",
        Some("Where the picture sits when it does not fill the window."),
        &mut v.position,
        VideoPosition::ALL,
        VideoPosition::label,
    );

    f.section("Snapshots");
    f.path(
        "Save to",
        Some("The folder snapshots are saved in. Empty uses your Pictures folder."),
        &mut v.snapshot_dir,
        Pick::Folder,
    );
    f.text(
        "File name prefix",
        Some("Text at the start of every snapshot's file name."),
        &mut v.snapshot_prefix,
        "vlcsnap-",
    );
    f.choice(
        "Format",
        Some("The image format snapshots are saved as."),
        &mut v.snapshot_format,
        SnapshotFormat::ALL,
        SnapshotFormat::label,
    );
    f.choice(
        "Size",
        Some("The size of the saved picture."),
        &mut v.snapshot_size,
        SnapshotSize::ALL,
        SnapshotSize::label,
    );
    f.toggle(
        "Number files sequentially",
        Some("Name snapshots 1, 2, 3... instead of using the time."),
        &mut v.snapshot_sequential,
    );
    f.toggle(
        "Show a preview after capture",
        Some("Briefly show the snapshot in the corner of the video."),
        &mut v.snapshot_preview,
    );

    f.advanced(|f| {
        f.text(
            "Custom aspect ratios",
            Some("Extra aspect ratios for the list above, separated by commas (for example 21:9, 32:9)."),
            &mut v.custom_aspect_ratios,
            "21:9, 32:9",
        );
        f.text(
            "Custom crop ratios",
            Some("Extra crop ratios for the list above, separated by commas."),
            &mut v.custom_crop_ratios,
            "2:1, 1.66:1",
        );
        f.text(
            "Monitor pixel aspect ratio",
            Some("The shape of your screen's pixels. Leave at 1:1 for almost every display."),
            &mut v.monitor_par,
            "1:1",
        );
    });
}

fn custom_list(s: &str) -> Vec<String> {
    s.split(',').map(str::trim).filter(|x| !x.is_empty()).map(str::to_string).collect()
}

pub fn subtitles(f: &mut Form, s: &mut Settings, _cx: &mut Cx) {
    let st = &mut s.subtitles;
    let langs = track_languages();
    f.section("Tracks & Languages");
    f.toggle("Enable subtitles", Some("Show subtitles when a file has them."), &mut st.enabled);
    f.pick(
        "Preferred language",
        Some("Pick the subtitle track in this language when a file has several."),
        &mut st.preferred_language,
        &langs,
    );
    f.pick(
        "Text encoding",
        Some("The character set of subtitle files. Change it if letters show up garbled."),
        &mut st.encoding,
        TEXT_ENCODINGS,
    );

    f.section("Loading Subtitles");
    f.toggle(
        "Load subtitle files automatically",
        Some("Load subtitle files found next to the video."),
        &mut st.autoload,
    );
    f.choice(
        "File name matching",
        Some("How closely a subtitle file's name must match the video's to be loaded."),
        &mut st.matching,
        SubMatching::ALL,
        SubMatching::label,
    );

    f.section("Appearance");
    f.text("Font", Some("The typeface used for subtitles."), &mut st.font, "System, sans-serif");
    let mut units = SizeUnits::ALL.iter().position(|u| *u == st.size_units).unwrap_or(0);
    if f.segmented(
        "Size values",
        Some("Size subtitles relative to the video, or in fixed pixels."),
        &mut units,
        &["Relative", "Pixels"],
    ) {
        st.size_units = SizeUnits::ALL[units];
    }
    match st.size_units {
        SizeUnits::Relative => {
            f.choice("Font size", None, &mut st.font_size, FontSize::ALL, FontSize::label);
        }
        SizeUnits::Pixels => {
            f.int("Font size", None, &mut st.font_size_px, 0..=400, "px");
        }
    }
    f.toggle("Bold", None, &mut st.bold);
    f.color("Text color", None, &mut st.text_color);
    let pct = SliderSpec::new().ticks(20).step(0.01).labels(&[(0.0, "0%"), (0.5, "50%"), (1.0, "100%")]);
    f.slider("Text opacity", None, &mut st.text_opacity, 0.0..=1.0, &pct);
    f.choice("Outline thickness", None, &mut st.outline, Outline::ALL, Outline::label);
    f.color("Outline color", None, &mut st.outline_color);
    f.toggle("Shadow", None, &mut st.shadow);
    f.color("Shadow color", None, &mut st.shadow_color);
    f.choice("Shadow distance", None, &mut st.shadow_distance, ShadowDistance::ALL, ShadowDistance::label);
    f.choice("Background", None, &mut st.background, SubBackground::ALL, SubBackground::label);
    f.color("Background color", None, &mut st.background_color);
    f.slider("Background opacity", None, &mut st.background_opacity, 0.0..=1.0, &pct);
    f.choice("Vertical position", None, &mut st.position, SubPosition::ALL, SubPosition::label);

    f.section("Timing");
    let mut delay = st.delay_ms as f32 / 1000.0;
    let spec =
        SliderSpec::new().ticks(20).step(0.05).origin(0.0).labels(&[(-5.0, "-5 s"), (0.0, "0 s"), (5.0, "+5 s")]);
    if f.slider("Subtitle delay", None, &mut delay, -5.0..=5.0, &spec) {
        st.delay_ms = (delay * 1000.0).round() as i64;
    }

    f.section("On-Screen Messages");
    f.toggle(
        "Show feedback for shortcuts",
        Some("Show a short message on the video when a shortcut changes something, such as the volume."),
        &mut st.osd_feedback,
    );
    let spec = SliderSpec::new().ticks(9).step(0.5).labels(&[(0.5, "0.5 s"), (5.0, "5 s")]);
    f.slider("Show feedback for", None, &mut st.osd_feedback_s, 0.5..=5.0, &spec);
    f.toggle(
        "Show filename when a video starts",
        Some("Show the title on the video for a moment when it starts."),
        &mut st.osd_filename,
    );
    let spec = SliderSpec::new().ticks(9).step(0.5).labels(&[(1.0, "1 s"), (10.0, "10 s")]);
    f.slider("Show filename for", None, &mut st.osd_filename_s, 1.0..=10.0, &spec);

    f.advanced(|f| {
        f.text(
            "Load subtitles from these folders",
            Some("Folders searched for subtitle files, relative to the video, separated by commas."),
            &mut st.subtitle_dirs,
            ".\\subtitles, .\\subs",
        );
        f.toggle(
            "Ignore styling in ASS and SSA subtitles",
            Some("Use your own font and colours instead of the styles built into these subtitle files."),
            &mut st.ignore_ass_styling,
        );
        let mut angle = st.shadow_angle as f64;
        if f.number(
            "Shadow angle",
            Some("The direction the shadow falls in, in degrees."),
            &mut angle,
            -360.0..=360.0,
            0,
            "°",
        ) {
            st.shadow_angle = angle as f32;
        }
        f.choice(
            "Text direction",
            Some("Right to left is for languages such as Arabic and Hebrew."),
            &mut st.text_direction,
            TextDirection::ALL,
            TextDirection::label,
        );
        f.text(
            "Monospace font",
            Some("The typeface used for subtitles that need fixed-width letters."),
            &mut st.monospace_font,
            "System monospace",
        );
        let mut speed = st.speed as f64;
        if f.number(
            "Subtitle speed",
            Some("Speeds subtitles up or slows them down, for files made for a different frame rate."),
            &mut speed,
            0.1..=10.0,
            2,
            "x",
        ) {
            st.speed = speed as f32;
        }
        let mut dur = st.duration_factor as f64;
        if f.number(
            "Subtitle duration factor",
            Some("Keeps each subtitle on screen longer. 0 leaves the timing as it is."),
            &mut dur,
            0.0..=20.0,
            2,
            "",
        ) {
            st.duration_factor = dur as f32;
        }
    });
}
