//! What the numbers panel's tape shows (`host/panel.rs`): the last shaders
//! translated or not, pipelines made or refused and pipelines first drawn
//! with, and how many of each since the start.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// What happened.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    /// A shader translated.
    Translated = 0,
    /// One that does not translate.
    Untranslated = 1,
    /// A host pipeline made.
    Made = 2,
    /// One the host refused.
    Refused = 3,
    /// A pipeline drawn with for the first time.
    Drawn = 4,
}

/// Entries kept.
const KEPT: usize = 24;

static RECENT: Mutex<VecDeque<(Instant, Kind, String)>> = Mutex::new(VecDeque::new());
static COUNTS: [AtomicU64; 5] = [const { AtomicU64::new(0) }; 5];

/// Notes `kind`, said by `what`.
pub(crate) fn note(kind: Kind, what: impl FnOnce() -> String) {
    COUNTS[kind as usize].fetch_add(1, Ordering::Relaxed);
    let text = what();
    let mut recent = RECENT.lock().unwrap_or_else(|e| e.into_inner());
    if recent.len() >= KEPT {
        recent.pop_front();
    }
    recent.push_back((Instant::now(), kind, text));
}

/// How many of each kind since the start, by [`Kind`]'s number.
pub(crate) fn counts() -> [u64; 5] {
    COUNTS.each_ref().map(|c| c.load(Ordering::Relaxed))
}

/// The last `n` entries, newest first: seconds ago, kind, text.
pub(crate) fn recent(n: usize) -> Vec<(f32, Kind, String)> {
    let recent = RECENT.lock().unwrap_or_else(|e| e.into_inner());
    recent.iter().rev().take(n).map(|(at, kind, text)| (at.elapsed().as_secs_f32(), *kind, text.clone())).collect()
}

/// A shader's hash as the tape shows it: its first eight digits.
pub(crate) fn short(hash: &str) -> &str {
    hash.get(..8).unwrap_or(hash)
}

/// Shaders as the tape shows them: stage and short hash, joined.
pub(crate) fn shaders(shaders: &[std::sync::Arc<crate::pipeline::Shader>]) -> String {
    shaders.iter().map(|s| format!("{} {}", s.stage, short(&s.hash))).collect::<Vec<_>>().join(" + ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The newest comes first, at most `KEPT` are kept, and every one is
    /// counted.
    #[test]
    fn the_tape_keeps_the_last() {
        let before = counts()[Kind::Made as usize];
        for i in 0..KEPT + 5 {
            note(Kind::Made, || format!("pipeline {i}"));
        }
        let last = recent(3);
        assert_eq!(last[0].2, format!("pipeline {}", KEPT + 4));
        assert_eq!(last[2].2, format!("pipeline {}", KEPT + 2));
        assert!(recent(100).len() <= KEPT);
        assert!(counts()[Kind::Made as usize] >= before + KEPT as u64 + 5);
        assert_eq!(short("6b82bc7d22016cb5"), "6b82bc7d");
        assert_eq!(short("abc"), "abc");
    }
}
