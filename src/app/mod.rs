//! Use cases and state: the queue, library, settings and everything the UI
//! asks for. Talks to libvlc and the disk only through `ports`.

pub mod actions;
pub mod fake;
pub mod library;
pub mod ports;
#[cfg(test)]
pub mod testing;
pub mod video;
pub mod workers;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::domain::effects::{AudioEffects, StereoMode, VideoEffects};
use crate::domain::library::Bookmark;
use crate::domain::media::{MediaItem, MediaKind, MediaMeta, MediaSource, kind_for_path, matching_subtitles};
use crate::domain::playlist::{Advance, Queue};
use crate::domain::playlist_format::{self, PlaylistFormat};
use crate::domain::settings::{ContinuePlayback, RepeatMode, SPEED_STEPS, Settings, WhenFinished};
use crate::domain::time::format_hms;
use library::Library;
use ports::*;
use workers::{Done, Work, Workers};

pub use actions::UiCommand;

/// A transient message shown at the bottom of the window.
#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    pub text: String,
    pub error: bool,
    pub until: Instant,
}

/// Feedback drawn over the video for a moment (volume, seek, speed...).
#[derive(Debug, Clone, PartialEq)]
pub struct Osd {
    pub text: String,
    pub until: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AbLoop {
    pub a: Option<i64>,
    pub b: Option<i64>,
}

/// How the picture is fitted into the video area. Applied by the renderer.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VideoGeometry {
    pub aspect: Option<String>,
    pub crop: Option<String>,
    /// 0 fits the window; otherwise a fixed scale of the source size.
    pub zoom: f32,
    pub rotate_deg: f32,
    pub flip_h: bool,
    pub flip_v: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ThumbState {
    Pending,
    Ready(PathBuf),
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JobState {
    pub id: u64,
    pub label: String,
    pub progress: f32,
    pub finished: Option<(bool, String)>,
}

pub struct App {
    pub engine: Arc<dyn MediaEngine>,
    store: Arc<dyn Store>,
    pub settings: Settings,
    settings_dirty: Option<Instant>,
    pub library: Library,
    pub queue: Queue,
    pub status: EngineStatus,
    workers: Workers,
    thumbs: HashMap<String, ThumbState>,
    probing: HashSet<String>,
    pub toasts: Vec<Toast>,
    pub osd: Option<Osd>,
    pub dialogs: VecDeque<DialogRequest>,
    pub jobs: Vec<JobState>,
    pub ab_loop: AbLoop,
    pub sleep_until: Option<Instant>,
    pub geometry: VideoGeometry,
    pub subtitle_timing: SubtitleTiming,
    pub stereo_mode: StereoMode,
    pub reverse: bool,
    /// Marks for "subtitle sync": (audio mark, subtitle mark) in ms.
    sync_marks: (Option<i64>, Option<i64>),
    last_save: Instant,
    pub quit_requested: bool,
    /// Index of the item whose position we last saved, to save on change.
    playing_source: Option<MediaSource>,
    pub system: system::SystemState,
}

impl App {
    pub fn new(engine: Arc<dyn MediaEngine>, probe: Arc<dyn MediaProbe>, store: Arc<dyn Store>) -> Self {
        let settings: Settings = store.load("settings").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let library = Library::load(store.clone());
        let threads = std::thread::available_parallelism().map(|n| n.get().clamp(2, 4)).unwrap_or(2);
        let workers = Workers::new(probe, threads);
        let mut queue: Queue = store.load("queue").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        queue.repeat = settings.playback.repeat;
        queue.set_shuffle(settings.playback.shuffle);
        let subtitle_timing =
            SubtitleTiming { speed: settings.subtitles.speed, duration_factor: settings.subtitles.duration_factor };
        let app = Self {
            engine,
            store,
            settings_dirty: None,
            library,
            queue,
            status: EngineStatus::default(),
            workers,
            thumbs: HashMap::new(),
            probing: HashSet::new(),
            toasts: Vec::new(),
            osd: None,
            dialogs: VecDeque::new(),
            jobs: Vec::new(),
            ab_loop: AbLoop::default(),
            sleep_until: None,
            geometry: VideoGeometry {
                aspect: settings.video.aspect_ratio.clone(),
                crop: settings.video.crop.clone(),
                zoom: settings.video.zoom,
                ..Default::default()
            },
            subtitle_timing,
            stereo_mode: StereoMode::Stereo,
            reverse: false,
            sync_marks: (None, None),
            last_save: Instant::now(),
            quit_requested: false,
            playing_source: None,
            system: Default::default(),
            settings,
        };
        app.engine.send(EngineCommand::SetVolume(app.settings.audio.volume));
        app.engine.send(EngineCommand::SetSubtitleTiming(app.subtitle_timing));
        app
    }

    /// Requests a repaint whenever background work completes.
    pub fn set_repaint(&self, f: impl Fn() + Send + Sync + Clone + 'static) {
        self.workers.set_notify(f.clone());
        self.engine.video().set_notify(f);
    }

    // ---- per-frame -------------------------------------------------------

    /// Drains engine and worker events and runs time-based rules. Call once
    /// per UI frame.
    pub fn tick(&mut self) {
        let now = Instant::now();
        for ev in self.engine.poll_events() {
            self.on_engine_event(ev);
        }
        self.status = self.engine.status();
        for done in self.workers.finished() {
            self.on_work_done(done);
        }
        // A-B loop.
        if let (Some(a), Some(b)) = (self.ab_loop.a, self.ab_loop.b)
            && self.status.state == PlayerState::Playing
            && self.status.time_ms >= b
        {
            self.engine.send(EngineCommand::SeekMs { ms: a, precise: true });
        }
        // Sleep timer.
        if let Some(t) = self.sleep_until
            && now >= t
        {
            self.sleep_until = None;
            self.engine.send(EngineCommand::Pause);
            self.toast("Sleep timer: playback paused");
        }
        // Periodic persistence.
        if now.duration_since(self.last_save) > Duration::from_secs(5) {
            self.last_save = now;
            self.save_position();
            self.library.save();
        }
        if let Some(t) = self.settings_dirty
            && now.duration_since(t) > Duration::from_millis(600)
        {
            self.save_settings_now();
        }
        self.tick_system();
        self.toasts.retain(|t| t.until > now);
        if self.osd.as_ref().is_some_and(|o| o.until <= now) {
            self.osd = None;
        }
    }

    fn on_engine_event(&mut self, ev: EngineEvent) {
        match ev {
            EngineEvent::EndReached => {
                if let Some(src) = self.queue.current_item().map(|i| i.source.clone()) {
                    // Finished: forget the resume point.
                    self.library.update_record(&src, |r| r.position_ms = 0);
                }
                self.advance_after_end();
            }
            EngineEvent::Error(e) => self.toast_error(e),
            EngineEvent::MetaChanged => {
                let meta = self.engine.status().meta;
                if let Some(item) = self.queue.current_item_mut() {
                    item.meta.merge(meta.clone());
                    let src = item.source.clone();
                    self.library.set_meta(&src, meta);
                }
            }
            EngineEvent::LengthChanged(len) => {
                if let Some(item) = self.queue.current_item_mut() {
                    item.meta.duration_ms = Some(len);
                }
            }
            EngineEvent::Dialog(d) => match d {
                DialogRequest::Cancel { id } => self.dialogs.retain(|x| dialog_id(x) != Some(id)),
                DialogRequest::UpdateProgress { id, position, text } => {
                    for x in self.dialogs.iter_mut() {
                        if let DialogRequest::Progress { id: pid, position: p, text: t, .. } = x
                            && *pid == id
                        {
                            *p = Some(position);
                            if !text.is_empty() {
                                *t = text.clone();
                            }
                        }
                    }
                }
                DialogRequest::Error { .. } if !self.settings.general.show_dialogs => {}
                other => self.dialogs.push_back(other),
            },
            EngineEvent::JobProgress { id, position } => {
                if let Some(j) = self.jobs.iter_mut().find(|j| j.id == id) {
                    j.progress = position;
                }
            }
            EngineEvent::JobFinished { id, ok, message } => {
                if let Some(j) = self.jobs.iter_mut().find(|j| j.id == id) {
                    j.progress = 1.0;
                    j.finished = Some((ok, message.clone()));
                    let label = j.label.clone();
                    if ok {
                        self.toast(format!("{label}: finished"));
                    } else {
                        self.toast_error(format!("{label}: {message}"));
                    }
                }
            }
            _ => {}
        }
    }

    fn advance_after_end(&mut self) {
        match self.queue.on_finished(self.settings.playback.stop_after_each) {
            Advance::Play(i) => self.play_index(i),
            Advance::Repeat => {
                if let Some(i) = self.queue.current {
                    self.play_index_from(i, Some(0));
                }
            }
            Advance::Finished => match self.settings.playback.when_finished {
                WhenFinished::Quit => self.quit_requested = true,
                WhenFinished::Stop | WhenFinished::ReturnHome => {}
            },
        }
    }

    fn on_work_done(&mut self, done: Done) {
        match done {
            Done::Meta(src, meta) => {
                let meta = *meta;
                self.probing.remove(&src.mrl());
                for item in self.queue.items.iter_mut().filter(|i| i.source == src) {
                    let mut m = meta.clone();
                    m.merge(item.meta.clone());
                    item.meta = m;
                }
                self.library.set_meta(&src, meta);
            }
            Done::Thumbnail(src, path) => {
                self.thumbs.insert(src.mrl(), ThumbState::Ready(path));
            }
            Done::Failed(work, err) => match work {
                Work::Probe(src) => {
                    log::debug!("probe failed for {}: {err}", src.mrl());
                    self.probing.remove(&src.mrl());
                }
                Work::Thumbnail { source, .. } => {
                    log::debug!("thumbnail failed for {}: {err}", source.mrl());
                    self.thumbs.insert(source.mrl(), ThumbState::Failed);
                }
            },
        }
    }

    // ---- messages --------------------------------------------------------

    pub fn toast(&mut self, text: impl Into<String>) {
        self.toasts.push(Toast { text: text.into(), error: false, until: Instant::now() + Duration::from_secs(4) });
    }

    pub fn toast_error(&mut self, text: impl Into<String>) {
        let text = text.into();
        log::warn!("{text}");
        self.toasts.push(Toast { text, error: true, until: Instant::now() + Duration::from_secs(6) });
    }

    pub fn show_osd(&mut self, text: impl Into<String>) {
        if !self.settings.subtitles.osd_feedback {
            return;
        }
        let secs = self.settings.subtitles.osd_feedback_s.max(0.3);
        self.osd = Some(Osd { text: text.into(), until: Instant::now() + Duration::from_secs_f32(secs) });
    }

    // ---- thumbnails and metadata -----------------------------------------

    /// The thumbnail (or cover art) for a source, scheduling it if needed.
    pub fn thumbnail(&mut self, item: &MediaItem) -> Option<PathBuf> {
        let mrl = item.source.mrl();
        if let Some(art) = item.meta.artwork_url.as_deref().and_then(crate::domain::media::file_url_to_path)
            && art.exists()
        {
            return Some(art);
        }
        match self.thumbs.get(&mrl) {
            Some(ThumbState::Ready(p)) => return Some(p.clone()),
            Some(_) => return None,
            None => {}
        }
        let path = self.library.thumbnail_path(&item.source);
        if path.exists() {
            self.thumbs.insert(mrl, ThumbState::Ready(path.clone()));
            return Some(path);
        }
        let file = item.source.path()?;
        match kind_for_path(file) {
            MediaKind::Video => {
                self.thumbs.insert(mrl, ThumbState::Pending);
                self.workers.submit(Work::Thumbnail { source: item.source.clone(), out: path, width: 480 });
            }
            MediaKind::Audio => {
                // Cover art comes from parsing.
                self.request_meta(item);
            }
            MediaKind::Image => {
                self.thumbs.insert(mrl, ThumbState::Ready(file.to_path_buf()));
                return Some(file.to_path_buf());
            }
            _ => {}
        }
        None
    }

    /// Parses an item in the background unless its duration is already known.
    pub fn request_meta(&mut self, item: &MediaItem) {
        let mrl = item.source.mrl();
        if item.meta.duration_ms.is_some() && (item.meta.artwork_url.is_some() || !item.is_audio()) {
            return;
        }
        if self.library.meta(&item.source).is_some_and(|m| m.duration_ms.is_some()) || !self.probing.insert(mrl) {
            return;
        }
        self.workers.submit(Work::Probe(item.source.clone()));
    }

    // ---- opening ---------------------------------------------------------

    /// Expands folders and playlist files into playable items.
    pub fn expand_paths(&self, paths: &[PathBuf]) -> Vec<MediaItem> {
        let recursive = self.settings.library.folder_open == crate::domain::settings::FolderOpen::IncludeSubfolders;
        let mut out = Vec::new();
        for p in paths {
            if p.is_dir() {
                out.extend(self.library.collect_media(
                    p,
                    &[MediaKind::Video, MediaKind::Audio],
                    recursive,
                    self.settings.library.show_hidden,
                ));
            } else if let Some(fmt) = p.extension().and_then(|e| e.to_str()).and_then(PlaylistFormat::from_extension) {
                match std::fs::read_to_string(p) {
                    Ok(text) => out.extend(playlist_format::parse(&text, fmt, p.parent())),
                    Err(e) => log::warn!("reading {}: {e}", p.display()),
                }
            } else {
                let mut item = MediaItem::file(p);
                self.library.enrich(&mut item);
                out.push(item);
            }
        }
        out
    }

    /// Opens files, folders or playlists the way the user asked: replace the
    /// queue and play, or append when "Add to the playlist" is on.
    pub fn open_paths(&mut self, paths: Vec<PathBuf>) {
        let (subs, media): (Vec<PathBuf>, Vec<PathBuf>) =
            paths.into_iter().partition(|p| kind_for_path(p) == MediaKind::Subtitle);
        if media.is_empty() && !subs.is_empty() {
            for s in subs {
                self.add_subtitle(s);
            }
            return;
        }
        let items = self.expand_paths(&media);
        if items.is_empty() {
            self.toast_error("Nothing playable was found.");
            return;
        }
        let origin =
            (media.len() == 1).then(|| media[0].file_name().map(|n| n.to_string_lossy().into_owned())).flatten();
        if self.settings.library.enqueue && !self.queue.is_empty() {
            let n = items.len();
            for i in items {
                self.queue.push(i);
            }
            self.toast(format!("Added {n} item{} to the queue", if n == 1 { "" } else { "s" }));
        } else {
            self.play_items(items, 0, origin);
        }
    }

    pub fn open_location(&mut self, url: &str) {
        let url = url.trim();
        if url.is_empty() {
            return;
        }
        let recents = &mut self.settings.network.recent_urls;
        recents.retain(|u| u != url);
        recents.insert(0, url.to_string());
        recents.truncate(20);
        self.mark_settings_dirty();
        let item = MediaItem::new(MediaSource::from_user_input(url));
        self.play_items(vec![item], 0, None);
    }

    /// Replaces the queue and starts playing at `start`.
    pub fn play_items(&mut self, items: Vec<MediaItem>, start: usize, origin: Option<String>) {
        self.save_position();
        let shuffle = self.queue.shuffle;
        let repeat = self.queue.repeat;
        self.queue = Queue::new(items);
        self.queue.repeat = repeat;
        self.queue.origin = origin;
        self.queue.current = Some(start.min(self.queue.len().saturating_sub(1)));
        self.queue.set_shuffle(shuffle);
        if !self.queue.is_empty() {
            self.play_index(self.queue.current.unwrap_or(0));
        }
        self.save_queue();
    }

    pub fn enqueue(&mut self, items: Vec<MediaItem>) {
        let was_empty = self.queue.is_empty();
        for i in items {
            self.queue.push(i);
        }
        if was_empty && !self.queue.is_empty() {
            self.play_index(0);
        }
        self.save_queue();
    }

    pub fn play_next(&mut self, item: MediaItem) {
        self.queue.insert_next(item);
        self.save_queue();
    }

    pub fn play_index(&mut self, index: usize) {
        self.play_index_from(index, None);
    }

    /// Starts queue item `index`. `start_ms` overrides resume behaviour.
    pub fn play_index_from(&mut self, index: usize, start_ms: Option<i64>) {
        self.save_position();
        let Some(mut item) = self.queue.items.get(index).cloned() else { return };
        self.library.enrich(&mut item);
        self.queue.current = Some(index);
        let record = self.library.record(&item.source);
        let resume = match (start_ms, self.settings.playback.continue_playback) {
            (Some(ms), _) => Some(ms),
            (None, ContinuePlayback::Never) => None,
            (None, _) if self.settings.privacy.remember_position && record.resumable() => {
                if !self.remember_excluded(&item) {
                    self.toast(format!("Resuming from {}", format_hms(record.position_ms)));
                    Some(record.position_ms)
                } else {
                    None
                }
            }
            _ => None,
        };
        let subtitles = item
            .source
            .path()
            .filter(|_| self.settings.subtitles.autoload)
            .map(|p| self.sidecar_subtitles(p))
            .unwrap_or_default();
        self.ab_loop = AbLoop::default();
        self.engine.send(EngineCommand::Open(Box::new(OpenRequest {
            item: item.clone(),
            start_ms: resume,
            paused: !self.settings.playback.autoplay,
            subtitles,
        })));
        if let Some(t) = record.audio_track {
            self.engine.send(EngineCommand::SetAudioTrack(t));
        }
        self.library.mark_played(&item, self.settings.privacy.save_recent && !self.remember_excluded(&item));
        self.request_meta(&item);
        self.playing_source = Some(item.source.clone());
        if self.settings.subtitles.osd_filename {
            let secs = self.settings.subtitles.osd_filename_s.max(0.5);
            self.osd = Some(Osd { text: item.title(), until: Instant::now() + Duration::from_secs_f32(secs) });
        }
    }

    fn remember_excluded(&self, item: &MediaItem) -> bool {
        let pat = self.settings.privacy.never_remember_pattern.trim();
        !pat.is_empty()
            && pat
                .split(',')
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .any(|p| item.source.mrl().to_lowercase().contains(&p.to_lowercase()))
    }

    /// Subtitles next to a video that VLC's own autodetection might miss
    /// (it only looks for exact names in some cases).
    fn sidecar_subtitles(&self, video: &Path) -> Vec<PathBuf> {
        let Some(dir) = video.parent() else { return Vec::new() };
        let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
        let candidates: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        use crate::domain::settings::SubMatching;
        match self.settings.subtitles.matching {
            SubMatching::AllInFolder => {
                candidates.into_iter().filter(|c| kind_for_path(c) == MediaKind::Subtitle).collect()
            }
            m => matching_subtitles(video, &candidates, m != SubMatching::Exact),
        }
    }

    fn save_position(&mut self) {
        let Some(src) = self.playing_source.clone() else { return };
        if !self.settings.privacy.remember_position || !self.status.state.is_active() {
            return;
        }
        if self.status.time_ms > 0 && self.status.length_ms > 0 {
            self.library.save_position(&src, self.status.time_ms, self.status.length_ms);
        }
    }

    pub fn save_queue(&self) {
        if let Ok(json) = serde_json::to_string(&self.queue) {
            let _ = self.store.save("queue", &json);
        }
    }

    // ---- transport -------------------------------------------------------

    pub fn has_media(&self) -> bool {
        self.queue.current_item().is_some()
    }

    pub fn is_playing(&self) -> bool {
        matches!(self.status.state, PlayerState::Playing | PlayerState::Buffering | PlayerState::Opening)
    }

    pub fn toggle_pause(&mut self) {
        match self.status.state {
            PlayerState::Idle | PlayerState::Stopped | PlayerState::Ended | PlayerState::Error => {
                if let Some(i) = self.queue.current.or(if self.queue.is_empty() { None } else { Some(0) }) {
                    self.play_index(i);
                }
            }
            _ => self.engine.send(EngineCommand::TogglePause),
        }
    }

    pub fn play(&mut self) {
        if matches!(self.status.state, PlayerState::Paused) {
            self.engine.send(EngineCommand::Play);
        } else if !self.is_playing() {
            self.toggle_pause();
        }
    }

    pub fn pause(&mut self) {
        self.engine.send(EngineCommand::Pause);
    }

    pub fn stop(&mut self) {
        self.save_position();
        self.engine.send(EngineCommand::Stop);
        self.ab_loop = AbLoop::default();
    }

    pub fn next(&mut self) {
        match self.queue.next_index() {
            Some(i) => self.play_index_from(i, None),
            None => self.toast("End of the queue"),
        }
    }

    /// Restarts the item when more than three seconds in, like VLC.
    pub fn previous(&mut self) {
        if self.status.time_ms > 3000 && self.status.seekable {
            self.seek_ms(0);
            return;
        }
        if let Some(i) = self.queue.previous_index() {
            self.play_index_from(i, Some(0));
        }
    }

    pub fn seek_ms(&mut self, ms: i64) {
        let ms = if self.status.length_ms > 0 { ms.clamp(0, self.status.length_ms) } else { ms.max(0) };
        self.engine.send(EngineCommand::SeekMs { ms, precise: self.settings.playback.seek_precisely });
        self.status.time_ms = ms;
    }

    pub fn seek_fraction(&mut self, f: f32) {
        if self.status.length_ms > 0 {
            self.seek_ms((self.status.length_ms as f64 * f.clamp(0.0, 1.0) as f64) as i64);
        } else {
            self.engine.send(EngineCommand::SeekPosition(f));
        }
    }

    pub fn jump(&mut self, delta_ms: i64) {
        if !self.status.seekable {
            return;
        }
        let t = self.status.time_ms + delta_ms;
        self.seek_ms(t);
        let secs = delta_ms / 1000;
        self.show_osd(format!("{}{}s  ({})", if secs >= 0 { "+" } else { "" }, secs, format_hms(t.max(0))));
    }

    pub fn set_rate(&mut self, rate: f32) {
        let r = rate.clamp(0.25, 8.0);
        self.engine.send(EngineCommand::SetRate(r));
        self.status.rate = r;
        self.show_osd(format!("Speed: {r:.2}x"));
    }

    /// Steps through the design's speed presets (fine steps use 0.1x).
    pub fn rate_step(&mut self, faster: bool, fine: bool) {
        let cur = if self.status.rate > 0.0 { self.status.rate } else { 1.0 };
        let next = if fine {
            ((cur + if faster { 0.1 } else { -0.1 }) * 100.0).round() / 100.0
        } else if faster {
            SPEED_STEPS.iter().copied().find(|s| *s > cur + 0.001).unwrap_or(cur * 1.5)
        } else {
            SPEED_STEPS.iter().rev().copied().find(|s| *s < cur - 0.001).unwrap_or(cur / 1.5)
        };
        self.set_rate(next);
    }

    pub fn frame_step(&mut self) {
        self.engine.send(EngineCommand::NextFrame);
    }

    pub fn set_volume(&mut self, v: i32) {
        let v = v.clamp(0, self.settings.audio.max_volume.clamp(100, 200));
        self.engine.send(EngineCommand::SetVolume(v));
        if self.status.muted && v > 0 {
            self.engine.send(EngineCommand::SetMute(false));
        }
        self.status.volume = v;
        self.settings.audio.volume = v;
        self.mark_settings_dirty();
    }

    pub fn change_volume(&mut self, delta: i32) {
        let v = self.status.volume + delta;
        self.set_volume(v);
        self.show_osd(format!("Volume: {}%", self.status.volume));
    }

    pub fn toggle_mute(&mut self) {
        let m = !self.status.muted;
        self.engine.send(EngineCommand::SetMute(m));
        self.status.muted = m;
        self.show_osd(if m { "Muted" } else { "Unmuted" });
    }

    pub fn set_repeat(&mut self, r: RepeatMode) {
        self.queue.repeat = r;
        self.settings.playback.repeat = r;
        self.mark_settings_dirty();
        self.show_osd(r.label());
    }

    pub fn set_shuffle(&mut self, on: bool) {
        self.queue.set_shuffle(on);
        self.settings.playback.shuffle = on;
        self.mark_settings_dirty();
        self.show_osd(if on { "Shuffle on" } else { "Shuffle off" });
    }

    /// None -> A -> A and B -> none.
    pub fn cycle_ab_loop(&mut self) {
        let t = self.status.time_ms;
        self.ab_loop = match self.ab_loop {
            AbLoop { a: None, .. } => AbLoop { a: Some(t), b: None },
            AbLoop { a: Some(a), b: None } if t > a => AbLoop { a: Some(a), b: Some(t) },
            AbLoop { a: Some(_), b: None } => AbLoop { a: Some(t), b: None },
            _ => AbLoop::default(),
        };
        let msg = match self.ab_loop {
            AbLoop { a: Some(a), b: None } => format!("Loop start: {}", format_hms(a)),
            AbLoop { a: Some(a), b: Some(b) } => format!("Looping {} - {}", format_hms(a), format_hms(b)),
            _ => "A-B loop off".into(),
        };
        self.show_osd(msg);
    }

    pub fn set_sleep_timer(&mut self, minutes: Option<u32>) {
        self.sleep_until = minutes.map(|m| Instant::now() + Duration::from_secs(m as u64 * 60));
    }

    // ---- tracks and sync -------------------------------------------------

    pub fn set_audio_track(&mut self, id: i32) {
        self.engine.send(EngineCommand::SetAudioTrack(id));
        if let Some(src) = self.playing_source.clone() {
            self.library.update_record(&src, |r| r.audio_track = Some(id));
        }
    }

    pub fn set_subtitle_track(&mut self, id: i32) {
        self.engine.send(EngineCommand::SetSubtitleTrack(id));
        if let Some(src) = self.playing_source.clone() {
            self.library.update_record(&src, |r| r.subtitle_track = Some(id));
        }
    }

    pub fn set_video_track(&mut self, id: i32) {
        self.engine.send(EngineCommand::SetVideoTrack(id));
    }

    /// Cycles through the track list (skipping "Disable" when `skip_off`).
    fn cycle(tracks: &[Track], cur: i32, reverse: bool, skip_off: bool) -> Option<&Track> {
        let list: Vec<&Track> = tracks.iter().filter(|t| !skip_off || t.id >= 0).collect();
        if list.is_empty() {
            return None;
        }
        let pos = list.iter().position(|t| t.id == cur);
        let n = list.len();
        let i = match (pos, reverse) {
            (Some(p), false) => (p + 1) % n,
            (Some(p), true) => (p + n - 1) % n,
            (None, _) => 0,
        };
        Some(list[i])
    }

    pub fn cycle_audio_track(&mut self) {
        if let Some(t) = Self::cycle(&self.status.audio_tracks, self.status.audio_track, false, true).cloned() {
            self.set_audio_track(t.id);
            self.show_osd(format!("Audio: {}", t.name));
        }
    }

    pub fn cycle_subtitle_track(&mut self, reverse: bool) {
        if let Some(t) = Self::cycle(&self.status.subtitle_tracks, self.status.subtitle_track, reverse, false).cloned()
        {
            self.set_subtitle_track(t.id);
            self.show_osd(format!("Subtitle: {}", t.name));
        }
    }

    pub fn cycle_video_track(&mut self) {
        if let Some(t) = Self::cycle(&self.status.video_tracks, self.status.video_track, false, false).cloned() {
            self.set_video_track(t.id);
            self.show_osd(format!("Video: {}", t.name));
        }
    }

    pub fn toggle_subtitles(&mut self) {
        if self.status.subtitle_track >= 0 {
            self.set_subtitle_track(-1);
            self.show_osd("Subtitles off");
        } else {
            self.cycle_subtitle_track(false);
        }
    }

    pub fn add_subtitle(&mut self, path: PathBuf) {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        self.engine.send(EngineCommand::AddSubtitle(path));
        self.toast(format!("Added subtitles: {name}"));
    }

    pub fn set_audio_delay(&mut self, us: i64) {
        self.engine.send(EngineCommand::SetAudioDelay(us));
        self.status.audio_delay_us = us;
        self.show_osd(format!("Audio delay: {} ms", us / 1000));
    }

    pub fn set_subtitle_delay(&mut self, us: i64) {
        self.engine.send(EngineCommand::SetSubtitleDelay(us));
        self.status.subtitle_delay_us = us;
        self.show_osd(format!("Subtitle delay: {} ms", us / 1000));
    }

    pub fn set_subtitle_timing(&mut self, t: SubtitleTiming) {
        self.subtitle_timing = t;
        self.engine.send(EngineCommand::SetSubtitleTiming(t));
    }

    /// VLC's subtitle sync: mark when you hear a line, mark when you see it,
    /// then apply the difference as subtitle delay.
    pub fn subtitle_sync_mark_audio(&mut self) {
        self.sync_marks.0 = Some(self.status.time_ms);
        self.show_osd("Sub sync: audio timestamp marked");
    }

    pub fn subtitle_sync_mark_subtitle(&mut self) {
        self.sync_marks.1 = Some(self.status.time_ms);
        self.show_osd("Sub sync: subtitle timestamp marked");
    }

    pub fn subtitle_sync_apply(&mut self) {
        if let (Some(a), Some(s)) = self.sync_marks {
            let us = self.status.subtitle_delay_us + (a - s) * 1000;
            self.set_subtitle_delay(us);
        } else {
            self.show_osd("Sub sync: mark both timestamps first");
        }
    }

    pub fn subtitle_sync_reset(&mut self) {
        self.sync_marks = (None, None);
        self.set_subtitle_delay(0);
    }

    pub fn set_stereo_mode(&mut self, m: StereoMode) {
        self.stereo_mode = m;
        self.engine.send(EngineCommand::SetStereoMode(m));
        self.show_osd(format!("Stereo mode: {}", m.label()));
    }

    pub fn set_audio_device(&mut self, id: Option<String>) {
        self.settings.audio.output_device = id.clone();
        self.mark_settings_dirty();
        self.engine.send(EngineCommand::SetAudioDevice(id));
    }

    pub fn cycle_audio_device(&mut self) {
        let devices = self.engine.audio_devices();
        if devices.is_empty() {
            return;
        }
        let cur = self.status.audio_device.clone().unwrap_or_default();
        let i = devices.iter().position(|d| d.id == cur).map(|p| (p + 1) % devices.len()).unwrap_or(0);
        let d = devices[i].clone();
        self.set_audio_device(Some(d.id));
        self.show_osd(format!("Audio device: {}", d.name));
    }

    // ---- chapters, titles, bookmarks -------------------------------------

    pub fn set_chapter(&mut self, c: i32) {
        self.engine.send(EngineCommand::SetChapter(c));
        if let Some(ch) = self.status.chapters.get(c.max(0) as usize) {
            self.show_osd(ch.name.clone());
        }
    }

    pub fn chapter_step(&mut self, forward: bool) {
        if self.status.chapters.is_empty() {
            return;
        }
        let c = (self.status.chapter + if forward { 1 } else { -1 }).clamp(0, self.status.chapters.len() as i32 - 1);
        self.set_chapter(c);
    }

    pub fn set_title(&mut self, t: i32) {
        self.engine.send(EngineCommand::SetTitle(t));
    }

    pub fn title_step(&mut self, forward: bool) {
        if self.status.titles.is_empty() {
            return;
        }
        let t = (self.status.title + if forward { 1 } else { -1 }).clamp(0, self.status.titles.len() as i32 - 1);
        self.set_title(t);
    }

    pub fn disc_menu(&mut self) {
        if let Some(i) = self.status.titles.iter().position(|t| t.is_menu) {
            self.set_title(i as i32);
        } else {
            self.engine.send(EngineCommand::Navigate(Nav::Popup));
        }
    }

    pub fn bookmarks(&self) -> Vec<Bookmark> {
        self.playing_source.as_ref().map(|s| self.library.record(s).bookmarks).unwrap_or_default()
    }

    pub fn add_bookmark(&mut self, name: Option<String>) {
        let Some(src) = self.playing_source.clone() else { return };
        let t = self.status.time_ms;
        let title = self.queue.current_item().map(|i| i.title()).unwrap_or_default();
        self.library.update_record(&src, |r| {
            let n = r.bookmarks.len() + 1;
            r.bookmarks.push(Bookmark { name: name.unwrap_or_else(|| format!("{title} {n}")), time_ms: t });
            r.bookmarks.sort_by_key(|b| b.time_ms);
        });
        self.show_osd(format!("Bookmark at {}", format_hms(t)));
    }

    pub fn remove_bookmark(&mut self, index: usize) {
        if let Some(src) = self.playing_source.clone() {
            self.library.update_record(&src, |r| {
                if index < r.bookmarks.len() {
                    r.bookmarks.remove(index);
                }
            });
        }
    }

    pub fn rename_bookmark(&mut self, index: usize, name: String) {
        if let Some(src) = self.playing_source.clone() {
            self.library.update_record(&src, |r| {
                if let Some(b) = r.bookmarks.get_mut(index) {
                    b.name = name;
                }
            });
        }
    }

    pub fn toggle_favorite_current(&mut self) -> Option<bool> {
        let src = self.playing_source.clone()?;
        let fav = self.library.toggle_favorite(&src);
        self.show_osd(if fav { "Added to favorites" } else { "Removed from favorites" });
        Some(fav)
    }

    pub fn current_source(&self) -> Option<&MediaSource> {
        self.playing_source.as_ref()
    }

    // ---- effects -----------------------------------------------------------

    pub fn set_audio_effects(&mut self, fx: AudioEffects) {
        if fx == self.settings.audio.effects {
            return;
        }
        self.engine.send(EngineCommand::SetAudioEffects(Box::new(fx.clone())));
        self.settings.audio.effects = fx;
        self.mark_settings_dirty();
    }

    pub fn set_video_effects(&mut self, fx: VideoEffects) {
        if fx == self.settings.video.effects {
            return;
        }
        self.engine.send(EngineCommand::SetVideoEffects(Box::new(fx.clone())));
        self.settings.video.effects = fx;
        self.mark_settings_dirty();
    }

    pub fn toggle_equalizer(&mut self) {
        let mut fx = self.settings.audio.effects.clone();
        fx.equalizer.enabled = !fx.equalizer.enabled;
        let on = fx.equalizer.enabled;
        self.set_audio_effects(fx);
        self.show_osd(if on { "Equalizer on" } else { "Equalizer off" });
    }

    pub fn set_deinterlace(&mut self, mode: crate::domain::effects::DeinterlaceMode, method: String) {
        self.settings.video.deinterlace = mode;
        self.settings.video.deinterlace_method = method.clone();
        self.mark_settings_dirty();
        self.engine.send(EngineCommand::SetDeinterlace(mode, method));
    }

    pub fn set_visualizer(&mut self, v: crate::domain::settings::Visualizer) {
        self.settings.audio.visualizer = v;
        self.mark_settings_dirty();
        self.engine.send(EngineCommand::SetVisualizer(v));
    }

    // ---- geometry (applied by the renderer) ------------------------------

    pub fn cycle_aspect(&mut self) {
        use crate::domain::effects::ASPECT_RATIOS;
        let mut list: Vec<Option<String>> = vec![None];
        list.extend(ASPECT_RATIOS.iter().map(|s| Some(s.to_string())));
        list.extend(
            self.settings
                .video
                .custom_aspect_ratios
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| Some(s.to_string())),
        );
        let i = list.iter().position(|a| *a == self.geometry.aspect).map(|p| (p + 1) % list.len()).unwrap_or(0);
        self.geometry.aspect = list[i].clone();
        self.show_osd(format!("Aspect ratio: {}", self.geometry.aspect.as_deref().unwrap_or("Default")));
    }

    pub fn cycle_crop(&mut self) {
        use crate::domain::effects::CROP_RATIOS;
        let mut list: Vec<Option<String>> = vec![None];
        list.extend(CROP_RATIOS.iter().map(|s| Some(s.to_string())));
        list.extend(
            self.settings
                .video
                .custom_crop_ratios
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| Some(s.to_string())),
        );
        let i = list.iter().position(|a| *a == self.geometry.crop).map(|p| (p + 1) % list.len()).unwrap_or(0);
        self.geometry.crop = list[i].clone();
        self.show_osd(format!("Crop: {}", self.geometry.crop.as_deref().unwrap_or("Default")));
    }

