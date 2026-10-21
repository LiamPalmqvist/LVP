//! Desktop integration through each system's own tools, so no extra
//! libraries are linked: keeping the display awake during video, and
//! desktop notifications.
//!
//! macOS: `caffeinate` and `osascript`. Linux: `systemd-inhibit` and
//! `notify-send`. Windows: not yet (both calls do nothing).

use std::process::{Child, Command, Stdio};

use parking_lot::Mutex;

use crate::app::ports::SystemHooks;

#[derive(Default)]
pub struct SystemIntegration {
    inhibitor: Mutex<Option<Child>>,
}

impl SystemIntegration {
    pub fn new() -> Self {
        Self::default()
    }
}

fn quiet(cmd: &mut Command) -> Option<Child> {
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().ok()
}

fn sleep_inhibitor() -> Option<Child> {
    if cfg!(target_os = "macos") {
        // -d: display, -i: idle sleep, -w: exit with us if we crash.
        let pid = std::process::id().to_string();
        quiet(Command::new("caffeinate").args(["-d", "-i", "-w", &pid]))
    } else if cfg!(target_os = "linux") {
        quiet(Command::new("systemd-inhibit").args([
            "--what=idle:sleep",
            "--who=LVP",
            "--why=Playing video",
            "--mode=block",
            "sleep",
            "infinity",
        ]))
    } else {
        None
    }
}

impl SystemHooks for SystemIntegration {
    fn inhibit_sleep(&self, on: bool) {
        let mut slot = self.inhibitor.lock();
        if on && slot.is_none() {
            *slot = sleep_inhibitor();
        } else if !on && let Some(mut child) = slot.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn notify(&self, title: &str, body: &str) {
        if cfg!(target_os = "macos") {
            let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
            let script = format!("display notification \"{}\" with title \"{}\"", esc(body), esc(title));
            quiet(Command::new("osascript").args(["-e", &script]));
        } else if cfg!(target_os = "linux") {
            quiet(Command::new("notify-send").args(["--app-name=LVP", "--icon=lvp", title, body]));
        }
    }
}

impl Drop for SystemIntegration {
    fn drop(&mut self) {
        self.inhibit_sleep(false);
    }
}
