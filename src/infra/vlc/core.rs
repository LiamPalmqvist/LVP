//! Thin owning wrappers over libvlc handles, plus access to VLC object
//! variables through libvlccore.

use std::ffi::{CStr, CString, c_char, c_void};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};

use super::ffi::{self, Api, VlcValue, var_type};
use crate::domain::media::{CodecInfo, MediaMeta, MediaSource};

static API: OnceLock<std::result::Result<Arc<Api>, String>> = OnceLock::new();

/// The process-wide libvlc function table, loaded on first use.
pub fn api() -> Result<Arc<Api>> {
    API.get_or_init(|| ffi::load().map(Arc::new).map_err(|e| format!("{e:#}"))).clone().map_err(|e| anyhow!(e))
}

pub fn cstring(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap_or_default()
}

/// Copies a borrowed C string.
pub unsafe fn borrowed(p: *const c_char) -> Option<String> {
    if p.is_null() { None } else { Some(unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()) }
}

/// Copies and frees a C string allocated by libvlc.
pub unsafe fn owned(api: &Api, p: *mut c_char) -> Option<String> {
    let s = unsafe { borrowed(p) };
    if !p.is_null() {
        unsafe { (api.libvlc_free)(p.cast()) };
    }
    s
}

pub fn version(api: &Api) -> String {
    unsafe { borrowed((api.libvlc_get_version)()) }.unwrap_or_default()
}

pub fn last_error(api: &Api) -> String {
    unsafe { borrowed((api.libvlc_errmsg)()) }.unwrap_or_else(|| "unknown libvlc error".into())
}

pub struct Instance {
    pub api: Arc<Api>,
    pub ptr: *mut ffi::Instance,
    /// Where libvlc's own log goes (a file or the null device).
    log: Option<*mut libc::FILE>,
}

unsafe impl Send for Instance {}
unsafe impl Sync for Instance {}

impl Instance {
    pub fn new(api: Arc<Api>, args: &[String]) -> Result<Self> {
        let cargs: Vec<CString> = args.iter().map(|a| cstring(a)).collect();
        let argv: Vec<*const c_char> = cargs.iter().map(|a| a.as_ptr()).collect();
        let ptr = unsafe { (api.libvlc_new)(argv.len() as i32, argv.as_ptr()) };
        if ptr.is_null() {
            return Err(anyhow!("libvlc_new failed: {}", last_error(&api)));
        }
        let name = cstring("LVP");
        let http = cstring(&format!("LVP/{} LibVLC/{}", env!("CARGO_PKG_VERSION"), version(&api)));
        let id = cstring("org.lvp.LVP");
        let ver = cstring(env!("CARGO_PKG_VERSION"));
        let icon = cstring("lvp");
        unsafe {
            (api.libvlc_set_user_agent)(ptr, name.as_ptr(), http.as_ptr());
            (api.libvlc_set_app_id)(ptr, id.as_ptr(), ver.as_ptr(), icon.as_ptr());
        }
        let mut inst = Self { api, ptr, log: None };
        inst.log_to(None);
        Ok(inst)
    }

    /// Sends libvlc's log to `path` (appending), or discards it when `None`
    /// so it never spills onto the terminal.
    pub fn log_to(&mut self, path: Option<&std::path::Path>) {
        let target = match path {
            Some(p) => {
                if let Some(dir) = p.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                p.to_string_lossy().into_owned()
            }
            None if cfg!(windows) => "NUL".to_string(),
            None => "/dev/null".to_string(),
        };
        let cpath = cstring(&target);
        let mode = cstring("a");
        let file = unsafe { libc::fopen(cpath.as_ptr(), mode.as_ptr()) };
        if file.is_null() {
            return;
        }
        unsafe {
            (self.api.libvlc_log_set_file)(self.ptr, file);
            if let Some(old) = self.log.replace(file) {
                libc::fclose(old);
            }
        }
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            (self.api.libvlc_log_unset)(self.ptr);
            (self.api.libvlc_release)(self.ptr);
            if let Some(f) = self.log.take() {
                libc::fclose(f);
            }
        }
    }
}

pub struct Media {
    pub api: Arc<Api>,
    pub ptr: *mut ffi::Media,
}

