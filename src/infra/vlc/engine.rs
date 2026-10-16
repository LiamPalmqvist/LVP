//! The playback engine: one libvlc media player driven from a dedicated
//! thread. The UI talks to it through `MediaEngine` commands and reads a
//! cached `EngineStatus`, so no UI frame ever waits on libvlc.

use std::ffi::{c_char, c_uint, c_void};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, select, unbounded};
use parking_lot::RwLock;

use super::core::{Instance, Media, Objects, Var, api, cstring, owned, track_list, version};
use super::dialogs::DialogBridge;
use super::ffi::{self, Api, event, state};
use super::helper::Helper;
use super::{args, filters};
use crate::app::ports::*;
use crate::app::video::VideoOutput;
use crate::domain::effects::{AudioEffects, DeinterlaceMode, StereoMode, VideoEffects};
use crate::domain::media::{MediaSource, path_to_file_url};
use crate::domain::settings::{HwDecoding, PostProcessing, Settings, Visualizer};

/// Largest picture we ask VLC to render into; bigger sources are scaled.
const MAX_PIXELS: u64 = 3840 * 2160;

enum Raw {
    Player(i32, u64),
    MediaMeta,
}

struct Session {
    api: Arc<Api>,
    inst: Instance,
    mp: *mut ffi::MediaPlayer,
    raw_tx: Box<Sender<Raw>>,
    video: *const VideoOutput,
    args: Vec<String>,
}

unsafe impl Send for Session {}
unsafe impl Sync for Session {}

const PLAYER_EVENTS: &[i32] = &[
    event::MP_OPENING,
    event::MP_BUFFERING,
    event::MP_PLAYING,
    event::MP_PAUSED,
    event::MP_STOPPED,
    event::MP_END_REACHED,
    event::MP_ENCOUNTERED_ERROR,
    event::MP_LENGTH_CHANGED,
    event::MP_VOUT,
    event::MP_ES_ADDED,
    event::MP_ES_DELETED,
    event::MP_ES_SELECTED,
    event::MP_TITLE_CHANGED,
    event::MP_CHAPTER_CHANGED,
    event::MP_SEEKABLE_CHANGED,
    event::MP_PAUSABLE_CHANGED,
    event::MP_SCRAMBLED_CHANGED,
    event::MP_MEDIA_CHANGED,
];

impl Session {
    fn new(
        api: Arc<Api>,
        args: Vec<String>,
        log: Option<std::path::PathBuf>,
        video: &Arc<VideoOutput>,
        raw_tx: Sender<Raw>,
        dialogs: &Arc<DialogBridge>,
    ) -> Result<Self> {
        let mut inst = Instance::new(api.clone(), &args)?;
        inst.log_to(log.as_deref());
        dialogs.attach(inst.ptr);
        let mp = unsafe { (api.libvlc_media_player_new)(inst.ptr) };
        if mp.is_null() {
            anyhow::bail!("could not create the media player");
        }
        let video_ptr = Arc::into_raw(video.clone());
        let raw_tx = Box::new(raw_tx);
        unsafe {
            (api.libvlc_video_set_callbacks)(
                mp,
                Some(lock_cb),
                Some(unlock_cb),
                Some(display_cb),
                video_ptr as *mut c_void,
            );
            (api.libvlc_video_set_format_callbacks)(mp, Some(format_cb), Some(cleanup_cb));
            let em = (api.libvlc_media_player_event_manager)(mp);
            let data = &*raw_tx as *const Sender<Raw> as *mut c_void;
            for &k in PLAYER_EVENTS {
                (api.libvlc_event_attach)(em, k, player_event_cb, data);
            }
        }
        Ok(Self { api, inst, mp, raw_tx, video: video_ptr, args })
    }

    fn objects(&self) -> Objects<'_> {
        Objects { api: &self.api }
    }

    fn media(&self) -> Option<Media> {
        let m = unsafe { (self.api.libvlc_media_player_get_media)(self.mp) };
        (!m.is_null()).then(|| unsafe { Media::from_owned(self.api.clone(), m) })
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            let em = (self.api.libvlc_media_player_event_manager)(self.mp);
            let data = &*self.raw_tx as *const Sender<Raw> as *mut c_void;
            for &k in PLAYER_EVENTS {
                (self.api.libvlc_event_detach)(em, k, player_event_cb, data);
            }
            (self.api.libvlc_media_player_stop)(self.mp);
            (self.api.libvlc_media_player_release)(self.mp);
            drop(Arc::from_raw(self.video));
        }
    }
}

unsafe extern "C" fn player_event_cb(ev: *const ffi::Event, data: *mut c_void) {
    let (Some(ev), Some(tx)) = (unsafe { ev.as_ref() }, unsafe { (data as *const Sender<Raw>).as_ref() }) else {
        return;
    };
    let _ = tx.send(Raw::Player(ev.kind, ev.u[0]));
}

unsafe extern "C" fn media_event_cb(ev: *const ffi::Event, data: *mut c_void) {
    let (Some(ev), Some(tx)) = (unsafe { ev.as_ref() }, unsafe { (data as *const Sender<Raw>).as_ref() }) else {
        return;
    };
    if ev.kind == event::MEDIA_META_CHANGED {
        let _ = tx.send(Raw::MediaMeta);
    }
}

unsafe extern "C" fn format_cb(
    opaque: *mut *mut c_void,
    chroma: *mut c_char,
    width: *mut c_uint,
    height: *mut c_uint,
    pitches: *mut c_uint,
    lines: *mut c_uint,
) -> c_uint {
    unsafe {
        let video = &*(*opaque as *const VideoOutput);
        let (mut w, mut h) = (*width, *height);
        if w == 0 || h == 0 {
            return 0;
        }
        let px = w as u64 * h as u64;
        if px > MAX_PIXELS {
            let k = (MAX_PIXELS as f64 / px as f64).sqrt();
            w = ((w as f64 * k) as u32).max(2) & !1;
            h = ((h as f64 * k) as u32).max(2) & !1;
        }
        std::ptr::copy_nonoverlapping(b"RGBA".as_ptr() as *const c_char, chroma, 4);
        *width = w;
        *height = h;
        *pitches = video.configure(w, h);
        *lines = h;
        1
    }
}

