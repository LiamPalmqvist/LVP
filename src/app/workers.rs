//! A small pool that parses media and renders thumbnails off the UI thread.
//! Newest requests run first, so whatever is on screen now wins.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender, unbounded};
use parking_lot::{Condvar, Mutex};

use crate::app::ports::MediaProbe;
use crate::domain::media::{MediaMeta, MediaSource};

type Notify = Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>;

#[derive(Debug, Clone, PartialEq)]
pub enum Work {
    Probe(MediaSource),
    Thumbnail { source: MediaSource, out: PathBuf, width: u32 },
}

#[derive(Debug, Clone)]
pub enum Done {
    Meta(MediaSource, Box<MediaMeta>),
    Thumbnail(MediaSource, PathBuf),
    Failed(Work, String),
}

struct Queue {
    items: Mutex<(VecDeque<Work>, bool)>,
    cv: Condvar,
}

pub struct Workers {
    queue: Arc<Queue>,
    done: Receiver<Done>,
    threads: Vec<std::thread::JoinHandle<()>>,
    notify: Notify,
}

impl Workers {
    pub fn new(probe: Arc<dyn MediaProbe>, threads: usize) -> Self {
        let queue = Arc::new(Queue { items: Mutex::new((VecDeque::new(), false)), cv: Condvar::new() });
        let (tx, rx) = unbounded();
        let notify: Notify = Arc::new(Mutex::new(None));
        let handles = (0..threads.max(1))
            .map(|i| {
                let (q, tx, probe, notify) = (queue.clone(), tx.clone(), probe.clone(), notify.clone());
                std::thread::Builder::new()
                    .name(format!("media-worker-{i}"))
                    .spawn(move || run(q, tx, probe, notify))
                    .expect("spawn worker")
            })
            .collect();
        Self { queue, done: rx, threads: handles, notify }
    }

    /// Called after each finished job, typically to request a repaint.
    pub fn set_notify(&self, f: impl Fn() + Send + Sync + 'static) {
        *self.notify.lock() = Some(Box::new(f));
    }

    pub fn submit(&self, work: Work) {
        let mut g = self.queue.items.lock();
        g.0.retain(|w| *w != work);
        g.0.push_front(work);
        // Keep the backlog bounded; stale requests are re-issued if still visible.
        g.0.truncate(400);
        self.queue.cv.notify_one();
    }

    pub fn finished(&self) -> Vec<Done> {
        self.done.try_iter().collect()
    }
}

impl Drop for Workers {
    fn drop(&mut self) {
        self.queue.items.lock().1 = true;
        self.queue.cv.notify_all();
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

fn run(q: Arc<Queue>, tx: Sender<Done>, probe: Arc<dyn MediaProbe>, notify: Notify) {
    loop {
        let work = {
            let mut g = q.items.lock();
            loop {
                if g.1 {
                    return;
                }
                if let Some(w) = g.0.pop_front() {
                    break w;
                }
                q.cv.wait(&mut g);
            }
        };
        let result = match &work {
            Work::Probe(src) => match probe.probe(src) {
                Ok(m) => Done::Meta(src.clone(), Box::new(m)),
                Err(e) => Done::Failed(work.clone(), format!("{e:#}")),
            },
            Work::Thumbnail { source, out, width } => match probe.thumbnail(source, out, *width, 0.3) {
                Ok(()) => Done::Thumbnail(source.clone(), out.clone()),
                Err(e) => Done::Failed(work.clone(), format!("{e:#}")),
            },
        };
        if tx.send(result).is_err() {
            return;
        }
        if let Some(n) = notify.lock().as_ref() {
            n();
        }
    }
}
