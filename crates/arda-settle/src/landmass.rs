//! Landmasses and solid realms (`logic/08` §realm-seats step 3,
//! Invariant 11).
//!
//! A landmass is a 4-connected piece of dry land (the partition flood can
//! cross water only orthogonally, so its land connectivity is 4-connected
//! too). The partition flood crosses the sea at ferry cost, so without a
//! rule a seat would claim the coast of another landmass across a strait:
//! on the 500 × 1000 km recipe-9 world (seed 42) realm 4 held a second
//! landmass of 678,686 ha, 41 % of its land. Two rules keep realms solid:
//!
//! 1. A landmass that holds a seat belongs to the realms seated on it: no
//!    seat floods onto it from the sea ([`seated`]).
//! 2. A landmass without a seat that is not an islet gets a seat of its
//!    own: its largest town ([`island_seats`]). An islet is a landmass of
//!    less than a tenth of a fair realm's land ([`islet_cells`]); it joins
//!    the realm that reaches it cheapest over the sea, as small islands
//!    off a coast do. A landmass with no town has no one to govern it and
//!    joins a neighbour too.

use crate::error::SettleError;
use crate::grid::{filled, Grid};
use crate::model::Settlement;
use std::collections::BTreeMap;

/// An islet is a landmass below this share of a fair realm's land (per
/// mille): a tenth (assumed, tunable).
pub const ISLET_SHARE_PM: u64 = 100;

/// The islet threshold in cells: a landmass (or a realm's detached piece)
/// smaller than this is an islet, for `land` land cells shared by `realms`
/// realms. At least one cell.
#[must_use]
pub fn islet_cells(land: u64, realms: usize) -> u64 {
    let n = u64::try_from(realms.max(1)).unwrap_or(1);
    (land * ISLET_SHARE_PM / (1000 * n)).max(1)
}

/// Visits the 4-connected component of `start` among cells for which
/// `inside` holds, marking `seen`; calls `each` on every cell.
fn component(
    width: usize,
    height: usize,
    start: usize,
    inside: &impl Fn(usize) -> bool,
    seen: &mut [bool],
    each: &mut impl FnMut(usize),
) {
    let mut stack = vec![start];
    seen[start] = true;
    while let Some(i) = stack.pop() {
        each(i);
        let (x, y) = (i % width, i / width);
        let mut push = |j: usize| {
            if !seen[j] && inside(j) {
                seen[j] = true;
                stack.push(j);
            }
        };
        if x > 0 {
            push(i - 1);
        }
        if x + 1 < width {
            push(i + 1);
        }
        if y > 0 {
            push(i - width);
        }
        if y + 1 < height {
            push(i + width);
        }
    }
}

/// Per cell, the landmass of the first seat (slot + 1, in `seat_cells`
/// order) that stands on it; 0 on water and on landmasses without a seat.
/// Two seats on one landmass share the label of the earlier.
///
/// # Errors
/// [`SettleError::Reserve`] when a raster cannot be allocated.
pub fn seated(g: &Grid, seat_cells: &[usize]) -> Result<Vec<u16>, SettleError> {
    let mut label = filled(g.len(), 0_u16, "seated landmass")?;
    let mut seen = filled(g.len(), false, "landmass visit")?;
    for (slot, &c) in seat_cells.iter().enumerate() {
        if seen[c] || !g.is_land(c) {
            continue;
        }
        let id = u16::try_from(slot + 1).unwrap_or(u16::MAX);
        component(
            g.width,
            g.height,
            c,
            &|j| g.is_land(j),
            &mut seen,
            &mut |j| label[j] = id,
        );
    }
    Ok(label)
}