unsafe impl Send for Media {}
unsafe impl Sync for Media {}

impl Clone for Media {
    fn clone(&self) -> Self {
        unsafe { (self.api.libvlc_media_retain)(self.ptr) };
        Self { api: self.api.clone(), ptr: self.ptr }
    }
}

impl Drop for Media {
    fn drop(&mut self) {
        unsafe { (self.api.libvlc_media_release)(self.ptr) };
    }
}

impl Media {
    pub fn new(inst: &Instance, source: &MediaSource) -> Result<Self> {
        let api = inst.api.clone();
        let ptr = match source {
            MediaSource::File(p) => {
                let c = cstring(&p.to_string_lossy());
                unsafe { (api.libvlc_media_new_path)(inst.ptr, c.as_ptr()) }
            }
            MediaSource::Location(l) => {
                let c = cstring(l);
                unsafe { (api.libvlc_media_new_location)(inst.ptr, c.as_ptr()) }
            }
        };
        if ptr.is_null() {
            return Err(anyhow!("cannot open {}: {}", source.mrl(), last_error(&api)));
        }
        Ok(Self { api, ptr })
    }

    /// Wraps a pointer the caller already holds a reference to.
    pub unsafe fn from_owned(api: Arc<Api>, ptr: *mut ffi::Media) -> Self {
        Self { api, ptr }
    }

    pub fn add_option(&self, opt: &str) {
        let c = cstring(opt);
        unsafe { (self.api.libvlc_media_add_option)(self.ptr, c.as_ptr()) };
    }

    pub fn mrl(&self) -> Option<String> {
        unsafe { owned(&self.api, (self.api.libvlc_media_get_mrl)(self.ptr)) }
    }

    pub fn meta_field(&self, field: u32) -> Option<String> {
        unsafe { owned(&self.api, (self.api.libvlc_media_get_meta)(self.ptr, field)) }.filter(|s| !s.trim().is_empty())
    }

    pub fn duration_ms(&self) -> i64 {
        unsafe { (self.api.libvlc_media_get_duration)(self.ptr) }
    }

    pub fn media_type(&self) -> i32 {
        unsafe { (self.api.libvlc_media_get_type)(self.ptr) }
    }

    /// Starts parsing and waits for it to finish or time out.
    pub fn parse_blocking(&self, flags: i32, timeout: Duration) -> i32 {
        let ms = timeout.as_millis() as i32;
        if unsafe { (self.api.libvlc_media_parse_with_options)(self.ptr, flags, ms) } != 0 {
            return ffi::parse::STATUS_FAILED;
        }
        let deadline = Instant::now() + timeout + Duration::from_millis(250);
        loop {
            let st = unsafe { (self.api.libvlc_media_get_parsed_status)(self.ptr) };
            if st != 0 {
                return st;
            }
            if Instant::now() > deadline {
                unsafe { (self.api.libvlc_media_parse_stop)(self.ptr) };
                return ffi::parse::STATUS_TIMEOUT;
            }
            std::thread::sleep(Duration::from_millis(15));
        }
    }

    pub fn meta(&self) -> MediaMeta {
        use ffi::meta::*;
        let f = |k| self.meta_field(k);
        let mut m = MediaMeta {
            title: f(TITLE),
            artist: f(ARTIST),
            album: f(ALBUM),
            album_artist: f(ALBUM_ARTIST),
            genre: f(GENRE),
            date: f(DATE),
            track_number: f(TRACK_NUMBER),
            disc_number: f(DISC_NUMBER),
            description: f(DESCRIPTION),
            copyright: f(COPYRIGHT),
            publisher: f(PUBLISHER),
            encoded_by: f(ENCODED_BY),
            language: f(LANGUAGE),
            url: f(URL),
            now_playing: f(NOW_PLAYING),
            show_name: f(SHOW_NAME),
            season: f(SEASON),
            episode: f(EPISODE),
            director: f(DIRECTOR),
            actors: f(ACTORS),
            artwork_url: f(ARTWORK_URL),
            ..Default::default()
        };
        let d = self.duration_ms();
        if d > 0 {
            m.duration_ms = Some(d);
        }
        let tracks = self.tracks();
        if !tracks.is_empty() {
            let video = tracks.iter().find(|t| t.kind == "Video");
            m.has_video = Some(video.is_some());
            if let Some(v) = video {
                m.width = v.width;
                m.height = v.height;
            }
            m.codecs = tracks;
        }
        m
    }

