//! A second libvlc instance for work that must not disturb playback:
//! parsing, thumbnails, network discovery, renderer discovery and
//! convert/stream jobs.

use std::collections::HashMap;
use std::ffi::{c_char, c_uint, c_void};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow, bail};
use crossbeam_channel::Sender;
use parking_lot::{Condvar, Mutex};

use super::core::{Instance, Media, borrowed, cstring, media_list_items};
use super::ffi::{self, Api, event};
use crate::app::ports::{BrowseItem, EngineEvent, Job, RendererInfo, ServiceCategory, ServiceInfo};
use crate::domain::media::{MediaMeta, MediaSource};

struct RendererEntry {
    id: u64,
    item: usize,
    info: RendererInfo,
}

struct RendererCtx {
    api: Arc<Api>,
    list: Mutex<Vec<RendererEntry>>,
    next: AtomicU64,
    tx: Sender<EngineEvent>,
}

struct JobHandle {
    cancel: Arc<AtomicBool>,
}

pub struct Helper {
    pub api: Arc<Api>,
    pub inst: Instance,
    discoverers: Mutex<HashMap<String, usize>>,
    renderer_ctx: Arc<RendererCtx>,
    renderer_discoverers: Mutex<Vec<usize>>,
    jobs: Mutex<HashMap<u64, JobHandle>>,
    next_job: AtomicU64,
    tx: Sender<EngineEvent>,
}

unsafe impl Send for Helper {}
unsafe impl Sync for Helper {}

impl Helper {
    pub fn new(api: Arc<Api>, args: &[String], tx: Sender<EngineEvent>) -> Result<Self> {
        let inst = Instance::new(api.clone(), args)?;
        let renderer_ctx = Arc::new(RendererCtx {
            api: api.clone(),
            list: Mutex::new(Vec::new()),
            next: AtomicU64::new(1),
            tx: tx.clone(),
        });
        Ok(Self {
            api,
            inst,
            discoverers: Mutex::new(HashMap::new()),
            renderer_ctx,
            renderer_discoverers: Mutex::new(Vec::new()),
            jobs: Mutex::new(HashMap::new()),
            next_job: AtomicU64::new(1),
            tx,
        })
    }

    // ---- discovery -------------------------------------------------------

    pub fn services(&self) -> Vec<ServiceInfo> {
        let mut out = Vec::new();
        for (cat, category) in [
            (ffi::discoverer_cat::LAN, ServiceCategory::Lan),
            (ffi::discoverer_cat::DEVICES, ServiceCategory::Devices),
            (ffi::discoverer_cat::LOCALDIRS, ServiceCategory::LocalDirs),
            (ffi::discoverer_cat::PODCASTS, ServiceCategory::Podcasts),
        ] {
            let mut list: *mut *mut ffi::DiscovererDescription = std::ptr::null_mut();
            let n = unsafe { (self.api.libvlc_media_discoverer_list_get)(self.inst.ptr, cat, &mut list) };
            if list.is_null() {
                continue;
            }
            for i in 0..n {
                let d = unsafe { &**list.add(i) };
                out.push(ServiceInfo {
                    name: unsafe { borrowed(d.name) }.unwrap_or_default(),
                    long_name: unsafe { borrowed(d.longname) }.unwrap_or_default(),
                    category,
                });
            }
            unsafe { (self.api.libvlc_media_discoverer_list_release)(list, n) };
        }
        out
    }

    pub fn service_items(&self, name: &str) -> Vec<BrowseItem> {
        let md = {
            let mut map = self.discoverers.lock();
            if let Some(p) = map.get(name) {
                *p as *mut ffi::MediaDiscoverer
            } else {
                let c = cstring(name);
                let md = unsafe { (self.api.libvlc_media_discoverer_new)(self.inst.ptr, c.as_ptr()) };
                if md.is_null() {
                    return Vec::new();
                }
                if unsafe { (self.api.libvlc_media_discoverer_start)(md) } != 0 {
                    unsafe { (self.api.libvlc_media_discoverer_release)(md) };
                    return Vec::new();
                }
                map.insert(name.to_string(), md as usize);
                md
            }
        };
        let ml = unsafe { (self.api.libvlc_media_discoverer_media_list)(md) };
        if ml.is_null() {
            return Vec::new();
        }
        let items = media_list_items(&self.api, ml);
        unsafe { (self.api.libvlc_media_list_release)(ml) };
        items.iter().map(browse_item).collect()
    }

