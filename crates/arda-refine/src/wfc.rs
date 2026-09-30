//! Wave-function collapse over the block's corner lattice.
//!
//! Choosing a corner tile for every square is the same as choosing a class
//! for every square corner such that each square's four corners form a tile
//! of the vocabulary (at most two classes, and a pair that may meet). The
//! solver keeps a class set per corner, collapses the most constrained
//! corner first (ties by a hashed priority, so there is no scan-order
//! bias), picks by prior weight and propagates to arc consistency.
//!
//! A contradiction first triggers a local repair: the corners around it are
//! reset and re-propagated. After [`MAX_REPAIRS`] repairs the attempt is
//! abandoned; after [`MAX_ATTEMPTS`] attempts the block falls back to a
//! deterministic relaxed fill that is flagged for review (goal 47). The
//! solver never fails.

use crate::classes::{Class, Mask, COUNT};
use crate::hash::{hash3, Stream};
use crate::tiles::tile_of;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

/// Attempts before the relaxed fill.
pub const MAX_ATTEMPTS: u8 = 6;
/// Local repairs allowed per attempt.
pub const MAX_REPAIRS: u32 = 48;
/// Radius of a local repair, in corners.
const REPAIR_RADIUS: i64 = 4;

/// One corner-lattice problem.
#[derive(Debug, Clone)]
pub struct Problem {
    /// Corners per side (65 for a block).
    pub n: usize,
    /// Initial class sets; a single bit fixes a corner.
    pub masks: Vec<Mask>,
    /// Choice weights per corner and class.
    pub weights: Vec<[f32; COUNT]>,
    /// World seed.
    pub seed: u64,
    /// Global key of the block (its cell).
    pub key: (i64, i64),
}

/// A solved corner lattice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Solution {
    /// Class per corner, row-major.
    pub classes: Vec<Class>,
    /// Whether the relaxed fill was used.
    pub relaxed: bool,
    /// Attempts used (1-based).
    pub attempts: u8,
    /// Local repairs over all attempts.
    pub repairs: u32,
    /// Squares whose corners form no tile (only after a relaxed fill).
    pub illegal: Vec<usize>,
}

fn supported(c: usize, others: [Mask; 3]) -> bool {
    let bit = 1 << c;
    if others.iter().all(|d| d & bit != 0) {
        return true;
    }
    crate::tiles::completable(bit, &others)
}

struct State<'p> {
    p: &'p Problem,
    dom: Vec<Mask>,
    queued: Vec<bool>,
    queue: VecDeque<usize>,
    heap: BinaryHeap<Reverse<(u32, u32, usize)>>,
    prio: Vec<u32>,
}

impl<'p> State<'p> {
    fn new(p: &'p Problem, attempt: u8) -> Self {
        let prio = (0..p.masks.len())
            .map(|i| {
                let h = hash3(
                    p.seed,
                    0xF1C0 + u64::from(attempt),
                    p.key.0,
                    p.key.1,
                    crate::grid::span(i),
                );
                u32::try_from(h >> 32).unwrap_or(0)
            })
            .collect();
        Self {
            p,
            dom: p.masks.clone(),
            queued: vec![false; p.masks.len()],
            queue: VecDeque::new(),
            heap: BinaryHeap::new(),
            prio,
        }
    }

    fn xy(&self, i: usize) -> (i64, i64) {
        let n = self.p.n;
        (crate::grid::span(i % n), crate::grid::span(i / n))
    }

    fn idx(&self, x: i64, y: i64) -> Option<usize> {
        let n = crate::grid::span(self.p.n);
        (x >= 0 && y >= 0 && x < n && y < n).then(|| usize::try_from(y * n + x).unwrap_or(0))
    }

    fn push_heap(&mut self, i: usize) {
        let c = self.dom[i].count_ones();
        if c > 1 {
            self.heap.push(Reverse((c, self.prio[i], i)));
        }
    }

