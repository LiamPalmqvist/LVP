//! Peak data for the music seek bar. VLC transcodes the track to a low-rate
//! mono WAV in the background (a convert job that never plays locally); the
//! WAV is reduced to `PEAKS` loudness values, cached in memory and as JSON
//! under the store's cache directory, keyed by the file and its mtime.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender, unbounded};
use serde::{Deserialize, Serialize};

use super::App;
use super::ports::Job;
use crate::domain::media::{MediaItem, MediaKind, MediaSource, kind_for_path};

/// Values per track.
pub const PEAKS: usize = 400;
/// Label of the convert jobs; their entries never stay in `App::jobs`.
const JOB_LABEL: &str = "Waveform analysis";
/// Samples per second of the analysis WAV.
const RATE: u32 = 2000;

#[derive(Default)]
pub struct Waveforms {
    ready: HashMap<MediaSource, Arc<Vec<f32>>>,
    /// Job id -> (source, WAV being written, cache file).
    jobs: HashMap<u64, (MediaSource, PathBuf, PathBuf)>,
    /// Sources being reduced to peaks on a background thread.
    reducing: HashSet<MediaSource>,
    failed: HashSet<MediaSource>,
    channel: Option<(Sender<Reduced>, Receiver<Reduced>)>,
}

type Reduced = (MediaSource, Option<Vec<f32>>);

#[derive(Serialize, Deserialize)]
struct CacheFile {
    mrl: String,
    size: u64,
    mtime: u64,
    peaks: Vec<f32>,
}

/// FNV-1a, stable across builds, for cache file names.
fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3))
}

fn file_stamp(path: &Path) -> Option<(u64, u64)> {
    let m = std::fs::metadata(path).ok()?;
    let mtime = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    Some((m.len(), mtime))
}

/// The samples of a 16-bit PCM WAV, mixed down to mono.
pub fn read_wav(bytes: &[u8]) -> Option<Vec<i16>> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let u16_at = |o: usize| bytes.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let u32_at = |o: usize| bytes.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let mut pos = 12;
    let mut channels = 0u16;
    let mut bits = 0u16;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32_at(pos + 4)? as usize;
        let body = pos + 8;
        if id == b"fmt " {
            let format = u16_at(body)?;
            channels = u16_at(body + 2)?;
            bits = u16_at(body + 14)?;
            // PCM, or WAVE_FORMAT_EXTENSIBLE carrying PCM.
            if format != 1 && format != 0xFFFE {
                return None;
            }
        } else if id == b"data" {
            if channels == 0 || bits != 16 {
                return None;
            }
            // A writer that never finalised the header leaves 0 or ~0 here.
            let end = if size == 0 || body + size > bytes.len() { bytes.len() } else { body + size };
            let frame = 2 * channels as usize;
            let data = &bytes[body..end];
            let out = data
                .chunks_exact(frame)
                .map(|f| {
                    let sum: i32 = f.chunks_exact(2).map(|s| i16::from_le_bytes([s[0], s[1]]) as i32).sum::<i32>();
                    (sum / channels as i32) as i16
                })
                .collect();
            return Some(out);
        }
        pos = body + size + (size & 1);
    }
    None
}

/// Reduces samples to `n` loudness values in [0, 1] (RMS per slice,
/// normalised to the loudest slice).
pub fn peaks(samples: &[i16], n: usize) -> Vec<f32> {
    if samples.is_empty() || n == 0 {
        return vec![0.0; n];
    }
    let mut out: Vec<f32> = (0..n)
        .map(|i| {
            let a = i * samples.len() / n;
            let b = ((i + 1) * samples.len() / n).max(a + 1).min(samples.len());
            let slice = &samples[a.min(samples.len() - 1)..b];
            let sum: f64 = slice.iter().map(|s| (*s as f64 / 32768.0).powi(2)).sum();
            (sum / slice.len() as f64).sqrt() as f32
        })
        .collect();
    let max = out.iter().cloned().fold(0.0f32, f32::max);
    if max > 1e-6 {
        for v in &mut out {
            *v = (*v / max).clamp(0.0, 1.0);
        }
    }
    out
}