    pub fn tracks(&self) -> Vec<CodecInfo> {
        let mut out = Vec::new();
        let mut list: *mut *mut ffi::MediaTrack = std::ptr::null_mut();
        let n = unsafe { (self.api.libvlc_media_tracks_get)(self.ptr, &mut list) };
        if list.is_null() {
            return out;
        }
        for i in 0..n as usize {
            let t = unsafe { &**list.add(i) };
            let mut info = CodecInfo {
                codec: fourcc(t.codec),
                language: unsafe { borrowed(t.language) },
                description: unsafe { borrowed(t.description) },
                bitrate: t.bitrate,
                ..Default::default()
            };
            match t.kind {
                ffi::track_type::AUDIO => {
                    info.kind = "Audio".into();
                    if !t.detail.is_null() {
                        let a = unsafe { &*(t.detail as *const ffi::AudioTrack) };
                        info.channels = Some(a.channels);
                        info.sample_rate = Some(a.rate);
                    }
                }
                ffi::track_type::VIDEO => {
                    info.kind = "Video".into();
                    if !t.detail.is_null() {
                        let v = unsafe { &*(t.detail as *const ffi::VideoTrack) };
                        info.width = Some(v.width);
                        info.height = Some(v.height);
                        if v.frame_rate_den > 0 {
                            info.frame_rate = Some(v.frame_rate_num as f32 / v.frame_rate_den as f32);
                        }
                    }
                }
                ffi::track_type::TEXT => info.kind = "Subtitle".into(),
                _ => info.kind = "Data".into(),
            }
            out.push(info);
        }
        unsafe { (self.api.libvlc_media_tracks_release)(list, n) };
        out
    }

    /// Sample aspect ratio of the video track with this id (or the first one).
    pub fn video_sar(&self, track_id: i32) -> Option<(u32, u32)> {
        let mut list: *mut *mut ffi::MediaTrack = std::ptr::null_mut();
        let n = unsafe { (self.api.libvlc_media_tracks_get)(self.ptr, &mut list) };
        if list.is_null() {
            return None;
        }
        let mut best = None;
        for i in 0..n as usize {
            let t = unsafe { &**list.add(i) };
            if t.kind == ffi::track_type::VIDEO && !t.detail.is_null() {
                let v = unsafe { &*(t.detail as *const ffi::VideoTrack) };
                let sar = (v.sar_num.max(1), v.sar_den.max(1));
                if t.id == track_id || best.is_none() {
                    best = Some(sar);
                }
            }
        }
        unsafe { (self.api.libvlc_media_tracks_release)(list, n) };
        best
    }

    /// Children of a container (folder, playlist, network share).
    pub fn subitems(&self) -> Vec<Media> {
        let ml = unsafe { (self.api.libvlc_media_subitems)(self.ptr) };
        if ml.is_null() {
            return Vec::new();
        }
        let out = media_list_items(&self.api, ml);
        unsafe { (self.api.libvlc_media_list_release)(ml) };
        out
    }

    pub fn stats(&self) -> Option<ffi::MediaStats> {
        let mut s = ffi::MediaStats::default();
        (unsafe { (self.api.libvlc_media_get_stats)(self.ptr, &mut s) } != 0).then_some(s)
    }
}

pub fn media_list_items(api: &Arc<Api>, ml: *mut ffi::MediaList) -> Vec<Media> {
    let mut out = Vec::new();
    unsafe {
        (api.libvlc_media_list_lock)(ml);
        let n = (api.libvlc_media_list_count)(ml);
        for i in 0..n.max(0) {
            let m = (api.libvlc_media_list_item_at_index)(ml, i);
            if !m.is_null() {
                out.push(Media::from_owned(api.clone(), m));
            }
        }
        (api.libvlc_media_list_unlock)(ml);
    }
    out
}

