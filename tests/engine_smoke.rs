//! End-to-end check against the real libvlc: decode a generated clip into the
//! video buffer, read tracks, probe metadata and make a thumbnail.
//!
//! Skips (passes) when libvlc or ffmpeg is not installed.

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use notvlc::app::ports::{EngineCommand, MediaEngine, MediaProbe, OpenRequest, PlayerState};
use notvlc::domain::media::{MediaItem, MediaSource};
use notvlc::domain::settings::Settings;
use notvlc::infra::vlc::{VlcEngine, VlcProbe};

fn make_clip(dir: &std::path::Path) -> Option<PathBuf> {
    let out = dir.join("clip.mp4");
    let ok = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-f", "lavfi", "-i", "testsrc=size=320x240:rate=25:duration=3"])
        .args(["-f", "lavfi", "-i", "sine=frequency=440:duration=3", "-shortest"])
        .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac"])
        .args(["-metadata", "title=Smoke Clip"])
        .arg(&out)
        .status()
        .ok()?
        .success();
    ok.then_some(out)
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

#[test]
fn plays_and_probes_a_clip() {
    let dir = tempfile::tempdir().unwrap();
    let Some(clip) = make_clip(dir.path()) else {
        eprintln!("ffmpeg unavailable, skipping");
        return;
    };
    let mut settings = Settings::default();
    settings.recording.discover_cast = false;
    // Decode audio but send it nowhere, so the test tone is never heard.
    settings.general.extra_args = "--aout=dummy".into();
    let engine = match VlcEngine::new(&settings) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("libvlc unavailable ({e:#}), skipping");
            return;
        }
    };
    let video = engine.video();
    engine.send(EngineCommand::Open(Box::new(OpenRequest {
        subtitles: vec![],
        ..OpenRequest::new(MediaItem::file(&clip))
    })));
    assert!(wait(|| video.size().is_some(), 10), "no video frame arrived");
    assert_eq!(video.size(), Some((320, 240)));
    assert!(wait(|| engine.status().state == PlayerState::Playing && engine.status().time_ms > 200, 10));
    let st = engine.status();
    assert!(st.length_ms >= 2900, "length {}", st.length_ms);
    assert!(st.audio_tracks.iter().any(|t| t.id >= 0), "audio tracks: {:?}", st.audio_tracks);
    assert_eq!(st.meta.title.as_deref(), Some("Smoke Clip"));

    engine.send(EngineCommand::SeekMs { ms: 2000, precise: true });
    assert!(wait(|| engine.status().time_ms >= 2000, 5));
    assert!(wait(|| engine.status().state == PlayerState::Ended, 10), "never ended");

    let probe = VlcProbe::new(engine.helper());
    let meta = probe.probe(&MediaSource::File(clip.clone())).unwrap();
    assert_eq!(meta.has_video, Some(true));
    assert_eq!(meta.width, Some(320));
    let thumb = dir.path().join("thumb.png");
    probe.thumbnail(&MediaSource::File(clip), &thumb, 160, 0.5).unwrap();
    let img = image::open(&thumb).unwrap();
    assert_eq!((img.width(), img.height()), (160, 120));
}
