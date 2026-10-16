//! Convert / Save and Stream: profiles persisted in the store under
//! "profiles", and jobs started from a profile and a destination.

use crate::app::App;
use crate::app::ports::Job;
use crate::domain::convert::{self, Destination, Profile};
use crate::domain::media::MediaItem;

const PROFILES_KEY: &str = "profiles";

impl App {
    /// The saved Convert profiles, or VLC's standard list when none were saved.
    pub fn conversion_profiles(&self) -> Vec<Profile> {
        self.store
            .load(PROFILES_KEY)
            .and_then(|s| serde_json::from_str::<Vec<Profile>>(&s).ok())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(convert::standard_profiles)
    }

    /// Persists the whole profile list (standard and user-made, in order).
    pub fn save_conversion_profiles(&mut self, profiles: &[Profile]) {
        match serde_json::to_string_pretty(profiles) {
            Ok(json) => {
                if let Err(e) = self.store.save(PROFILES_KEY, &json) {
                    self.toast_error(format!("Could not save the profiles: {e:#}"));
                }
            }
            Err(e) => log::error!("serialising profiles: {e}"),
        }
    }

    /// Restores VLC's standard profiles, dropping user changes.
    pub fn reset_conversion_profiles(&mut self) -> Vec<Profile> {
        let list = convert::standard_profiles();
        self.save_conversion_profiles(&list);
        list
    }

    /// Starts converting or streaming `source` and returns the job id.
    pub fn start_conversion(&mut self, source: MediaItem, profile: &Profile, dest: &Destination) -> u64 {
        let sout = convert::sout(profile, dest);
        let label = match dest {
            Destination::File(path) => {
                let name = std::path::Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.clone());
                format!("{} to {name}", source.title())
            }
            Destination::Stream { protocol, port, .. } => {
                format!("Streaming {} ({} :{port})", source.title(), protocol.label())
            }
        };
        log::info!("starting job: {sout}");
        self.start_job(label, Job { source, sout, play_locally: false })
    }

    /// Cancels a running job.
    pub fn cancel_job(&mut self, id: u64) {
        self.engine.cancel_job(id);
    }

    /// Forgets finished jobs from the list.
    pub fn clear_finished_jobs(&mut self) {
        self.jobs.retain(|j| j.finished.is_none());
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::app::App;
    use crate::app::fake::{FakeEngine, FakeProbe};
    use crate::app::ports::MediaEngine;
    use crate::app::testing::MemStore;
    use crate::domain::convert::{Destination, standard_profiles};
    use crate::domain::media::MediaItem;

    fn app() -> (tempfile::TempDir, App) {
        let dir = tempfile::tempdir().unwrap();
        let engine: Arc<dyn MediaEngine> = Arc::new(FakeEngine::new());
        let app = App::new(engine, Arc::new(FakeProbe), Arc::new(MemStore::new(dir.path())));
        (dir, app)
    }

    #[test]
    fn profiles_round_trip_through_the_store() {
        let (_d, mut app) = app();
        assert_eq!(app.conversion_profiles(), standard_profiles());
        let mut list = standard_profiles();
        list[0].name = "Mine".into();
        list.truncate(3);
        app.save_conversion_profiles(&list);
        assert_eq!(app.conversion_profiles(), list);
        assert_eq!(app.reset_conversion_profiles(), standard_profiles());
    }

    #[test]
    fn conversion_starts_a_job() {
        let (_d, mut app) = app();
        let p = &standard_profiles()[0];
        let id = app.start_conversion(MediaItem::file("/m/a.mkv"), p, &Destination::File("/m/a.mp4".into()));
        assert_eq!(app.jobs.len(), 1);
        assert_eq!(app.jobs[0].id, id);
        assert!(app.jobs[0].label.contains("a.mp4"));
    }
}