    pub fn zoom_step(&mut self, inp: bool) {
        use crate::domain::effects::ZOOM_LEVELS;
        let cur = self.geometry.zoom;
        let i = ZOOM_LEVELS.iter().position(|(_, z)| (*z - cur).abs() < 0.001).unwrap_or(0);
        let n = ZOOM_LEVELS.len();
        let j = if inp { (i + 1).min(n - 1) } else { i.saturating_sub(1) };
        self.geometry.zoom = ZOOM_LEVELS[j].1;
        self.show_osd(format!("Zoom: {}", ZOOM_LEVELS[j].0));
    }

    pub fn crop_pixels(&mut self, edge: char, more: bool) {
        let mut fx = self.settings.video.effects.clone();
        let c = &mut fx.crop;
        let v = match edge {
            't' => &mut c.top,
            'b' => &mut c.bottom,
            'l' => &mut c.left,
            _ => &mut c.right,
        };
        *v = if more { *v + 1 } else { v.saturating_sub(1) };
        self.set_video_effects(fx);
    }

    // ---- recording, snapshots, renderers ---------------------------------

    pub fn toggle_record(&mut self) {
        let on = !self.status.recording;
        self.engine.send(EngineCommand::SetRecording(on));
        self.show_osd(if on { "Recording" } else { "Recording stopped" });
    }

