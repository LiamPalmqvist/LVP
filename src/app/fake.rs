//! A scriptable engine and probe with no libvlc, for UI snapshots and tests.

use std::path::Path;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::app::ports::*;
use crate::app::video::VideoOutput;
use crate::domain::media::{MediaMeta, MediaSource};

pub struct FakeEngine {
    pub status: Mutex<EngineStatus>,
    pub sent: Mutex<Vec<EngineCommand>>,
    pub video: Arc<VideoOutput>,
    pub devices: Vec<AudioDevice>,
}

impl Default for FakeEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeEngine {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(EngineStatus { volume: 100, rate: 1.0, ..Default::default() }),
            sent: Mutex::new(Vec::new()),
            video: Arc::new(VideoOutput::new()),
            devices: vec![
                AudioDevice { id: "speakers".into(), name: "Speakers".into() },
                AudioDevice { id: "headphones".into(), name: "Headphones".into() },
            ],
        }
    }

    /// Puts an RGBA image into the video output, as if decoded.
    pub fn show_picture(&self, w: u32, h: u32, rgba: &[u8]) {
        let pitch = self.video.configure(w, h) as usize;
        let dst = self.video.back_ptr();
        for row in 0..h as usize {
            let src = &rgba[row * w as usize * 4..(row + 1) * w as usize * 4];
            unsafe { std::ptr::copy_nonoverlapping(src.as_ptr(), dst.add(row * pitch), src.len()) };
        }
        self.video.publish();
    }
}

impl MediaEngine for FakeEngine {
    fn send(&self, cmd: EngineCommand) {
        let mut st = self.status.lock();
        match &cmd {
            EngineCommand::Open(req) => {
                st.state = PlayerState::Playing;
                st.mrl = Some(req.item.source.mrl());
                st.meta = req.item.meta.clone();
                st.length_ms = req.item.meta.duration_ms.unwrap_or(1_540_000);
                st.time_ms = req.start_ms.unwrap_or(0);
            }
            EngineCommand::TogglePause => {
                st.state = if st.state == PlayerState::Playing { PlayerState::Paused } else { PlayerState::Playing }
            }
            EngineCommand::Pause => st.state = PlayerState::Paused,
            EngineCommand::Play => st.state = PlayerState::Playing,
            EngineCommand::Stop => st.state = PlayerState::Stopped,
            EngineCommand::SeekMs { ms, .. } => st.time_ms = *ms,
            EngineCommand::SetVolume(v) => st.volume = *v,
            EngineCommand::SetMute(m) => st.muted = *m,
            EngineCommand::SetRate(r) => st.rate = *r,
            _ => {}
        }
        drop(st);
        self.sent.lock().push(cmd);
    }
    fn status(&self) -> EngineStatus {
        self.status.lock().clone()
    }
    fn poll_events(&self) -> Vec<EngineEvent> {
        Vec::new()
    }
    fn video(&self) -> Arc<VideoOutput> {
        self.video.clone()
    }
    fn version(&self) -> String {
        "3.0.24 (fake)".into()
    }
    fn audio_devices(&self) -> Vec<AudioDevice> {
        self.devices.clone()
    }
    fn renderers(&self) -> Vec<RendererInfo> {
        vec![RendererInfo { id: 1, name: "Living Room TV".into(), kind: "chromecast".into(), can_video: true }]
    }
    fn reply_dialog(&self, _id: u64, _reply: DialogReply) {}
    fn services(&self) -> Vec<ServiceInfo> {
        vec![
            ServiceInfo {
                name: "upnp".into(),
                long_name: "Universal Plug'n'Play".into(),
                category: ServiceCategory::Lan,
            },
            ServiceInfo {
                name: "dsm-sd".into(),
                long_name: "Windows Network (SMB)".into(),
                category: ServiceCategory::Lan,
            },
            ServiceInfo {
                name: "sap".into(),
                long_name: "Network streams (SAP)".into(),
                category: ServiceCategory::Lan,
            },
        ]
    }
    fn service_items(&self, _name: &str) -> Vec<BrowseItem> {
        vec![BrowseItem { name: "NAS".into(), source: MediaSource::Location("smb://nas/".into()), is_container: true }]
    }
    fn browse(&self, _source: &MediaSource) -> anyhow::Result<Vec<BrowseItem>> {
        Ok(Vec::new())
    }
    fn start_job(&self, _job: Job) -> u64 {
        1
    }
    fn cancel_job(&self, _id: u64) {}
}

pub struct FakeProbe;

impl MediaProbe for FakeProbe {
    fn probe(&self, _source: &MediaSource) -> anyhow::Result<MediaMeta> {
        Ok(MediaMeta::default())
    }
    fn thumbnail(&self, _source: &MediaSource, _out: &Path, _width: u32, _fraction: f32) -> anyhow::Result<()> {
        anyhow::bail!("no thumbnails in the fake probe")
    }
}
