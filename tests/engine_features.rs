//! libvlc features beyond plain playback, against the real engine: live
//! video filters through libvlccore, recording, visualisations and
//! rebuilding the instance from new settings.
//!
//! Silent (dummy audio output). Skips when libvlc or ffmpeg is missing.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use notvlc::app::ports::{EngineCommand, MediaEngine, OpenRequest, PlayerState};
use notvlc::app::video::VideoOutput;
use notvlc::domain::effects::VideoEffects;
use notvlc::domain::media::MediaItem;
use notvlc::domain::settings::{Settings, Visualizer};
use notvlc::infra::vlc::VlcEngine;

fn ffmpeg(args: &[&str], out: &Path) -> bool {
    Command::new("ffmpeg")
        .args(["-v", "error", "-y"])
        .args(args)
        .arg(out)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

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

/// Average RGB over the whole picture.
fn mean_rgb(v: &VideoOutput) -> Option<[f32; 3]> {
    v.with_front(|_, px, w, h, pitch| {
        let mut sum = [0f64; 3];
        for y in 0..h as usize {
            for x in 0..w as usize {
                let i = y * pitch as usize + x * 4;
                for c in 0..3 {
                    sum[c] += px[i + c] as f64;
                }
            }
        }
        let n = (w * h) as f64;
        [(sum[0] / n) as f32, (sum[1] / n) as f32, (sum[2] / n) as f32]
    })
}

fn open(engine: &VlcEngine, path: &Path) {
    engine.send(EngineCommand::Open(Box::new(OpenRequest {
        subtitles: vec![],
        ..OpenRequest::new(MediaItem::file(path))
    })));
}

#[test]
fn live_filters_record_visualiser_and_reconfigure() {
    let dir = tempfile::tempdir().unwrap();
    // A dark test card: mostly black, so inverting it makes it mostly white.
    let video = dir.path().join("dark.mp4");
    let audio = dir.path().join("tone.m4a");
    if !ffmpeg(
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=0x202020:size=320x240:rate=25:duration=12",
            "-f",
            "lavfi",
            "-i",
            "sine=duration=12",
        ],
        &video,
    ) || !ffmpeg(&["-f", "lavfi", "-i", "sine=frequency=440:duration=8", "-c:a", "aac"], &audio)
    {
        eprintln!("ffmpeg unavailable, skipping");
        return;
    }
    let records = dir.path().join("records");
    std::fs::create_dir_all(&records).unwrap();
    let mut settings = Settings::default();
    settings.recording.discover_cast = false;
    settings.recording.record_dir = records.to_string_lossy().into_owned();
    settings.general.extra_args = "--aout=dummy".into();
    let engine = match VlcEngine::new(&settings) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("libvlc unavailable ({e:#}), skipping");
            return;
        }
    };
    if !engine.status().live_effects {
        eprintln!("libvlccore unavailable, skipping live checks");
        return;
    }
    let out = engine.video();

    // 1. Live video filter: invert the picture mid-playback.
    open(&engine, &video);
    assert!(wait(|| out.size().is_some(), 10), "no picture");
    std::thread::sleep(Duration::from_millis(500));
    let before = mean_rgb(&out).unwrap();
    assert!(before[0] < 60.0, "test card should be dark, got {before:?}");
    let mut fx = VideoEffects::default();
    fx.colors.invert = true;
    engine.send(EngineCommand::SetVideoEffects(Box::new(fx)));
    assert!(wait(|| mean_rgb(&out).is_some_and(|m| m[0] > 180.0), 5), "invert did not reach the picture");

    // 2. Recording the playing input to the record folder.
    engine.send(EngineCommand::SetRecording(true));
    assert!(wait(|| engine.status().recording, 3), "recording did not start");
    std::thread::sleep(Duration::from_millis(2500));
    engine.send(EngineCommand::SetRecording(false));
    let recorded = || -> Vec<PathBuf> {
        std::fs::read_dir(&records)
            .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.metadata().is_ok_and(|m| m.len() > 1000)).collect())
            .unwrap_or_default()
    };
    assert!(wait(|| !recorded().is_empty(), 5), "no recording written");

    // 3. Rebuilding the instance from new settings resumes the item.
    let mut changed = settings.clone();
    changed.network.network_caching_ms = 1500;
    changed.video.effects = VideoEffects::default();
    engine.send(EngineCommand::Reconfigure(Box::new(changed)));
    engine.send(EngineCommand::SetVideoEffects(Box::default()));
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        wait(|| engine.status().state == PlayerState::Playing && mean_rgb(&out).is_some_and(|m| m[0] < 60.0), 10),
        "playback did not resume after reconfigure"
    );
    engine.send(EngineCommand::Stop);

    // 4. A visualisation turns audio into pictures through the same output.
    engine.send(EngineCommand::SetVisualizer(Visualizer::Spectrum));
    std::thread::sleep(Duration::from_millis(200));
    out.clear();
    open(&engine, &audio);
    assert!(wait(|| out.size().is_some(), 10), "the visualisation produced no picture");
    engine.send(EngineCommand::Stop);
}
