//! Subtitles must reach the picture: switching to an embedded track (ASS,
//! as in most anime releases) and loading an external SRT both draw text
//! into the decoded frames. Silent. Skips without libvlc or ffmpeg.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use notvlc::app::ports::{EngineCommand, MediaEngine, OpenRequest, PlayerState};
use notvlc::app::video::VideoOutput;
use notvlc::domain::media::MediaItem;
use notvlc::domain::settings::Settings;
use notvlc::infra::vlc::VlcEngine;

const ASS_HEAD: &str = "[Script Info]\nScriptType: v4.00+\nPlayResX: 640\nPlayResY: 360\n\n[V4+ Styles]\n\
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n\
Style: Default,Arial,28,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,0,2,10,10,20,1\n\n[Events]\n\
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n";

fn wait(mut f: impl FnMut() -> bool, secs: u64) -> bool {
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(secs) {
        if f() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

/// Count of bright pixels in the lower third, where subtitles sit.
fn bright_pixels(v: &VideoOutput) -> usize {
    v.with_front(|_, px, w, h, pitch| {
        let mut n = 0;
        for y in (h * 2 / 3) as usize..h as usize {
            for x in 0..w as usize {
                let i = y * pitch as usize + x * 4;
                if px[i] > 200 && px[i + 1] > 200 && px[i + 2] > 200 {
                    n += 1;
                }
            }
        }
        n
    })
    .unwrap_or(0)
}

fn engine() -> Option<VlcEngine> {
    let mut s = Settings::default();
    s.recording.discover_cast = false;
    s.general.extra_args = "--aout=dummy".into();
    if std::env::var("SUBTEST_ADJUST").is_ok() {
        s.video.effects.essential.adjust_enabled = true;
    }
    if std::env::var("SUBTEST_SWDEC").is_ok() {
        s.performance.hw_decoding = notvlc::domain::settings::HwDecoding::Disabled;
        s.general.extra_args.push_str(" --codec=avcodec,all");
    }
    if let Ok(log) = std::env::var("SUBTEST_LOG") {
        s.general.save_log = true;
        s.general.log_file = log;
        s.general.log_level = notvlc::domain::settings::LogLevel::Debug;
    }
    VlcEngine::new(&s).map_err(|e| eprintln!("libvlc unavailable ({e:#}), skipping")).ok()
}

fn make(dir: &Path) -> bool {
    // A cue every second, like real dialogue, so a track picked mid-play has
    // something new to show.
    let cues = |text: &str| -> String {
        (0..10)
            .map(|i| format!("Dialogue: 0,0:00:{i:02}.00,0:00:{:02}.00,Default,,0,0,0,,{text} {i}\n", i + 1))
            .collect()
    };
    std::fs::write(
        dir.join("songs.ass"),
        format!("{ASS_HEAD}Dialogue: 0,0:00:09.00,0:00:10.00,Default,,0,0,0,,La la\n"),
    )
    .unwrap();
    std::fs::write(dir.join("full.ass"), format!("{ASS_HEAD}{}", cues("FULL DIALOGUE LINE"))).unwrap();
    let srt: String = (0..10)
        .map(|i| format!("{}\n00:00:{i:02},000 --> 00:00:{:02},000\nEXTERNAL SUBTITLE LINE {i}\n\n", i + 1, i + 1))
        .collect();
    std::fs::write(dir.join("plain.srt"), srt).unwrap();
    Command::new("ffmpeg")
        .current_dir(dir)
        .args(["-v", "error", "-y", "-f", "lavfi", "-i", "color=c=black:size=640x360:rate=25:duration=10"])
        .args(["-i", "songs.ass", "-i", "full.ass", "-map", "0:v", "-map", "1", "-map", "2"])
        .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:s", "ass"])
        .args(["-metadata:s:s:0", "title=Signs", "-metadata:s:s:1", "title=Full", "two_subs.mkv"])
        .status()
        .is_ok_and(|s| s.success())
        && Command::new("ffmpeg")
            .current_dir(dir)
            .args(["-v", "error", "-y", "-f", "lavfi", "-i", "color=c=black:size=640x360:rate=25:duration=10"])
            .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "plain.mp4"])
            .status()
            .is_ok_and(|s| s.success())
}

fn open(e: &VlcEngine, path: &Path, subtitles: Vec<std::path::PathBuf>) {
    e.send(EngineCommand::Open(Box::new(OpenRequest { subtitles, ..OpenRequest::new(MediaItem::file(path)) })));
}

#[test]
fn switching_to_an_embedded_ass_track_draws_it() {
    let dir = tempfile::tempdir().unwrap();
    if !make(dir.path()) {
        eprintln!("ffmpeg unavailable, skipping");
        return;
    }
    let Some(e) = engine() else { return };
    let v = e.video();
    open(&e, &dir.path().join("two_subs.mkv"), vec![]);
    assert!(wait(|| e.status().state == PlayerState::Playing && v.size().is_some(), 10));
    assert!(wait(|| e.status().subtitle_tracks.iter().filter(|t| t.id >= 0).count() == 2, 5), "two subtitle tracks");
    let tracks = e.status().subtitle_tracks;
    let full = tracks.iter().filter(|t| t.id >= 0).nth(1).unwrap().id;
    e.send(EngineCommand::SetSubtitleTrack(full));
    if std::env::var("SUBTEST_TRACE").is_ok() {
        for _ in 0..25 {
            let st = e.status();
            eprintln!(
                "t={} state={:?} sub={} subs={:?} bright={}",
                st.time_ms,
                st.state,
                st.subtitle_track,
                st.subtitle_tracks.iter().map(|t| t.id).collect::<Vec<_>>(),
                bright_pixels(&v)
            );
            std::thread::sleep(Duration::from_millis(250));
        }
    }
    assert!(wait(|| e.status().subtitle_track == full, 3), "track switch reported");
    let ok = wait(|| bright_pixels(&v) > 150, 8);
    let st = e.status();
    assert!(
        ok,
        "the full track's text never appeared (tracks {tracks:?}, selected {}, state {:?}, time {}, bright {})",
        st.subtitle_track,
        st.state,
        st.time_ms,
        bright_pixels(&v)
    );
}