    /// Saves the current picture with the snapshot settings. Returns the file.
    pub fn snapshot(&mut self) -> Option<PathBuf> {
        let Some(frame) = self.engine.video().snapshot() else {
            self.toast_error("There is no picture to capture.");
            return None;
        };
        let v = &self.settings.video;
        let dir = if v.snapshot_dir.is_empty() {
            directories::UserDirs::new()
                .and_then(|u| u.picture_dir().map(Path::to_path_buf))
                .unwrap_or_else(std::env::temp_dir)
        } else {
            PathBuf::from(expand_home(&v.snapshot_dir))
        };
        let _ = std::fs::create_dir_all(&dir);
        let ext = v.snapshot_format.ext();
        let path = if v.snapshot_sequential {
            (1..100_000)
                .map(|n| dir.join(format!("{}{n:05}.{ext}", v.snapshot_prefix)))
                .find(|p| !p.exists())
                .unwrap_or_else(|| dir.join(format!("{}.{ext}", v.snapshot_prefix)))
        } else {
            let stamp = chrono_like_stamp();
            dir.join(format!("{}{stamp}.{ext}", v.snapshot_prefix))
        };
        let mut rgba = Vec::with_capacity((frame.width * frame.height * 4) as usize);
        for row in 0..frame.height as usize {
            let start = row * frame.pitch as usize;
            rgba.extend_from_slice(&frame.pixels[start..start + frame.width as usize * 4]);
        }
        let img = image::RgbaImage::from_raw(frame.width, frame.height, rgba)?;
        let mut img = image::DynamicImage::ImageRgba8(img);
        if v.snapshot_size == crate::domain::settings::SnapshotSize::Half {
            img = img.resize(frame.width / 2, frame.height / 2, image::imageops::FilterType::Triangle);
        }
        match img.to_rgb8().save(&path) {
            Ok(()) => {
                self.toast(format!("Snapshot saved: {}", path.display()));
                Some(path)
            }
            Err(e) => {
                self.toast_error(format!("Could not save the snapshot: {e}"));
                None
            }
        }
    }

