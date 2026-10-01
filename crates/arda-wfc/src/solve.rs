//! The solver: most-constrained cell first (fewest options, ties by a
//! hashed priority), a weighted pick from the problem's integer weights,
//! and arc-consistency propagation after every choice.
//!
//! Before the free fill, **anchors** are placed: each names candidate
//! `(cell, tiles)` options and one is collapsed by weighted choice (the way
//! a programme demands "a hearth somewhere in the hall"). **Limits** cap
//! how many cells of a scope may take a tile group; reaching a cap removes
//! the group from the scope's undecided cells. A finished grid must also
//! pass the caller's global check (reachability, required rooms).
//!
//! A contradiction or a failed check abandons the attempt; the next one is
//! subseeded by `(seed, key, attempt)`. After the last attempt the caller
//! applies its deterministic relaxed fill and marks it (goal 47): the
//! solver itself never panics and never loops unboundedly.

use crate::frontier::{Frontier, WorkQueue};
use crate::hash::{hash3, mix, Stream};
use crate::rules::{Dir, Rules};
use crate::set::TileSet;

/// Default attempts before the relaxed fill (logic/03 step 4: ≤ 8).
pub const MAX_ATTEMPTS: u8 = 8;

/// Before the free fill, make at least `count` of these options hold.
#[derive(Debug, Clone, Default)]
pub struct Anchor {
    /// `(cell, tiles)` pairs; a chosen cell takes one of its tiles.
    pub options: Vec<(usize, TileSet)>,
    /// How many options must hold.
    pub count: u32,
}

/// At most `max` cells of `cells` (every cell when empty) take a tile of
/// `tiles`.
#[derive(Debug, Clone, Default)]
pub struct Limit {
    /// The tile group.
    pub tiles: TileSet,
    /// The scope; empty means the whole grid.
    pub cells: Vec<usize>,
    /// The cap.
    pub max: u32,
}

/// Weight of tile `t` at cell `i`; zero weights are used only when every
/// option of the cell is zero.
pub type Weight<'a> = &'a dyn Fn(usize, usize) -> u32;

/// One WFC problem over a `w × h` grid.
pub struct Problem<'a> {
    /// The adjacency rules.
    pub rules: &'a Rules,
    /// Width.
    pub w: usize,
    /// Height.
    pub h: usize,
    /// Initial options per cell, row-major; a single tile fixes a cell.
    pub domains: Vec<TileSet>,
    /// Edge kind per cell and side (`Dir::index`), including the grid's
    /// outer sides. Both cells of an edge must agree.
    pub edges: Vec<[u8; 4]>,
    /// Choice weights.
    pub weight: Weight<'a>,
    /// Anchors, placed in order.
    pub anchors: Vec<Anchor>,
    /// Caps.
    pub limits: Vec<Limit>,
    /// World seed.
    pub seed: u64,
    /// Stable key of the problem (e.g. settlement, building, kind).
    pub key: [i64; 3],
}

/// A solved grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Solution {
    /// Tile per cell, row-major.
    pub tiles: Vec<u16>,
    /// Attempts used (1-based).
    pub attempts: u8,
}

/// Every attempt failed; the caller applies its relaxed fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Failure {
    /// Attempts made.
    pub attempts: u8,
    /// Attempts that finished but failed the caller's check (the rest met
    /// a contradiction).
    pub rejected: u8,
}

impl Problem<'_> {
    pub(crate) fn neighbour(&self, i: usize, d: Dir) -> Option<usize> {
        let (x, y) = (i % self.w, i / self.w);
        match d {
            Dir::N => (y > 0).then(|| i - self.w),
            Dir::S => (y + 1 < self.h).then(|| i + self.w),
            Dir::W => (x > 0).then(|| i - 1),
            Dir::E => (x + 1 < self.w).then(|| i + 1),
        }
    }

    pub(crate) fn kind(&self, i: usize, d: Dir) -> usize {
        self.edges.get(i).map_or(0, |e| usize::from(e[d.index()]))
    }
}

/// A contradiction at a cell (or at none, for a failed anchor).
#[derive(Debug, Clone, Copy)]
struct Contradiction(Option<usize>);

/// Local repairs allowed per attempt.
pub const MAX_REPAIRS: u32 = 24;
/// Radius of a local repair, in cells (Chebyshev).
const REPAIR_RADIUS: usize = 2;

struct Run<'p, 'a> {
    p: &'p Problem<'a>,
    base: Vec<TileSet>,
    dom: Vec<TileSet>,
    queue: WorkQueue,
    front: Frontier,
    prio: Vec<u32>,
    decided: Vec<bool>,
    pinned: Vec<bool>,
    counts: Vec<u32>,
    scopes: Vec<Vec<bool>>,
    repairs: u32,
}