    fn enqueue(&mut self, i: usize) {
        if !self.queued[i] {
            self.queued[i] = true;
            self.queue.push_back(i);
        }
    }

    fn enqueue_around(&mut self, i: usize) {
        let (x, y) = self.xy(i);
        for dy in -1..=1 {
            for dx in -1..=1 {
                if let Some(j) = self.idx(x + dx, y + dy) {
                    if j != i {
                        self.enqueue(j);
                    }
                }
            }
        }
    }

    /// Removes unsupported classes at `i`; returns the new domain.
    fn revise(&self, i: usize) -> Mask {
        let (x, y) = self.xy(i);
        let mut dom = self.dom[i];
        // The four squares having this corner, as the other three corners.
        for (sx, sy) in [(-1, -1), (0, -1), (-1, 0), (0, 0)] {
            let (x0, y0) = (x + sx, y + sy);
            let corners = [(x0, y0), (x0 + 1, y0), (x0 + 1, y0 + 1), (x0, y0 + 1)];
            let mut others = [Mask::MAX; 3];
            let mut k = 0;
            let mut inside = true;
            for (cx, cy) in corners {
                if (cx, cy) == (x, y) {
                    continue;
                }
                match self.idx(cx, cy) {
                    Some(j) => others[k] = self.dom[j],
                    None => inside = false,
                }
                k += 1;
            }
            if !inside {
                continue;
            }
            let mut bits = dom;
            while bits != 0 {
                let c = bits.trailing_zeros() as usize;
                bits &= bits - 1;
                if !supported(c, others) {
                    dom &= !(1 << c);
                }
            }
        }
        dom
    }

    /// Propagates to a fixed point; returns a corner left empty, if any.
    fn propagate(&mut self) -> Option<usize> {
        while let Some(i) = self.queue.pop_front() {
            self.queued[i] = false;
            let d = self.revise(i);
            if d != self.dom[i] {
                self.dom[i] = d;
                if d == 0 {
                    self.queue.clear();
                    self.queued.iter_mut().for_each(|q| *q = false);
                    return Some(i);
                }
                self.push_heap(i);
                self.enqueue_around(i);
            }
        }
        None
    }

    fn repair(&mut self, at: usize) {
        let (x, y) = self.xy(at);
        let outer = REPAIR_RADIUS + 3;
        for dy in -outer..=outer {
            for dx in -outer..=outer {
                let Some(j) = self.idx(x + dx, y + dy) else {
                    continue;
                };
                let near = dx.abs() <= REPAIR_RADIUS && dy.abs() <= REPAIR_RADIUS;
                if near || self.dom[j].count_ones() != 1 {
                    self.dom[j] = self.p.masks[j];
                    self.push_heap(j);
                }
            }
        }
        for dy in -outer - 1..=outer + 1 {
            for dx in -outer - 1..=outer + 1 {
                if let Some(j) = self.idx(x + dx, y + dy) {
                    self.enqueue(j);
                }
            }
        }
    }

    /// Transition-aware weight: a class also earns half the weight of the
    /// best class it may neighbour that this corner can no longer take, so
    /// a corner cut off from its preferred class picks the transition that
    /// leads back to it instead of spreading an unwanted class.
    fn effective(&self, i: usize, c: usize) -> f64 {
        let w = &self.p.weights[i];
        let mut lost = crate::classes::PARTNERS[c] & !self.dom[i];
        let mut boost = 0.0f32;
        while lost != 0 {
            let d = lost.trailing_zeros() as usize;
            lost &= lost - 1;
            boost = boost.max(w[d]);
        }
        // Classes the prior all but rules out here (another climate or
        // cover) never earn a transition boost.
        if w[c] < 1e-4 * boost {
            return f64::from(w[c]);
        }
        f64::from(w[c]) + 0.5 * f64::from(boost)
    }