    pub fn set_renderer(&mut self, id: Option<u64>) {
        self.engine.send(EngineCommand::SetRenderer(id));
        let name = id.and_then(|i| self.engine.renderers().into_iter().find(|r| r.id == i)).map(|r| r.name);
        self.toast(match name {
            Some(n) => format!("Casting to {n}"),
            None => "Playing locally".into(),
        });
    }

    pub fn start_job(&mut self, label: String, job: Job) -> u64 {
        let id = self.engine.start_job(job);
        self.jobs.push(JobState { id, label, progress: 0.0, finished: None });
        id
    }

    // ---- dialogs from VLC ------------------------------------------------

    pub fn reply_dialog(&mut self, id: u64, reply: DialogReply) {
        self.engine.reply_dialog(id, reply);
        self.dialogs.retain(|d| dialog_id(d) != Some(id));
    }

    pub fn dismiss_error(&mut self) {
        if matches!(self.dialogs.front(), Some(DialogRequest::Error { .. })) {
            self.dialogs.pop_front();
        }
    }

    // ---- playlists -------------------------------------------------------

    pub fn import_playlist(&mut self, path: &Path) -> Option<u64> {
        let fmt = path.extension().and_then(|e| e.to_str()).and_then(PlaylistFormat::from_extension)?;
        let text = std::fs::read_to_string(path).ok()?;
        let items = playlist_format::parse(&text, fmt, path.parent());
        let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Imported".into());
        Some(self.library.create_playlist(&name, items))
    }

