//! `POST /v1/tactical/bake` (1.0 task 04; logic/16 §api-tactical,
//! §api-cache, §api-prefetch): a background, uncached, job-style render of
//! a client-edited cell layout into the WebP tile set the client stores.
//!
//! - **Job-style.** The post answers at once with a content-addressed job
//!   id ([`job_id`]): re-posting the same body returns the same job. The
//!   client polls `GET /bake/{id}`, downloads every tile, then `DELETE`s it.
//! - **Keeps nothing.** A bake reads and writes none of the shared caches
//!   (renders, pyramids, PNGs, tiles); finished tile sets wait in a byte
//!   budget ([`super::bake_store`]) until deleted or evicted, never on disk.
//! - **Never slows requests.** At most [`QUEUE_JOBS`] jobs wait and one
//!   renders, in its own lane (`raw.rs`); tiles encode one at a time
//!   through the shared encode slot, so a request's encode waits for at
//!   most one tile. Admission counts the lane, its pyramid, the queued
//!   bodies and the result budget ([`admitted_bytes`]).
//! - **Seamless.** The posted cell is spliced into the cell's apron block
//!   ([`super::splice`]) and rendered exactly as the cell route renders,
//!   so an unedited layout bakes to the cell route's tiles byte for byte.

pub use super::bake_dto::{
    job_id, BakeAccepted, BakeRequest, BakeRequestParsed, BakeState, BakeStatus,
};
use super::bake_store::{Entry, Slot, Store, TileId};
use super::pyramid::{self, TACTICAL_TILE_PX};
use super::{Encoded, TacticalLimits, PPSQ_OPTIONS};
use crate::error::{ServerError, ServerResult};
use crate::AppState;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak};
use std::time::{Duration, Instant};

/// Version of the bake bodies.
pub const BAKE_FORMAT: u32 = 1;
/// Most jobs waiting at once; a full queue answers 503 `busy`.
pub const QUEUE_JOBS: usize = 8;
/// Default `--bake-workers`: one bake renders at a time (0 disables).
pub const DEFAULT_WORKERS: usize = 1;
/// Default `--bake-result-mib`: the finished tile sets' budget, enough
/// for one worst-case 128-ppsq set ([`worst_set_bytes`], about 362 MiB)
/// and several typical ones.
pub const DEFAULT_RESULT_MIB: usize = 384;
/// Squares a side of a baked cell.
pub const CELL_SQUARES: u32 = 64;
/// Upper bound of one encoded 512 px lossless WebP tile: its RGBA plus a
/// sixteenth for container and incompressible-data overhead.
pub const TILE_WORST_BYTES: u64 = (TACTICAL_TILE_PX as u64).pow(2) * 4 * 17 / 16;

/// Tiles in one baked cell's pyramid at `ppsq`.
#[must_use]
pub fn tile_count(ppsq: u32) -> u32 {
    let edge = CELL_SQUARES.saturating_mul(ppsq);
    let top = pyramid::max_zoom(edge, edge);
    (0..=top)
        .map(|z| {
            let (c, r) = pyramid::tile_grid(edge, edge, top, z);
            c * r
        })
        .sum()
}

/// Worst-case encoded bytes of one baked cell at the largest ppsq.
#[must_use]
pub fn worst_set_bytes() -> u64 {
    let ppsq = PPSQ_OPTIONS
        .iter()
        .copied()
        .max()
        .unwrap_or(super::DEFAULT_PPSQ);
    u64::from(tile_count(ppsq)) * TILE_WORST_BYTES
}

/// Worst-case resident bytes of the bake lane: one transient render with
/// its compositor buffers and pyramid, the parsed bodies of the queued
/// jobs (twice their JSON), one tile being encoded and the result budget;
/// nothing when `workers` is 0.
#[must_use]
pub fn admitted_bytes(limits: &TacticalLimits, workers: usize, result_bytes: u64) -> u64 {
    if workers == 0 {
        return 0;
    }
    let bodies = (QUEUE_JOBS as u64 + 1) * 2 * limits.max_body_bytes as u64;
    limits
        .transient_render_bytes()
        .saturating_add(bodies)
        .saturating_add(2 * TILE_WORST_BYTES)
        .saturating_add(result_bytes)
}

