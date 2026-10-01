//! Tactical prefetch (goal 67; logic/16 §api-prefetch):
//! `POST /v1/tactical/prefetch` queues the cells around a centre, nearest
//! ring first, and a small pool of background workers warms their block,
//! JSON body and image pyramid caches.
//!
//! - **Never blocks requests.** The handler only touches the queue; workers
//!   are their own threads, and they render in the prefetch lane (`raw.rs`),
//!   never holding the request lane. A request for a cell a worker is
//!   rendering waits for that render instead of starting a second one.
//! - **Bounded.** At most [`QUEUE_CELLS`] cells wait, deduplicated; a full
//!   queue drops the rest (reported as `dropped`). Images warm only for the
//!   nearest cells whose renders and pyramids fit the configured caches
//!   beside the open cell's; farther cells warm their JSON and blocks.
//! - **Invisible.** Warming fills the same caches under the same keys, so
//!   prefetching never changes any response's bytes; open-sea cells simply
//!   have nothing to warm, and the world edge clips the rings.

use super::block::BlockRequest;
use super::raw::Lane;
use super::{Tactical, TacticalLimits};
use crate::error::{ServerError, ServerResult};
use crate::AppState;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak};
use std::time::{Duration, Instant};
use ts_rs::TS;

/// Largest prefetch radius, in cells.
pub const MAX_RADIUS: u32 = 2;
/// Default prefetch radius.
pub const DEFAULT_RADIUS: u32 = 1;
/// Most cells waiting at once.
pub const QUEUE_CELLS: usize = 64;
/// Default worker count.
pub const DEFAULT_WORKERS: usize = 2;
/// Largest worker count.
pub const MAX_WORKERS: usize = 8;

/// Body of `POST /v1/tactical/prefetch` (TS `PrefetchRequest`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrefetchRequest {
    /// Centre cell column.
    pub gx: u32,
    /// Centre cell row.
    pub gy: u32,
    /// Rings of neighbours to warm, 1–2 (default 1: the 8 neighbours).
    #[serde(default)]
    #[ts(optional)]
    pub radius: Option<u32>,
    /// Pixels per square of the images to warm (the cell route's options;
    /// default 128).
    #[serde(default)]
    #[ts(optional)]
    pub ppsq: Option<u32>,
    /// Warm the demo-overlay blocks instead (`?demo_overlays=1`).
    #[serde(default)]
    #[ts(optional)]
    pub demo_overlays: Option<bool>,
}

/// What happened to one neighbour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PrefetchStatus {
    /// Added to the queue.
    Queued,
    /// Already queued or warming.
    Pending,
    /// The queue was full.
    Dropped,
}

/// One neighbour of a prefetch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
pub struct PrefetchCell {
    /// Cell column.
    pub gx: u32,
    /// Cell row.
    pub gy: u32,
    /// Queue outcome.
    pub status: PrefetchStatus,
    /// Whether its image pyramid warms too (else JSON and blocks only).
    pub image: bool,
}

/// `202` body of `POST /v1/tactical/prefetch` (TS `PrefetchAccepted`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
pub struct PrefetchAccepted {
    /// The radius used.
    pub radius: u32,
    /// The ppsq warmed.
    pub ppsq: u32,
    /// The neighbours inside the world, nearest ring first, row-major.
    pub cells: Vec<PrefetchCell>,
    /// Cells waiting after this request.
    pub queue_len: u32,
}

/// The cells of rings `1..=radius` around `(gx, gy)` inside a `w × h`-cell
/// world, nearest ring first, each ring row-major.
#[must_use]
pub fn rings(gx: u32, gy: u32, radius: u32, (w, h): (u32, u32)) -> Vec<(u32, u32)> {
    let (cx, cy) = (i64::from(gx), i64::from(gy));
    let r = i64::from(radius);
    let mut out = Vec::new();
    for d in 1..=r {
        for y in cy - d..=cy + d {
            for x in cx - d..=cx + d {
                if (x - cx).abs().max((y - cy).abs()) != d {
                    continue;
                }
                if let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) {
                    if x < w && y < h {
                        out.push((x, y));
                    }
                }
            }
        }
    }
    out
}

/// How many neighbours may warm images at `ppsq` without evicting the open
/// cell: the renders and pyramids of that many cells, plus one, must fit
/// their caches.
#[must_use]
pub fn image_slots(limits: &TacticalLimits, ppsq: u32) -> usize {
    let edge = 64 * u64::from(ppsq);
    let raw = edge * edge * 4;
    let pyramid = raw + raw.div_ceil(3);
    let fit = |budget: usize, each: u64| usize::try_from(budget as u64 / each.max(1)).unwrap_or(0);
    fit(limits.render_cache_bytes, raw)
        .min(fit(limits.pyramid_cache_bytes, pyramid))
        .saturating_sub(1)
}

