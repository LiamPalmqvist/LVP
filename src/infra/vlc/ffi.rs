//! Raw libvlc 3.x bindings, resolved at runtime with `libloading`.
//!
//! Nothing here is safe to call directly from outside `infra::vlc`; the
//! wrappers in the sibling modules own the pointers and their lifetimes.
//! Signatures mirror `include/vlc/*.h` from VLC 3.0.

#![allow(non_camel_case_types, dead_code)]

use std::ffi::{c_char, c_float, c_int, c_uint, c_void};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use libloading::Library;

pub type Instance = c_void;
pub type Media = c_void;
pub type MediaPlayer = c_void;
pub type MediaList = c_void;
pub type EventManager = c_void;
pub type Equalizer = c_void;
pub type MediaDiscoverer = c_void;
pub type RendererDiscoverer = c_void;
pub type RendererItem = c_void;
pub type DialogId = c_void;
pub type VlcObject = c_void;

pub type EventCallback = unsafe extern "C" fn(event: *const Event, data: *mut c_void);

pub type VideoLockCb = unsafe extern "C" fn(opaque: *mut c_void, planes: *mut *mut c_void) -> *mut c_void;
pub type VideoUnlockCb = unsafe extern "C" fn(opaque: *mut c_void, picture: *mut c_void, planes: *const *mut c_void);
pub type VideoDisplayCb = unsafe extern "C" fn(opaque: *mut c_void, picture: *mut c_void);
pub type VideoFormatCb = unsafe extern "C" fn(
    opaque: *mut *mut c_void,
    chroma: *mut c_char,
    width: *mut c_uint,
    height: *mut c_uint,
    pitches: *mut c_uint,
    lines: *mut c_uint,
) -> c_uint;
pub type VideoCleanupCb = unsafe extern "C" fn(opaque: *mut c_void);

#[repr(C)]
pub struct Event {
    pub kind: c_int,
    pub obj: *mut c_void,
    /// The event union. Its widest member is two pointers.
    pub u: [u64; 2],
}

impl Event {
    pub fn u_i32(&self) -> i32 {
        self.u[0] as u32 as i32
    }
    pub fn u_i64(&self) -> i64 {
        self.u[0] as i64
    }
    pub fn u_f32(&self) -> f32 {
        f32::from_bits(self.u[0] as u32)
    }
    pub fn u_ptr(&self) -> *mut c_void {
        self.u[0] as usize as *mut c_void
    }
    /// `media_player_es_changed { i_type, i_id }`.
    pub fn u_i32_pair(&self) -> (i32, i32) {
        (self.u[0] as u32 as i32, (self.u[0] >> 32) as u32 as i32)
    }
}

pub mod event {
    pub const MEDIA_META_CHANGED: i32 = 0;
    pub const MEDIA_SUBITEM_ADDED: i32 = 1;
    pub const MEDIA_DURATION_CHANGED: i32 = 2;
    pub const MEDIA_PARSED_CHANGED: i32 = 3;
    pub const MEDIA_SUBITEM_TREE_ADDED: i32 = 6;

    pub const MP_MEDIA_CHANGED: i32 = 0x100;
    pub const MP_NOTHING_SPECIAL: i32 = 0x101;
    pub const MP_OPENING: i32 = 0x102;
    pub const MP_BUFFERING: i32 = 0x103;
    pub const MP_PLAYING: i32 = 0x104;
    pub const MP_PAUSED: i32 = 0x105;
    pub const MP_STOPPED: i32 = 0x106;
    pub const MP_FORWARD: i32 = 0x107;
    pub const MP_BACKWARD: i32 = 0x108;
    pub const MP_END_REACHED: i32 = 0x109;
    pub const MP_ENCOUNTERED_ERROR: i32 = 0x10a;
    pub const MP_TIME_CHANGED: i32 = 0x10b;
    pub const MP_POSITION_CHANGED: i32 = 0x10c;
    pub const MP_SEEKABLE_CHANGED: i32 = 0x10d;
    pub const MP_PAUSABLE_CHANGED: i32 = 0x10e;
    pub const MP_TITLE_CHANGED: i32 = 0x10f;
    pub const MP_SNAPSHOT_TAKEN: i32 = 0x110;
    pub const MP_LENGTH_CHANGED: i32 = 0x111;
    pub const MP_VOUT: i32 = 0x112;
    pub const MP_SCRAMBLED_CHANGED: i32 = 0x113;
    pub const MP_ES_ADDED: i32 = 0x114;
    pub const MP_ES_DELETED: i32 = 0x115;
    pub const MP_ES_SELECTED: i32 = 0x116;
    pub const MP_CORKED: i32 = 0x117;
    pub const MP_UNCORKED: i32 = 0x118;
    pub const MP_MUTED: i32 = 0x119;
    pub const MP_UNMUTED: i32 = 0x11a;
    pub const MP_AUDIO_VOLUME: i32 = 0x11b;
    pub const MP_AUDIO_DEVICE: i32 = 0x11c;
    pub const MP_CHAPTER_CHANGED: i32 = 0x11d;

