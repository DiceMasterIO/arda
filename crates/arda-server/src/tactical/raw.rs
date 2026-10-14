//! Full-resolution renders in two cached lanes, plus the uncached bake lane
//! (goal 67, 1.0 task 04; logic/16 §api-cache, §api-prefetch).
//!
//! Requests render one at a time in the request lane; the prefetch workers
//! render one at a time in their own lane, so warming neighbours never
//! queues a request behind a speculative render (admission counts one
//! transient render per lane). Every build is single-flight
//! ([`crate::flight::Flights`]): a request for a render, pyramid, PNG,
//! tile, body or block already building, in either lane, waits for that
//! build and takes its result instead of starting a second one (review
//! 3.6-1). A request whose client went away leaves the lane's queue and
//! starts no further stage (review 3.6-2), so live requests are not
//! queued behind abandoned ones.
//!
//! The bake lane (`POST /v1/tactical/bake`) renders a client-edited cell
//! one job at a time and never touches the render cache: its own lock, no
//! cache reads or writes, no in-flight sharing.

use super::{cached, stage, RenderKey, Tactical};
use crate::error::{lock, ServerResult};
use crate::flight::{Flights, Gate, Pass};
use arda_tactical::{
    render, render_region, render_region_tinted, RenderOptions, Rgba, TacticalLayout,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

/// Which lane a render runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lane {
    /// A client request.
    Request,
    /// A prefetch worker.
    Prefetch,
}

/// The lane gates, the keys building now and the build counters.
#[derive(Debug, Default)]
pub(crate) struct Lanes {
    request: Gate,
    prefetch: Gate,
    /// `POST /bake` jobs: one render, pyramid and tile encode at a time.
    bake: Mutex<()>,
    /// PNG encoding and pyramid building after a render: one at a time
    /// across both lanes, so admission counts one transient encode (review
    /// round 2 #32: they ran outside the render locks, unbounded).
    encode: Gate,
    /// Full-resolution renders building now.
    pub(crate) renders: Flights<RenderKey>,
    /// Tile pyramids building now.
    pub(crate) pyramids: Flights<RenderKey>,
    /// PNGs encoding now.
    pub(crate) pngs: Flights<RenderKey>,
    /// Tiles encoding now.
    pub(crate) tiles: Flights<super::TileKey>,
    /// JSON bodies building now.
    pub(crate) bodies: Flights<super::cells::BodyKey>,
    /// Blocks composing now (the source keeps the composed cache).
    pub(crate) blocks: Flights<super::block::BlockRequest>,
    /// Renders and pyramid builds run (tests).
    built: [AtomicU64; 2],
}

impl Lanes {
    /// Holds the single encode slot until the pass drops; waits for it
    /// when another encode runs.
    ///
    /// # Errors
    /// The current request was abandoned while it waited.
    pub(crate) fn encode_slot(&self) -> ServerResult<Pass<'_>> {
        self.encode.enter()
    }

    /// As [`Self::encode_slot`], for the bake worker (never cancelled).
    pub(crate) fn encode_slot_uncancelled(&self) -> Pass<'_> {
        self.encode.enter_uncancelled()
    }

    /// Counts one pyramid build.
    pub(crate) fn count_pyramid(&self) {
        self.built[1].fetch_add(1, Ordering::SeqCst);
    }

    /// Renders and pyramid builds run so far.
    pub(crate) fn built(&self) -> [u64; 2] {
        [
            self.built[0].load(Ordering::SeqCst),
            self.built[1].load(Ordering::SeqCst),
        ]
    }

    /// The request lane's gate (tests hold it shut).
    pub(crate) const fn request_gate(&self) -> &Gate {
        &self.request
    }

    /// Callers parked at the request lane, the encode slot or a render.
    pub(crate) fn parked(&self) -> usize {
        self.request.parked()
            + self.encode.parked()
            + self.renders.parked()
            + self.pyramids.parked()
            + self.pngs.parked()
            + self.tiles.parked()
            + self.bodies.parked()
            + self.blocks.parked()
    }
}