#[derive(Debug)]
struct Inner {
    store: Store,
    waiting: VecDeque<String>,
    active: usize,
    closed: bool,
    started: bool,
    /// Tests: a running job waits here, holding the bake lane.
    hold: bool,
    /// Tests: the next jobs panic instead of baking.
    panic: bool,
}

#[derive(Debug)]
struct Shared {
    inner: Mutex<Inner>,
    wake: Condvar,
    changed: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The bake queue, its worker and the finished tile sets.
#[derive(Debug)]
pub struct Baker {
    shared: Arc<Shared>,
    workers: usize,
}

fn status_of(id: &str, e: &Entry) -> BakeStatus {
    let (tiles, tile_count, total_bytes, error) = match &e.slot {
        Slot::Done(r) => (
            Some(r.tiles.clone()),
            u32::try_from(r.encoded.len()).unwrap_or(u32::MAX),
            u32::try_from(r.bytes).unwrap_or(u32::MAX),
            None,
        ),
        Slot::Failed(err) => (None, 0, 0, Some(err.clone())),
        Slot::Queued(_) | Slot::Running(_) => (None, 0, 0, None),
    };
    BakeStatus {
        format_version: BAKE_FORMAT,
        job_id: id.to_owned(),
        state: e.slot.state(),
        gx: e.gx,
        gy: e.gy,
        origin_gs: [
            i64::from(e.gx) * i64::from(CELL_SQUARES),
            i64::from(e.gy) * i64::from(CELL_SQUARES),
        ],
        world_grade: e.world_grade,
        tiles,
        tile_count,
        total_bytes,
        error,
        generator_version: crate::versions::SERVER_VERSION.to_owned(),
        ignored: e.ignored,
    }
}

fn unknown(id: &str) -> ServerError {
    ServerError::NotFound(format!(
        "no bake job {id:?}: never posted, deleted or evicted"
    ))
}

impl Baker {
    /// A queue served by `workers` threads (0 disables baking, above 1 is
    /// one: bakes render one at a time), holding finished sets within
    /// `result_bytes`.
    #[must_use]
    pub fn new(workers: usize, result_bytes: u64) -> Self {
        Self {
            shared: Arc::new(Shared {
                inner: Mutex::new(Inner {
                    store: Store::new(result_bytes),
                    waiting: VecDeque::new(),
                    active: 0,
                    closed: false,
                    started: false,
                    hold: false,
                    panic: false,
                }),
                wake: Condvar::new(),
                changed: Condvar::new(),
            }),
            workers: workers.min(1),
        }
    }

    /// Worker threads (0 or 1).
    #[must_use]
    pub const fn workers(&self) -> usize {
        self.workers
    }

    /// Queues `req` as job `id`, or reports the job already known under it.
    ///
    /// # Errors
    /// 501 when baking is off, 503 `busy` when the queue is full, 500 if
    /// the worker cannot start.
    pub fn submit(
        &self,
        state: &Arc<AppState>,
        id: String,
        req: BakeRequestParsed,
    ) -> ServerResult<BakeAccepted> {
        if self.workers == 0 {
            return Err(ServerError::Disabled(
                "baking is off on this server (--bake-workers 0)".into(),
            ));
        }
        let ignored = req.ignored;
        let accepted = |id: String, state| BakeAccepted {
            format_version: BAKE_FORMAT,
            status_url: format!("/v1/tactical/bake/{id}"),
            job_id: id,
            state,
            generator_version: crate::versions::SERVER_VERSION.to_owned(),
            ignored,
        };
        let mut q = self.shared.lock();
        if let Some(e) = q.store.get(&id) {
            return Ok(accepted(id, e.slot.state()));
        }
        if q.waiting.len() >= QUEUE_JOBS {
            return Err(ServerError::Busy(format!(
                "{QUEUE_JOBS} bake jobs are waiting; retry after one finishes"
            )));
        }
        if !q.started {
            self.start(state)?;
            q.started = true;
        }
        let entry = Entry {
            gx: req.gx,
            gy: req.gy,
            ppsq: req.ppsq,
            world_grade: req.world_grade,
            ignored: req.ignored,
            slot: Slot::Queued(Arc::new(req)),
        };
        q.store.queue(id.clone(), entry);
        q.waiting.push_back(id.clone());
        drop(q);
        self.shared.wake.notify_all();
        Ok(accepted(id, BakeState::Queued))
    }

