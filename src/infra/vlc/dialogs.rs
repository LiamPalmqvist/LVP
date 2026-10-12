//! Bridges libvlc's dialog callbacks (logins for network shares, questions,
//! progress, errors) to engine events, and replies back.

use std::collections::HashMap;
use std::ffi::{c_char, c_float, c_int, c_void};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crossbeam_channel::Sender;
use parking_lot::Mutex;

use super::core::{borrowed, cstring};
use super::ffi::{self, Api, DialogCallbacks, DialogId};
use crate::app::ports::{DialogReply, DialogRequest, EngineEvent};

pub struct DialogBridge {
    api: Arc<Api>,
    tx: Sender<EngineEvent>,
    next: AtomicU64,
    /// Live dialog handles by our id. Removed on reply or cancel.
    ids: Mutex<HashMap<u64, usize>>,
}

impl DialogBridge {
    pub fn new(api: Arc<Api>, tx: Sender<EngineEvent>) -> Arc<Self> {
        Arc::new(Self { api, tx, next: AtomicU64::new(1), ids: Mutex::new(HashMap::new()) })
    }

    /// Registers the callbacks on an instance. The bridge must outlive it.
    pub fn attach(self: &Arc<Self>, inst: *mut ffi::Instance) {
        static CBS: DialogCallbacks = DialogCallbacks {
            display_error,
            display_login,
            display_question,
            display_progress,
            cancel,
            update_progress,
        };
        unsafe { (self.api.libvlc_dialog_set_callbacks)(inst, &CBS, Arc::as_ptr(self) as *mut c_void) };
    }

    pub fn detach(&self, inst: *mut ffi::Instance) {
        unsafe { (self.api.libvlc_dialog_set_callbacks)(inst, std::ptr::null(), std::ptr::null_mut()) };
    }

    fn register(&self, id: *mut DialogId) -> u64 {
        let n = self.next.fetch_add(1, Ordering::Relaxed);
        self.ids.lock().insert(n, id as usize);
        n
    }

    fn lookup(&self, id: *mut DialogId) -> Option<u64> {
        self.ids.lock().iter().find(|(_, v)| **v == id as usize).map(|(k, _)| *k)
    }

    pub fn reply(&self, id: u64, reply: DialogReply) {
        let Some(ptr) = self.ids.lock().remove(&id) else { return };
        let ptr = ptr as *mut DialogId;
        unsafe {
            match reply {
                DialogReply::Login { username, password, store } => {
                    let u = cstring(&username);
                    let p = cstring(&password);
                    (self.api.libvlc_dialog_post_login)(ptr, u.as_ptr(), p.as_ptr(), store);
                }
                DialogReply::Action(a) => {
                    (self.api.libvlc_dialog_post_action)(ptr, a);
                }
                DialogReply::Dismiss => {
                    (self.api.libvlc_dialog_dismiss)(ptr);
                }
            }
        }
    }
}

unsafe fn bridge<'a>(data: *mut c_void) -> Option<&'a DialogBridge> {
    unsafe { (data as *const DialogBridge).as_ref() }
}

fn s(p: *const c_char) -> String {
    unsafe { borrowed(p) }.unwrap_or_default()
}

fn opt(p: *const c_char) -> Option<String> {
    unsafe { borrowed(p) }.filter(|s| !s.is_empty())
}

unsafe extern "C" fn display_error(data: *mut c_void, title: *const c_char, text: *const c_char) {
    if let Some(b) = unsafe { bridge(data) } {
        let _ = b.tx.send(EngineEvent::Dialog(DialogRequest::Error { title: s(title), text: s(text) }));
    }
}

unsafe extern "C" fn display_login(
    data: *mut c_void,
    id: *mut DialogId,
    title: *const c_char,
    text: *const c_char,
    user: *const c_char,
    ask_store: bool,
) {
    if let Some(b) = unsafe { bridge(data) } {
        let n = b.register(id);
        let _ = b.tx.send(EngineEvent::Dialog(DialogRequest::Login {
            id: n,
            title: s(title),
            text: s(text),
            default_username: s(user),
            ask_store,
        }));
    }
}

unsafe extern "C" fn display_question(
    data: *mut c_void,
    id: *mut DialogId,
    title: *const c_char,
    text: *const c_char,
    _kind: c_int,
    cancel: *const c_char,
    action1: *const c_char,
    action2: *const c_char,
) {
    if let Some(b) = unsafe { bridge(data) } {
        let n = b.register(id);
        let _ = b.tx.send(EngineEvent::Dialog(DialogRequest::Question {
            id: n,
            title: s(title),
            text: s(text),
            cancel: opt(cancel).unwrap_or_else(|| "Cancel".into()),
            action1: opt(action1),
            action2: opt(action2),
        }));
    }
}

unsafe extern "C" fn display_progress(
    data: *mut c_void,
    id: *mut DialogId,
    title: *const c_char,
    text: *const c_char,
    indeterminate: bool,
    position: c_float,
    cancel: *const c_char,
) {
    if let Some(b) = unsafe { bridge(data) } {
        let n = b.register(id);
        let _ = b.tx.send(EngineEvent::Dialog(DialogRequest::Progress {
            id: n,
            title: s(title),
            text: s(text),
            position: (!indeterminate).then_some(position),
            cancel: opt(cancel),
        }));
    }
}

unsafe extern "C" fn cancel(data: *mut c_void, id: *mut DialogId) {
    if let Some(b) = unsafe { bridge(data) }
        && let Some(n) = b.lookup(id)
    {
        b.ids.lock().remove(&n);
        unsafe { (b.api.libvlc_dialog_dismiss)(id) };
        let _ = b.tx.send(EngineEvent::Dialog(DialogRequest::Cancel { id: n }));
    }
}

unsafe extern "C" fn update_progress(data: *mut c_void, id: *mut DialogId, position: c_float, text: *const c_char) {
    if let Some(b) = unsafe { bridge(data) }
        && let Some(n) = b.lookup(id)
    {
        let _ = b.tx.send(EngineEvent::Dialog(DialogRequest::UpdateProgress { id: n, position, text: s(text) }));
    }
}