impl Tactical {
    /// The full-resolution render of `key`, from cache or rendered in the
    /// request lane.
    pub(super) fn raw(&self, key: &RenderKey, layout: &TacticalLayout) -> ServerResult<Arc<Rgba>> {
        self.raw_in(key, layout, Lane::Request)
    }

    /// As [`Self::raw`], in `lane`.
    pub(super) fn raw_in(
        &self,
        key: &RenderKey,
        layout: &TacticalLayout,
        lane: Lane,
    ) -> ServerResult<Arc<Rgba>> {
        let lanes = &self.lanes;
        lanes.renders.run(
            key,
            || cached(&self.renders, key),
            || {
                // An abandoned request leaves the lane's queue here, and
                // checks once more after entering (logic/16 §api-cache).
                let _lane = match lane {
                    Lane::Request => lanes.request.enter()?,
                    Lane::Prefetch => lanes.prefetch.enter()?,
                };
                if let Some(hit) = cached(&self.renders, key)? {
                    return Ok(hit);
                }
                crate::flight::check()?;
                lanes.built[0].fetch_add(1, Ordering::SeqCst);
                let img = self.render_key(key, layout)?;
                let bytes = img.data.len();
                Ok(lock(&self.renders)?.insert(key.clone(), Arc::new(img), bytes))
            },
        )
    }

    /// Holds the bake lane until the guard drops: one bake renders at a
    /// time, beside (never in) the request and prefetch lanes.
    pub(super) fn bake_lane(&self) -> std::sync::MutexGuard<'_, ()> {
        self.lanes
            .bake
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Renders `key` without reading or writing the render cache or the
    /// in-flight set (`POST /bake`); the caller holds [`Self::bake_lane`].
    pub(super) fn render_uncached(
        &self,
        key: &RenderKey,
        layout: &TacticalLayout,
    ) -> ServerResult<Rgba> {
        self.render_key(key, layout)
    }

    /// Admits `layout` and renders `key` (no cache, no lane).
    fn render_key(&self, key: &RenderKey, layout: &TacticalLayout) -> ServerResult<Rgba> {
        self.admit(layout, key.ppsq)?;
        let opts = RenderOptions {
            ppsq: key.ppsq,
            grid: key.grid,
            lighting: true,
        };
        let start = Instant::now();
        // A crop renders only its own pixels (the apron still shapes them).
        let img = match (key.crop, key.world_grade) {
            (Some(c), false) => {
                let px = c.map(|v| v.saturating_mul(key.ppsq));
                render_region(layout, &self.library, key.seed(), &opts, px)?
            }
            (None, false) => render(layout, &self.library, key.seed(), &opts)?,
            // The opt-in world grade (goal 49).
            (crop, true) => {
                let c = crop.unwrap_or([0, 0, layout.width, layout.height]);
                let px = c.map(|v| v.saturating_mul(key.ppsq));
                let tint = self.world_tint(layout)?;
                render_region_tinted(layout, &self.library, key.seed(), &opts, px, &tint)?
            }
        };
        stage("render", &img, start);
        Ok(img)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Review round 2 #32: PNG encodes and pyramid builds share one slot,
    /// so admission's single transient encode holds however many requests
    /// finish their renders at once.
    #[test]
    fn one_encode_runs_at_a_time() {
        let lanes = Arc::new(Lanes::default());
        let running = Arc::new(Mutex::new((0_u32, 0_u32)));
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let (lanes, running) = (Arc::clone(&lanes), Arc::clone(&running));
                std::thread::spawn(move || {
                    let _slot = lanes.encode_slot().unwrap();
                    {
                        let mut r = running.lock().unwrap();
                        r.0 += 1;
                        r.1 = r.1.max(r.0);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    running.lock().unwrap().0 -= 1;
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
        assert_eq!(running.lock().unwrap().1, 1);
        // A slot held elsewhere makes the next encode wait.
        let held = lanes.encode_slot().unwrap();
        assert_eq!(lanes.encode.inside(), 1);
        drop(held);
        assert_eq!(lanes.encode.inside(), 0);
    }
}