    pub const ML_ITEM_ADDED: i32 = 0x200;
    pub const ML_WILL_ADD_ITEM: i32 = 0x201;
    pub const ML_ITEM_DELETED: i32 = 0x202;
    pub const ML_WILL_DELETE_ITEM: i32 = 0x203;
    pub const ML_END_REACHED: i32 = 0x204;

    pub const MD_STARTED: i32 = 0x500;
    pub const MD_ENDED: i32 = 0x501;
    pub const RD_ITEM_ADDED: i32 = 0x502;
    pub const RD_ITEM_DELETED: i32 = 0x503;
}

pub mod meta {
    pub const TITLE: u32 = 0;
    pub const ARTIST: u32 = 1;
    pub const GENRE: u32 = 2;
    pub const COPYRIGHT: u32 = 3;
    pub const ALBUM: u32 = 4;
    pub const TRACK_NUMBER: u32 = 5;
    pub const DESCRIPTION: u32 = 6;
    pub const RATING: u32 = 7;
    pub const DATE: u32 = 8;
    pub const SETTING: u32 = 9;
    pub const URL: u32 = 10;
    pub const LANGUAGE: u32 = 11;
    pub const NOW_PLAYING: u32 = 12;
    pub const PUBLISHER: u32 = 13;
    pub const ENCODED_BY: u32 = 14;
    pub const ARTWORK_URL: u32 = 15;
    pub const TRACK_ID: u32 = 16;
    pub const TRACK_TOTAL: u32 = 17;
    pub const DIRECTOR: u32 = 18;
    pub const SEASON: u32 = 19;
    pub const EPISODE: u32 = 20;
    pub const SHOW_NAME: u32 = 21;
    pub const ACTORS: u32 = 22;
    pub const ALBUM_ARTIST: u32 = 23;
    pub const DISC_NUMBER: u32 = 24;
    pub const DISC_TOTAL: u32 = 25;
}

pub mod state {
    pub const NOTHING_SPECIAL: i32 = 0;
    pub const OPENING: i32 = 1;
    pub const BUFFERING: i32 = 2;
    pub const PLAYING: i32 = 3;
    pub const PAUSED: i32 = 4;
    pub const STOPPED: i32 = 5;
    pub const ENDED: i32 = 6;
    pub const ERROR: i32 = 7;
}

pub mod parse {
    pub const LOCAL: c_int = 0x00;
    pub const NETWORK: c_int = 0x01;
    pub const FETCH_LOCAL: c_int = 0x02;
    pub const FETCH_NETWORK: c_int = 0x04;
    pub const DO_INTERACT: c_int = 0x08;
    use std::ffi::c_int;

    pub const STATUS_SKIPPED: i32 = 1;
    pub const STATUS_FAILED: i32 = 2;
    pub const STATUS_TIMEOUT: i32 = 3;
    pub const STATUS_DONE: i32 = 4;
}

pub mod track_type {
    pub const UNKNOWN: i32 = -1;
    pub const AUDIO: i32 = 0;
    pub const VIDEO: i32 = 1;
    pub const TEXT: i32 = 2;
}

pub mod adjust {
    pub const ENABLE: u32 = 0;
    pub const CONTRAST: u32 = 1;
    pub const BRIGHTNESS: u32 = 2;
    pub const HUE: u32 = 3;
    pub const SATURATION: u32 = 4;
    pub const GAMMA: u32 = 5;
}

pub mod marquee {
    pub const ENABLE: u32 = 0;
    pub const TEXT: u32 = 1;
    pub const COLOR: u32 = 2;
    pub const OPACITY: u32 = 3;
    pub const POSITION: u32 = 4;
    pub const REFRESH: u32 = 5;
    pub const SIZE: u32 = 6;
    pub const TIMEOUT: u32 = 7;
    pub const X: u32 = 8;
    pub const Y: u32 = 9;
}

pub mod logo {
    pub const ENABLE: u32 = 0;
    pub const FILE: u32 = 1;
    pub const X: u32 = 2;
    pub const Y: u32 = 3;
    pub const DELAY: u32 = 4;
    pub const REPEAT: u32 = 5;
    pub const OPACITY: u32 = 6;
    pub const POSITION: u32 = 7;
}

pub mod discoverer_cat {
    pub const DEVICES: i32 = 0;
    pub const LAN: i32 = 1;
    pub const PODCASTS: i32 = 2;
    pub const LOCALDIRS: i32 = 3;
}

pub mod slave_type {
    pub const SUBTITLE: i32 = 0;
    pub const AUDIO: i32 = 1;
}

#[repr(C)]
pub struct TrackDescription {
    pub id: c_int,
    pub name: *mut c_char,
    pub next: *mut TrackDescription,
}

