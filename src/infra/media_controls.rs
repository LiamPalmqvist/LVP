//! Media keys and the system Now Playing widget through `souvlaki`
//! (MPRemoteCommandCenter on macOS, MPRIS on Linux, SMTC on Windows).

use std::time::Duration;

use crossbeam_channel::{Receiver, unbounded};
use souvlaki::{MediaControlEvent, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig, SeekDirection};

use crate::app::ports::{MediaControls, NowPlaying, RemoteCommand};

pub struct SystemMediaControls {
    inner: souvlaki::MediaControls,
    rx: Receiver<RemoteCommand>,
    shown: Option<NowPlaying>,
}

impl SystemMediaControls {
    /// Must be called on the main thread.
    pub fn new() -> Option<Self> {
        let config = PlatformConfig { dbus_name: "notvlc", display_name: "NotVLC", hwnd: None };
        let mut inner = souvlaki::MediaControls::new(config).ok()?;
        let (tx, rx) = unbounded();
        inner
            .attach(move |e| {
                let cmd = match e {
                    MediaControlEvent::Play => RemoteCommand::Play,
                    MediaControlEvent::Pause => RemoteCommand::Pause,
                    MediaControlEvent::Toggle => RemoteCommand::Toggle,
                    MediaControlEvent::Next => RemoteCommand::Next,
                    MediaControlEvent::Previous => RemoteCommand::Previous,
                    MediaControlEvent::Stop => RemoteCommand::Stop,
                    MediaControlEvent::Seek(SeekDirection::Forward) => RemoteCommand::SeekBy(10_000),
                    MediaControlEvent::Seek(SeekDirection::Backward) => RemoteCommand::SeekBy(-10_000),
                    MediaControlEvent::SeekBy(SeekDirection::Forward, d) => RemoteCommand::SeekBy(d.as_millis() as i64),
                    MediaControlEvent::SeekBy(SeekDirection::Backward, d) => {
                        RemoteCommand::SeekBy(-(d.as_millis() as i64))
                    }
                    MediaControlEvent::SetPosition(MediaPosition(d)) => RemoteCommand::SeekTo(d.as_millis() as i64),
                    _ => return,
                };
                let _ = tx.send(cmd);
            })
            .ok()?;
        Some(Self { inner, rx, shown: None })
    }
}

impl MediaControls for SystemMediaControls {
    fn update(&mut self, now: Option<&NowPlaying>) {
        let Some(np) = now else {
            if self.shown.take().is_some() {
                let _ = self.inner.set_playback(MediaPlayback::Stopped);
            }
            return;
        };
        let prev = self.shown.as_ref();
        let meta_changed = prev.is_none_or(|p| {
            p.title != np.title || p.artist != np.artist || p.artwork != np.artwork || p.duration_ms != np.duration_ms
        });
        let state_changed =
            prev.is_none_or(|p| p.playing != np.playing || (p.position_ms - np.position_ms).abs() > 2_500);
        if meta_changed {
            let _ = self.inner.set_metadata(MediaMetadata {
                title: Some(&np.title),
                artist: np.artist.as_deref(),
                album: np.album.as_deref(),
                cover_url: np.artwork.as_deref(),
                duration: (np.duration_ms > 0).then(|| Duration::from_millis(np.duration_ms as u64)),
            });
        }
        if meta_changed || state_changed {
            let progress = Some(MediaPosition(Duration::from_millis(np.position_ms.max(0) as u64)));
            let _ = self.inner.set_playback(if np.playing {
                MediaPlayback::Playing { progress }
            } else {
                MediaPlayback::Paused { progress }
            });
            self.shown = Some(np.clone());
        } else if let Some(s) = self.shown.as_mut() {
            // Track position without pushing an update every frame.
            s.position_ms = np.position_ms;
        }
    }

    fn poll(&mut self) -> Vec<RemoteCommand> {
        self.rx.try_iter().collect()
    }
}
