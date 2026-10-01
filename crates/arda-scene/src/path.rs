//! SRD 5.1 movement cost on the square grid.
//!
//! - Each square entered costs 5 ft, diagonal steps included (the SRD
//!   default; the alternating 5/10 ft diagonal is a DMG variant, not SRD
//!   content, so it is not offered).
//! - Entering difficult terrain, shallow water (wading) or deep water
//!   (swimming) costs 1 extra foot per foot.
//! - A step flagged in the climb mask (an elevation step of 10 ft or more)
//!   is climbed, which also costs 1 extra foot per foot. The extras are
//!   separate rules, so they add: climbing out of a wade costs 15 ft.
//! - Impassable squares and wall edges that block movement stop a step; a
//!   diagonal step may not cut a vertex any movement-blocking wall touches,
//!   nor squeeze between two impassable squares. These two corner rules are
//!   Arda house rules (SRD 5.1 says nothing about grid corners).

use crate::index::SceneIndex;
use crate::squares::DIRS;
use crate::types::{Movement, Sq};
use arda_tactical::layout::EdgeAxis;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

impl SceneIndex<'_> {
    /// Whether a single step from `(x, y)` in direction `dir` (climb-mask
    /// bit order) is allowed.
    #[must_use]
    pub fn can_step(&self, x: i64, y: i64, dir: usize) -> bool {
        let Some(&(dx, dy)) = DIRS.get(dir) else {
            return false;
        };
        let (nx, ny) = (x + dx, y + dy);
        if self.movement(nx, ny) == Movement::Impassable {
            return false;
        }
        let blocks = |axis, ex, ey| self.edge(axis, ex, ey).movement;
        match (dx, dy) {
            (0, _) => !blocks(EdgeAxis::Horizontal, x, y.max(ny)),
            (_, 0) => !blocks(EdgeAxis::Vertical, x.max(nx), y),
            _ => {
                let (vx, vy) = (x.max(nx), y.max(ny));
                let cut = blocks(EdgeAxis::Vertical, vx, vy - 1)
                    || blocks(EdgeAxis::Vertical, vx, vy)
                    || blocks(EdgeAxis::Horizontal, vx - 1, vy)
                    || blocks(EdgeAxis::Horizontal, vx, vy);
                let squeeze = self.movement(nx, y) == Movement::Impassable
                    && self.movement(x, ny) == Movement::Impassable;
                !cut && !squeeze
            }
        }
    }

    /// Feet of movement for the cheapest route from `a` to `b`, or `None`
    /// when `b` cannot be reached.
    #[must_use]
    pub fn path_cost(&self, a: Sq, b: Sq) -> Option<u32> {
        self.shortest_path(a, b).map(|(c, _)| c)
    }

    /// The cheapest route from `a` to `b`: its cost in feet and the squares
    /// from `a` to `b` inclusive. Ties break deterministically.
    #[must_use]
    pub fn shortest_path(&self, a: Sq, b: Sq) -> Option<(u32, Vec<Sq>)> {
        let (w, h) = (self.scene.width, self.scene.height);
        if a.0 >= w || a.1 >= h || b.0 >= w || b.1 >= h {
            return None;
        }
        if a == b {
            return Some((0, vec![a]));
        }
        let n = w as usize * h as usize;
        let state = |x: u32, y: u32| y as usize * w as usize + x as usize;
        let mut best = vec![u32::MAX; n];
        let mut prev = vec![usize::MAX; n];
        let mut heap = BinaryHeap::new();
        let start = state(a.0, a.1);
        best[start] = 0;
        heap.push(Reverse((0u32, start)));
        while let Some(Reverse((cost, s))) = heap.pop() {
            if cost > best[s] {
                continue;
            }
            let x = i64::try_from(s % w as usize).unwrap_or(0);
            let y = i64::try_from(s / w as usize).unwrap_or(0);
            if (x, y) == (i64::from(b.0), i64::from(b.1)) {
                return Some((cost, self.unwind(&prev, s, w)));
            }
            for (dir, (dx, dy)) in DIRS.iter().enumerate() {
                if !self.can_step(x, y, dir) {
                    continue;
                }
                let (nx, ny) = (x + dx, y + dy);
                let mut mult = 1;
                if self.movement(nx, ny).is_costly() {
                    mult += 1;
                }
                if self.climb(x, y) & (1 << dir) != 0 {
                    mult += 1;
                }
                let (Ok(ux), Ok(uy)) = (u32::try_from(nx), u32::try_from(ny)) else {
                    continue;
                };
                let ns = state(ux, uy);
                let nc = cost + 5 * mult;
                if nc < best[ns] {
                    best[ns] = nc;
                    prev[ns] = s;
                    heap.push(Reverse((nc, ns)));
                }
            }
        }
        None
    }

    fn unwind(&self, prev: &[usize], mut s: usize, w: u32) -> Vec<Sq> {
        let mut out = Vec::new();
        loop {
            let (x, y) = (s % w as usize, s / w as usize);
            out.push(Sq(
                u32::try_from(x).unwrap_or(0),
                u32::try_from(y).unwrap_or(0),
            ));
            match prev.get(s) {
                Some(&p) if p != usize::MAX => s = p,
                _ => break,
            }
        }
        out.reverse();
        out
    }
}

impl crate::types::Scene {
    /// Movement cost in feet from `a` to `b`; see [`SceneIndex::path_cost`].
    #[must_use]
    pub fn path_cost(&self, a: Sq, b: Sq) -> Option<u32> {
        self.queries().path_cost(a, b)
    }
}