#[repr(C)]
pub struct TitleDescription {
    pub duration: i64,
    pub name: *mut c_char,
    pub flags: c_uint,
}

#[repr(C)]
pub struct ChapterDescription {
    pub time_offset: i64,
    pub duration: i64,
    pub name: *mut c_char,
}

#[repr(C)]
pub struct AudioOutputDevice {
    pub next: *mut AudioOutputDevice,
    pub device: *mut c_char,
    pub description: *mut c_char,
}

#[repr(C)]
pub struct AudioOutput {
    pub name: *mut c_char,
    pub description: *mut c_char,
    pub next: *mut AudioOutput,
}

#[repr(C)]
pub struct AudioTrack {
    pub channels: c_uint,
    pub rate: c_uint,
}

#[repr(C)]
pub struct Viewpoint {
    pub yaw: c_float,
    pub pitch: c_float,
    pub roll: c_float,
    pub field_of_view: c_float,
}

#[repr(C)]
pub struct VideoTrack {
    pub height: c_uint,
    pub width: c_uint,
    pub sar_num: c_uint,
    pub sar_den: c_uint,
    pub frame_rate_num: c_uint,
    pub frame_rate_den: c_uint,
    pub orientation: c_int,
    pub projection: c_int,
    pub pose: Viewpoint,
}

#[repr(C)]
pub struct SubtitleTrack {
    pub encoding: *mut c_char,
}