    pub fn export_items(&mut self, name: &str, items: &[MediaItem], path: &Path) {
        let fmt = path
            .extension()
            .and_then(|e| e.to_str())
            .and_then(PlaylistFormat::from_extension)
            .unwrap_or(PlaylistFormat::Xspf);
        let text = playlist_format::write(name, items, fmt);
        match std::fs::write(path, text) {
            Ok(()) => self.toast(format!("Saved {}", path.display())),
            Err(e) => self.toast_error(format!("Could not save the playlist: {e}")),
        }
    }

    // ---- settings --------------------------------------------------------

    pub fn mark_settings_dirty(&mut self) {
        self.settings_dirty.get_or_insert_with(Instant::now);
    }

    /// Applies a settings change: persists it and rebuilds the engine when an
    /// option that libvlc only reads at start-up changed.
    pub fn update_settings(&mut self, f: impl FnOnce(&mut Settings)) {
        let before = self.settings.clone();
        f(&mut self.settings);
        if self.settings == before {
            return;
        }
        self.queue.repeat = self.settings.playback.repeat;
        if self.queue.shuffle != self.settings.playback.shuffle {
            self.queue.set_shuffle(self.settings.playback.shuffle);
        }
        if self.settings.audio.effects != before.audio.effects {
            self.engine.send(EngineCommand::SetAudioEffects(Box::new(self.settings.audio.effects.clone())));
        }
        if self.settings.video.effects != before.video.effects {
            self.engine.send(EngineCommand::SetVideoEffects(Box::new(self.settings.video.effects.clone())));
        }
        if (self.settings.video.deinterlace, &self.settings.video.deinterlace_method)
            != (before.video.deinterlace, &before.video.deinterlace_method)
        {
            self.engine.send(EngineCommand::SetDeinterlace(
                self.settings.video.deinterlace,
                self.settings.video.deinterlace_method.clone(),
            ));
        }
        if self.settings.audio.visualizer != before.audio.visualizer {
            self.engine.send(EngineCommand::SetVisualizer(self.settings.audio.visualizer));
        }
        if self.settings.video.aspect_ratio != before.video.aspect_ratio {
            self.geometry.aspect = self.settings.video.aspect_ratio.clone();
        }
        if self.settings.video.crop != before.video.crop {
            self.geometry.crop = self.settings.video.crop.clone();
        }
        if self.settings.video.zoom != before.video.zoom {
            self.geometry.zoom = self.settings.video.zoom;
        }
        self.engine.send(EngineCommand::Reconfigure(Box::new(self.settings.clone())));
        self.mark_settings_dirty();
    }

