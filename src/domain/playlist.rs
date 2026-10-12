//! Saved playlists and the play queue.

use serde::{Deserialize, Serialize};

use super::media::MediaItem;
use super::settings::RepeatMode;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Playlist {
    pub id: u64,
    pub name: String,
    pub items: Vec<MediaItem>,
    #[serde(default)]
    pub created: i64,
}

impl Playlist {
    pub fn total_duration_ms(&self) -> i64 {
        self.items.iter().filter_map(|i| i.meta.duration_ms).sum()
    }
}

/// What to do when the current item ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Advance {
    /// Play the item at this queue index.
    Play(usize),
    /// Replay the current item.
    Repeat,
    /// Nothing left to play.
    Finished,
}

/// The ordered list of items to play, with repeat and shuffle state.
///
/// Shuffle keeps a separate play order so turning it off returns to the
/// original order without losing the current item.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Queue {
    pub items: Vec<MediaItem>,
    pub current: Option<usize>,
    pub repeat: RepeatMode,
    pub shuffle: bool,
    /// Indices into `items` in play order while shuffled.
    order: Vec<usize>,
    /// Name of the playlist or folder the queue came from, for the toolbar.
    pub origin: Option<String>,
}

impl Queue {
    pub fn new(items: Vec<MediaItem>) -> Self {
        let mut q = Queue { items, ..Default::default() };
        q.rebuild_order();
        q
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn current_item(&self) -> Option<&MediaItem> {
        self.current.and_then(|i| self.items.get(i))
    }

    pub fn current_item_mut(&mut self) -> Option<&mut MediaItem> {
        self.current.and_then(|i| self.items.get_mut(i))
    }

    pub fn total_duration_ms(&self) -> i64 {
        self.items.iter().filter_map(|i| i.meta.duration_ms).sum()
    }

    pub fn set_shuffle(&mut self, on: bool) {
        self.shuffle = on;
        self.rebuild_order();
    }

    /// Recomputes the shuffled order, keeping the current item first so
    /// playback continues from it.
    pub fn rebuild_order(&mut self) {
        let n = self.items.len();
        self.order = (0..n).collect();
        if self.shuffle {
            fastrand::shuffle(&mut self.order);
            if let Some(cur) = self.current
                && let Some(pos) = self.order.iter().position(|&i| i == cur)
            {
                self.order.swap(0, pos);
            }
        }
    }

    fn order_pos(&self) -> Option<usize> {
        let cur = self.current?;
        self.order.iter().position(|&i| i == cur)
    }

    /// Index to play after the current one when the user presses Next.
    pub fn next_index(&self) -> Option<usize> {
        if self.items.is_empty() {
            return None;
        }
        let pos = match self.order_pos() {
            Some(p) => p + 1,
            None => 0,
        };
        if pos < self.order.len() {
            Some(self.order[pos])
        } else if self.repeat != RepeatMode::Off {
            self.order.first().copied()
        } else {
            None
        }
    }

    /// Index to play when the user presses Previous.
    pub fn previous_index(&self) -> Option<usize> {
        if self.items.is_empty() {
            return None;
        }
        match self.order_pos() {
            Some(0) if self.repeat != RepeatMode::Off => self.order.last().copied(),
            Some(0) => Some(self.order[0]),
            Some(p) => Some(self.order[p - 1]),
            None => self.order.first().copied(),
        }
    }

    /// Decides what follows when the current item finishes on its own.
    pub fn on_finished(&self, stop_after_each: bool) -> Advance {
        if self.repeat == RepeatMode::One && self.current.is_some() {
            return Advance::Repeat;
        }
        if stop_after_each {
            return Advance::Finished;
        }
        match self.next_index() {
            Some(i) => Advance::Play(i),
            None => Advance::Finished,
        }
    }

    pub fn push(&mut self, item: MediaItem) {
        self.items.push(item);
        let idx = self.items.len() - 1;
        if self.shuffle {
            // Insert somewhere after the current position.
            let start = self.order_pos().map(|p| p + 1).unwrap_or(0);
            let at =
                if start >= self.order.len() { self.order.len() } else { fastrand::usize(start..=self.order.len()) };
            self.order.insert(at, idx);
        } else {
            self.order.push(idx);
        }
    }

    /// Inserts right after the current item ("Play next").
    pub fn insert_next(&mut self, item: MediaItem) {
        let at = self.current.map(|c| c + 1).unwrap_or(0).min(self.items.len());
        self.items.insert(at, item);
        if let Some(c) = self.current
            && at <= c
        {
            self.current = Some(c + 1);
        }
        self.rebuild_order_preserving();
    }

    pub fn remove(&mut self, index: usize) -> Option<MediaItem> {
        if index >= self.items.len() {
            return None;
        }
        let item = self.items.remove(index);
        self.current = match self.current {
            Some(c) if c == index => None,
            Some(c) if c > index => Some(c - 1),
            c => c,
        };
        self.rebuild_order_preserving();
        Some(item)
    }

    /// Moves an item, used by drag-and-drop in the queue panel.
    pub fn move_item(&mut self, from: usize, to: usize) {
        if from >= self.items.len() || to >= self.items.len() || from == to {
            return;
        }
        let item = self.items.remove(from);
        self.items.insert(to, item);
        self.current = self.current.map(|c| {
            if c == from {
                to
            } else if from < c && c <= to {
                c - 1
            } else if to <= c && c < from {
                c + 1
            } else {
                c
            }
        });
        self.rebuild_order_preserving();
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.order.clear();
        self.current = None;
    }

    fn rebuild_order_preserving(&mut self) {
        if self.shuffle {
            self.rebuild_order();
        } else {
            self.order = (0..self.items.len()).collect();
        }
    }

    /// Sorts the queue in place, keeping the current item current.
    pub fn sort_by_key<K: Ord>(&mut self, mut key: impl FnMut(&MediaItem) -> K) {
        let cur = self.current.map(|c| self.items[c].source.clone());
        self.items.sort_by_key(|i| key(i));
        self.current = cur.and_then(|s| self.items.iter().position(|i| i.source == s));
        self.rebuild_order_preserving();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(n: usize) -> Queue {
        Queue::new((0..n).map(|i| MediaItem::file(format!("/m/{i}.mp4"))).collect())
    }

    #[test]
    fn linear_next_and_previous() {
        let mut q = q(3);
        assert_eq!(q.next_index(), Some(0));
        q.current = Some(0);
        assert_eq!(q.next_index(), Some(1));
        assert_eq!(q.previous_index(), Some(0));
        q.current = Some(2);
        assert_eq!(q.next_index(), None);
        q.repeat = RepeatMode::All;
        assert_eq!(q.next_index(), Some(0));
    }

    #[test]
    fn finished_rules() {
        let mut q = q(2);
        q.current = Some(0);
        assert_eq!(q.on_finished(false), Advance::Play(1));
        assert_eq!(q.on_finished(true), Advance::Finished);
        q.repeat = RepeatMode::One;
        assert_eq!(q.on_finished(false), Advance::Repeat);
        q.repeat = RepeatMode::Off;
        q.current = Some(1);
        assert_eq!(q.on_finished(false), Advance::Finished);
    }

    #[test]
    fn shuffle_visits_everything_once() {
        let mut q = q(20);
        q.current = Some(7);
        q.set_shuffle(true);
        let mut seen = vec![7];
        while let Some(n) = q.next_index() {
            seen.push(n);
            q.current = Some(n);
        }
        seen.sort();
        assert_eq!(seen, (0..20).collect::<Vec<_>>());
    }

    #[test]
    fn move_keeps_current() {
        let mut q = q(5);
        q.current = Some(1);
        q.move_item(1, 4);
        assert_eq!(q.current, Some(4));
        q.move_item(0, 4);
        assert_eq!(q.current, Some(3));
        q.remove(0);
        assert_eq!(q.current, Some(2));
        q.insert_next(MediaItem::file("/m/x.mp4"));
        assert_eq!(q.items[3].source.display_name(), "x.mp4");
    }
}
