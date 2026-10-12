//! The hand-off point for decoded pictures between the engine and the UI.
//!
//! The engine renders into a back buffer and publishes it with
//! [`VideoOutput::publish`]; the UI reads the front buffer under the same lock
//! when the serial number changes. Two buffers mean the decoder never writes
//! into the picture being uploaded.

use parking_lot::Mutex;

pub struct Frame {
    pub width: u32,
    pub height: u32,
    /// Bytes per row; at least `width * 4`.
    pub pitch: u32,
    /// RGBA8 pixels, `pitch * height` bytes.
    pub pixels: Vec<u8>,
}

struct Buffers {
    width: u32,
    height: u32,
    pitch: u32,
    front: Vec<u8>,
    back: Vec<u8>,
    serial: u64,
    has_frame: bool,
}

pub struct VideoOutput {
    buffers: Mutex<Buffers>,
    notify: Mutex<Option<Box<dyn Fn() + Send + Sync>>>,
}

impl Default for VideoOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoOutput {
    pub fn new() -> Self {
        Self {
            buffers: Mutex::new(Buffers {
                width: 0,
                height: 0,
                pitch: 0,
                front: Vec::new(),
                back: Vec::new(),
                serial: 0,
                has_frame: false,
            }),
            notify: Mutex::new(None),
        }
    }

    /// Called whenever a new frame is published, typically to request a repaint.
    pub fn set_notify(&self, f: impl Fn() + Send + Sync + 'static) {
        *self.notify.lock() = Some(Box::new(f));
    }

    /// Resizes both buffers. Returns the pitch to report to the decoder.
    pub fn configure(&self, width: u32, height: u32) -> u32 {
        let pitch = (width * 4).div_ceil(64) * 64;
        let len = pitch as usize * height as usize + 64;
        let mut b = self.buffers.lock();
        b.width = width;
        b.height = height;
        b.pitch = pitch;
        b.front = vec![0; len];
        b.back = vec![0; len];
        b.has_frame = false;
        b.serial += 1;
        pitch
    }

    /// Pointer to the back buffer for the decoder to write into. Stays valid
    /// until the next `configure` call.
    pub fn back_ptr(&self) -> *mut u8 {
        self.buffers.lock().back.as_mut_ptr()
    }

    /// Swaps the freshly written back buffer to the front.
    pub fn publish(&self) {
        {
            let mut b = self.buffers.lock();
            let Buffers { front, back, .. } = &mut *b;
            std::mem::swap(front, back);
            b.serial += 1;
            b.has_frame = true;
        }
        if let Some(n) = self.notify.lock().as_ref() {
            n();
        }
    }

    /// Forgets the current picture (playback stopped).
    pub fn clear(&self) {
        let mut b = self.buffers.lock();
        b.has_frame = false;
        b.serial += 1;
    }

    pub fn serial(&self) -> u64 {
        self.buffers.lock().serial
    }

    pub fn size(&self) -> Option<(u32, u32)> {
        let b = self.buffers.lock();
        b.has_frame.then_some((b.width, b.height))
    }

    /// Runs `f` with the current front frame, if any, holding the lock.
    pub fn with_front<R>(&self, f: impl FnOnce(u64, &[u8], u32, u32, u32) -> R) -> Option<R> {
        let b = self.buffers.lock();
        if !b.has_frame {
            return None;
        }
        Some(f(b.serial, &b.front, b.width, b.height, b.pitch))
    }

    /// Copies the current picture out, for snapshots.
    pub fn snapshot(&self) -> Option<Frame> {
        self.with_front(|_, px, w, h, pitch| Frame {
            width: w,
            height: h,
            pitch,
            pixels: px[..pitch as usize * h as usize].to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_swaps_and_counts() {
        let v = VideoOutput::new();
        let pitch = v.configure(3, 2);
        assert_eq!(pitch, 64);
        assert!(v.size().is_none());
        unsafe { *v.back_ptr() = 42 };
        let s0 = v.serial();
        v.publish();
        assert!(v.serial() > s0);
        assert_eq!(v.size(), Some((3, 2)));
        assert_eq!(v.with_front(|_, px, _, _, _| px[0]), Some(42));
        let snap = v.snapshot().unwrap();
        assert_eq!(snap.pixels.len(), 128);
    }
}