#[test]
fn an_external_srt_draws_with_default_style() {
    let dir = tempfile::tempdir().unwrap();
    if !make(dir.path()) {
        eprintln!("ffmpeg unavailable, skipping");
        return;
    }
    let Some(e) = engine() else { return };
    let v = e.video();
    open(&e, &dir.path().join("plain.mp4"), vec![dir.path().join("plain.srt")]);
    assert!(wait(|| e.status().state == PlayerState::Playing && v.size().is_some(), 10));
    assert!(wait(|| bright_pixels(&v) > 150, 6), "SRT text never appeared (bright {})", bright_pixels(&v));
}

#[test]
fn adding_subtitles_to_a_hardware_decoded_video_shows_them() {
    let dir = tempfile::tempdir().unwrap();
    if !make(dir.path()) {
        eprintln!("ffmpeg unavailable, skipping");
        return;
    }
    let Some(e) = engine() else { return };
    let v = e.video();
    // No subtitles known at open: hardware decoding.
    std::fs::rename(dir.path().join("plain.srt"), dir.path().join("hidden.txt")).unwrap();
    open(&e, &dir.path().join("plain.mp4"), vec![]);
    assert!(wait(|| e.status().state == PlayerState::Playing && v.size().is_some(), 10));
    assert!(wait(|| e.status().time_ms > 2000, 6), "playback did not advance");
    // A name VLC will not autodetect, so the file opens hardware-decoded.
    std::fs::rename(dir.path().join("hidden.txt"), dir.path().join("other-name.srt")).unwrap();
    let before = e.status().time_ms;
    e.send(EngineCommand::AddSubtitle(dir.path().join("other-name.srt")));
    if std::env::var("SUBTEST_TRACE").is_ok() {
        for _ in 0..16 {
            let st = e.status();
            eprintln!("t={} state={:?} sub={} bright={}", st.time_ms, st.state, st.subtitle_track, bright_pixels(&v));
            std::thread::sleep(Duration::from_millis(250));
        }
    }
    assert!(wait(|| bright_pixels(&v) > 150, 8), "added subtitles never appeared (bright {})", bright_pixels(&v));
    // Restarting from zero would need another two seconds to get back here.
    assert!(wait(|| e.status().time_ms >= before, 1), "playback restarted from the beginning instead of in place");
}

/// Bright pixels averaged over a few frames, to smooth out cue changes.
fn bright_avg(v: &VideoOutput) -> usize {
    let mut sum = 0;
    for _ in 0..4 {
        sum += bright_pixels(v);
        std::thread::sleep(Duration::from_millis(120));
    }
    sum / 4
}

fn measure_scale(e: &VlcEngine, v: &VideoOutput) -> (usize, usize) {
    assert!(wait(|| bright_pixels(v) > 150, 6), "no subtitle text");
    let normal = bright_avg(v);
    e.send(EngineCommand::SetSubtitleScale(200));
    std::thread::sleep(Duration::from_millis(600));
    let big = bright_avg(v);
    (normal, big)
}

#[test]
fn subtitle_size_changes_live() {
    let dir = tempfile::tempdir().unwrap();
    if !make(dir.path()) {
        eprintln!("ffmpeg unavailable, skipping");
        return;
    }
    let Some(e) = engine() else { return };
    let v = e.video();
    open(&e, &dir.path().join("plain.mp4"), vec![dir.path().join("plain.srt")]);
    assert!(wait(|| e.status().state == PlayerState::Playing && v.size().is_some(), 10));
    let (normal, big) = measure_scale(&e, &v);
    eprintln!("srt: normal {normal}, at 200% {big}");
    assert!(big > normal * 2, "SRT text did not grow ({normal} -> {big})");
    let t = e.status().time_ms;
    assert!(t > 1000, "playback should have kept going");

    open(&e, &dir.path().join("two_subs.mkv"), vec![]);
    assert!(wait(|| e.status().state == PlayerState::Playing && v.size().is_some(), 10));
    e.send(EngineCommand::SetSubtitleScale(100));
    e.send(EngineCommand::SetSubtitleTrack(2));
    let (normal, big) = measure_scale(&e, &v);
    eprintln!("ass: normal {normal}, at 200% {big}");
}

#[test]
#[ignore = "experiment: plain-text decoding of ASS"]
fn ass_as_plain_text_experiment() {
    let dir = tempfile::tempdir().unwrap();
    if !make(dir.path()) {
        return;
    }
    let mut s = Settings::default();
    s.recording.discover_cast = false;
    s.general.extra_args = "--aout=dummy".into();
    s.performance.hw_decoding = notvlc::domain::settings::HwDecoding::Disabled;
    let Ok(e) = VlcEngine::new(&s) else { return };
    let v = e.video();
    let mut item = MediaItem::file(dir.path().join("two_subs.mkv"));
    item.options = vec![std::env::var("ASS_CODEC").unwrap_or_else(|_| ":codec=subsdec,all".into())];
    e.send(EngineCommand::Open(Box::new(OpenRequest::new(item))));
    assert!(wait(|| e.status().state == PlayerState::Playing && v.size().is_some(), 10));
    e.send(EngineCommand::SetSubtitleTrack(2));
    let (normal, big) = measure_scale(&e, &v);
    eprintln!("ass via subsdec: normal {normal}, at 200% {big}");
}
