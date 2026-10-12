//! macOS integration through the system's own command-line tools, so no
//! extra frameworks are linked: `caffeinate` keeps the display awake and
//! `osascript` posts notifications.

use std::process::{Child, Command, Stdio};

use parking_lot::Mutex;

use crate::app::ports::SystemHooks;

#[derive(Default)]
pub struct MacHooks {
    caffeinate: Mutex<Option<Child>>,
}

impl MacHooks {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SystemHooks for MacHooks {
    fn inhibit_sleep(&self, on: bool) {
        let mut slot = self.caffeinate.lock();
        if on && slot.is_none() {
            // -d: display, -i: idle sleep, -w: exit with us if we crash.
            let pid = std::process::id().to_string();
            *slot = Command::new("caffeinate")
                .args(["-d", "-i", "-w", &pid])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .ok();
        } else if !on && let Some(mut child) = slot.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn notify(&self, title: &str, body: &str) {
        let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!("display notification \"{}\" with title \"{}\"", esc(body), esc(title));
        let _ = Command::new("osascript")
            .args(["-e", &script])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

impl Drop for MacHooks {
    fn drop(&mut self) {
        self.inhibit_sleep(false);
    }
}