    fn start(&self, state: &Arc<AppState>) -> ServerResult<()> {
        for i in 0..self.workers {
            let shared = Arc::clone(&self.shared);
            let weak = Arc::downgrade(state);
            std::thread::Builder::new()
                .name(format!("arda-bake-{i}"))
                .spawn(move || work(&shared, &weak))
                .map_err(|e| ServerError::Internal(format!("bake worker: {e}")))?;
        }
        Ok(())
    }

    /// The status of job `id`.
    ///
    /// # Errors
    /// [`ServerError::NotFound`] for unknown, deleted or evicted jobs.
    pub fn status(&self, id: &str) -> ServerResult<BakeStatus> {
        let q = self.shared.lock();
        q.store
            .get(id)
            .map(|e| status_of(id, e))
            .ok_or_else(|| unknown(id))
    }

    /// Tile `zxy` of finished job `id`.
    ///
    /// # Errors
    /// [`ServerError::NotFound`] for unknown jobs, jobs not `done` and
    /// tiles outside the pyramid.
    pub fn tile(&self, id: &str, zxy: TileId) -> ServerResult<Arc<Encoded>> {
        let q = self.shared.lock();
        let e = q.store.get(id).ok_or_else(|| unknown(id))?;
        let Slot::Done(r) = &e.slot else {
            return Err(ServerError::NotFound(format!(
                "bake job {id} is {:?}, not done",
                e.slot.state()
            )));
        };
        r.encoded.get(&zxy).cloned().ok_or_else(|| {
            ServerError::NotFound(format!(
                "tile {}/{}/{} is outside the pyramid (zoom 0..={})",
                zxy.0, zxy.1, zxy.2, r.tiles.max_zoom
            ))
        })
    }

    /// Forgets job `id` and frees its tiles; a running job's result is
    /// dropped when it finishes.
    ///
    /// # Errors
    /// [`ServerError::NotFound`] for unknown jobs.
    pub fn delete(&self, id: &str) -> ServerResult<()> {
        let mut q = self.shared.lock();
        if !q.store.forget(id) {
            return Err(unknown(id));
        }
        q.waiting.retain(|w| w != id);
        drop(q);
        self.shared.changed.notify_all();
        Ok(())
    }

    /// Waits until no job waits or runs, up to `timeout`; whether it did.
    #[must_use]
    pub fn wait_idle(&self, timeout: Duration) -> bool {
        self.wait(timeout, |q| q.waiting.is_empty() && q.active == 0)
    }

    /// Tests: while `on`, a running job waits holding the bake lane.
    #[doc(hidden)]
    pub fn hold(&self, on: bool) {
        self.shared.lock().hold = on;
        self.shared.changed.notify_all();
    }

    /// Tests: while `on`, jobs panic where they would bake (the worker
    /// must survive, review 3.6-7).
    #[doc(hidden)]
    pub fn inject_panic(&self, on: bool) {
        self.shared.lock().panic = on;
    }

    /// Tests: waits until job `id` is in `state`, up to `timeout`.
    #[doc(hidden)]
    #[must_use]
    pub fn wait_state(&self, id: &str, state: BakeState, timeout: Duration) -> bool {
        self.wait(timeout, |q| {
            q.store.get(id).is_some_and(|e| e.slot.state() == state)
        })
    }

