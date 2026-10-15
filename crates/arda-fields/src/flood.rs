//! The fields a window can see, found by flooding outward from it rather
//! than locating every square of the plan's margin (review 3.5-14: the
//! margin is 128 times the window; most of it lies in fields the window
//! never touches).
//!
//! A field is a 4-connected component of open (`Wild`) squares that the
//! partition puts in one site. The components that matter to a window are
//! those with a square within [`SEEN`] of it, the reach of its dressing
//! (zone distances, hedgerow-tree cells): every wall, gate and zone the
//! window shows lies between such fields. Each is flooded whole, so a
//! field is the same in every window that sees it, and the list is in the
//! row-major order of first squares, as a full labelling of the plan
//! would give.

use crate::geom::{Grid, Sq};
use crate::partition::Partition;
use crate::plan::Cover;
use rayon::prelude::*;

/// Squares around the window whose fields its output depends on: the
/// dressing pad (18), the zone-distance cap (5) and a hedgerow-tree cell
/// (14), rounded up.
pub const SEEN: i64 = 40;

/// The `(site, squares)` of every field component the window `win` can
/// see, squares in row-major order, components by first square; and how
/// many squares were located.
#[must_use]
pub fn fields_seen(
    cover: &Grid<Cover>,
    partition: &Partition,
    win: (i64, i64, i64, i64),
) -> (Vec<(usize, Vec<Sq>)>, usize) {
    let open = |s: Sq| cover.get(s) == Some(&Cover::Wild);
    let mut site: Grid<Option<Option<usize>>> =
        Grid::new(cover.x0, cover.y0, cover.w, cover.h, None);
    let mut located = 0_usize;
    // The seed rectangle, located at once in parallel.
    let (x0, y0) = ((win.0 - SEEN).max(cover.x0), (win.1 - SEEN).max(cover.y0));
    let x1 = (win.0 + win.2 + SEEN).min(cover.x0 + cover.w);
    let y1 = (win.1 + win.3 + SEEN).min(cover.y0 + cover.h);
    let seeds: Vec<Sq> = (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| Sq::new(x, y)))
        .filter(|&s| open(s))
        .collect();
    let found: Vec<Option<usize>> = seeds
        .par_iter()
        .map(|s| partition.locate(s.centre()))
        .collect();
    located += seeds.len();
    for (s, f) in seeds.iter().zip(found) {
        site.set(*s, Some(f));
    }
    let mut done: Grid<bool> = Grid::new(cover.x0, cover.y0, cover.w, cover.h, false);
    let mut comps: Vec<(usize, Vec<Sq>)> = Vec::new();
    // Floods the component of `start`, wave by wave, locating each wave's
    // new squares in parallel.
    let flood = |start: Sq,
                 site: &mut Grid<Option<Option<usize>>>,
                 done: &mut Grid<bool>,
                 located: &mut usize|
     -> Option<(usize, Vec<Sq>)> {
        if done.get(start) != Some(&false) || !open(start) {
            return None;
        }
        done.set(start, true);
        let v = match site.get(start) {
            Some(Some(v)) => *v,
            _ => {
                *located += 1;
                let v = partition.locate(start.centre());
                site.set(start, Some(v));
                v
            }
        }?;
        let mut members = vec![start];
        let mut wave = vec![start];
        while !wave.is_empty() {
            let mut next: Vec<Sq> = wave
                .iter()
                .flat_map(|c| [(1, 0), (-1, 0), (0, 1), (0, -1)].map(|(dx, dy)| c.offset(dx, dy)))
                .filter(|&n| done.get(n) == Some(&false) && open(n))
                .collect();
            next.sort_unstable_by_key(|q| (q.y, q.x));
            next.dedup();
            let unknown: Vec<Sq> = next
                .iter()
                .copied()
                .filter(|&n| site.get(n) == Some(&None))
                .collect();
            let found: Vec<Option<usize>> = unknown
                .par_iter()
                .map(|n| partition.locate(n.centre()))
                .collect();
            *located += unknown.len();
            for (n, f) in unknown.into_iter().zip(found) {
                site.set(n, Some(f));
            }
            next.retain(|&n| site.get(n) == Some(&Some(Some(v))));
            for &n in &next {
                done.set(n, true);
            }
            members.extend_from_slice(&next);
            wave = next;
        }
        members.sort_unstable_by_key(|q| (q.y, q.x));
        Some((v, members))
    };
    for &s in &seeds {
        if let Some(c) = flood(s, &mut site, &mut done, &mut located) {
            comps.push(c);
        }
    }
    comps.sort_by_key(|(_, m)| m.first().map(|q| (q.y, q.x)));
    (comps, located)
}