impl<'p, 'a> Run<'p, 'a> {
    fn new(p: &'p Problem<'a>, attempt: u8) -> Self {
        let n = p.w * p.h;
        let tag = 0x7F11 ^ u64::from(attempt);
        let prio = (0..n)
            .map(|i| {
                let h = hash3(p.seed, tag, p.key[0], p.key[1], mix_key(p.key[2], i));
                u32::try_from(h >> 32).unwrap_or(0)
            })
            .collect();
        let scopes = p
            .limits
            .iter()
            .map(|l| {
                if l.cells.is_empty() {
                    vec![true; n]
                } else {
                    let mut s = vec![false; n];
                    for &c in &l.cells {
                        if let Some(v) = s.get_mut(c) {
                            *v = true;
                        }
                    }
                    s
                }
            })
            .collect();
        Self {
            p,
            base: p.domains.clone(),
            dom: p.domains.clone(),
            queue: WorkQueue::new(n),
            front: Frontier::new(),
            prio,
            decided: vec![false; n],
            pinned: vec![false; n],
            counts: vec![0; p.limits.len()],
            scopes,
            repairs: 0,
        }
    }

    fn enqueue_around(&mut self, i: usize) {
        for d in Dir::ALL {
            if let Some(j) = self.p.neighbour(i, d) {
                self.queue.push(j);
            }
        }
    }

    /// Narrows cell `i` to `d`, keeping limits and the frontier current.
    fn set(&mut self, i: usize, d: TileSet) -> Result<(), Contradiction> {
        if d.is_empty() {
            return Err(Contradiction(Some(i)));
        }
        if d == self.dom[i] {
            return Ok(());
        }
        self.dom[i] = d;
        self.front.push(d.count(), self.prio[i], i);
        self.enqueue_around(i);
        if d.count() == 1 {
            self.decide(i)?;
        }
        Ok(())
    }

    fn decide(&mut self, i: usize) -> Result<(), Contradiction> {
        if self.decided[i] {
            return Ok(());
        }
        self.decided[i] = true;
        let Some(t) = self.dom[i].first() else {
            return Err(Contradiction(Some(i)));
        };
        for k in 0..self.p.limits.len() {
            if self.scopes[k][i] && self.p.limits[k].tiles.contains(t) {
                self.counts[k] += 1;
                self.enforce(k, i)?;
            }
        }
        Ok(())
    }

    /// Applies limit `k` after its count changed at cell `at`.
    fn enforce(&mut self, k: usize, at: usize) -> Result<(), Contradiction> {
        let lim = &self.p.limits[k];
        if self.counts[k] > lim.max {
            return Err(Contradiction(Some(at)));
        }
        if self.counts[k] == lim.max {
            let group = lim.tiles;
            for j in 0..self.dom.len() {
                if self.scopes[k][j] && !self.decided[j] && self.dom[j].meets(&group) {
                    let d = self.dom[j].minus(group);
                    self.set(j, d)?;
                }
            }
        }
        Ok(())
    }

    fn propagate(&mut self) -> Result<(), Contradiction> {
        while let Some(i) = self.queue.pop() {
            let mut d = self.dom[i];
            for dir in Dir::ALL {
                if let Some(j) = self.p.neighbour(i, dir) {
                    let k = self.p.kind(i, dir);
                    d = d.and(self.p.rules.support(self.dom[j], dir.opposite(), k));
                }
            }
            if let Err(e) = self.set(i, d) {
                self.queue.clear();
                return Err(e);
            }
        }
        Ok(())
    }