pub fn fourcc(code: u32) -> String {
    let b = code.to_le_bytes();
    let s: String = b.iter().map(|&c| if c.is_ascii_graphic() || c == b' ' { c as char } else { '?' }).collect();
    s.trim().to_string()
}

/// Walks a libvlc track description list into `(id, name)` pairs and frees it.
pub unsafe fn track_list(api: &Api, mut p: *mut ffi::TrackDescription) -> Vec<(i32, String)> {
    let head = p;
    let mut out = Vec::new();
    while !p.is_null() {
        let d = unsafe { &*p };
        out.push((d.id, unsafe { borrowed(d.name) }.unwrap_or_else(|| format!("Track {}", d.id))));
        p = d.next;
    }
    if !head.is_null() {
        unsafe { (api.libvlc_track_description_list_release)(head) };
    }
    out
}

/// Typed value for a VLC object variable.
#[derive(Debug, Clone, PartialEq)]
pub enum Var {
    Bool(bool),
    Int(i64),
    Float(f32),
    Str(String),
}

impl Var {
    fn kind(&self) -> i32 {
        match self {
            Var::Bool(_) => var_type::BOOL,
            Var::Int(_) => var_type::INTEGER,
            Var::Float(_) => var_type::FLOAT,
            Var::Str(_) => var_type::STRING,
        }
    }
}

/// Access to VLC objects below a media player (its input, audio output and
/// video outputs) through libvlccore. All functions are no-ops when
/// libvlccore could not be loaded.
pub struct Objects<'a> {
    pub api: &'a Api,
}

impl Objects<'_> {
    pub fn available(&self) -> bool {
        self.api.core.is_some()
    }

    /// Creates the variable if needed and sets it.
    pub fn set(&self, obj: *mut c_void, name: &str, value: &Var) -> bool {
        let Some(core) = &self.api.core else { return false };
        if obj.is_null() {
            return false;
        }
        let cname = cstring(name);
        let kind = value.kind();
        unsafe {
            let existing = (core.var_type)(obj, cname.as_ptr());
            if existing == 0 {
                (core.var_create)(obj, cname.as_ptr(), kind);
            } else if existing & var_type::TYPE_MASK != kind {
                log::debug!("variable {name} has type {existing:#x}, wanted {kind:#x}");
                return false;
            }
            let cstr_holder;
            let v = match value {
                Var::Bool(b) => VlcValue { b_bool: *b },
                Var::Int(i) => VlcValue { i_int: *i },
                Var::Float(f) => VlcValue { f_float: *f },
                Var::Str(s) => {
                    cstr_holder = cstring(s);
                    VlcValue { psz_string: cstr_holder.as_ptr() as *mut c_char }
                }
            };
            (core.var_set_checked)(obj, cname.as_ptr(), kind, v) == 0
        }
    }

    /// Calls `f` for every descendant of `root` whose VLC object type matches,
    /// holding a reference for the duration of the call.
    pub fn for_each(&self, root: *mut c_void, object_type: &str, f: &mut dyn FnMut(*mut c_void)) {
        self.walk(root, object_type, f, 0);
    }

    fn walk(&self, root: *mut c_void, object_type: &str, f: &mut dyn FnMut(*mut c_void), depth: u32) {
        let Some(core) = &self.api.core else { return };
        if root.is_null() || depth > 4 {
            return;
        }
        unsafe {
            let list = (core.list_children)(root);
            if list.is_null() {
                return;
            }
            let l = &*list;
            for i in 0..l.count.max(0) as usize {
                let child = (*l.values.add(i)).p_address;
                if child.is_null() {
                    continue;
                }
                // `struct vlc_common_members` starts with `const char *object_type`.
                let ty = borrowed(*(child as *const *const c_char)).unwrap_or_default();
                if ty == object_type {
                    f(child);
                }
                self.walk(child, object_type, f, depth + 1);
            }
            (core.list_release)(list);
        }
    }

    /// Sets a variable on the player (so future outputs inherit it) and on
    /// every live object of the given type.
    pub fn set_everywhere(&self, mp: *mut c_void, object_type: &str, name: &str, value: &Var) {
        self.set(mp, name, value);
        self.for_each(mp, object_type, &mut |o| {
            self.set(o, name, value);
        });
    }
}