    pub fn save_settings_now(&mut self) {
        self.settings_dirty = None;
        match serde_json::to_string_pretty(&self.settings) {
            Ok(json) => {
                if let Err(e) = self.store.save("settings", &json) {
                    self.toast_error(format!("Could not save settings: {e:#}"));
                }
            }
            Err(e) => log::error!("serialising settings: {e}"),
        }
    }

    pub fn reset_settings(&mut self) {
        let keep_onboarding = self.settings.general.onboarding_done;
        self.update_settings(|s| {
            *s = Settings::default();
            s.general.onboarding_done = keep_onboarding;
        });
        self.toast("All settings were reset");
    }

    pub fn data_dir(&self) -> PathBuf {
        self.store.data_dir()
    }

    pub fn clear_history(&mut self) {
        self.library.clear_history();
        self.library.save();
        self.toast("History cleared");
    }

    /// Persists everything; call before exit.
    pub fn shutdown(&mut self) {
        self.save_position();
        self.library.save();
        self.save_queue();
        self.save_settings_now();
        self.engine.send(EngineCommand::Stop);
    }

    /// Known metadata for display (queue items first, then the library).
    pub fn meta_for(&self, source: &MediaSource) -> Option<MediaMeta> {
        self.queue
            .items
            .iter()
            .find(|i| i.source == *source)
            .map(|i| i.meta.clone())
            .or_else(|| self.library.meta(source).cloned())
    }
}

