//! Full-resolution renders in two lanes (goal 67; logic/16 §api-cache,
//! §api-prefetch).
//!
//! Requests render one at a time in the request lane; the prefetch workers
//! render one at a time in their own lane, so warming neighbours never
//! queues a request behind a speculative render (admission counts one
//! transient render per lane). A key renders at most once at a time: a
//! request for a key already rendering, in either lane, waits for that
//! render and takes its result instead of starting a second one.

use super::{cached, stage, RenderKey, Tactical};
use crate::error::{lock, ServerResult};
use arda_tactical::{
    render, render_region, render_region_tinted, RenderOptions, Rgba, TacticalLayout,
};
use std::collections::BTreeSet;
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::Instant;

/// Which lane a render runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lane {
    /// A client request.
    Request,
    /// A prefetch worker.
    Prefetch,
}

/// The lane locks and the keys rendering now.
#[derive(Debug, Default)]
pub(crate) struct Lanes {
    request: Mutex<()>,
    prefetch: Mutex<()>,
    /// PNG encoding and pyramid building after a render: one at a time
    /// across both lanes, so admission counts one transient encode (review
    /// round 2 #32: they ran outside the render locks, unbounded).
    encode: Mutex<()>,
    inflight: Mutex<BTreeSet<RenderKey>>,
    done: Condvar,
}

/// Clears an in-flight key and wakes its waiters, even on a panic.
struct InFlight<'a> {
    lanes: &'a Lanes,
    key: RenderKey,
}

impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        let mut set = self
            .lanes
            .inflight
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        set.remove(&self.key);
        drop(set);
        self.lanes.done.notify_all();
    }
}

impl Lanes {
    /// Holds the single encode slot until the guard drops; waits for it
    /// when another encode runs. Poisoning is ignored: the lock guards no data.
    pub(crate) fn encode_slot(&self) -> std::sync::MutexGuard<'_, ()> {
        self.encode.lock().unwrap_or_else(PoisonError::into_inner)
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
        if let Some(hit) = cached(&self.renders, key)? {
            return Ok(hit);
        }
        let lanes = &self.lanes;
        let _marker = {
            let mut set = lanes
                .inflight
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            while set.contains(key) {
                set = lanes.done.wait(set).unwrap_or_else(PoisonError::into_inner);
            }
            if let Some(hit) = cached(&self.renders, key)? {
                return Ok(hit);
            }
            set.insert(key.clone());
            InFlight {
                lanes,
                key: key.clone(),
            }
        };
        // The lane guards protect no data, so a panicked render must not
        // wedge every later one behind a poisoned lock.
        let _one = match lane {
            Lane::Request => &lanes.request,
            Lane::Prefetch => &lanes.prefetch,
        }
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
        if let Some(hit) = cached(&self.renders, key)? {
            return Ok(hit);
        }
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
        let bytes = img.data.len();
        Ok(lock(&self.renders)?.insert(key.clone(), Arc::new(img), bytes))
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
                    let _slot = lanes.encode_slot();
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
        let held = lanes.encode_slot();
        assert!(lanes.encode.try_lock().is_err());
        drop(held);
        assert!(lanes.encode.try_lock().is_ok());
    }
}