#[derive(Debug, Clone, Copy)]
struct Job {
    req: BlockRequest,
    ppsq: u32,
    image: bool,
    world: (i64, i64),
}

#[derive(Debug, Default)]
struct Queue {
    jobs: VecDeque<Job>,
    known: BTreeSet<(BlockRequest, u32)>,
    active: usize,
    closed: bool,
    started: bool,
}

#[derive(Debug, Default)]
struct Shared {
    queue: Mutex<Queue>,
    wake: Condvar,
    idle: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, Queue> {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The prefetch queue and its worker pool.
#[derive(Debug)]
pub struct Prefetcher {
    shared: Arc<Shared>,
    workers: usize,
}

impl Prefetcher {
    /// A queue served by `workers` threads (started on first use; 0
    /// disables prefetching: every cell is `dropped`).
    #[must_use]
    pub fn new(workers: usize) -> Self {
        Self {
            shared: Arc::default(),
            workers: workers.min(MAX_WORKERS),
        }
    }

    /// Worker threads.
    #[must_use]
    pub const fn workers(&self) -> usize {
        self.workers
    }

    /// Queues `request` around its centre. Returns at once.
    ///
    /// # Errors
    /// 400 for a bad radius, ppsq or cell; 500 if no worker could start.
    pub fn submit(
        &self,
        state: &Arc<AppState>,
        request: &PrefetchRequest,
    ) -> ServerResult<PrefetchAccepted> {
        let radius = request.radius.unwrap_or(DEFAULT_RADIUS);
        if !(1..=MAX_RADIUS).contains(&radius) {
            return Err(ServerError::BadRequest(format!(
                "radius must be 1..={MAX_RADIUS}, not {radius}"
            )));
        }
        let ppsq = request.ppsq.unwrap_or(super::DEFAULT_PPSQ);
        if !super::PPSQ_OPTIONS.contains(&ppsq) && !super::world_routes::WINDOW_PPSQ.contains(&ppsq)
        {
            return Err(ServerError::BadRequest(format!(
                "ppsq must be one of {:?} {:?}, not {ppsq}",
                super::PPSQ_OPTIONS,
                super::world_routes::WINDOW_PPSQ
            )));
        }
        let (w, h) = state.query.cells();
        let (gx, gy) = (request.gx, request.gy);
        if gx >= w || gy >= h {
            return Err(ServerError::OutOfRange(format!(
                "cell {gx},{gy} is outside the {w}x{h}-cell world"
            )));
        }
        let world = (i64::from(w) * 64, i64::from(h) * 64);
        let slots = image_slots(state.tactical.limits(), ppsq);
        let demo = request.demo_overlays.unwrap_or(false);
        let mut cells = Vec::new();
        let mut q = self.shared.lock();
        if self.workers > 0 && !q.started {
            self.start(state)?;
            q.started = true;
        }
        for (i, (x, y)) in rings(gx, gy, radius, (w, h)).into_iter().enumerate() {
            let mut req = BlockRequest::cell(x, y);
            if demo {
                req.demo_at = Some([i64::from(x), i64::from(y)]);
            }
            let image = i < slots;
            let status = if q.known.contains(&(req, ppsq)) {
                PrefetchStatus::Pending
            } else if self.workers == 0 || q.jobs.len() >= QUEUE_CELLS {
                PrefetchStatus::Dropped
            } else {
                q.known.insert((req, ppsq));
                q.jobs.push_back(Job {
                    req,
                    ppsq,
                    image,
                    world,
                });
                PrefetchStatus::Queued
            };
            cells.push(PrefetchCell {
                gx: x,
                gy: y,
                status,
                image,
            });
        }
        let queue_len = u32::try_from(q.jobs.len()).unwrap_or(u32::MAX);
        drop(q);
        self.shared.wake.notify_all();
        Ok(PrefetchAccepted {
            radius,
            ppsq,
            cells,
            queue_len,
        })
    }

    fn start(&self, state: &Arc<AppState>) -> ServerResult<()> {
        for i in 0..self.workers {
            let shared = Arc::clone(&self.shared);
            let weak = Arc::downgrade(state);
            std::thread::Builder::new()
                .name(format!("arda-prefetch-{i}"))
                .spawn(move || work(&shared, &weak))
                .map_err(|e| ServerError::Internal(format!("prefetch worker: {e}")))?;
        }
        Ok(())
    }

    /// Waits until the queue is empty and no cell is warming, up to
    /// `timeout`; whether it became idle.
    #[must_use]
    pub fn wait_idle(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut q = self.shared.lock();
        while !q.jobs.is_empty() || q.active > 0 {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            q = self
                .shared
                .idle
                .wait_timeout(q, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        true
    }
}

impl Drop for Prefetcher {
    fn drop(&mut self) {
        self.shared.lock().closed = true;
        self.shared.wake.notify_all();
    }
}

/// One worker: take a cell, warm it, repeat until the state is gone.
fn work(shared: &Shared, state: &Weak<AppState>) {
    loop {
        let job = {
            let mut q = shared.lock();
            loop {
                if q.closed || state.strong_count() == 0 {
                    return;
                }
                if let Some(job) = q.jobs.pop_front() {
                    q.active += 1;
                    break job;
                }
                q = shared
                    .wake
                    .wait_timeout(q, Duration::from_secs(1))
                    .unwrap_or_else(PoisonError::into_inner)
                    .0;
            }
        };
        if let Some(s) = state.upgrade() {
            let start = Instant::now();
            let outcome = match s.tactical.warm(&job.req, job.world, job.ppsq, job.image) {
                Ok(()) => "warm".to_owned(),
                Err(e) => e.to_string(),
            };
            eprintln!(
                "arda-server tactical prefetch cell {},{} ppsq={} image={} {outcome} {:.3} ms",
                job.req.gsx0 / 64,
                job.req.gsy0 / 64,
                job.ppsq,
                u8::from(job.image),
                start.elapsed().as_secs_f64() * 1e3
            );
        }
        let mut q = shared.lock();
        q.active -= 1;
        q.known.remove(&(job.req, job.ppsq));
        if q.jobs.is_empty() && q.active == 0 {
            shared.idle.notify_all();
        }
    }
}

impl Tactical {
    /// Warms `req`'s composed block and JSON body at `ppsq`, and with
    /// `image` its render and tile pyramid (grid off), in the prefetch lane.
    ///
    /// # Errors
    /// Block, render or limit failures (open sea is `no_block`).
    pub fn warm(
        &self,
        req: &BlockRequest,
        world: (i64, i64),
        ppsq: u32,
        image: bool,
    ) -> ServerResult<()> {
        self.world_body(req, ppsq)?;
        if image {
            let (block, key) = self.world_render(req, world, ppsq, false)?;
            self.pyramid_in(&block.layout, &key, Lane::Prefetch)?;
        }
        Ok(())
    }

    /// Whether `req`'s image pyramid at `ppsq` (grid off) is cached.
    ///
    /// # Errors
    /// Block failures.
    pub fn image_warm(
        &self,
        req: &BlockRequest,
        world: (i64, i64),
        ppsq: u32,
    ) -> ServerResult<bool> {
        let (_, key) = self.world_render(req, world, ppsq, false)?;
        Ok(super::cached(&self.pyramids, &key)?.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rings_are_nearest_first_and_clipped_to_the_world() {
        let one = rings(5, 5, 1, (10, 10));
        assert_eq!(
            one,
            vec![
                (4, 4),
                (5, 4),
                (6, 4),
                (4, 5),
                (6, 5),
                (4, 6),
                (5, 6),
                (6, 6)
            ]
        );
        let two = rings(5, 5, 2, (10, 10));
        assert_eq!(two.len(), 24);
        assert_eq!(&two[..8], one.as_slice());
        assert_eq!(rings(0, 0, 1, (10, 10)), vec![(1, 0), (0, 1), (1, 1)]);
        assert_eq!(rings(9, 9, 2, (10, 10)).len(), 8);
        assert!(rings(0, 0, 1, (1, 1)).is_empty());
    }

    #[test]
    fn image_warming_fits_the_caches_beside_the_open_cell() {
        let limits = TacticalLimits::default();
        // 64 px per square: 64 MiB renders into 768 MiB (12), just over
        // 85 MiB pyramids into 1 GiB (11), less the open cell.
        assert_eq!(image_slots(&limits, 64), 10);
        assert_eq!(image_slots(&limits, 128), 1);
        assert!(image_slots(&limits, 16) >= 24);
        let tiny = TacticalLimits {
            render_cache_bytes: 1,
            ..limits
        };
        assert_eq!(image_slots(&tiny, 64), 0);
    }
}