    pub fn browse(&self, source: &MediaSource) -> Result<Vec<BrowseItem>> {
        let media = Media::new(&self.inst, source)?;
        let flags = ffi::parse::NETWORK | ffi::parse::LOCAL | ffi::parse::DO_INTERACT;
        let st = media.parse_blocking(flags, Duration::from_secs(20));
        if st == ffi::parse::STATUS_FAILED {
            bail!("could not open {}", source.display_name());
        }
        Ok(media.subitems().iter().map(browse_item).collect())
    }

    // ---- renderers -------------------------------------------------------

    pub fn start_renderer_discovery(&self) {
        let mut started = self.renderer_discoverers.lock();
        if !started.is_empty() {
            return;
        }
        let mut list: *mut *mut ffi::RendererDescription = std::ptr::null_mut();
        let n = unsafe { (self.api.libvlc_renderer_discoverer_list_get)(self.inst.ptr, &mut list) };
        let mut names = Vec::new();
        if !list.is_null() {
            for i in 0..n {
                names.push(unsafe { borrowed((**list.add(i)).name) }.unwrap_or_default());
            }
            unsafe { (self.api.libvlc_renderer_discoverer_list_release)(list, n) };
        }
        for name in names {
            let c = cstring(&name);
            let rd = unsafe { (self.api.libvlc_renderer_discoverer_new)(self.inst.ptr, c.as_ptr()) };
            if rd.is_null() {
                continue;
            }
            let em = unsafe { (self.api.libvlc_renderer_discoverer_event_manager)(rd) };
            let data = Arc::as_ptr(&self.renderer_ctx) as *mut c_void;
            unsafe {
                (self.api.libvlc_event_attach)(em, event::RD_ITEM_ADDED, renderer_event, data);
                (self.api.libvlc_event_attach)(em, event::RD_ITEM_DELETED, renderer_event, data);
                (self.api.libvlc_renderer_discoverer_start)(rd);
            }
            started.push(rd as usize);
        }
    }

    pub fn renderers(&self) -> Vec<RendererInfo> {
        self.renderer_ctx.list.lock().iter().map(|e| e.info.clone()).collect()
    }

    /// The held renderer item for an id, for `libvlc_media_player_set_renderer`.
    pub fn renderer_item(&self, id: u64) -> Option<*mut ffi::RendererItem> {
        self.renderer_ctx.list.lock().iter().find(|e| e.id == id).map(|e| e.item as *mut ffi::RendererItem)
    }

    // ---- jobs ------------------------------------------------------------

    pub fn start_job(self: &Arc<Self>, job: Job) -> u64 {
        let id = self.next_job.fetch_add(1, Ordering::Relaxed);
        let cancel = Arc::new(AtomicBool::new(false));
        self.jobs.lock().insert(id, JobHandle { cancel: cancel.clone() });
        let me = self.clone();
        std::thread::Builder::new()
            .name(format!("vlc-job-{id}"))
            .spawn(move || {
                let res = me.run_job(id, &job, &cancel);
                me.jobs.lock().remove(&id);
                let (ok, message) = match res {
                    Ok(()) if cancel.load(Ordering::Relaxed) => (false, "Cancelled".to_string()),
                    Ok(()) => (true, "Done".to_string()),
                    Err(e) => (false, format!("{e:#}")),
                };
                let _ = me.tx.send(EngineEvent::JobFinished { id, ok, message });
            })
            .ok();
        id
    }

    pub fn cancel_job(&self, id: u64) {
        if let Some(j) = self.jobs.lock().get(&id) {
            j.cancel.store(true, Ordering::Relaxed);
        }
    }

    fn run_job(&self, id: u64, job: &Job, cancel: &AtomicBool) -> Result<()> {
        let media = Media::new(&self.inst, &job.source.source)?;
        for o in &job.source.options {
            media.add_option(o);
        }
        media.add_option(&format!(":sout={}", job.sout));
        media.add_option(":sout-all");
        media.add_option(":sout-keep");
        let mp = unsafe { (self.api.libvlc_media_player_new)(self.inst.ptr) };
        if mp.is_null() {
            bail!("could not create a player for the job");
        }
        unsafe {
            (self.api.libvlc_media_player_set_media)(mp, media.ptr);
            (self.api.libvlc_media_player_play)(mp);
        }
        let started = Instant::now();
        let result = loop {
            std::thread::sleep(Duration::from_millis(250));
            if cancel.load(Ordering::Relaxed) {
                break Ok(());
            }
            let st = unsafe { (self.api.libvlc_media_player_get_state)(mp) };
            match st {
                ffi::state::ENDED => break Ok(()),
                ffi::state::ERROR => break Err(anyhow!("VLC reported an error while converting")),
                ffi::state::STOPPED if started.elapsed() > Duration::from_secs(2) => break Ok(()),
                _ => {}
            }
            let pos = unsafe { (self.api.libvlc_media_player_get_position)(mp) };
            let _ = self.tx.send(EngineEvent::JobProgress { id, position: pos.clamp(0.0, 1.0) });
        };
        unsafe {
            (self.api.libvlc_media_player_stop)(mp);
            (self.api.libvlc_media_player_release)(mp);
        }
        result
    }