fn dialog_id(d: &DialogRequest) -> Option<u64> {
    match d {
        DialogRequest::Login { id, .. }
        | DialogRequest::Question { id, .. }
        | DialogRequest::Progress { id, .. }
        | DialogRequest::UpdateProgress { id, .. }
        | DialogRequest::Cancel { id } => Some(*id),
        DialogRequest::Error { .. } => None,
    }
}

pub fn expand_home(p: &str) -> String {
    if let Some(rest) = p.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return Path::new(&home).join(rest).to_string_lossy().into_owned();
    }
    p.to_string()
}

fn chrono_like_stamp() -> String {
    let secs = library::now();
    let (d, s) = (secs / 86_400, secs % 86_400);
    format!("{d}-{:02}h{:02}m{:02}s", s / 3600, (s / 60) % 60, s % 60)
}

#[cfg(test)]
mod tests;

/// Sleep prevention and track notifications, driven from `tick`.
pub mod system {
    use std::sync::Arc;

    use super::App;
    use super::ports::{PlayerState, SystemHooks};
    use crate::domain::settings::TrackPopup;

    #[derive(Default)]
    pub struct SystemState {
        pub hooks: Option<Arc<dyn SystemHooks>>,
        inhibiting: bool,
        announced: Option<String>,
        /// Set by the UI from the window state.
        pub window_minimized: bool,
    }

    impl App {
        pub fn set_system_hooks(&mut self, hooks: Arc<dyn SystemHooks>) {
            self.system.hooks = Some(hooks);
        }

        pub(super) fn tick_system(&mut self) {
            let Some(hooks) = self.system.hooks.clone() else { return };
            let want = self.settings.video.stop_screensaver
                && self.status.state == PlayerState::Playing
                && self.status.has_video;
            if want != self.system.inhibiting {
                hooks.inhibit_sleep(want);
                self.system.inhibiting = want;
            }
            let current = self.queue.current_item().map(|i| i.source.mrl());
            if self.status.state == PlayerState::Playing && current != self.system.announced {
                self.system.announced = current;
                let show = match self.settings.interface.track_popup {
                    TrackPopup::Never => false,
                    TrackPopup::WhenMinimized => self.system.window_minimized,
                    TrackPopup::Always => true,
                };
                if show && let Some(item) = self.queue.current_item() {
                    let body = item.meta.artist.clone().unwrap_or_default();
                    hooks.notify(&item.title(), &body);
                }
            }
        }
    }
}
