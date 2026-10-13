//! Receives "open documents" Apple Events, so double-clicking a media file
//! in Finder (or dropping it on the Dock icon) opens it in the running app.
//!
//! The handler is installed when the application posts
//! `NSApplicationWillFinishLaunchingNotification`, Apple's recommended point:
//! early enough to catch the file the app was launched with.

use std::path::PathBuf;
use std::ptr::NonNull;
use std::sync::OnceLock;

use block2::RcBlock;
use crossbeam_channel::Sender;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{ClassType, define_class, msg_send, sel};
use objc2_foundation::{
    NSAppleEventDescriptor, NSAppleEventManager, NSNotification, NSNotificationCenter, NSString, NSURL,
};

static SENDER: OnceLock<Sender<Vec<PathBuf>>> = OnceLock::new();

const fn four(code: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*code)
}

const CORE_EVENT_CLASS: u32 = four(b"aevt");
const OPEN_DOCUMENTS: u32 = four(b"odoc");
const DIRECT_OBJECT: u32 = four(b"----");

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "NotVlcOpenDocumentsHandler"]
    struct OpenHandler;

    impl OpenHandler {
        #[unsafe(method(handleOpen:withReply:))]
        fn handle_open(&self, event: &NSAppleEventDescriptor, _reply: &NSAppleEventDescriptor) {
            let paths = unsafe { file_paths(event) };
            log::debug!("open documents event: {paths:?}");
            if !paths.is_empty()
                && let Some(tx) = SENDER.get()
            {
                let _ = tx.send(paths);
            }
        }
    }
);

// Raw messages: the typed bindings need the CoreServices crate for the
// four-character code types, which are plain u32s here.
unsafe fn file_paths(event: &NSAppleEventDescriptor) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let list: Option<Retained<NSAppleEventDescriptor>> =
        unsafe { msg_send![event, paramDescriptorForKeyword: DIRECT_OBJECT] };
    let Some(list) = list else { return out };
    let n: isize = unsafe { msg_send![&*list, numberOfItems] };
    for i in 1..=n {
        let item: Option<Retained<NSAppleEventDescriptor>> = unsafe { msg_send![&*list, descriptorAtIndex: i] };
        let Some(item) = item else { continue };
        // A file reference descriptor converts straight to an NSURL.
        let url: Option<Retained<NSURL>> = unsafe { msg_send![&*item, fileURLValue] };
        let path: Option<Retained<NSString>> = url.and_then(|u| unsafe { msg_send![&*u, path] });
        if let Some(p) = path {
            out.push(PathBuf::from(p.to_string()));
        }
    }
    out
}

/// Starts listening. Call once from `main`, before the event loop runs.
pub fn install(tx: Sender<Vec<PathBuf>>) {
    if SENDER.set(tx).is_err() {
        return;
    }
    let block = RcBlock::new(|_note: NonNull<NSNotification>| {
        let handler: Retained<OpenHandler> = unsafe { msg_send![OpenHandler::class(), new] };
        let manager = NSAppleEventManager::sharedAppleEventManager();
        let target: &AnyObject = &handler;
        let _: () = unsafe {
            msg_send![
                &*manager,
                setEventHandler: target,
                andSelector: sel!(handleOpen:withReply:),
                forEventClass: CORE_EVENT_CLASS,
                andEventID: OPEN_DOCUMENTS
            ]
        };
        log::debug!("open documents handler installed");
        // The manager does not retain its handler.
        std::mem::forget(handler);
    });
    let name = NSString::from_str("NSApplicationWillFinishLaunchingNotification");
    let center = NSNotificationCenter::defaultCenter();
    let token = unsafe { center.addObserverForName_object_queue_usingBlock(Some(&name), None, None, &block) };
    std::mem::forget(token);
}