    // ---- probing ---------------------------------------------------------

    pub fn probe(&self, source: &MediaSource) -> Result<MediaMeta> {
        let media = Media::new(&self.inst, source)?;
        let mut flags = ffi::parse::LOCAL | ffi::parse::FETCH_LOCAL;
        if source.path().is_none() {
            flags |= ffi::parse::NETWORK;
        }
        let st = media.parse_blocking(flags, Duration::from_secs(8));
        if st == ffi::parse::STATUS_FAILED {
            bail!("could not parse {}", source.display_name());
        }
        Ok(media.meta())
    }

    pub fn thumbnail(&self, source: &MediaSource, out: &Path, width: u32, fraction: f32) -> Result<()> {
        let media = Media::new(&self.inst, source)?;
        let mut duration = media.duration_ms();
        if duration <= 0 {
            media.parse_blocking(ffi::parse::LOCAL, Duration::from_secs(4));
            duration = media.duration_ms();
        }
        media.add_option(":no-audio");
        media.add_option(":no-spu");
        media.add_option(":no-osd");
        if duration > 0 {
            let at = duration as f64 * fraction.clamp(0.0, 0.95) as f64 / 1000.0;
            media.add_option(&format!(":start-time={at:.2}"));
        }
        let grab = Arc::new(Grab::new(width));
        let mp = unsafe { (self.api.libvlc_media_player_new)(self.inst.ptr) };
        if mp.is_null() {
            bail!("could not create a thumbnail player");
        }
        let opaque = Arc::as_ptr(&grab) as *mut c_void;
        unsafe {
            (self.api.libvlc_video_set_callbacks)(mp, Some(grab_lock), None, Some(grab_display), opaque);
            (self.api.libvlc_video_set_format_callbacks)(mp, Some(grab_format), None);
            (self.api.libvlc_media_player_set_media)(mp, media.ptr);
            (self.api.libvlc_media_player_play)(mp);
        }
        let frame = grab.wait(Duration::from_secs(10), || {
            let st = unsafe { (self.api.libvlc_media_player_get_state)(mp) };
            st == ffi::state::ERROR || st == ffi::state::ENDED
        });
        unsafe {
            (self.api.libvlc_media_player_stop)(mp);
            (self.api.libvlc_media_player_release)(mp);
        }
        let (w, h, px) = frame.ok_or_else(|| anyhow!("no picture from {}", source.display_name()))?;
        let img = image::RgbaImage::from_raw(w, h, px).ok_or_else(|| anyhow!("bad thumbnail buffer"))?;
        if let Some(dir) = out.parent() {
            std::fs::create_dir_all(dir)?;
        }
        image::DynamicImage::ImageRgba8(img).to_rgb8().save(out)?;
        Ok(())
    }
}

impl Drop for Helper {
    fn drop(&mut self) {
        for (_, md) in self.discoverers.lock().drain() {
            unsafe {
                (self.api.libvlc_media_discoverer_stop)(md as *mut _);
                (self.api.libvlc_media_discoverer_release)(md as *mut _);
            }
        }
        for rd in self.renderer_discoverers.lock().drain(..) {
            unsafe {
                (self.api.libvlc_renderer_discoverer_stop)(rd as *mut _);
                (self.api.libvlc_renderer_discoverer_release)(rd as *mut _);
            }
        }
        for e in self.renderer_ctx.list.lock().drain(..) {
            unsafe { (self.api.libvlc_renderer_item_release)(e.item as *mut _) };
        }
    }
}

fn browse_item(m: &Media) -> BrowseItem {
    let mrl = m.mrl().unwrap_or_default();
    let source = MediaSource::from_user_input(&mrl);
    let name = m.meta_field(ffi::meta::TITLE).unwrap_or_else(|| source.display_name());
    let ty = m.media_type();
    BrowseItem { name, source, is_container: ty == 2 || ty == 5 }
}

