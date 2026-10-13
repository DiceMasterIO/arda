//! The bake job table and its result budget (1.0 task 04).
//!
//! Every job is keyed by its content-addressed id. Queued and running jobs
//! hold their parsed request; finished jobs hold their encoded tiles (or
//! their error) inside a byte budget. When a new result does not fit, the
//! oldest finished results are evicted first, in the order they finished;
//! there is no wall-clock TTL. A result larger than the whole budget is
//! refused (the job fails with `resource_limit`).

use super::bake_dto::{BakeRequestParsed, BakeState};
use super::dto::TacticalTilesDto;
use super::Encoded;
use crate::error::ApiErrorBody;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

/// Bytes a failed job is charged against the budget, so failures cannot
/// pile up without bound.
pub const FAILED_ENTRY_BYTES: u64 = 1 << 10;

/// Tile `(z, x, y)` of a set.
pub type TileId = (u32, u32, u32);

/// One finished tile set: encoded WebP tiles only.
#[derive(Debug)]
pub struct BakeResult {
    /// Pyramid geometry.
    pub tiles: TacticalTilesDto,
    /// Every tile, in `(z, x, y)` order.
    pub encoded: BTreeMap<TileId, Arc<Encoded>>,
    /// Sum of the encoded tile bytes.
    pub bytes: u64,
}

/// Where a job stands.
#[derive(Debug, Clone)]
pub enum Slot {
    /// Waiting, with its request.
    Queued(Arc<BakeRequestParsed>),
    /// Rendering now.
    Running(Arc<BakeRequestParsed>),
    /// Finished with tiles.
    Done(Arc<BakeResult>),
    /// Finished with an error.
    Failed(ApiErrorBody),
}

impl Slot {
    /// The wire state.
    #[must_use]
    pub const fn state(&self) -> BakeState {
        match self {
            Self::Queued(_) => BakeState::Queued,
            Self::Running(_) => BakeState::Running,
            Self::Done(_) => BakeState::Done,
            Self::Failed(_) => BakeState::Failed,
        }
    }

    fn charge(&self) -> u64 {
        match self {
            Self::Done(r) => r.bytes,
            Self::Failed(_) => FAILED_ENTRY_BYTES,
            Self::Queued(_) | Self::Running(_) => 0,
        }
    }
}

/// One job: its cell and where it stands.
#[derive(Debug, Clone)]
pub struct Entry {
    /// Cell column.
    pub gx: u32,
    /// Cell row.
    pub gy: u32,
    /// Output pixels per square.
    pub ppsq: u32,
    /// Whether the world grade applies.
    pub world_grade: bool,
    /// Where the job stands.
    pub slot: Slot,
}

/// Every known job, the waiting order and the finished results' budget.
#[derive(Debug)]
pub struct Store {
    budget: u64,
    used: u64,
    jobs: BTreeMap<String, Entry>,
    /// Finished job ids, oldest first.
    finished: VecDeque<String>,
}

impl Store {
    /// An empty table holding finished results within `budget` bytes.
    #[must_use]
    pub const fn new(budget: u64) -> Self {
        Self {
            budget,
            used: 0,
            jobs: BTreeMap::new(),
            finished: VecDeque::new(),
        }
    }

