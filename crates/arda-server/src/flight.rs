//! Single-flight builds, cancellable gates and request cancellation
//! (review 3.6-1, 3.6-2; logic/16 §api-cache).
//!
//! - [`Flights`]: a key builds at most once at a time. A request for a key
//!   already building waits for that build and takes its result; if the
//!   build fails or is cancelled, one waiter takes over.
//! - [`Gate`]: at most `n` holders at a time (a render lane, the encode
//!   slot, composition). Waiting is cancellable, and tests can hold a gate
//!   shut to line requests up behind it.
//! - [`Cancel`]: set when a request's future is dropped (its client went
//!   away). Work running for it checks [`check`] before each costly stage
//!   and while waiting at a flight or gate, so abandoned requests leave
//!   the queue instead of rendering ahead of live ones. A stage already
//!   running finishes; nothing it caches can differ, so bytes stay
//!   deterministic.
//!
//! The current request's token travels in a thread-local set by
//! [`Cancel::scope`] around the blocking work, so the tactical code reads
//! it without threading a parameter through every call. Work outside a
//! scope (prefetch and bake workers) is never cancelled.

use crate::error::{ServerError, ServerResult};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

/// How often a parked waiter rechecks its cancellation flag.
const TICK: Duration = Duration::from_millis(20);

thread_local! {
    static CURRENT: RefCell<Option<Cancel>> = const { RefCell::new(None) };
}

/// A request's cancellation flag.
#[derive(Debug, Clone, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    /// Marks the request abandoned.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Whether the request was abandoned.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    /// Runs `work` with this token as the thread's current one.
    pub fn scope<T>(&self, work: impl FnOnce() -> T) -> T {
        struct Restore(Option<Cancel>);
        impl Drop for Restore {
            fn drop(&mut self) {
                let old = self.0.take();
                CURRENT.with(|c| *c.borrow_mut() = old);
            }
        }
        let old = CURRENT.with(|c| c.borrow_mut().replace(self.clone()));
        let _restore = Restore(old);
        work()
    }

    /// A guard that cancels this token when dropped, unless disarmed: the
    /// async side of a request holds it while the blocking work runs.
    #[must_use]
    pub fn on_drop(&self) -> CancelOnDrop {
        CancelOnDrop(Some(self.clone()))
    }
}

/// Cancels its token when dropped before [`CancelOnDrop::disarm`].
#[derive(Debug)]
pub struct CancelOnDrop(Option<Cancel>);

impl CancelOnDrop {
    /// The work finished: dropping no longer cancels.
    pub fn disarm(mut self) {
        self.0 = None;
    }
}

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if let Some(c) = self.0.take() {
            c.cancel();
        }
    }
}

/// Whether the current request (if any) was abandoned.
#[must_use]
pub fn cancelled() -> bool {
    CURRENT.with(|c| c.borrow().as_ref().is_some_and(Cancel::is_cancelled))
}

/// Stops the current request's work if its client went away.
///
/// # Errors
/// [`ServerError::Busy`] (never sent: the client is gone).
pub fn check() -> ServerResult<()> {
    if cancelled() {
        Err(ServerError::Busy(
            "the request was abandoned before its work ran".into(),
        ))
    } else {
        Ok(())
    }
}

fn relock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Counts a parked waiter for as long as it lives.
struct Parked<'a>(&'a AtomicUsize);

impl<'a> Parked<'a> {
    fn new(n: &'a AtomicUsize) -> Self {
        n.fetch_add(1, Ordering::SeqCst);
        Self(n)
    }
}

impl Drop for Parked<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Keys building now; see the module docs.
#[derive(Debug)]
pub struct Flights<K> {
    set: Mutex<BTreeSet<K>>,
    done: Condvar,
    parked: AtomicUsize,
}

impl<K> Default for Flights<K> {
    fn default() -> Self {
        Self {
            set: Mutex::new(BTreeSet::new()),
            done: Condvar::new(),
            parked: AtomicUsize::new(0),
        }
    }
}

/// Clears an in-flight key and wakes its waiters, even on a panic.
struct Landing<'a, K: Ord> {
    flights: &'a Flights<K>,
    key: Option<K>,
}

impl<K: Ord> Drop for Landing<'_, K> {
    fn drop(&mut self) {
        if let Some(key) = self.key.take() {
            relock(&self.flights.set).remove(&key);
            self.flights.done.notify_all();
        }
    }
}

impl<K: Ord + Clone> Flights<K> {
    /// `lookup`'s value, or `build`'s, built by this caller alone: while
    /// another caller builds `key`, this one waits and looks up again.
    ///
    /// # Errors
    /// `lookup`'s or `build`'s errors; [`check`]'s when the current request
    /// is abandoned before its build starts.
    pub fn run<V>(
        &self,
        key: &K,
        lookup: impl Fn() -> ServerResult<Option<V>>,
        build: impl FnOnce() -> ServerResult<V>,
    ) -> ServerResult<V> {
        if let Some(v) = lookup()? {
            return Ok(v);
        }
        {
            let mut set = relock(&self.set);
            if set.contains(key) {
                let _parked = Parked::new(&self.parked);
                while set.contains(key) {
                    if cancelled() {
                        drop(set);
                        check()?;
                        set = relock(&self.set);
                        continue;
                    }
                    set = self
                        .done
                        .wait_timeout(set, TICK)
                        .unwrap_or_else(PoisonError::into_inner)
                        .0;
                }
            }
            if let Some(v) = lookup()? {
                return Ok(v);
            }
            check()?;
            set.insert(key.clone());
        }
        let _landing = Landing {
            flights: self,
            key: Some(key.clone()),
        };
        build()
    }