    /// Propagates, repairing locally on contradiction: the cells around it
    /// (except anchored ones) go back to their initial options and the
    /// limits are recounted. Bounded by [`MAX_REPAIRS`].
    fn settle(&mut self) -> Result<(), Contradiction> {
        loop {
            match self.propagate() {
                Ok(()) => return Ok(()),
                Err(Contradiction(Some(at))) if self.repairs < MAX_REPAIRS => {
                    self.repairs += 1;
                    self.repair(at);
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn repair(&mut self, at: usize) {
        let (w, h) = (self.p.w, self.p.h);
        let (x, y) = (at % w, at / w);
        let r = REPAIR_RADIUS;
        for yy in y.saturating_sub(r + 1)..(y + r + 2).min(h) {
            for xx in x.saturating_sub(r + 1)..(x + r + 2).min(w) {
                let j = yy * w + xx;
                let inner = xx.abs_diff(x) <= r && yy.abs_diff(y) <= r;
                if inner && !self.pinned[j] {
                    self.dom[j] = self.base[j];
                    self.decided[j] = self.base[j].count() == 1;
                    self.front.push(self.dom[j].count(), self.prio[j], j);
                }
                self.queue.push(j);
            }
        }
        for k in 0..self.p.limits.len() {
            let lim = &self.p.limits[k];
            self.counts[k] = (0..self.dom.len())
                .filter(|&j| {
                    self.scopes[k][j]
                        && self.decided[j]
                        && self.dom[j].first().is_some_and(|t| lim.tiles.contains(t))
                })
                .count()
                .try_into()
                .unwrap_or(u32::MAX);
        }
    }

    fn init(&mut self) -> Result<(), Contradiction> {
        for i in 0..self.dom.len() {
            let mut d = self.dom[i];
            for dir in Dir::ALL {
                d = d.and(self.p.rules.edge_set(dir, self.p.kind(i, dir)));
            }
            if d.is_empty() {
                return Err(Contradiction(None));
            }
            self.dom[i] = d;
            self.base[i] = d;
            self.front.push(d.count(), self.prio[i], i);
            self.queue.push(i);
        }
        for i in 0..self.dom.len() {
            if self.dom[i].count() == 1 {
                self.pinned[i] = true;
                self.decide(i)?;
            }
        }
        self.propagate()
    }

    fn weighted(&self, opts: &[(usize, usize)], r: &mut Stream) -> Option<(usize, usize)> {
        let ws: Vec<u64> = opts
            .iter()
            .map(|&(i, t)| u64::from((self.p.weight)(i, t)))
            .collect();
        let total: u64 = ws.iter().sum();
        if total == 0 {
            return opts.first().copied();
        }
        let mut target = r.below(total);
        for (o, w) in opts.iter().zip(ws) {
            if target < w {
                return Some(*o);
            }
            target -= w;
        }
        opts.last().copied()
    }

    fn held(&self, a: &Anchor) -> u32 {
        let n = a
            .options
            .iter()
            .filter(|(i, s)| {
                self.dom
                    .get(*i)
                    .is_some_and(|d| d.count() == 1 && d.meets(s))
            })
            .count();
        u32::try_from(n).unwrap_or(u32::MAX)
    }

    fn options(&self, a: &Anchor) -> Vec<(usize, usize)> {
        a.options
            .iter()
            .filter_map(|(i, s)| {
                let d = self.dom.get(*i)?;
                (d.count() > 1).then(|| (*i, d.and(*s)))
            })
            .flat_map(|(i, s)| s.iter().map(move |t| (i, t)).collect::<Vec<_>>())
            .collect()
    }

    /// Places anchors, the one with the fewest options left first (ties
    /// by order), until every anchor holds.
    fn anchors(&mut self, r: &mut Stream) -> Result<(), Contradiction> {
        let anchors = &self.p.anchors;
        loop {
            let mut best: Option<(usize, Vec<(usize, usize)>)> = None;
            for a in anchors {
                if self.held(a) >= a.count {
                    continue;
                }
                let opts = self.options(a);
                if best.as_ref().is_none_or(|b| opts.len() < b.1.len()) {
                    best = Some((opts.len(), opts));
                }
            }
            let Some((_, opts)) = best else {
                return Ok(());
            };
            let (i, t) = self.weighted(&opts, r).ok_or(Contradiction(None))?;
            self.pinned[i] = true;
            self.set(i, TileSet::single(t))?;
            self.settle()?;
        }
    }

    fn run(&mut self, r: &mut Stream) -> Result<(), Contradiction> {
        self.init()?;
        self.anchors(r)?;
        while let Some((count, _, i)) = self.front.pop() {
            if self.dom[i].count() != count || count <= 1 {
                continue;
            }
            let opts: Vec<(usize, usize)> = self.dom[i].iter().map(|t| (i, t)).collect();
            let (_, t) = self.weighted(&opts, r).ok_or(Contradiction(Some(i)))?;
            self.set(i, TileSet::single(t))?;
            self.settle()?;
        }
        if self.dom.iter().all(|d| d.count() == 1) {
            Ok(())
        } else {
            Err(Contradiction(None))
        }
    }
}

fn mix_key(k: i64, i: usize) -> i64 {
    let i = u64::try_from(i).unwrap_or(u64::MAX);
    mix(k.cast_unsigned() ^ i.wrapping_mul(0x9E37_79B9_7F4A_7C15)).cast_signed()
}

/// Solves a problem in at most `attempts` attempts; `check` must accept
/// the finished grid.
///
/// # Errors
/// [`Failure`] when every attempt met a contradiction or failed the check.
pub fn solve(
    p: &Problem<'_>,
    attempts: u8,
    check: &dyn Fn(&[u16]) -> bool,
) -> Result<Solution, Failure> {
    if p.domains.len() != p.w * p.h || p.edges.len() != p.w * p.h {
        return Err(Failure {
            attempts: 0,
            rejected: 0,
        });
    }
    let mut rejected = 0;
    for attempt in 0..attempts {
        let mut r = Stream::new(
            p.seed,
            0xC0_11A5 ^ u64::from(attempt),
            p.key[0],
            p.key[1] ^ p.key[2].rotate_left(21),
        );
        let mut run = Run::new(p, attempt);
        if run.run(&mut r).is_err() {
            continue;
        }
        let tiles: Vec<u16> = run
            .dom
            .iter()
            .map(|d| d.first().and_then(|t| u16::try_from(t).ok()).unwrap_or(0))
            .collect();
        if check(&tiles) {
            return Ok(Solution {
                tiles,
                attempts: attempt + 1,
            });
        }
        rejected += 1;
    }
    Err(Failure { attempts, rejected })
}