    /// The job `id`.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.jobs.get(id)
    }

    /// The finished results' budget, bytes.
    #[must_use]
    pub const fn budget(&self) -> u64 {
        self.budget
    }

    /// Bytes the finished results hold.
    #[must_use]
    pub const fn used(&self) -> u64 {
        self.used
    }

    /// Adds a queued job.
    pub fn queue(&mut self, id: String, entry: Entry) {
        self.forget(&id);
        self.jobs.insert(id, entry);
    }

    /// Marks a queued job running; `false` if it is gone or not queued.
    pub fn start(&mut self, id: &str) -> bool {
        match self.jobs.get_mut(id) {
            Some(e) => match &e.slot {
                Slot::Queued(r) => {
                    e.slot = Slot::Running(Arc::clone(r));
                    true
                }
                _ => false,
            },
            None => false,
        }
    }

    /// Records a running job's outcome, evicting the oldest finished
    /// results until it fits. Returns the evicted ids. A job deleted while
    /// it ran is not recorded; a result larger than the whole budget is
    /// recorded as `failed` with `too_big`.
    pub fn finish(&mut self, id: &str, outcome: Slot, too_big: ApiErrorBody) -> Vec<String> {
        if !matches!(self.jobs.get(id).map(|e| &e.slot), Some(Slot::Running(_))) {
            return Vec::new();
        }
        let outcome = if outcome.charge() > self.budget {
            Slot::Failed(too_big)
        } else {
            outcome
        };
        let need = outcome.charge();
        let mut evicted = Vec::new();
        while self.used + need > self.budget {
            let Some(old) = self.finished.pop_front() else {
                break;
            };
            if let Some(e) = self.jobs.remove(&old) {
                self.used -= e.slot.charge();
            }
            evicted.push(old);
        }
        if let Some(e) = self.jobs.get_mut(id) {
            e.slot = outcome;
            self.used += need;
            self.finished.push_back(id.to_owned());
        }
        evicted
    }

    /// Removes job `id` in any state; whether it existed.
    pub fn forget(&mut self, id: &str) -> bool {
        let Some(e) = self.jobs.remove(id) else {
            return false;
        };
        self.used -= e.slot.charge();
        self.finished.retain(|f| f != id);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err() -> ApiErrorBody {
        ApiErrorBody {
            code: "resource_limit".into(),
            status: 500,
            message: "too big".into(),
        }
    }

    fn entry() -> Entry {
        let req = Arc::new(BakeRequestParsed {
            gx: 1,
            gy: 2,
            ppsq: 64,
            world_grade: false,
            layout: arda_tactical::TacticalLayout::new("c", 64, 64, "grass"),
        });
        Entry {
            gx: 1,
            gy: 2,
            ppsq: 64,
            world_grade: false,
            slot: Slot::Queued(req),
        }
    }

    fn done(bytes: u64) -> Slot {
        Slot::Done(Arc::new(BakeResult {
            tiles: super::super::tiles_dto_sized(64, 64, 64),
            encoded: BTreeMap::new(),
            bytes,
        }))
    }

    fn run(s: &mut Store, id: &str, outcome: Slot) -> Vec<String> {
        s.queue(id.into(), entry());
        assert!(s.start(id));
        s.finish(id, outcome, err())
    }

    #[test]
    fn the_oldest_finished_results_are_evicted_first() {
        let mut s = Store::new(10_000);
        assert!(run(&mut s, "a", done(4_000)).is_empty());
        assert!(run(&mut s, "b", done(4_000)).is_empty());
        assert_eq!(s.used(), 8_000);
        // c needs 3,000: evicting a (the oldest) is enough.
        assert_eq!(run(&mut s, "c", done(3_000)), vec!["a".to_owned()]);
        assert!(s.get("a").is_none());
        assert_eq!(s.used(), 7_000);
        // d needs 9,000: b and c both go, oldest first.
        assert_eq!(
            run(&mut s, "d", done(9_000)),
            vec!["b".to_owned(), "c".to_owned()]
        );
        assert_eq!(s.used(), 9_000);
        assert_eq!(s.get("d").unwrap().slot.state(), BakeState::Done);
    }

    #[test]
    fn queued_and_running_jobs_are_never_evicted_and_cost_nothing() {
        let mut s = Store::new(10_000);
        s.queue("q".into(), entry());
        s.queue("r".into(), entry());
        assert!(s.start("r"));
        assert!(run(&mut s, "a", done(10_000)).is_empty());
        assert_eq!(run(&mut s, "b", done(6_000)), vec!["a".to_owned()]);
        assert_eq!(s.get("q").unwrap().slot.state(), BakeState::Queued);
        assert_eq!(s.get("r").unwrap().slot.state(), BakeState::Running);
    }

    #[test]
    fn oversized_results_fail_and_deleted_jobs_stay_deleted() {
        let mut s = Store::new(10_000);
        assert!(run(&mut s, "a", done(5_000)).is_empty());
        assert!(run(&mut s, "big", done(10_001)).is_empty());
        assert_eq!(s.get("big").unwrap().slot.state(), BakeState::Failed);
        assert_eq!(s.used(), 5_000 + FAILED_ENTRY_BYTES);
        // A job deleted while running is not recorded when it finishes.
        s.queue("gone".into(), entry());
        assert!(s.start("gone"));
        assert!(s.forget("gone"));
        assert!(s.finish("gone", done(1_000), err()).is_empty());
        assert!(s.get("gone").is_none());
        assert!(s.forget("a"));
        assert!(!s.forget("a"));
    }
}