    fn wait(&self, timeout: Duration, done: impl Fn(&Inner) -> bool) -> bool {
        let deadline = Instant::now() + timeout;
        let mut q = self.shared.lock();
        while !done(&q) {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            q = self
                .shared
                .changed
                .wait_timeout(q, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        true
    }
}

impl Drop for Baker {
    fn drop(&mut self) {
        let mut q = self.shared.lock();
        q.closed = true;
        q.hold = false;
        drop(q);
        self.shared.wake.notify_all();
        self.shared.changed.notify_all();
    }
}

/// The worker: take a job, bake it, record the outcome, repeat until the
/// state is gone.
fn work(shared: &Shared, state: &Weak<AppState>) {
    loop {
        let (id, req) = {
            let mut q = shared.lock();
            loop {
                if q.closed || state.strong_count() == 0 {
                    return;
                }
                if let Some(id) = q.waiting.pop_front() {
                    if q.store.start(&id) {
                        if let Some(Entry {
                            slot: Slot::Running(req),
                            ..
                        }) = q.store.get(&id)
                        {
                            let req = Arc::clone(req);
                            q.active += 1;
                            break (id, req);
                        }
                    }
                    continue;
                }
                q = shared
                    .wake
                    .wait_timeout(q, Duration::from_secs(1))
                    .unwrap_or_else(PoisonError::into_inner)
                    .0;
            }
        };
        shared.changed.notify_all();
        // A panicking job fails with `internal`; the worker lives on.
        let outcome = state.upgrade().map(|s| {
            crate::flight::isolate("bake job", || Ok(run(shared, &s, &id, &req))).unwrap_or_else(
                |e| {
                    eprintln!("arda-server tactical bake job {id} {e}");
                    Slot::Failed(e.body().error)
                },
            )
        });
        let mut q = shared.lock();
        if let Some(outcome) = outcome {
            let too_big = ServerError::ResourceLimit(format!(
                "the tile set exceeds the --bake-result-mib budget of {} bytes",
                q.store.budget()
            ))
            .body()
            .error;
            q.store.finish(&id, outcome, too_big);
        }
        q.active -= 1;
        drop(q);
        shared.changed.notify_all();
    }
}

/// Bakes one job in the bake lane and logs one line.
fn run(shared: &Shared, s: &AppState, id: &str, req: &BakeRequestParsed) -> Slot {
    let start = Instant::now();
    let (w, h) = s.query.cells();
    let world = (i64::from(w) * 64, i64::from(h) * 64);
    let lane = s.tactical.bake_lane();
    let wanted = {
        let mut q = shared.lock();
        while q.hold && !q.closed {
            q = shared
                .changed
                .wait(q)
                .unwrap_or_else(PoisonError::into_inner);
        }
        !q.closed && matches!(q.store.get(id).map(|e| &e.slot), Some(Slot::Running(_)))
    };
    if shared.lock().panic {
        panic!("injected bake panic (tests)");
    }
    // A job deleted before its render started is not rendered.
    let result = if wanted {
        s.tactical.bake(req, world)
    } else {
        Err(ServerError::NotFound(format!("bake job {id} was deleted")))
    };
    drop(lane);
    let (slot, outcome) = match result {
        Ok(r) => {
            let text = format!("done tiles={} bytes={}", r.encoded.len(), r.bytes);
            (Slot::Done(Arc::new(r)), text)
        }
        Err(e) => {
            let text = e.to_string();
            (Slot::Failed(e.body().error), text)
        }
    };
    eprintln!(
        "arda-server tactical bake job {id} cell {},{} ppsq={} {outcome} {:.3} ms",
        req.gx,
        req.gy,
        req.ppsq,
        start.elapsed().as_secs_f64() * 1e3
    );
    slot
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cell_set_has_the_documented_tile_count() {
        // 4096 px at 64 ppsq: 1 + 4 + 16 + 64 tiles; 8192 px at 128: 341.
        assert_eq!(tile_count(64), 85);
        assert_eq!(tile_count(128), 341);
        assert!(worst_set_bytes() < (DEFAULT_RESULT_MIB as u64) << 20);
    }

    #[test]
    fn admission_counts_nothing_when_baking_is_off() {
        let limits = TacticalLimits::default();
        assert_eq!(admitted_bytes(&limits, 0, 1 << 30), 0);
        let on = admitted_bytes(&limits, 1, 512 << 20);
        assert!(on > limits.transient_render_bytes() + (512 << 20));
    }
}