fn reduce(wav: &Path) -> Option<Vec<f32>> {
    let bytes = std::fs::read(wav).ok()?;
    let samples = read_wav(&bytes)?;
    // Less than a tenth of a second of audio is not worth drawing.
    if samples.len() < (RATE / 10) as usize {
        return None;
    }
    Some(peaks(&samples, PEAKS))
}

impl App {
    fn waveform_dir(&self) -> PathBuf {
        self.store.cache_dir().join("waveforms")
    }

    fn waveform_cache_file(&self, file: &Path) -> PathBuf {
        let key = format!("{:016x}", fnv(&file.to_string_lossy()));
        self.waveform_dir().join(format!("{key}.json"))
    }

    /// Loudness peaks for a local audio file, or `None` while they are being
    /// computed (the first call starts the work) or when they cannot be.
    pub fn waveform(&mut self, source: &MediaSource) -> Option<Vec<f32>> {
        let w = &self.waveforms;
        if let Some(p) = w.ready.get(source) {
            return Some(p.as_ref().clone());
        }
        if w.failed.contains(source) || w.reducing.contains(source) || w.jobs.values().any(|j| j.0 == *source) {
            return None;
        }
        let Some(file) = source.path().map(Path::to_path_buf) else {
            self.waveforms.failed.insert(source.clone());
            return None;
        };
        if kind_for_path(&file) != MediaKind::Audio {
            self.waveforms.failed.insert(source.clone());
            return None;
        }
        let Some((size, mtime)) = file_stamp(&file) else {
            self.waveforms.failed.insert(source.clone());
            return None;
        };
        let cache = self.waveform_cache_file(&file);
        if let Some(c) = std::fs::read_to_string(&cache).ok().and_then(|s| serde_json::from_str::<CacheFile>(&s).ok())
            && c.size == size
            && c.mtime == mtime
            && !c.peaks.is_empty()
        {
            let peaks = Arc::new(c.peaks);
            self.waveforms.ready.insert(source.clone(), peaks.clone());
            return Some(peaks.as_ref().clone());
        }
        // One analysis at a time; the next request starts the next one.
        if !self.waveforms.jobs.is_empty() || size == 0 {
            if size == 0 {
                self.waveforms.failed.insert(source.clone());
            }
            return None;
        }
        let dir = self.waveform_dir();
        if std::fs::create_dir_all(&dir).is_err() {
            self.waveforms.failed.insert(source.clone());
            return None;
        }
        let wav = cache.with_extension("wav");
        let _ = std::fs::remove_file(&wav);
        let mut item = MediaItem::new(source.clone());
        item.options = vec![":no-video".into(), ":no-spu".into(), ":no-sout-video".into()];
        let dst = wav.to_string_lossy().replace('\'', "\\'");
        let sout =
            format!("#transcode{{acodec=s16l,channels=1,samplerate={RATE}}}:std{{access=file,mux=wav,dst='{dst}'}}");
        let id = self.start_job(JOB_LABEL.into(), Job { source: item, sout, play_locally: false });
        self.waveforms.jobs.insert(id, (source.clone(), wav, cache));
        None
    }

    /// Puts known peaks in the cache (demo content and tests).
    pub fn set_waveform(&mut self, source: &MediaSource, peaks: Vec<f32>) {
        self.waveforms.failed.remove(source);
        self.waveforms.ready.insert(source.clone(), Arc::new(peaks));
    }