unsafe extern "C" fn renderer_event(ev: *const ffi::Event, data: *mut c_void) {
    let (Some(ev), Some(ctx)) = (unsafe { ev.as_ref() }, unsafe { (data as *const RendererCtx).as_ref() }) else {
        return;
    };
    let item = ev.u_ptr();
    if item.is_null() {
        return;
    }
    let api = &ctx.api;
    match ev.kind {
        event::RD_ITEM_ADDED => {
            let held = unsafe { (api.libvlc_renderer_item_hold)(item) };
            let flags = unsafe { (api.libvlc_renderer_item_flags)(held) };
            let info = RendererInfo {
                id: ctx.next.fetch_add(1, Ordering::Relaxed),
                name: unsafe { borrowed((api.libvlc_renderer_item_name)(held)) }.unwrap_or_default(),
                kind: unsafe { borrowed((api.libvlc_renderer_item_type)(held)) }.unwrap_or_default(),
                can_video: flags & 0x0002 != 0,
            };
            ctx.list.lock().push(RendererEntry { id: info.id, item: held as usize, info });
        }
        event::RD_ITEM_DELETED => {
            let mut list = ctx.list.lock();
            let name = unsafe { borrowed((api.libvlc_renderer_item_name)(item)) };
            if let Some(i) = list.iter().position(|e| Some(&e.info.name) == name.as_ref()) {
                let e = list.remove(i);
                unsafe { (api.libvlc_renderer_item_release)(e.item as *mut _) };
            }
        }
        _ => return,
    }
    let _ = ctx.tx.send(EngineEvent::RenderersChanged);
}

/// Captures one decoded picture for a thumbnail.
struct Grab {
    width: u32,
    state: Mutex<GrabState>,
    cv: Condvar,
}

struct GrabState {
    w: u32,
    h: u32,
    buf: Vec<u8>,
    /// Receives pictures after the first, so the kept one is never overwritten.
    scratch: Vec<u8>,
    done: bool,
}

impl Grab {
    fn new(width: u32) -> Self {
        Self {
            width,
            state: Mutex::new(GrabState { w: 0, h: 0, buf: Vec::new(), scratch: Vec::new(), done: false }),
            cv: Condvar::new(),
        }
    }

    fn wait(&self, timeout: Duration, failed: impl Fn() -> bool) -> Option<(u32, u32, Vec<u8>)> {
        let deadline = Instant::now() + timeout;
        let mut st = self.state.lock();
        while !st.done {
            if Instant::now() > deadline || failed() {
                return None;
            }
            self.cv.wait_for(&mut st, Duration::from_millis(100));
        }
        let len = (st.w * st.h * 4) as usize;
        Some((st.w, st.h, st.buf[..len].to_vec()))
    }
}

unsafe extern "C" fn grab_format(
    opaque: *mut *mut c_void,
    chroma: *mut c_char,
    width: *mut c_uint,
    height: *mut c_uint,
    pitches: *mut c_uint,
    lines: *mut c_uint,
) -> c_uint {
    unsafe {
        let g = &*(*opaque as *const Grab);
        let (sw, sh) = (*width, *height);
        if sw == 0 || sh == 0 {
            return 0;
        }
        let w = g.width.min(sw).max(16) & !1;
        let h = ((w as u64 * sh as u64 / sw as u64) as u32).max(2) & !1;
        std::ptr::copy_nonoverlapping(b"RGBA".as_ptr() as *const c_char, chroma, 4);
        *width = w;
        *height = h;
        *pitches = w * 4;
        *lines = h;
        let mut st = g.state.lock();
        st.w = w;
        st.h = h;
        st.buf = vec![0; (w * h * 4) as usize + 64];
        1
    }
}

unsafe extern "C" fn grab_lock(opaque: *mut c_void, planes: *mut *mut c_void) -> *mut c_void {
    unsafe {
        let g = &*(opaque as *const Grab);
        let mut st = g.state.lock();
        if st.done {
            let len = st.buf.len();
            st.scratch.resize(len, 0);
            *planes = st.scratch.as_mut_ptr().cast();
        } else {
            *planes = st.buf.as_mut_ptr().cast();
        }
    }
    std::ptr::null_mut()
}

unsafe extern "C" fn grab_display(opaque: *mut c_void, _picture: *mut c_void) {
    let g = unsafe { &*(opaque as *const Grab) };
    g.state.lock().done = true;
    g.cv.notify_all();
}