/// Towns (indices into `settlements`) to seat on the landmasses that hold
/// no seat of `seats` and are not islets for `realms` realms: on each, the
/// town with the largest population (ties to the lower id). In scan order
/// of the landmasses, so deterministic.
///
/// # Errors
/// [`SettleError::Reserve`] when a raster cannot be allocated.
pub fn island_seats(
    g: &Grid,
    settlements: &[Settlement],
    seats: &[usize],
    realms: usize,
) -> Result<Vec<usize>, SettleError> {
    let land = u64::try_from(g.land_cells()).unwrap_or(0);
    let islet = islet_cells(land, realms);
    let seat_cells: Vec<usize> = seats
        .iter()
        .map(|&k| settlements[k].index(g.width))
        .collect();
    // Seated landmasses are visited first, so the scan below meets only
    // the others.
    let mut seen = filled(g.len(), false, "landmass visit")?;
    for &c in &seat_cells {
        if !seen[c] && g.is_land(c) {
            component(
                g.width,
                g.height,
                c,
                &|j| g.is_land(j),
                &mut seen,
                &mut |_| {},
            );
        }
    }
    let mut towns: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (k, s) in settlements.iter().enumerate() {
        if s.tier.is_urban() {
            towns.entry(s.index(g.width)).or_default().push(k);
        }
    }
    let mut out = Vec::new();
    for i in 0..g.len() {
        if seen[i] || !g.is_land(i) {
            continue;
        }
        let mut size = 0_u64;
        let mut best: Option<usize> = None;
        component(
            g.width,
            g.height,
            i,
            &|j| g.is_land(j),
            &mut seen,
            &mut |j| {
                size += 1;
                for &k in towns.get(&j).into_iter().flatten() {
                    let better = best.is_none_or(|b| {
                        let (s, t) = (&settlements[k], &settlements[b]);
                        (s.population, std::cmp::Reverse(s.id))
                            > (t.population, std::cmp::Reverse(t.id))
                    });
                    if better {
                        best = Some(k);
                    }
                }
            },
        );
        if size >= islet {
            out.extend(best);
        }
    }
    Ok(out)
}

/// The 4-connected pieces of every realm of a `width × height` realm map
/// (0 = water), largest first: `out[r]` lists the cell counts of realm
/// `r + 1`'s pieces, for realms `1..=n`.
#[must_use]
pub fn pieces(width: usize, height: usize, map: &[u16], n: usize) -> Vec<Vec<u64>> {
    let mut out = vec![Vec::new(); n];
    let mut seen = vec![false; map.len()];
    for i in 0..map.len() {
        let r = map[i];
        if r == 0 || seen[i] {
            continue;
        }
        let mut size = 0_u64;
        component(width, height, i, &|j| map[j] == r, &mut seen, &mut |_| {
            size += 1;
        });
        if let Some(v) = out.get_mut(usize::from(r) - 1) {
            v.push(size);
        }
    }
    for v in &mut out {
        v.sort_unstable_by(|a, b| b.cmp(a));
    }
    out
}

/// Realms (1-based) that are not solid: their largest piece holds less
/// than nine tenths of their land and some other piece is not an islet
/// (logic/08 Invariant 11). Empty when every realm is solid.
#[must_use]
pub fn broken(width: usize, height: usize, map: &[u16], n: usize) -> Vec<usize> {
    let all = pieces(width, height, map, n);
    let land: u64 = all.iter().flatten().sum();
    let islet = islet_cells(land, n);
    all.iter()
        .enumerate()
        .filter(|(_, p)| {
            let total: u64 = p.iter().sum();
            let first = p.first().copied().unwrap_or(0);
            first * 10 < total * 9 && p.iter().skip(1).any(|&q| q >= islet)
        })
        .map(|(r, _)| r + 1)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pieces_are_four_connected_and_sorted() {
        // Realm 1: a 2-cell piece and a diagonal single; realm 2: one cell.
        #[rustfmt::skip]
        let map = [
            1, 1, 0,
            0, 0, 1,
            2, 0, 0,
        ];
        let p = pieces(3, 3, &map, 2);
        assert_eq!(p, vec![vec![2, 1], vec![1]]);
    }

    #[test]
    fn islets_are_a_tenth_of_a_fair_realm() {
        assert_eq!(islet_cells(10_000, 2), 500);
        assert_eq!(islet_cells(5, 10), 1);
    }
}