    /// Collects finished analysis jobs and reduced peaks. Called from `tick`.
    pub(super) fn tick_waveforms(&mut self) {
        if self.waveforms.jobs.is_empty() && self.waveforms.reducing.is_empty() {
            return;
        }
        let finished: Vec<(u64, bool)> = self
            .jobs
            .iter()
            .filter(|j| self.waveforms.jobs.contains_key(&j.id))
            .filter_map(|j| j.finished.as_ref().map(|(ok, _)| (j.id, *ok)))
            .collect();
        if !finished.is_empty() {
            // These are not user conversions: no list entry, no toast.
            let ids: HashSet<u64> = finished.iter().map(|f| f.0).collect();
            self.jobs.retain(|j| !ids.contains(&j.id));
            let prefix = format!("{JOB_LABEL}: ");
            self.toasts.retain(|t| !t.text.starts_with(&prefix));
        }
        for (id, ok) in finished {
            let Some((source, wav, cache)) = self.waveforms.jobs.remove(&id) else { continue };
            if !ok {
                log::debug!("waveform analysis failed for {}", source.mrl());
                let _ = std::fs::remove_file(&wav);
                self.waveforms.failed.insert(source);
                continue;
            }
            let tx = self.waveforms.channel.get_or_insert_with(unbounded).0.clone();
            self.waveforms.reducing.insert(source.clone());
            let stamp = source.path().and_then(file_stamp);
            let mrl = source.mrl();
            let spawned = std::thread::Builder::new().name("waveform".into()).spawn(move || {
                let peaks = reduce(&wav);
                let _ = std::fs::remove_file(&wav);
                if let (Some(p), Some((size, mtime))) = (&peaks, stamp) {
                    let rounded = p.iter().map(|v| (v * 1000.0).round() / 1000.0).collect();
                    let file = CacheFile { mrl, size, mtime, peaks: rounded };
                    if let Ok(json) = serde_json::to_string(&file) {
                        let _ = std::fs::write(&cache, json);
                    }
                }
                let _ = tx.send((source, peaks));
            });
            if spawned.is_err() {
                log::debug!("could not start the waveform thread");
            }
        }
        let done: Vec<Reduced> = match &self.waveforms.channel {
            Some((_, rx)) => rx.try_iter().collect(),
            None => Vec::new(),
        };
        for (source, peaks) in done {
            self.waveforms.reducing.remove(&source);
            match peaks {
                Some(p) => {
                    self.waveforms.ready.insert(source, Arc::new(p));
                }
                None => {
                    self.waveforms.failed.insert(source);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::fake::{FakeEngine, FakeProbe};
    use crate::app::ports::{EngineEvent, MediaEngine};
    use crate::app::testing::MemStore;

    fn app_at(root: &Path) -> (App, Arc<FakeEngine>) {
        let fake = Arc::new(FakeEngine::new());
        let engine: Arc<dyn MediaEngine> = fake.clone();
        (App::new(engine, Arc::new(FakeProbe), Arc::new(MemStore::new(root))), fake)
    }

    fn setup() -> (App, Arc<FakeEngine>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let (app, fake) = app_at(dir.path());
        (app, fake, dir)
    }

    /// A mono 16-bit WAV with a header written the way VLC's muxer does.
    fn wav(samples: &[i16], rate: u32, channels: u16) -> Vec<u8> {
        let data_len = (samples.len() * 2) as u32;
        let mut v = Vec::new();
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + data_len).to_le_bytes());
        v.extend_from_slice(b"WAVE");
        v.extend_from_slice(b"fmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&channels.to_le_bytes());
        v.extend_from_slice(&rate.to_le_bytes());
        v.extend_from_slice(&(rate * 2 * channels as u32).to_le_bytes());
        v.extend_from_slice(&(2 * channels).to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        // An odd-sized extra chunk before the data, to exercise padding.
        v.extend_from_slice(b"LIST");
        v.extend_from_slice(&3u32.to_le_bytes());
        v.extend_from_slice(&[1, 2, 3, 0]);
        v.extend_from_slice(b"data");
        v.extend_from_slice(&data_len.to_le_bytes());
        for s in samples {
            v.extend_from_slice(&s.to_le_bytes());
        }
        v
    }

    #[test]
    fn reads_pcm_and_mixes_channels() {
        let mono = read_wav(&wav(&[1, -2, 300], 2000, 1)).unwrap();
        assert_eq!(mono, vec![1, -2, 300]);
        let stereo = read_wav(&wav(&[100, 300, -50, -150], 2000, 2)).unwrap();
        assert_eq!(stereo, vec![200, -100]);
        assert!(read_wav(b"not a wav file").is_none());
    }

    #[test]
    fn reads_an_unfinalised_data_chunk() {
        let mut bytes = wav(&[5, 6, 7, 8], 2000, 1);
        let at = bytes.len() - 8 - 4;
        bytes[at..at + 4].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(read_wav(&bytes).unwrap(), vec![5, 6, 7, 8]);
    }

    #[test]
    fn peaks_follow_loudness() {
        // Quiet first half, loud second half.
        let samples: Vec<i16> = (0..4000).map(|i| if i < 2000 { 1000 } else { 16000 }).collect();
        let p = peaks(&samples, 10);
        assert_eq!(p.len(), 10);
        assert!(p[0] < 0.1 && (p[9] - 1.0).abs() < 1e-6);
        assert!(peaks(&[], 4).iter().all(|v| *v == 0.0));
    }

    #[test]
    fn analyses_in_the_background_and_caches() {
        let (mut app, fake, dir) = setup();
        let song = dir.path().join("song.mp3");
        std::fs::write(&song, b"not really an mp3").unwrap();
        let source = MediaSource::File(song.clone());
        assert!(app.waveform(&source).is_none());
        assert_eq!(app.jobs.len(), 1, "a convert job was started");
        let (wav_path, cache_path) = {
            let (_, w, c) = app.waveforms.jobs.values().next().unwrap();
            (w.clone(), c.clone())
        };
        // The job writes the WAV, then reports success.
        let samples: Vec<i16> = (0..8000).map(|i| ((i as f32 * 0.05).sin() * (i as f32) * 2.0) as i16).collect();
        std::fs::write(&wav_path, wav(&samples, RATE, 1)).unwrap();
        let id = app.jobs[0].id;
        fake.events.lock().push(EngineEvent::JobFinished { id, ok: true, message: "Done".into() });
        let mut peaks = None;
        for _ in 0..200 {
            app.tick();
            if let Some(p) = app.waveform(&source) {
                peaks = Some(p);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let peaks = peaks.expect("peaks computed");
        assert_eq!(peaks.len(), PEAKS);
        assert!(peaks[PEAKS - 1] > peaks[10], "the signal grows louder");
        assert!(app.jobs.is_empty(), "the analysis job is not listed");
        assert!(app.toasts.iter().all(|t| !t.text.starts_with(JOB_LABEL)), "no toast");
        assert!(!wav_path.exists(), "the WAV is removed");
        assert!(cache_path.exists(), "peaks are cached on disk");

        // A fresh app reads the cached peaks without a new job.
        let (mut again, _fake) = app_at(dir.path());
        assert!(again.waveform(&source).is_some_and(|p| p.len() == PEAKS));
        assert!(again.jobs.is_empty());
    }

    #[test]
    fn skips_video_and_network_sources() {
        let (mut app, _fake, dir) = setup();
        let video = dir.path().join("clip.mp4");
        std::fs::write(&video, b"x").unwrap();
        assert!(app.waveform(&MediaSource::File(video)).is_none());
        assert!(app.waveform(&MediaSource::Location("http://example.com/a.mp3".into())).is_none());
        assert!(app.jobs.is_empty());
    }

    #[test]
    fn a_failed_job_is_not_retried() {
        let (mut app, fake, dir) = setup();
        let song = dir.path().join("broken.flac");
        std::fs::write(&song, b"x").unwrap();
        let source = MediaSource::File(song);
        assert!(app.waveform(&source).is_none());
        let id = app.jobs[0].id;
        fake.events.lock().push(EngineEvent::JobFinished { id, ok: false, message: "VLC error".into() });
        app.tick();
        assert!(app.jobs.is_empty());
        assert!(app.toasts.is_empty(), "{:?}", app.toasts);
        assert!(app.waveform(&source).is_none());
        assert!(app.jobs.is_empty(), "no second job");
    }
}
