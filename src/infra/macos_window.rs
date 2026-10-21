//! Native window chrome on macOS. The window keeps AppKit's frame (rounded
//! corners, shadow, traffic lights) with a transparent title bar, and the
//! app's own title bar is drawn underneath it. AppKit lays the traffic lights
//! out for its 28pt title bar, so they are moved to sit centred in the taller
//! bar, the way Electron's `trafficLightPosition` does. AppKit puts them back
//! on resize and when leaving fullscreen, so `place` runs every frame and
//! only touches the views when something moved. `set_dock_icon` swaps the
//! Dock icon, which follows the accent colour.

use objc2::encode::{Encode, Encoding};
use objc2::msg_send;
use objc2::rc::{Allocated, Retained};
use objc2::runtime::AnyObject;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
struct CGSize {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

unsafe impl Encode for CGPoint {
    const ENCODING: Encoding = Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
}

unsafe impl Encode for CGSize {
    const ENCODING: Encoding = Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]);
}

unsafe impl Encode for CGRect {
    const ENCODING: Encoding = Encoding::Struct("CGRect", &[CGPoint::ENCODING, CGSize::ENCODING]);
}

/// `NSWindowStyleMaskFullScreen`.
const FULLSCREEN: usize = 1 << 14;

/// The close, minimise and zoom buttons of the app's window.
pub struct TrafficLights {
    window: Retained<AnyObject>,
    /// Distance between buttons, as AppKit spaced them.
    spacing: Option<f64>,
}

impl TrafficLights {
    /// The window behind eframe's creation context; None if it is not AppKit.
    pub fn new(handle: &impl HasWindowHandle) -> Option<Self> {
        let RawWindowHandle::AppKit(h) = handle.window_handle().ok()?.as_raw() else { return None };
        let view = h.ns_view.as_ptr() as *mut AnyObject;
        let window: Option<Retained<AnyObject>> = unsafe { msg_send![view, window] };
        Some(Self { window: window?, spacing: None })
    }

    /// Centres the buttons in a title bar `bar_height` points tall, with the
    /// same margin on the left as above and below, or hides them.
    pub fn place(&mut self, bar_height: f64, visible: bool) {
        // Safety: AppKit calls on the main thread (eframe's update), on views
        // the window owns.
        unsafe {
            let window = &*self.window;
            let buttons: Vec<Retained<AnyObject>> =
                (0usize..3).filter_map(|kind| msg_send![window, standardWindowButton: kind]).collect();
            if buttons.len() != 3 {
                return;
            }
            for b in &buttons {
                let hidden: bool = msg_send![&**b, isHidden];
                if hidden == visible {
                    let _: () = msg_send![&**b, setHidden: !visible];
                }
            }
            let mask: usize = msg_send![window, styleMask];
            if !visible || mask & FULLSCREEN != 0 {
                return;
            }
            let first: CGRect = msg_send![&*buttons[0], frame];
            let spacing = *self.spacing.get_or_insert_with(|| {
                let second: CGRect = msg_send![&*buttons[1], frame];
                second.origin.x - first.origin.x
            });
            // The buttons live in the title bar view, inside its container.
            let titlebar: Option<Retained<AnyObject>> = msg_send![&*buttons[0], superview];
            let Some(titlebar) = titlebar else { return };
            let container: Option<Retained<AnyObject>> = msg_send![&*titlebar, superview];
            let Some(container) = container else { return };
            let frame: CGRect = msg_send![window, frame];
            let current: CGRect = msg_send![&*container, frame];
            let wanted = CGRect {
                origin: CGPoint { x: current.origin.x, y: frame.size.height - bar_height },
                size: CGSize { width: current.size.width, height: bar_height },
            };
            if current != wanted {
                log::debug!(
                    "title bar container {}x{} -> {}x{}, window {}x{}",
                    current.size.width,
                    current.size.height,
                    wanted.size.width,
                    wanted.size.height,
                    frame.size.width,
                    frame.size.height
                );
                let _: () = msg_send![&*container, setFrame: wanted];
            }
            let margin = ((bar_height - first.size.height) / 2.0).round();
            for (i, b) in buttons.iter().enumerate() {
                let f: CGRect = msg_send![&**b, frame];
                let origin = CGPoint { x: margin + i as f64 * spacing, y: margin };
                if f.origin != origin {
                    log::debug!(
                        "traffic light {i}: ({}, {}) {}x{} -> ({}, {})",
                        f.origin.x,
                        f.origin.y,
                        f.size.width,
                        f.size.height,
                        origin.x,
                        origin.y
                    );
                    let _: () = msg_send![&**b, setFrameOrigin: origin];
                }
            }
        }
    }
}

/// Replaces the app's Dock icon while it runs; the bundle's icon file still
/// shows when it is not running. `rgba` is straight (not premultiplied).
pub fn set_dock_icon(rgba: &[u8], width: u32, height: u32) {
    let Some(image) = image::RgbaImage::from_raw(width, height, rgba.to_vec()) else { return };
    let mut png = Vec::new();
    if let Err(e) = image.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png) {
        log::warn!("Dock icon not encoded: {e}");
        return;
    }
    // Safety: AppKit calls on the main thread (eframe's update); the bytes
    // outlive the NSData, which copies them.
    unsafe {
        let data: Option<Retained<AnyObject>> = msg_send![
            objc2::class!(NSData),
            dataWithBytes: png.as_ptr().cast::<std::ffi::c_void>(),
            length: png.len()
        ];
        let Some(data) = data else { return };
        let alloc: Allocated<AnyObject> = msg_send![objc2::class!(NSImage), alloc];
        let icon: Option<Retained<AnyObject>> = msg_send![alloc, initWithData: &*data];
        let Some(icon) = icon else {
            log::warn!("Dock icon not decoded by AppKit");
            return;
        };
        let app: Option<Retained<AnyObject>> = msg_send![objc2::class!(NSApplication), sharedApplication];
        if let Some(app) = app {
            let _: () = msg_send![&*app, setApplicationIconImage: &*icon];
        }
    }
}