#[repr(C)]
pub struct MediaTrack {
    pub codec: u32,
    pub original_fourcc: u32,
    pub id: c_int,
    pub kind: c_int,
    pub profile: c_int,
    pub level: c_int,
    /// Points at an `AudioTrack`, `VideoTrack` or `SubtitleTrack` depending on `kind`.
    pub detail: *mut c_void,
    pub bitrate: c_uint,
    pub language: *mut c_char,
    pub description: *mut c_char,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct MediaStats {
    pub read_bytes: c_int,
    pub input_bitrate: c_float,
    pub demux_read_bytes: c_int,
    pub demux_bitrate: c_float,
    pub demux_corrupted: c_int,
    pub demux_discontinuity: c_int,
    pub decoded_video: c_int,
    pub decoded_audio: c_int,
    pub displayed_pictures: c_int,
    pub lost_pictures: c_int,
    pub played_abuffers: c_int,
    pub lost_abuffers: c_int,
    pub sent_packets: c_int,
    pub sent_bytes: c_int,
    pub send_bitrate: c_float,
}

#[repr(C)]
pub struct DiscovererDescription {
    pub name: *mut c_char,
    pub longname: *mut c_char,
    pub category: c_int,
}

#[repr(C)]
pub struct RendererDescription {
    pub name: *mut c_char,
    pub longname: *mut c_char,
}

#[repr(C)]
pub struct DialogCallbacks {
    pub display_error: unsafe extern "C" fn(data: *mut c_void, title: *const c_char, text: *const c_char),
    pub display_login: unsafe extern "C" fn(
        data: *mut c_void,
        id: *mut DialogId,
        title: *const c_char,
        text: *const c_char,
        default_username: *const c_char,
        ask_store: bool,
    ),
    pub display_question: unsafe extern "C" fn(
        data: *mut c_void,
        id: *mut DialogId,
        title: *const c_char,
        text: *const c_char,
        kind: c_int,
        cancel: *const c_char,
        action1: *const c_char,
        action2: *const c_char,
    ),
    pub display_progress: unsafe extern "C" fn(
        data: *mut c_void,
        id: *mut DialogId,
        title: *const c_char,
        text: *const c_char,
        indeterminate: bool,
        position: c_float,
        cancel: *const c_char,
    ),
    pub cancel: unsafe extern "C" fn(data: *mut c_void, id: *mut DialogId),
    pub update_progress:
        unsafe extern "C" fn(data: *mut c_void, id: *mut DialogId, position: c_float, text: *const c_char),
}

/// `vlc_value_t` from libvlccore: an 8-byte union passed by value.
#[repr(C)]
#[derive(Clone, Copy)]
pub union VlcValue {
    pub i_int: i64,
    pub b_bool: bool,
    pub f_float: c_float,
    pub psz_string: *mut c_char,
    pub p_address: *mut c_void,
}

/// `vlc_list_t` from libvlccore.
#[repr(C)]
pub struct VlcList {
    pub kind: c_int,
    pub count: c_int,
    pub values: *mut VlcValue,
}

pub mod var_type {
    pub const VOID: i32 = 0x0010;
    pub const BOOL: i32 = 0x0020;
    pub const INTEGER: i32 = 0x0030;
    pub const STRING: i32 = 0x0040;
    pub const FLOAT: i32 = 0x0050;
    pub const ADDRESS: i32 = 0x0070;
    pub const COORDS: i32 = 0x00A0;
    pub const TYPE_MASK: i32 = 0x00ff;
    pub const ISCOMMAND: i32 = 0x2000;
    pub const DOINHERIT: i32 = 0x8000;
}

macro_rules! api {
    ($( fn $name:ident( $($arg:ident : $ty:ty),* $(,)? ) $(-> $ret:ty)?; )*) => {
        /// Function table for libvlc.
        pub struct Api {
            $( pub $name: unsafe extern "C" fn($($ty),*) $(-> $ret)?, )*
            pub core: Option<CoreApi>,
            pub lib_dir: PathBuf,
            pub plugin_dir: Option<PathBuf>,
            _lib: Library,
        }

        impl Api {
            unsafe fn resolve(lib: Library, lib_dir: PathBuf, plugin_dir: Option<PathBuf>, core: Option<CoreApi>) -> Result<Self> {
                unsafe {
                    Ok(Self {
                        $( $name: *lib
                            .get::<unsafe extern "C" fn($($ty),*) $(-> $ret)?>(concat!(stringify!($name), "\0").as_bytes())
                            .with_context(|| concat!("missing libvlc symbol ", stringify!($name)))?, )*
                        core,
                        lib_dir,
                        plugin_dir,
                        _lib: lib,
                    })
                }
            }
        }
    };
}

api! {
    // core
    fn libvlc_new(argc: c_int, argv: *const *const c_char) -> *mut Instance;
    fn libvlc_release(p: *mut Instance);
    fn libvlc_get_version() -> *const c_char;
    fn libvlc_get_changeset() -> *const c_char;
    fn libvlc_errmsg() -> *const c_char;
    fn libvlc_set_user_agent(p: *mut Instance, name: *const c_char, http: *const c_char);
    fn libvlc_set_app_id(p: *mut Instance, id: *const c_char, version: *const c_char, icon: *const c_char);
    fn libvlc_free(ptr: *mut c_void);
    fn libvlc_log_set_file(p: *mut Instance, stream: *mut libc::FILE);
    fn libvlc_log_unset(p: *mut Instance);
    fn libvlc_event_attach(em: *mut EventManager, kind: c_int, cb: EventCallback, data: *mut c_void) -> c_int;
    fn libvlc_event_detach(em: *mut EventManager, kind: c_int, cb: EventCallback, data: *mut c_void);
    fn libvlc_dialog_set_callbacks(p: *mut Instance, cbs: *const DialogCallbacks, data: *mut c_void);
    fn libvlc_dialog_post_login(id: *mut DialogId, user: *const c_char, pass: *const c_char, store: bool) -> c_int;
    fn libvlc_dialog_post_action(id: *mut DialogId, action: c_int) -> c_int;
    fn libvlc_dialog_dismiss(id: *mut DialogId) -> c_int;

    // media
    fn libvlc_media_new_location(p: *mut Instance, mrl: *const c_char) -> *mut Media;
    fn libvlc_media_new_path(p: *mut Instance, path: *const c_char) -> *mut Media;
    fn libvlc_media_release(m: *mut Media);
    fn libvlc_media_retain(m: *mut Media);
    fn libvlc_media_add_option(m: *mut Media, opt: *const c_char);
    fn libvlc_media_get_mrl(m: *mut Media) -> *mut c_char;
    fn libvlc_media_get_meta(m: *mut Media, meta: c_uint) -> *mut c_char;
    fn libvlc_media_set_meta(m: *mut Media, meta: c_uint, value: *const c_char);
    fn libvlc_media_save_meta(m: *mut Media) -> c_int;
    fn libvlc_media_get_state(m: *mut Media) -> c_int;
    fn libvlc_media_get_stats(m: *mut Media, stats: *mut MediaStats) -> c_int;
    fn libvlc_media_subitems(m: *mut Media) -> *mut MediaList;
    fn libvlc_media_event_manager(m: *mut Media) -> *mut EventManager;
    fn libvlc_media_get_duration(m: *mut Media) -> i64;
    fn libvlc_media_parse_with_options(m: *mut Media, flags: c_int, timeout: c_int) -> c_int;
    fn libvlc_media_parse_stop(m: *mut Media);
    fn libvlc_media_get_parsed_status(m: *mut Media) -> c_int;
    fn libvlc_media_tracks_get(m: *mut Media, tracks: *mut *mut *mut MediaTrack) -> c_uint;
    fn libvlc_media_tracks_release(tracks: *mut *mut MediaTrack, count: c_uint);
    fn libvlc_media_get_type(m: *mut Media) -> c_int;
    fn libvlc_media_slaves_add(m: *mut Media, kind: c_int, priority: c_uint, uri: *const c_char) -> c_int;

    // media list
    fn libvlc_media_list_release(ml: *mut MediaList);
    fn libvlc_media_list_count(ml: *mut MediaList) -> c_int;
    fn libvlc_media_list_item_at_index(ml: *mut MediaList, i: c_int) -> *mut Media;
    fn libvlc_media_list_lock(ml: *mut MediaList);
    fn libvlc_media_list_unlock(ml: *mut MediaList);
    fn libvlc_media_list_event_manager(ml: *mut MediaList) -> *mut EventManager;

    // discovery
    fn libvlc_media_discoverer_new(p: *mut Instance, name: *const c_char) -> *mut MediaDiscoverer;
    fn libvlc_media_discoverer_start(md: *mut MediaDiscoverer) -> c_int;
    fn libvlc_media_discoverer_stop(md: *mut MediaDiscoverer);
    fn libvlc_media_discoverer_release(md: *mut MediaDiscoverer);
    fn libvlc_media_discoverer_media_list(md: *mut MediaDiscoverer) -> *mut MediaList;
    fn libvlc_media_discoverer_list_get(p: *mut Instance, cat: c_int, out: *mut *mut *mut DiscovererDescription) -> usize;
    fn libvlc_media_discoverer_list_release(list: *mut *mut DiscovererDescription, count: usize);

    // renderers
    fn libvlc_renderer_discoverer_new(p: *mut Instance, name: *const c_char) -> *mut RendererDiscoverer;
    fn libvlc_renderer_discoverer_release(rd: *mut RendererDiscoverer);
    fn libvlc_renderer_discoverer_start(rd: *mut RendererDiscoverer) -> c_int;
    fn libvlc_renderer_discoverer_stop(rd: *mut RendererDiscoverer);
    fn libvlc_renderer_discoverer_event_manager(rd: *mut RendererDiscoverer) -> *mut EventManager;
    fn libvlc_renderer_discoverer_list_get(p: *mut Instance, out: *mut *mut *mut RendererDescription) -> usize;
    fn libvlc_renderer_discoverer_list_release(list: *mut *mut RendererDescription, count: usize);
    fn libvlc_renderer_item_hold(item: *mut RendererItem) -> *mut RendererItem;
    fn libvlc_renderer_item_release(item: *mut RendererItem);
    fn libvlc_renderer_item_name(item: *const RendererItem) -> *const c_char;
    fn libvlc_renderer_item_type(item: *const RendererItem) -> *const c_char;
    fn libvlc_renderer_item_flags(item: *const RendererItem) -> c_int;
    fn libvlc_media_player_set_renderer(mp: *mut MediaPlayer, item: *mut RendererItem) -> c_int;

    // media player
    fn libvlc_media_player_new(p: *mut Instance) -> *mut MediaPlayer;
    fn libvlc_media_player_release(mp: *mut MediaPlayer);
    fn libvlc_media_player_set_media(mp: *mut MediaPlayer, m: *mut Media);
    fn libvlc_media_player_get_media(mp: *mut MediaPlayer) -> *mut Media;
    fn libvlc_media_player_event_manager(mp: *mut MediaPlayer) -> *mut EventManager;
    fn libvlc_media_player_is_playing(mp: *mut MediaPlayer) -> c_int;
    fn libvlc_media_player_play(mp: *mut MediaPlayer) -> c_int;
    fn libvlc_media_player_set_pause(mp: *mut MediaPlayer, pause: c_int);
    fn libvlc_media_player_pause(mp: *mut MediaPlayer);
    fn libvlc_media_player_stop(mp: *mut MediaPlayer);
    fn libvlc_media_player_get_length(mp: *mut MediaPlayer) -> i64;
    fn libvlc_media_player_get_time(mp: *mut MediaPlayer) -> i64;
    fn libvlc_media_player_set_time(mp: *mut MediaPlayer, ms: i64);
    fn libvlc_media_player_get_position(mp: *mut MediaPlayer) -> c_float;
    fn libvlc_media_player_set_position(mp: *mut MediaPlayer, pos: c_float);
    fn libvlc_media_player_set_chapter(mp: *mut MediaPlayer, chapter: c_int);
    fn libvlc_media_player_get_chapter(mp: *mut MediaPlayer) -> c_int;
    fn libvlc_media_player_get_chapter_count(mp: *mut MediaPlayer) -> c_int;
    fn libvlc_media_player_previous_chapter(mp: *mut MediaPlayer);
    fn libvlc_media_player_next_chapter(mp: *mut MediaPlayer);
    fn libvlc_media_player_set_title(mp: *mut MediaPlayer, title: c_int);
    fn libvlc_media_player_get_title(mp: *mut MediaPlayer) -> c_int;
    fn libvlc_media_player_get_title_count(mp: *mut MediaPlayer) -> c_int;
    fn libvlc_media_player_get_full_title_descriptions(mp: *mut MediaPlayer, out: *mut *mut *mut TitleDescription) -> c_int;
    fn libvlc_title_descriptions_release(list: *mut *mut TitleDescription, count: c_uint);
    fn libvlc_media_player_get_full_chapter_descriptions(mp: *mut MediaPlayer, title: c_int, out: *mut *mut *mut ChapterDescription) -> c_int;
    fn libvlc_chapter_descriptions_release(list: *mut *mut ChapterDescription, count: c_uint);
    fn libvlc_media_player_get_rate(mp: *mut MediaPlayer) -> c_float;
    fn libvlc_media_player_set_rate(mp: *mut MediaPlayer, rate: c_float) -> c_int;
    fn libvlc_media_player_get_state(mp: *mut MediaPlayer) -> c_int;
    fn libvlc_media_player_has_vout(mp: *mut MediaPlayer) -> c_uint;
    fn libvlc_media_player_is_seekable(mp: *mut MediaPlayer) -> c_int;
    fn libvlc_media_player_can_pause(mp: *mut MediaPlayer) -> c_int;
    fn libvlc_media_player_program_scrambled(mp: *mut MediaPlayer) -> c_int;
    fn libvlc_media_player_next_frame(mp: *mut MediaPlayer);
    fn libvlc_media_player_navigate(mp: *mut MediaPlayer, nav: c_uint);
    fn libvlc_media_player_set_video_title_display(mp: *mut MediaPlayer, position: c_int, timeout: c_uint);
    fn libvlc_media_player_add_slave(mp: *mut MediaPlayer, kind: c_int, uri: *const c_char, select: bool) -> c_int;
    fn libvlc_media_player_set_equalizer(mp: *mut MediaPlayer, eq: *mut Equalizer) -> c_int;
    fn libvlc_media_player_set_role(mp: *mut MediaPlayer, role: c_uint) -> c_int;
    fn libvlc_track_description_list_release(list: *mut TrackDescription);

    // video
    fn libvlc_video_set_callbacks(mp: *mut MediaPlayer, lock: Option<VideoLockCb>, unlock: Option<VideoUnlockCb>, display: Option<VideoDisplayCb>, opaque: *mut c_void);
    fn libvlc_video_set_format_callbacks(mp: *mut MediaPlayer, setup: Option<VideoFormatCb>, cleanup: Option<VideoCleanupCb>);
    fn libvlc_video_get_size(mp: *mut MediaPlayer, num: c_uint, w: *mut c_uint, h: *mut c_uint) -> c_int;
    fn libvlc_video_get_scale(mp: *mut MediaPlayer) -> c_float;
    fn libvlc_video_set_scale(mp: *mut MediaPlayer, scale: c_float);
    fn libvlc_video_get_aspect_ratio(mp: *mut MediaPlayer) -> *mut c_char;
    fn libvlc_video_set_aspect_ratio(mp: *mut MediaPlayer, aspect: *const c_char);
    fn libvlc_video_get_crop_geometry(mp: *mut MediaPlayer) -> *mut c_char;
    fn libvlc_video_set_crop_geometry(mp: *mut MediaPlayer, geometry: *const c_char);
    fn libvlc_video_get_spu(mp: *mut MediaPlayer) -> c_int;
    fn libvlc_video_get_spu_count(mp: *mut MediaPlayer) -> c_int;
    fn libvlc_video_get_spu_description(mp: *mut MediaPlayer) -> *mut TrackDescription;
    fn libvlc_video_set_spu(mp: *mut MediaPlayer, id: c_int) -> c_int;
    fn libvlc_video_get_spu_delay(mp: *mut MediaPlayer) -> i64;
    fn libvlc_video_set_spu_delay(mp: *mut MediaPlayer, delay_us: i64) -> c_int;
    fn libvlc_video_get_track(mp: *mut MediaPlayer) -> c_int;
    fn libvlc_video_get_track_description(mp: *mut MediaPlayer) -> *mut TrackDescription;
    fn libvlc_video_set_track(mp: *mut MediaPlayer, id: c_int) -> c_int;
    fn libvlc_video_get_teletext(mp: *mut MediaPlayer) -> c_int;
    fn libvlc_video_set_teletext(mp: *mut MediaPlayer, page: c_int);
    fn libvlc_video_take_snapshot(mp: *mut MediaPlayer, num: c_uint, path: *const c_char, w: c_uint, h: c_uint) -> c_int;
    fn libvlc_video_set_deinterlace(mp: *mut MediaPlayer, mode: *const c_char);
    fn libvlc_video_get_marquee_int(mp: *mut MediaPlayer, option: c_uint) -> c_int;
    fn libvlc_video_set_marquee_int(mp: *mut MediaPlayer, option: c_uint, value: c_int);
    fn libvlc_video_set_marquee_string(mp: *mut MediaPlayer, option: c_uint, value: *const c_char);
    fn libvlc_video_set_logo_int(mp: *mut MediaPlayer, option: c_uint, value: c_int);
    fn libvlc_video_set_logo_string(mp: *mut MediaPlayer, option: c_uint, value: *const c_char);
    fn libvlc_video_get_adjust_int(mp: *mut MediaPlayer, option: c_uint) -> c_int;
    fn libvlc_video_set_adjust_int(mp: *mut MediaPlayer, option: c_uint, value: c_int);
    fn libvlc_video_get_adjust_float(mp: *mut MediaPlayer, option: c_uint) -> c_float;
    fn libvlc_video_set_adjust_float(mp: *mut MediaPlayer, option: c_uint, value: c_float);
    fn libvlc_video_new_viewpoint() -> *mut Viewpoint;
    fn libvlc_video_update_viewpoint(mp: *mut MediaPlayer, vp: *const Viewpoint, absolute: bool) -> c_int;

    // audio
    fn libvlc_audio_output_list_get(p: *mut Instance) -> *mut AudioOutput;
    fn libvlc_audio_output_list_release(list: *mut AudioOutput);
    fn libvlc_audio_output_set(mp: *mut MediaPlayer, name: *const c_char) -> c_int;
    fn libvlc_audio_output_device_enum(mp: *mut MediaPlayer) -> *mut AudioOutputDevice;
    fn libvlc_audio_output_device_list_release(list: *mut AudioOutputDevice);
    fn libvlc_audio_output_device_set(mp: *mut MediaPlayer, module: *const c_char, device: *const c_char);
    fn libvlc_audio_output_device_get(mp: *mut MediaPlayer) -> *mut c_char;
    fn libvlc_audio_toggle_mute(mp: *mut MediaPlayer);
    fn libvlc_audio_get_mute(mp: *mut MediaPlayer) -> c_int;
    fn libvlc_audio_set_mute(mp: *mut MediaPlayer, mute: c_int);
    fn libvlc_audio_get_volume(mp: *mut MediaPlayer) -> c_int;
    fn libvlc_audio_set_volume(mp: *mut MediaPlayer, volume: c_int) -> c_int;
    fn libvlc_audio_get_track_count(mp: *mut MediaPlayer) -> c_int;
    fn libvlc_audio_get_track_description(mp: *mut MediaPlayer) -> *mut TrackDescription;
    fn libvlc_audio_get_track(mp: *mut MediaPlayer) -> c_int;
    fn libvlc_audio_set_track(mp: *mut MediaPlayer, id: c_int) -> c_int;
    fn libvlc_audio_get_channel(mp: *mut MediaPlayer) -> c_int;
    fn libvlc_audio_set_channel(mp: *mut MediaPlayer, channel: c_int) -> c_int;
    fn libvlc_audio_get_delay(mp: *mut MediaPlayer) -> i64;
    fn libvlc_audio_set_delay(mp: *mut MediaPlayer, delay_us: i64) -> c_int;

    // equalizer
    fn libvlc_audio_equalizer_get_preset_count() -> c_uint;
    fn libvlc_audio_equalizer_get_preset_name(index: c_uint) -> *const c_char;
    fn libvlc_audio_equalizer_get_band_count() -> c_uint;
    fn libvlc_audio_equalizer_get_band_frequency(index: c_uint) -> c_float;
    fn libvlc_audio_equalizer_new() -> *mut Equalizer;
    fn libvlc_audio_equalizer_new_from_preset(index: c_uint) -> *mut Equalizer;
    fn libvlc_audio_equalizer_release(eq: *mut Equalizer);
    fn libvlc_audio_equalizer_set_preamp(eq: *mut Equalizer, preamp: c_float) -> c_int;
    fn libvlc_audio_equalizer_get_preamp(eq: *mut Equalizer) -> c_float;
    fn libvlc_audio_equalizer_set_amp_at_index(eq: *mut Equalizer, amp: c_float, band: c_uint) -> c_int;
    fn libvlc_audio_equalizer_get_amp_at_index(eq: *mut Equalizer, band: c_uint) -> c_float;
}

/// Private libvlccore entry points used to reach VLC object variables that
/// libvlc 3 does not expose: live audio filter parameters, the input's
/// "record" switch, program selection.
pub struct CoreApi {
    pub var_create: unsafe extern "C" fn(obj: *mut VlcObject, name: *const c_char, kind: c_int) -> c_int,
    pub var_type: unsafe extern "C" fn(obj: *mut VlcObject, name: *const c_char) -> c_int,
    pub var_set_checked:
        unsafe extern "C" fn(obj: *mut VlcObject, name: *const c_char, kind: c_int, value: VlcValue) -> c_int,
    pub var_get_checked:
        unsafe extern "C" fn(obj: *mut VlcObject, name: *const c_char, kind: c_int, value: *mut VlcValue) -> c_int,
    pub list_children: unsafe extern "C" fn(obj: *mut VlcObject) -> *mut VlcList,
    pub list_release: unsafe extern "C" fn(list: *mut VlcList),
    _lib: Library,
}

impl CoreApi {
    unsafe fn load(path: &Path) -> Result<Self> {
        unsafe {
            let lib = Library::new(path)?;
            Ok(Self {
                var_create: *lib.get(b"var_Create\0")?,
                var_type: *lib.get(b"var_Type\0")?,
                var_set_checked: *lib.get(b"var_SetChecked\0")?,
                var_get_checked: *lib.get(b"var_GetChecked\0")?,
                list_children: *lib.get(b"vlc_list_children\0")?,
                list_release: *lib.get(b"vlc_list_release\0")?,
                _lib: lib,
            })
        }
    }
}

/// Where a usable libvlc lives: the library file and its plugin directory.
#[derive(Debug, Clone)]
pub struct Location {
    pub lib: PathBuf,
    pub core: Option<PathBuf>,
    pub plugins: Option<PathBuf>,
}

/// Candidate libvlc locations, most specific first. `NOTVLC_LIBVLC` may name
/// either the library file or a VLC install directory.
pub fn candidate_locations() -> Vec<Location> {
    fn in_dir(lib_dir: PathBuf, plugins: PathBuf) -> Location {
        let (lib, core) = if cfg!(target_os = "macos") {
            (lib_dir.join("libvlc.dylib"), lib_dir.join("libvlccore.dylib"))
        } else if cfg!(windows) {
            (lib_dir.join("libvlc.dll"), lib_dir.join("libvlccore.dll"))
        } else {
            (lib_dir.join("libvlc.so.5"), lib_dir.join("libvlccore.so.9"))
        };
        Location { lib, core: Some(core), plugins: Some(plugins) }
    }
    let mut out = Vec::new();
    if let Ok(p) = std::env::var("NOTVLC_LIBVLC") {
        let p = PathBuf::from(p);
        if p.is_file() {
            let dir = p.parent().map(Path::to_path_buf).unwrap_or_default();
            let core = dir.join(if cfg!(target_os = "macos") { "libvlccore.dylib" } else { "libvlccore.so.9" });
            out.push(Location {
                lib: p,
                core: Some(core),
                plugins: std::env::var_os("VLC_PLUGIN_PATH").map(PathBuf::from),
            });
        } else if p.join("Contents/MacOS/lib").is_dir() {
            out.push(in_dir(p.join("Contents/MacOS/lib"), p.join("Contents/MacOS/plugins")));
        } else {
            out.push(in_dir(p.clone(), p.join("plugins")));
        }
    }
    if cfg!(target_os = "macos") {
        let mut apps = vec![PathBuf::from("/Applications/VLC.app")];
        if let Some(home) = std::env::var_os("HOME") {
            apps.push(PathBuf::from(home).join("Applications/VLC.app"));
        }
        for app in apps {
            out.push(in_dir(app.join("Contents/MacOS/lib"), app.join("Contents/MacOS/plugins")));
        }
    } else if cfg!(windows) {
        for base in ["C:\\Program Files\\VideoLAN\\VLC", "C:\\Program Files (x86)\\VideoLAN\\VLC"] {
            out.push(in_dir(PathBuf::from(base), PathBuf::from(base).join("plugins")));
        }
    } else {
        for dir in
            ["/usr/lib", "/usr/lib/x86_64-linux-gnu", "/usr/lib/aarch64-linux-gnu", "/usr/lib64", "/usr/local/lib"]
        {
            out.push(in_dir(PathBuf::from(dir), PathBuf::from(dir).join("vlc/plugins")));
        }
    }
    out
}

/// Loads libvlc from the first location that works.
pub fn load() -> Result<Api> {
    let mut errors = Vec::new();
    for loc in candidate_locations() {
        if !loc.lib.exists() {
            continue;
        }
        match unsafe { load_from(&loc) } {
            Ok(api) => return Ok(api),
            Err(e) => errors.push(format!("{}: {e:#}", loc.lib.display())),
        }
    }
    if errors.is_empty() {
        Err(anyhow!(
            "libvlc was not found. Install VLC 3 (https://www.videolan.org/vlc/) or set NOTVLC_LIBVLC to its location."
        ))
    } else {
        Err(anyhow!("libvlc could not be loaded:\n{}", errors.join("\n")))
    }
}

unsafe fn load_from(loc: &Location) -> Result<Api> {
    unsafe {
        if let Some(plugins) = &loc.plugins
            && plugins.is_dir()
        {
            // SAFETY: called before any libvlc instance exists, during start-up.
            std::env::set_var("VLC_PLUGIN_PATH", plugins);
        }
        // libvlc refers to libvlccore through @rpath on macOS, which a host
        // executable cannot resolve. Loading libvlccore first by absolute path
        // lets the loader reuse it.
        let core = loc.core.as_deref().filter(|p| p.exists()).and_then(|p| match CoreApi::load(p) {
            Ok(c) => Some(c),
            Err(e) => {
                log::warn!("libvlccore extras unavailable ({e:#}); live effect changes will apply on next file");
                None
            }
        });
        let lib = Library::new(&loc.lib)?;
        let dir = loc.lib.parent().map(Path::to_path_buf).unwrap_or_default();
        Api::resolve(lib, dir, loc.plugins.clone(), core)
    }
}