    fn pick(&self, i: usize, r: &mut Stream) -> Mask {
        let mut total = 0.0;
        let mut bits = self.dom[i];
        while bits != 0 {
            let c = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            total += self.effective(i, c);
        }
        let mut target = r.next_unit() * total;
        let mut bits = self.dom[i];
        let mut last = 0;
        while bits != 0 {
            let c = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            last = c;
            target -= self.effective(i, c);
            if target < 0.0 {
                return 1 << c;
            }
        }
        1 << last
    }

    /// One attempt. `Ok` when every corner is decided, `Err` with repairs
    /// used when the attempt is abandoned.
    fn run(&mut self, attempt: u8) -> Result<u32, u32> {
        let mut r = Stream::new(
            self.p.seed,
            0xC011 + u64::from(attempt),
            self.p.key.0,
            self.p.key.1,
        );
        let mut repairs = 0;
        for i in 0..self.dom.len() {
            self.enqueue(i);
            self.push_heap(i);
        }
        if let Some(bad) = self.propagate() {
            repairs += 1;
            self.repair(bad);
            if self.propagate().is_some() {
                return Err(repairs);
            }
        }
        while let Some(Reverse((count, _, i))) = self.heap.pop() {
            if self.dom[i].count_ones() != count || count <= 1 {
                continue;
            }
            self.dom[i] = self.pick(i, &mut r);
            self.enqueue_around(i);
            while let Some(bad) = self.propagate() {
                repairs += 1;
                if repairs > MAX_REPAIRS {
                    return Err(repairs);
                }
                self.repair(bad);
            }
        }
        if self.dom.iter().all(|d| d.count_ones() == 1) {
            Ok(repairs)
        } else {
            Err(repairs)
        }
    }
}

fn classes_of(dom: &[Mask]) -> Vec<Class> {
    dom.iter()
        .map(|d| crate::classes::first(*d).unwrap_or(Class::Grass))
        .collect()
}

/// Squares whose corners form no tile.
#[must_use]
pub fn illegal_squares(n: usize, classes: &[Class]) -> Vec<usize> {
    let mut bad = Vec::new();
    for y in 0..n - 1 {
        for x in 0..n - 1 {
            let c = [
                classes[y * n + x],
                classes[y * n + x + 1],
                classes[(y + 1) * n + x + 1],
                classes[(y + 1) * n + x],
            ];
            if tile_of(c).is_none() {
                bad.push(y * (n - 1) + x);
            }
        }
    }
    bad
}

/// Solves a corner lattice; never fails.
#[must_use]
pub fn solve(p: &Problem) -> Solution {
    let mut repairs_total = 0;
    for attempt in 0..MAX_ATTEMPTS {
        let mut st = State::new(p, attempt);
        match st.run(attempt) {
            Ok(r) => {
                repairs_total += r;
                let classes = classes_of(&st.dom);
                if illegal_squares(p.n, &classes).is_empty() {
                    return Solution {
                        classes,
                        relaxed: false,
                        attempts: attempt + 1,
                        repairs: repairs_total,
                        illegal: Vec::new(),
                    };
                }
            }
            Err(r) => repairs_total += r,
        }
    }
    relaxed(p, repairs_total)
}

/// The deterministic relaxed fill: fixed corners kept, every other corner
/// takes its highest-weight allowed class.
#[must_use]
pub fn relaxed(p: &Problem, repairs: u32) -> Solution {
    let classes: Vec<Class> = p
        .masks
        .iter()
        .zip(&p.weights)
        .map(|(m, w)| {
            let mut best: Option<(f32, usize)> = None;
            let mut bits = *m;
            while bits != 0 {
                let c = bits.trailing_zeros() as usize;
                bits &= bits - 1;
                if best.is_none_or(|b| w[c] > b.0) {
                    best = Some((w[c], c));
                }
            }
            best.and_then(|b| Class::from_index(b.1))
                .unwrap_or(Class::Grass)
        })
        .collect();
    let illegal = illegal_squares(p.n, &classes);
    Solution {
        classes,
        relaxed: true,
        attempts: MAX_ATTEMPTS,
        repairs,
        illegal,
    }
}