    /// Callers waiting for another's build now (tests).
    #[must_use]
    pub fn parked(&self) -> usize {
        self.parked.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Default)]
struct GateState {
    inside: usize,
    hold: bool,
}

/// At most `capacity` holders at a time; see the module docs.
#[derive(Debug)]
pub struct Gate {
    capacity: usize,
    state: Mutex<GateState>,
    free: Condvar,
    parked: AtomicUsize,
}

impl Default for Gate {
    fn default() -> Self {
        Self::new(1)
    }
}

/// One place inside a [`Gate`], given back on drop (even on a panic).
#[derive(Debug)]
pub struct Pass<'a>(&'a Gate);

impl Drop for Pass<'_> {
    fn drop(&mut self) {
        relock(&self.0.state).inside -= 1;
        self.0.free.notify_all();
    }
}

impl Gate {
    /// A gate admitting `capacity` holders (at least one).
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            state: Mutex::new(GateState::default()),
            free: Condvar::new(),
            parked: AtomicUsize::new(0),
        }
    }

    /// Holders admitted at once.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Waits for a place. An abandoned request gives up its wait.
    ///
    /// # Errors
    /// [`check`]'s when the current request is abandoned.
    pub fn enter(&self) -> ServerResult<Pass<'_>> {
        check()?;
        let mut s = relock(&self.state);
        if s.hold || s.inside >= self.capacity {
            let _parked = Parked::new(&self.parked);
            while s.hold || s.inside >= self.capacity {
                if cancelled() {
                    drop(s);
                    check()?;
                    s = relock(&self.state);
                    continue;
                }
                s = self
                    .free
                    .wait_timeout(s, TICK)
                    .unwrap_or_else(PoisonError::into_inner)
                    .0;
            }
        }
        s.inside += 1;
        Ok(Pass(self))
    }

    /// As [`Self::enter`], for work that is never cancelled (workers).
    pub fn enter_uncancelled(&self) -> Pass<'_> {
        let mut s = relock(&self.state);
        if s.hold || s.inside >= self.capacity {
            let _parked = Parked::new(&self.parked);
            while s.hold || s.inside >= self.capacity {
                s = self
                    .free
                    .wait_timeout(s, TICK)
                    .unwrap_or_else(PoisonError::into_inner)
                    .0;
            }
        }
        s.inside += 1;
        Pass(self)
    }

    /// Tests: while `on`, nobody new enters.
    pub fn hold(&self, on: bool) {
        relock(&self.state).hold = on;
        self.free.notify_all();
    }

    /// Callers waiting at the gate now (tests).
    #[must_use]
    pub fn parked(&self) -> usize {
        self.parked.load(Ordering::SeqCst)
    }

    /// Holders inside now.
    #[must_use]
    pub fn inside(&self) -> usize {
        relock(&self.state).inside
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    #[test]
    fn concurrent_callers_of_one_key_build_once() {
        let flights = Arc::new(Flights::<u32>::default());
        let cache = Arc::new(Mutex::new(None::<u32>));
        let builds = Arc::new(AtomicU32::new(0));
        let gate = Arc::new(Gate::new(1));
        gate.hold(true);
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let (f, c, b, g) = (
                    Arc::clone(&flights),
                    Arc::clone(&cache),
                    Arc::clone(&builds),
                    Arc::clone(&gate),
                );
                std::thread::spawn(move || {
                    f.run(
                        &7,
                        || Ok(*c.lock().unwrap()),
                        || {
                            let _pass = g.enter()?;
                            b.fetch_add(1, Ordering::SeqCst);
                            *c.lock().unwrap() = Some(42);
                            Ok(42)
                        },
                    )
                })
            })
            .collect();
        while flights.parked() + gate.parked() < 8 {
            std::thread::yield_now();
        }
        gate.hold(false);
        for t in threads {
            assert_eq!(t.join().unwrap().unwrap(), 42);
        }
        assert_eq!(builds.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn an_abandoned_waiter_leaves_and_a_waiter_takes_over_its_build() {
        let gate = Gate::new(1);
        gate.hold(true);
        let flights = Flights::<u32>::default();
        let cancel = Cancel::default();
        std::thread::scope(|s| {
            let owner = s.spawn(|| {
                cancel.scope(|| flights.run(&1, || Ok(None::<u32>), || gate.enter().map(|_| 1)))
            });
            let waiter = s.spawn(|| {
                while flights.parked() + gate.parked() < 1 {
                    std::thread::yield_now();
                }
                flights.run(&1, || Ok(None::<u32>), || gate.enter().map(|_| 2))
            });
            while flights.parked() + gate.parked() < 2 {
                std::thread::yield_now();
            }
            cancel.cancel();
            assert!(matches!(owner.join().unwrap(), Err(ServerError::Busy(_))));
            gate.hold(false);
            assert_eq!(waiter.join().unwrap().unwrap(), 2);
        });
        assert_eq!(gate.inside(), 0);
    }
}