unsafe extern "C" fn cleanup_cb(opaque: *mut c_void) {
    if let Some(v) = unsafe { (opaque as *const VideoOutput).as_ref() } {
        v.clear();
    }
}

unsafe extern "C" fn lock_cb(opaque: *mut c_void, planes: *mut *mut c_void) -> *mut c_void {
    unsafe {
        let video = &*(opaque as *const VideoOutput);
        *planes = video.back_ptr().cast();
    }
    std::ptr::null_mut()
}

unsafe extern "C" fn unlock_cb(_opaque: *mut c_void, _picture: *mut c_void, _planes: *const *mut c_void) {}

unsafe extern "C" fn display_cb(opaque: *mut c_void, _picture: *mut c_void) {
    if let Some(v) = unsafe { (opaque as *const VideoOutput).as_ref() } {
        v.publish();
    }
}

pub struct VlcEngine {
    api: Arc<Api>,
    cmd_tx: Sender<EngineCommand>,
    status: Arc<RwLock<EngineStatus>>,
    events: Receiver<EngineEvent>,
    video: Arc<VideoOutput>,
    session: Arc<RwLock<Arc<Session>>>,
    helper: Arc<Helper>,
    dialogs: Arc<DialogBridge>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl VlcEngine {
    pub fn new(settings: &Settings) -> Result<Self> {
        let api = api()?;
        let (ev_tx, ev_rx) = unbounded();
        let (raw_tx, raw_rx) = unbounded();
        let (cmd_tx, cmd_rx) = unbounded();
        let video = Arc::new(VideoOutput::new());
        let dialogs = DialogBridge::new(api.clone(), ev_tx.clone());
        let session =
            Session::new(api.clone(), args::playback(settings), log_path(settings), &video, raw_tx.clone(), &dialogs)?;
        let helper = Arc::new(Helper::new(api.clone(), &args::helper(settings), ev_tx.clone())?);
        dialogs.attach(helper.inst.ptr);
        if settings.recording.discover_cast {
            helper.start_renderer_discovery();
        }
        let session = Arc::new(RwLock::new(Arc::new(session)));
        let status = Arc::new(RwLock::new(EngineStatus {
            volume: settings.audio.volume,
            rate: 1.0,
            live_effects: api.core.is_some(),
            ..Default::default()
        }));
        let mut worker = Worker {
            api: api.clone(),
            session: session.clone(),
            status: status.clone(),
            out: ev_tx,
            raw_tx,
            raw_rx,
            video: video.clone(),
            dialogs: dialogs.clone(),
            helper: helper.clone(),
            current: None,
            audio_fx: settings.audio.effects.clone(),
            video_fx: settings.video.effects.clone(),
            deinterlace: (settings.video.deinterlace, settings.video.deinterlace_method.clone()),
            stereo: StereoMode::Stereo,
            visualizer: settings.audio.visualizer,
            sub_timing: SubtitleTiming {
                speed: settings.subtitles.speed,
                duration_factor: settings.subtitles.duration_factor,
            },
            settings: settings.clone(),
            applied_audio_chain: None,
            applied_video_chain: None,
            applied_sub_chain: None,
            tracks_dirty: true,
            meta_dirty: false,
            last_slow_poll: Instant::now(),
            media_events: None,
            software_decoding: false,
            restore: Restore::default(),
            wanted_audio: None,
            wanted_subtitle: None,
            subtitle_scale: 100,
        };
        worker.apply_audio_effects();
        worker.apply_video_effects();
        let thread = std::thread::Builder::new().name("vlc-engine".into()).spawn(move || worker.run(cmd_rx))?;
        Ok(Self { api, cmd_tx, status, events: ev_rx, video, session, helper, dialogs, thread: Some(thread) })
    }

    pub fn helper(&self) -> Arc<Helper> {
        self.helper.clone()
    }
}

impl Drop for VlcEngine {
    fn drop(&mut self) {
        let (tx, _) = unbounded();
        // Closing the command channel ends the worker loop.
        self.cmd_tx = tx;
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl MediaEngine for VlcEngine {
    fn send(&self, cmd: EngineCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    fn status(&self) -> EngineStatus {
        self.status.read().clone()
    }

    fn poll_events(&self) -> Vec<EngineEvent> {
        self.events.try_iter().collect()
    }

    fn video(&self) -> Arc<VideoOutput> {
        self.video.clone()
    }

    fn version(&self) -> String {
        version(&self.api)
    }

    fn audio_devices(&self) -> Vec<AudioDevice> {
        let s = self.session.read().clone();
        let mut out = Vec::new();
        unsafe {
            let head = (s.api.libvlc_audio_output_device_enum)(s.mp);
            let mut p = head;
            while !p.is_null() {
                let d = &*p;
                out.push(AudioDevice {
                    id: super::core::borrowed(d.device).unwrap_or_default(),
                    name: super::core::borrowed(d.description).unwrap_or_default(),
                });
                p = d.next;
            }
            if !head.is_null() {
                (s.api.libvlc_audio_output_device_list_release)(head);
            }
        }
        out
    }

    fn renderers(&self) -> Vec<RendererInfo> {
        self.helper.renderers()
    }

    fn reply_dialog(&self, id: u64, reply: DialogReply) {
        self.dialogs.reply(id, reply);
    }

    fn services(&self) -> Vec<ServiceInfo> {
        self.helper.services()
    }

    fn service_items(&self, name: &str) -> Vec<BrowseItem> {
        self.helper.service_items(name)
    }

    fn browse(&self, source: &MediaSource) -> Result<Vec<BrowseItem>> {
        self.helper.browse(source)
    }

    fn start_job(&self, job: Job) -> u64 {
        self.helper.start_job(job)
    }

    fn cancel_job(&self, id: u64) {
        self.helper.cancel_job(id)
    }
}

struct Current {
    request: OpenRequest,
    _media: Media,
}

struct Worker {
    api: Arc<Api>,
    session: Arc<RwLock<Arc<Session>>>,
    status: Arc<RwLock<EngineStatus>>,
    out: Sender<EngineEvent>,
    raw_tx: Sender<Raw>,
    raw_rx: Receiver<Raw>,
    video: Arc<VideoOutput>,
    dialogs: Arc<DialogBridge>,
    helper: Arc<Helper>,
    current: Option<Current>,
    audio_fx: AudioEffects,
    video_fx: VideoEffects,
    deinterlace: (DeinterlaceMode, String),
    stereo: StereoMode,
    visualizer: Visualizer,
    sub_timing: SubtitleTiming,
    settings: Settings,
    applied_audio_chain: Option<String>,
    applied_video_chain: Option<String>,
    applied_sub_chain: Option<String>,
    tracks_dirty: bool,
    meta_dirty: bool,
    last_slow_poll: Instant,
    /// Media whose meta events we listen to, and the boxed sender they use.
    media_events: Option<(Media, Box<Sender<Raw>>)>,
    /// Whether the current item was opened with software decoding.
    software_decoding: bool,
    /// Per-file choices to re-apply once the item is playing.
    restore: Restore,
    /// Track choices still waiting for their tracks to exist (a choice
    /// made while a file is opening or restarting).
    wanted_audio: Option<i32>,
    wanted_subtitle: Option<i32>,
    /// `sub-text-scale` in percent.
    subtitle_scale: i32,
}

#[derive(Default)]
struct Restore {
    subtitles_off: bool,
    audio_delay_us: i64,
    subtitle_delay_us: i64,
    delays_applied: bool,
}

impl Worker {
    fn s(&self) -> Arc<Session> {
        self.session.read().clone()
    }

    fn emit(&self, e: EngineEvent) {
        let _ = self.out.send(e);
    }

    fn run(&mut self, cmd_rx: Receiver<EngineCommand>) {
        loop {
            select! {
                recv(cmd_rx) -> cmd => match cmd {
                    Ok(c) => self.handle(c),
                    Err(_) => break,
                },
                recv(self.raw_rx) -> raw => if let Ok(r) = raw { self.on_raw(r) },
                default(Duration::from_millis(50)) => {}
            }
            while let Ok(r) = self.raw_rx.try_recv() {
                self.on_raw(r);
            }
            self.refresh();
        }
        self.detach_media_events();
        let s = self.s();
        unsafe { (s.api.libvlc_media_player_stop)(s.mp) };
    }

    fn handle(&mut self, cmd: EngineCommand) {
        let s = self.s();
        let mp = s.mp;
        let api = &self.api;
        unsafe {
            match cmd {
                EngineCommand::Open(req) => self.open(*req),
                EngineCommand::Play => {
                    let st = (api.libvlc_media_player_get_state)(mp);
                    if st == state::PAUSED {
                        (api.libvlc_media_player_set_pause)(mp, 0);
                    } else if st != state::PLAYING {
                        if st == state::ENDED || st == state::STOPPED || st == state::ERROR {
                            self.video.clear();
                        }
                        (api.libvlc_media_player_play)(mp);
                    }
                }
                EngineCommand::Pause => (api.libvlc_media_player_set_pause)(mp, 1),
                EngineCommand::TogglePause => {
                    let st = (api.libvlc_media_player_get_state)(mp);
                    match st {
                        state::PLAYING | state::BUFFERING | state::OPENING => (api.libvlc_media_player_pause)(mp),
                        state::PAUSED => (api.libvlc_media_player_set_pause)(mp, 0),
                        _ => {
                            (api.libvlc_media_player_play)(mp);
                        }
                    }
                }
                EngineCommand::Stop => {
                    (api.libvlc_media_player_stop)(mp);
                    self.video.clear();
                    let mut st = self.status.write();
                    st.state = PlayerState::Stopped;
                    st.time_ms = 0;
                    st.position = 0.0;
                }
                EngineCommand::SeekMs { ms, .. } => {
                    (api.libvlc_media_player_set_time)(mp, ms.max(0));
                    let mut st = self.status.write();
                    st.time_ms = ms.max(0);
                    if st.length_ms > 0 {
                        st.position = (ms as f32 / st.length_ms as f32).clamp(0.0, 1.0);
                    }
                }
                EngineCommand::SeekPosition(p) => {
                    (api.libvlc_media_player_set_position)(mp, p.clamp(0.0, 1.0));
                    let mut st = self.status.write();
                    st.position = p.clamp(0.0, 1.0);
                    st.time_ms = (st.length_ms as f32 * st.position) as i64;
                }
                EngineCommand::SetRate(r) => {
                    (api.libvlc_media_player_set_rate)(mp, r.clamp(0.03, 32.0));
                }
                EngineCommand::NextFrame => (api.libvlc_media_player_next_frame)(mp),
                EngineCommand::SetVolume(v) => {
                    (api.libvlc_audio_set_volume)(mp, v.clamp(0, 200));
                    self.status.write().volume = v.clamp(0, 200);
                }
                EngineCommand::SetMute(m) => {
                    (api.libvlc_audio_set_mute)(mp, m as i32);
                    self.status.write().muted = m;
                }
                EngineCommand::SetAudioTrack(id) => {
                    (api.libvlc_audio_set_track)(mp, id);
                    self.wanted_audio = Some(id);
                    self.tracks_dirty = true;
                }
                EngineCommand::SetVideoTrack(id) => {
                    (api.libvlc_video_set_track)(mp, id);
                    self.tracks_dirty = true;
                }
                EngineCommand::SetSubtitleTrack(id) => {
                    (api.libvlc_video_set_spu)(mp, id);
                    self.wanted_subtitle = Some(id);
                    self.restore.subtitles_off = false;
                    self.tracks_dirty = true;
                }
                EngineCommand::AddSubtitle(path) => {
                    // Kept on the request so a restart (software decoding) keeps it.
                    if let Some(cur) = self.current.as_mut()
                        && !cur.request.subtitles.contains(&path)
                    {
                        cur.request.subtitles.push(path.clone());
                    }
                    let uri = cstring(&path_to_file_url(&path));
                    (api.libvlc_media_player_add_slave)(mp, ffi::slave_type::SUBTITLE, uri.as_ptr(), true);
                    self.tracks_dirty = true;
                }
                EngineCommand::AddAudioTrack(path) => {
                    let uri = cstring(&path_to_file_url(&path));
                    (api.libvlc_media_player_add_slave)(mp, ffi::slave_type::AUDIO, uri.as_ptr(), true);
                    self.tracks_dirty = true;
                }
                EngineCommand::SetAudioDelay(us) => {
                    (api.libvlc_audio_set_delay)(mp, us);
                    self.status.write().audio_delay_us = us;
                }
                EngineCommand::SetSubtitleDelay(us) => {
                    (api.libvlc_video_set_spu_delay)(mp, us);
                    self.status.write().subtitle_delay_us = us;
                }
                EngineCommand::SetSubtitleScale(pct) => {
                    self.subtitle_scale = pct.clamp(10, 500);
                    self.apply_subtitle_scale();
                }
                EngineCommand::SetSubtitleTiming(t) => {
                    self.sub_timing = t;
                    self.apply_sub_timing();
                }
                EngineCommand::SetChapter(c) => (api.libvlc_media_player_set_chapter)(mp, c),
                EngineCommand::SetTitle(t) => {
                    (api.libvlc_media_player_set_title)(mp, t);
                    self.tracks_dirty = true;
                }
                EngineCommand::Navigate(n) => {
                    let mode = match n {
                        Nav::Activate => 0,
                        Nav::Up => 1,
                        Nav::Down => 2,
                        Nav::Left => 3,
                        Nav::Right => 4,
                        Nav::Popup => 5,
                    };
                    (api.libvlc_media_player_navigate)(mp, mode);
                }
                EngineCommand::SetAudioEffects(fx) => {
                    self.audio_fx = *fx;
                    self.apply_audio_effects();
                }
                EngineCommand::SetVideoEffects(fx) => {
                    self.video_fx = *fx;
                    self.apply_video_effects();
                }
                EngineCommand::SetDeinterlace(mode, method) => {
                    self.deinterlace = (mode, method);
                    self.apply_deinterlace();
                }
                EngineCommand::SetStereoMode(m) => {
                    let ch = match m {
                        StereoMode::Stereo => 1,
                        StereoMode::Reverse => 2,
                        StereoMode::Left => 3,
                        StereoMode::Right => 4,
                        StereoMode::DolbySurround => 5,
                    };
                    (api.libvlc_audio_set_channel)(mp, ch);
                    self.stereo = m;
                    self.status.write().stereo_mode = m;
                }
                EngineCommand::SetAudioDevice(dev) => {
                    let id = cstring(dev.as_deref().unwrap_or(""));
                    (api.libvlc_audio_output_device_set)(mp, std::ptr::null(), id.as_ptr());
                    self.status.write().audio_device = dev;
                }
                EngineCommand::SetRenderer(id) => self.set_renderer(id),
                EngineCommand::SetRecording(on) => {
                    let o = s.objects();
                    o.set(mp, "input-record-path", &Var::Str(expand_home(&self.settings.recording.record_dir)));
                    let mut done = false;
                    o.for_each(mp, "input", &mut |input| {
                        done |= o.set(input, "record", &Var::Bool(on));
                    });
                    self.status.write().recording = on && done;
                    if on && !done {
                        self.emit(EngineEvent::Error("Recording needs a playing item and libvlccore.".into()));
                    }
                }
                EngineCommand::SetTeletext(page) => (api.libvlc_video_set_teletext)(mp, page),
                EngineCommand::SetVisualizer(v) => {
                    self.visualizer = v;
                    self.apply_visualizer();
                }
                EngineCommand::Reconfigure(settings) => self.reconfigure(*settings),
            }
        }
    }

    fn open(&mut self, req: OpenRequest) {
        let s = self.s();
        self.detach_media_events();
        let media = match Media::new(&s.inst, &req.item.source) {
            Ok(m) => m,
            Err(e) => {
                self.emit(EngineEvent::Error(format!("{e:#}")));
                return;
            }
        };
        for o in &req.item.options {
            media.add_option(o);
        }
        if let Some(ms) = req.start_ms.filter(|m| *m > 0) {
            media.add_option(&format!(":start-time={:.3}", ms as f64 / 1000.0));
        }
        if req.paused {
            media.add_option(":start-paused");
        }
        // Remembered track choices take effect as the file opens.
        if let Some(t) = req.audio_track.filter(|t| *t >= 0) {
            media.add_option(&format!(":audio-track-id={t}"));
        }
        if let Some(t) = req.subtitle_track.filter(|t| *t >= 0) {
            media.add_option(&format!(":sub-track-id={t}"));
        }
        let mut soft = self.wants_software_decoding(&req);
        if !soft && self.settings.performance.hw_decoding == HwDecoding::Automatic && req.item.source.path().is_some() {
            // A quick local parse tells us whether the file has subtitle
            // tracks before decoding starts, so it never needs a restart.
            // Subtitle files VLC will load from beside the video count too.
            media.parse_blocking(ffi::parse::LOCAL, Duration::from_millis(1500));
            soft = media.tracks().iter().any(|t| t.kind == "Subtitle")
                || (self.settings.subtitles.autoload && req.item.source.path().is_some_and(has_sidecar_subtitles));
            log::debug!("opening {} with software decoding: {soft}", req.item.source.display_name());
        }
        if soft && self.settings.performance.hw_decoding != HwDecoding::Disabled {
            media.add_option(":codec=avcodec,all");
            media.add_option(":avcodec-hw=none");
        }
        self.software_decoding = soft;
        if self.current.as_ref().map(|c| &c.request.item.source) != Some(&req.item.source) {
            self.wanted_audio = None;
            self.wanted_subtitle = None;
        }
        self.restore = Restore {
            subtitles_off: req.subtitle_track == Some(-1),
            audio_delay_us: req.audio_delay_us,
            subtitle_delay_us: req.subtitle_delay_us,
            delays_applied: false,
        };
        for (i, sub) in req.subtitles.iter().enumerate() {
            if i == 0 {
                media.add_option(&format!(":sub-file={}", sub.display()));
            } else {
                let uri = cstring(&path_to_file_url(sub));
                unsafe { (self.api.libvlc_media_slaves_add)(media.ptr, ffi::slave_type::SUBTITLE, 4, uri.as_ptr()) };
            }
        }
        // Meta change notifications for streams ("now playing").
        let sender = Box::new(self.raw_tx.clone());
        unsafe {
            let em = (self.api.libvlc_media_event_manager)(media.ptr);
            (self.api.libvlc_event_attach)(
                em,
                event::MEDIA_META_CHANGED,
                media_event_cb,
                &*sender as *const _ as *mut c_void,
            );
        }
        self.media_events = Some((media.clone(), sender));
        self.video.clear();
        unsafe {
            (self.api.libvlc_media_player_set_media)(s.mp, media.ptr);
            (self.api.libvlc_media_player_play)(s.mp);
        }
        {
            let mut st = self.status.write();
            st.mrl = Some(req.item.source.mrl());
            st.meta = req.item.meta.clone();
            st.time_ms = req.start_ms.unwrap_or(0);
            st.length_ms = req.item.meta.duration_ms.unwrap_or(0);
            st.position = 0.0;
            st.state = PlayerState::Opening;
            st.recording = false;
            st.audio_tracks.clear();
            st.subtitle_tracks.clear();
            st.video_tracks.clear();
            st.chapters.clear();
            st.titles.clear();
            st.video_size = None;
            st.has_video = false;
        }
        self.current = Some(Current { request: req, _media: media });
        self.tracks_dirty = true;
        self.meta_dirty = true;
    }

    /// Software decoding when subtitles will be shown and the user left
    /// hardware decoding on Automatic (or turned it off).
    fn wants_software_decoding(&self, req: &OpenRequest) -> bool {
        match self.settings.performance.hw_decoding {
            HwDecoding::Disabled => true,
            HwDecoding::VideoToolbox => false,
            HwDecoding::Automatic => {
                req.expect_subtitles || !req.subtitles.is_empty() || req.subtitle_track.is_some_and(|t| t >= 0)
            }
        }
    }

    /// Re-applies remembered delays and "subtitles off" once tracks exist,
    /// and reopens a hardware-decoded item in software mode when a
    /// subtitle track becomes active, so its text can be drawn.
    fn after_tracks(&mut self) {
        let st = self.status.read().clone();
        if !st.state.is_active() {
            return;
        }
        let s = self.s();
        if !self.restore.delays_applied && st.state == PlayerState::Playing {
            self.restore.delays_applied = true;
            unsafe {
                if self.restore.audio_delay_us != 0 {
                    (self.api.libvlc_audio_set_delay)(s.mp, self.restore.audio_delay_us);
                }
                if self.restore.subtitle_delay_us != 0 {
                    (self.api.libvlc_video_set_spu_delay)(s.mp, self.restore.subtitle_delay_us);
                }
            }
        }
        // Choices made while the file was opening apply once their tracks exist.
        if let Some(w) = self.wanted_subtitle {
            if st.subtitle_track == w {
                self.wanted_subtitle = None;
            } else if st.subtitle_tracks.iter().any(|t| t.id == w) {
                unsafe { (self.api.libvlc_video_set_spu)(s.mp, w) };
                self.tracks_dirty = true;
                return;
            }
        }
        if let Some(w) = self.wanted_audio {
            if st.audio_track == w {
                self.wanted_audio = None;
            } else if st.audio_tracks.iter().any(|t| t.id == w) {
                unsafe { (self.api.libvlc_audio_set_track)(s.mp, w) };
                self.tracks_dirty = true;
                return;
            }
        }
        if self.restore.subtitles_off && st.subtitle_track >= 0 {
            self.restore.subtitles_off = false;
            unsafe { (self.api.libvlc_video_set_spu)(s.mp, -1) };
            self.tracks_dirty = true;
            return;
        }
        let automatic = self.settings.performance.hw_decoding == HwDecoding::Automatic;
        if automatic
            && !self.software_decoding
            && st.subtitle_track >= 0
            && st.has_video
            && let Some(cur) = self.current.take()
        {
            let mut req = cur.request;
            req.start_ms = Some(st.time_ms);
            req.paused = st.state == PlayerState::Paused;
            req.subtitle_track = Some(self.wanted_subtitle.unwrap_or(st.subtitle_track));
            req.audio_track = Some(self.wanted_audio.unwrap_or(st.audio_track));
            req.audio_delay_us = st.audio_delay_us;
            req.subtitle_delay_us = st.subtitle_delay_us;
            req.expect_subtitles = true;
            log::debug!("subtitles turned on: reopening in software decoding at {} ms", st.time_ms);
            unsafe { (self.api.libvlc_media_player_stop)(s.mp) };
            self.open(req);
        }
    }

    fn detach_media_events(&mut self) {
        if let Some((media, sender)) = self.media_events.take() {
            unsafe {
                let em = (self.api.libvlc_media_event_manager)(media.ptr);
                (self.api.libvlc_event_detach)(
                    em,
                    event::MEDIA_META_CHANGED,
                    media_event_cb,
                    &*sender as *const _ as *mut c_void,
                );
            }
        }
    }

    fn on_raw(&mut self, raw: Raw) {
        match raw {
            Raw::MediaMeta => self.meta_dirty = true,
            Raw::Player(kind, u) => {
                let new_state = match kind {
                    event::MP_OPENING => Some(PlayerState::Opening),
                    event::MP_PLAYING => Some(PlayerState::Playing),
                    event::MP_PAUSED => Some(PlayerState::Paused),
                    event::MP_STOPPED => Some(PlayerState::Stopped),
                    event::MP_END_REACHED => Some(PlayerState::Ended),
                    event::MP_ENCOUNTERED_ERROR => Some(PlayerState::Error),
                    _ => None,
                };
                if let Some(ns) = new_state {
                    self.status.write().state = ns;
                    self.emit(EngineEvent::StateChanged(ns));
                }
                match kind {
                    event::MP_PLAYING => {
                        self.tracks_dirty = true;
                        self.meta_dirty = true;
                        // A fresh audio output may have been created.
                        self.applied_audio_chain = None;
                        self.apply_audio_effects();
                        self.apply_sub_timing();
                    }
                    event::MP_END_REACHED => self.emit(EngineEvent::EndReached),
                    event::MP_ENCOUNTERED_ERROR => {
                        let name =
                            self.current.as_ref().map(|c| c.request.item.title()).unwrap_or_else(|| "the media".into());
                        self.emit(EngineEvent::Error(format!("VLC could not open {name}.")));
                    }
                    event::MP_BUFFERING => self.status.write().buffering = f32::from_bits(u as u32),
                    event::MP_LENGTH_CHANGED => {
                        let len = u as i64;
                        self.status.write().length_ms = len;
                        self.emit(EngineEvent::LengthChanged(len));
                    }
                    event::MP_VOUT => {
                        self.tracks_dirty = true;
                        self.applied_video_chain = None;
                        self.apply_video_effects();
                        self.apply_deinterlace();
                        self.apply_subtitle_scale();
                        self.emit(EngineEvent::VideoOutput(u as u32));
                    }
                    event::MP_ES_ADDED | event::MP_ES_DELETED | event::MP_ES_SELECTED => {
                        self.tracks_dirty = true;
                    }
                    event::MP_CHAPTER_CHANGED => {
                        let c = u as u32 as i32;
                        self.status.write().chapter = c;
                        self.emit(EngineEvent::ChapterChanged(c));
                    }
                    event::MP_TITLE_CHANGED => {
                        self.tracks_dirty = true;
                        let t = u as u32 as i32;
                        self.status.write().title = t;
                        self.emit(EngineEvent::TitleChanged(t));
                    }
                    event::MP_SCRAMBLED_CHANGED => self.status.write().scrambled = u as u32 != 0,
                    _ => {}
                }
            }
        }
    }

    /// Updates the cached status: cheap values every tick, track lists and
    /// metadata when something changed.
    fn refresh(&mut self) {
        let s = self.s();
        let mp = s.mp;
        let api = &self.api;
        unsafe {
            let raw_state = (api.libvlc_media_player_get_state)(mp);
            let time = (api.libvlc_media_player_get_time)(mp);
            let length = (api.libvlc_media_player_get_length)(mp);
            let position = (api.libvlc_media_player_get_position)(mp);
            let rate = (api.libvlc_media_player_get_rate)(mp);
            let volume = (api.libvlc_audio_get_volume)(mp);
            let muted = (api.libvlc_audio_get_mute)(mp);
            let seekable = (api.libvlc_media_player_is_seekable)(mp) != 0;
            let pausable = (api.libvlc_media_player_can_pause)(mp) != 0;
            let vouts = (api.libvlc_media_player_has_vout)(mp);
            let mut st = self.status.write();
            st.state = match raw_state {
                state::OPENING => PlayerState::Opening,
                state::BUFFERING => PlayerState::Buffering,
                state::PLAYING => PlayerState::Playing,
                state::PAUSED => PlayerState::Paused,
                state::STOPPED => PlayerState::Stopped,
                state::ENDED => PlayerState::Ended,
                state::ERROR => PlayerState::Error,
                _ if self.current.is_none() => PlayerState::Idle,
                _ => st.state,
            };
            if time >= 0 && st.state.is_active() {
                st.time_ms = time;
            }
            if length > 0 {
                st.length_ms = length;
            }
            if position >= 0.0 && st.state.is_active() {
                st.position = position;
            }
            st.rate = rate;
            if volume >= 0 {
                st.volume = volume;
            }
            if muted >= 0 {
                st.muted = muted != 0;
            }
            st.seekable = seekable;
            st.pausable = pausable;
            st.has_video = vouts > 0;
            st.video_size = self.video.size();
        }
        if self.tracks_dirty {
            self.tracks_dirty = false;
            self.refresh_tracks(&s);
            self.emit(EngineEvent::TracksChanged);
            self.after_tracks();
        } else if !self.restore.delays_applied {
            self.after_tracks();
        }
        let slow = self.last_slow_poll.elapsed() > Duration::from_secs(1);
        if self.meta_dirty || (slow && self.status.read().state == PlayerState::Playing) {
            self.last_slow_poll = Instant::now();
            let was_dirty = std::mem::take(&mut self.meta_dirty);
            self.refresh_meta(&s, was_dirty);
        }
    }

    fn refresh_tracks(&self, s: &Session) {
        let api = &self.api;
        let mp = s.mp;
        unsafe {
            let audio = track_list(api, (api.libvlc_audio_get_track_description)(mp));
            let video = track_list(api, (api.libvlc_video_get_track_description)(mp));
            let spu = track_list(api, (api.libvlc_video_get_spu_description)(mp));
            let to_tracks =
                |v: Vec<(i32, String)>| v.into_iter().map(|(id, name)| Track { id, name }).collect::<Vec<_>>();

            let mut chapters = Vec::new();
            let title = (api.libvlc_media_player_get_title)(mp);
            let mut list: *mut *mut ffi::ChapterDescription = std::ptr::null_mut();
            let n = (api.libvlc_media_player_get_full_chapter_descriptions)(mp, -1, &mut list);
            if n > 0 && !list.is_null() {
                for i in 0..n as usize {
                    let c = &**list.add(i);
                    chapters.push(Chapter {
                        name: super::core::borrowed(c.name).unwrap_or_else(|| format!("Chapter {}", i + 1)),
                        start_ms: c.time_offset,
                        duration_ms: c.duration,
                    });
                }
                (api.libvlc_chapter_descriptions_release)(list, n as u32);
            }
            let mut titles = Vec::new();
            let mut tlist: *mut *mut ffi::TitleDescription = std::ptr::null_mut();
            let tn = (api.libvlc_media_player_get_full_title_descriptions)(mp, &mut tlist);
            if tn > 0 && !tlist.is_null() {
                for i in 0..tn as usize {
                    let t = &**tlist.add(i);
                    titles.push(Title {
                        name: super::core::borrowed(t.name).unwrap_or_else(|| format!("Title {}", i + 1)),
                        duration_ms: t.duration,
                        is_menu: t.flags & 0x01 != 0,
                    });
                }
                (api.libvlc_title_descriptions_release)(tlist, tn as u32);
            }
            let video_track = (api.libvlc_video_get_track)(mp);
            let sar = s.media().and_then(|m| m.video_sar(video_track)).unwrap_or((1, 1));
            let mut st = self.status.write();
            st.audio_tracks = to_tracks(audio);
            st.video_tracks = to_tracks(video);
            st.subtitle_tracks = to_tracks(spu);
            st.audio_track = (api.libvlc_audio_get_track)(mp);
            st.video_track = video_track;
            st.subtitle_track = (api.libvlc_video_get_spu)(mp);
            st.chapters = chapters;
            st.chapter = (api.libvlc_media_player_get_chapter)(mp);
            st.titles = titles;
            st.title = title;
            st.audio_delay_us = (api.libvlc_audio_get_delay)(mp);
            st.subtitle_delay_us = (api.libvlc_video_get_spu_delay)(mp);
            st.video_sar = sar;
            st.teletext_page = (api.libvlc_video_get_teletext)(mp);
            st.audio_device = owned(api, (api.libvlc_audio_output_device_get)(mp));
        }
    }

    fn refresh_meta(&self, s: &Session, full: bool) {
        let Some(media) = s.media() else { return };
        if full {
            let mut meta = media.meta();
            if let Some(cur) = &self.current {
                // Keep what the library already knew when the demuxer has nothing better.
                let mut base = cur.request.item.meta.clone();
                if meta.title.as_deref() == Some(&cur.request.item.source.display_name()) {
                    meta.title = None;
                }
                base.merge(meta);
                meta = base;
            }
            self.status.write().meta = meta;
            self.emit(EngineEvent::MetaChanged);
        } else if let Some(np) = media.meta_field(ffi::meta::NOW_PLAYING) {
            let mut st = self.status.write();
            if st.meta.now_playing.as_deref() != Some(&np) {
                st.meta.now_playing = Some(np);
                drop(st);
                self.emit(EngineEvent::MetaChanged);
            }
        }
        if self.settings.general.collect_stats
            && let Some(x) = media.stats()
        {
            self.status.write().stats = PlaybackStats {
                input_bitrate_kbps: x.input_bitrate * 8000.0,
                demux_bitrate_kbps: x.demux_bitrate * 8000.0,
                read_bytes: x.read_bytes as i64,
                decoded_video: x.decoded_video as i64,
                decoded_audio: x.decoded_audio as i64,
                displayed_pictures: x.displayed_pictures as i64,
                lost_pictures: x.lost_pictures as i64,
                played_abuffers: x.played_abuffers as i64,
                lost_abuffers: x.lost_abuffers as i64,
                demux_corrupted: x.demux_corrupted as i64,
                demux_discontinuity: x.demux_discontinuity as i64,
            };
        }
    }

    fn apply_audio_effects(&mut self) {
        let s = self.s();
        let o = s.objects();
        if !o.available() {
            // Without libvlccore only the equalizer is reachable.
            unsafe {
                if self.audio_fx.equalizer.enabled {
                    let eq = (self.api.libvlc_audio_equalizer_new)();
                    (self.api.libvlc_audio_equalizer_set_preamp)(eq, self.audio_fx.equalizer.preamp);
                    for (i, b) in self.audio_fx.equalizer.bands.iter().enumerate() {
                        (self.api.libvlc_audio_equalizer_set_amp_at_index)(eq, *b, i as u32);
                    }
                    (self.api.libvlc_media_player_set_equalizer)(s.mp, eq);
                    (self.api.libvlc_audio_equalizer_release)(eq);
                } else {
                    (self.api.libvlc_media_player_set_equalizer)(s.mp, std::ptr::null_mut());
                }
            }
            return;
        }
        let (chain, vars) = filters::audio(&self.audio_fx);
        for (name, value) in &vars {
            o.set_everywhere(s.mp, "audio output", name, value);
        }
        if self.applied_audio_chain.as_deref() != Some(chain.as_str()) {
            o.set_everywhere(s.mp, "audio output", "audio-filter", &Var::Str(chain.clone()));
            self.applied_audio_chain = Some(chain);
        }
    }

    fn apply_video_effects(&mut self) {
        let s = self.s();
        let o = s.objects();
        let (mut chain, vars) = filters::video(&self.video_fx);
        if self.settings.performance.post_processing != PostProcessing::Off {
            chain = if chain.is_empty() { "postproc".into() } else { format!("postproc:{chain}") };
        }
        if !o.available() {
            // libvlc's own adjust API is the only live control left.
            let e = &self.video_fx.essential;
            unsafe {
                let on = (!e.is_neutral_adjust() || e.adjust_enabled) as i32;
                (self.api.libvlc_video_set_adjust_int)(s.mp, ffi::adjust::ENABLE, on);
                (self.api.libvlc_video_set_adjust_float)(s.mp, ffi::adjust::CONTRAST, e.contrast);
                (self.api.libvlc_video_set_adjust_float)(s.mp, ffi::adjust::BRIGHTNESS, e.brightness);
                (self.api.libvlc_video_set_adjust_float)(s.mp, ffi::adjust::HUE, e.hue);
                (self.api.libvlc_video_set_adjust_float)(s.mp, ffi::adjust::SATURATION, e.saturation);
                (self.api.libvlc_video_set_adjust_float)(s.mp, ffi::adjust::GAMMA, e.gamma);
            }
            return;
        }
        for (name, value) in &vars {
            o.set_everywhere(s.mp, "video output", name, value);
        }
        if self.applied_video_chain.as_deref() != Some(chain.as_str()) {
            o.set_everywhere(s.mp, "video output", "video-filter", &Var::Str(chain.clone()));
            self.applied_video_chain = Some(chain);
        }
        let (subs, sub_vars) = filters::sub_sources(&self.video_fx);
        for (name, value) in &sub_vars {
            o.set_everywhere(s.mp, "video output", name, value);
        }
        if self.applied_sub_chain.as_deref() != Some(subs.as_str()) {
            o.set_everywhere(s.mp, "video output", "sub-source", &Var::Str(subs.clone()));
            self.applied_sub_chain = Some(subs);
        }
    }

    /// Subtitle size, live: VLC scales subtitle text by `sub-text-scale`.
    fn apply_subtitle_scale(&self) {
        let s = self.s();
        s.objects().set_everywhere(s.mp, "video output", "sub-text-scale", &Var::Int(self.subtitle_scale as i64));
    }

    fn apply_deinterlace(&self) {
        let s = self.s();
        let o = s.objects();
        let (mode, method) = &self.deinterlace;
        if o.available() {
            let v = match mode {
                DeinterlaceMode::Off => 0,
                DeinterlaceMode::On => 1,
                DeinterlaceMode::Automatic => -1,
            };
            o.set_everywhere(s.mp, "video output", "deinterlace-mode", &Var::Str(method.clone()));
            o.set_everywhere(s.mp, "video output", "deinterlace", &Var::Int(v));
        } else {
            let m = (*mode != DeinterlaceMode::Off).then(|| cstring(if method == "auto" { "blend" } else { method }));
            unsafe {
                (self.api.libvlc_video_set_deinterlace)(s.mp, m.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()))
            };
        }
    }

    fn apply_sub_timing(&self) {
        let s = self.s();
        let o = s.objects();
        if !o.available() {
            return;
        }
        let t = self.sub_timing;
        o.for_each(s.mp, "input", &mut |input| {
            o.set(input, "sub-fps", &Var::Float(t.speed));
        });
        if t.duration_factor > 0.0 {
            o.set_everywhere(s.mp, "video output", "subsdelay-mode", &Var::Int(1));
            o.set_everywhere(s.mp, "video output", "subsdelay-factor", &Var::Float(t.duration_factor));
            o.set_everywhere(s.mp, "video output", "sub-filter", &Var::Str("subsdelay".into()));
        } else {
            o.set_everywhere(s.mp, "video output", "sub-filter", &Var::Str(String::new()));
        }
    }

    fn apply_visualizer(&self) {
        let s = self.s();
        let o = s.objects();
        let (module, effect) = match self.visualizer.vlc_module() {
            Some((m, e)) => (m.to_string(), e.unwrap_or(m).to_string()),
            None => (String::new(), String::new()),
        };
        o.set(s.mp, "audio-visual", &Var::Str(module));
        o.set(s.mp, "effect-list", &Var::Str(effect.clone()));
        o.for_each(s.mp, "audio output", &mut |aout| {
            o.set(aout, "visual", &Var::Str(effect.clone()));
        });
    }

    fn set_renderer(&mut self, id: Option<u64>) {
        let s = self.s();
        let item = id.and_then(|i| self.helper.renderer_item(i));
        unsafe { (self.api.libvlc_media_player_set_renderer)(s.mp, item.unwrap_or(std::ptr::null_mut())) };
        self.status.write().renderer = id.filter(|_| item.is_some());
        // The renderer applies when the input starts, so restart in place.
        self.restart_current();
    }

    fn restart_current(&mut self) {
        let Some(cur) = self.current.take() else { return };
        let st = self.status.read().clone();
        let mut req = cur.request;
        req.start_ms = Some(st.time_ms);
        req.paused = st.state == PlayerState::Paused;
        let s = self.s();
        unsafe { (self.api.libvlc_media_player_stop)(s.mp) };
        self.open(req);
    }

    fn reconfigure(&mut self, settings: Settings) {
        let new_args = args::playback(&settings);
        let changed = new_args != self.s().args
            || (settings.general.save_log, &settings.general.log_file)
                != (self.settings.general.save_log, &self.settings.general.log_file);
        let post_changed = settings.performance.post_processing != self.settings.performance.post_processing;
        self.settings = settings;
        if post_changed {
            self.applied_video_chain = None;
            self.apply_video_effects();
        }
        if !changed {
            return;
        }
        let st = self.status.read().clone();
        let resume = self.current.take().map(|c| {
            let mut r = c.request;
            r.start_ms = Some(st.time_ms);
            r.paused = st.state == PlayerState::Paused;
            (r, st.state.is_active())
        });
        self.detach_media_events();
        match Session::new(
            self.api.clone(),
            new_args,
            log_path(&self.settings),
            &self.video,
            self.raw_tx.clone(),
            &self.dialogs,
        ) {
            Ok(new) => {
                let old = std::mem::replace(&mut *self.session.write(), Arc::new(new));
                self.dialogs.detach(old.inst.ptr);
                drop(old);
            }
            Err(e) => {
                self.emit(EngineEvent::Error(format!("Could not apply settings: {e:#}")));
                return;
            }
        }
        let s = self.s();
        unsafe {
            (self.api.libvlc_audio_set_volume)(s.mp, st.volume);
            (self.api.libvlc_media_player_set_rate)(s.mp, if st.rate > 0.0 { st.rate } else { 1.0 });
        }
        self.applied_audio_chain = None;
        self.applied_video_chain = None;
        self.applied_sub_chain = None;
        self.apply_audio_effects();
        self.apply_video_effects();
        self.apply_deinterlace();
        if let Some((req, was_active)) = resume
            && was_active
        {
            self.open(req);
        }
    }
}

/// Subtitle files next to a video whose name starts with the video's name,
/// which VLC's autodetection would load.
fn has_sidecar_subtitles(video: &std::path::Path) -> bool {
    let (Some(dir), Some(stem)) = (video.parent(), video.file_stem().map(|s| s.to_string_lossy().to_lowercase()))
    else {
        return false;
    };
    std::fs::read_dir(dir).is_ok_and(|rd| {
        rd.flatten().any(|e| {
            let p = e.path();
            crate::domain::media::kind_for_path(&p) == crate::domain::media::MediaKind::Subtitle
                && p.file_stem().is_some_and(|s| s.to_string_lossy().to_lowercase().starts_with(&stem))
        })
    })
}

/// The libvlc log file chosen in General > Diagnostics, if logging is on.
fn log_path(settings: &Settings) -> Option<std::path::PathBuf> {
    let g = &settings.general;
    if !g.save_log {
        return None;
    }
    if !g.log_file.trim().is_empty() {
        return Some(std::path::PathBuf::from(expand_home(g.log_file.trim())));
    }
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
    Some(if cfg!(target_os = "macos") {
        home.join("Library/Logs/NotVLC/vlc.log")
    } else {
        home.join(".cache/notvlc/vlc.log")
    })
}

fn expand_home(p: &str) -> String {
    if let Some(rest) = p.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return std::path::Path::new(&home).join(rest).to_string_lossy().into_owned();
    }
    p.to_string()
}

/// Metadata and thumbnails through the helper instance.
pub struct VlcProbe {
    helper: Arc<Helper>,
}

impl VlcProbe {
    pub fn new(helper: Arc<Helper>) -> Self {
        Self { helper }
    }
}

impl MediaProbe for VlcProbe {
    fn probe(&self, source: &MediaSource) -> Result<crate::domain::media::MediaMeta> {
        self.helper.probe(source)
    }

    fn thumbnail(&self, source: &MediaSource, out: &std::path::Path, width: u32, fraction: f32) -> Result<()> {
        self.helper.thumbnail(source, out, width, fraction)
    }
}
