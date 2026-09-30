//! Drained steepest-descent routing for the post-formation passes
//! (logic/02 §fine-formation terraces and shore).
//!
//! Before the final drainage guarantee the lattice still holds exact flats
//! (floodplain floors, infill) and a few closed pits, where plain
//! steepest descent stops. Routing therefore runs on a filled copy (1 mm
//! per cell toward the outlet); the lattice itself is not changed.

use super::drainage::{fill, open_sea_flags, receiver_index, receivers, upstream_order, FIXED};
use super::lattice::{alloc, Lattice};
use super::FormationError;

/// Receivers, contributing area (cells) and open-sea flags of a lattice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flow {
    /// D8 receiver codes on the filled copy.
    pub rcv: Vec<u8>,
    /// Contributing area in cells, the cell itself included.
    pub area: Vec<u32>,
    /// `FIXED` on open sea (and the domain rim).
    pub open: Vec<u8>,
    /// Upstream-first order.
    pub order: Vec<u32>,
    /// One cell's area, m².
    pub cell_m2: u64,
}

impl Flow {
    /// Receiver index of `i` (itself at an outlet).
    #[must_use]
    pub fn receiver(&self, i: usize, w: usize, h: usize) -> usize {
        receiver_index(i, w, h, self.rcv[i])
    }

    /// Catchment of `i` in km² (rounded down).
    #[must_use]
    pub fn km2(&self, i: usize) -> u64 {
        u64::from(self.area[i]) * self.cell_m2 / 1_000_000
    }

    /// Whether `i` is open sea.
    #[must_use]
    pub fn is_open_sea(&self, i: usize, z: &[i32]) -> bool {
        self.open[i] & FIXED != 0 && z[i] <= 0
    }
}

/// Routes `g` over a drained copy of itself.
///
/// # Errors
/// Allocation failure.
pub fn route(g: &Lattice) -> Result<Flow, FormationError> {
    let (w, h) = (g.width, g.height);
    let n = w * h;
    let cell_m2 = u64::try_from((g.spacing_um / 1_000).pow(2) / 1_000_000)
        .map_err(|_| FormationError::ArithmeticOverflow)?
        .max(1);
    let mut open: Vec<u8> = alloc(n)?;
    open_sea_flags(&g.z, w, h, &mut open);
    let mut rcv: Vec<u8> = alloc(n)?;
    {
        let mut z = g.z.clone();
        let mut next: Vec<u32> = alloc(n)?;
        let mut closed: Vec<u8> = alloc(n)?;
        fill(&mut z, w, h, &open, 1, &mut next, &mut closed)?;
        drop(next);
        drop(closed);
        receivers(&z, w, h, &open, 0, 0, &mut rcv);
    }
    let mut order: Vec<u32> = alloc(n)?;
    {
        let mut indeg: Vec<u8> = alloc(n)?;
        upstream_order(&rcv, w, h, &mut indeg, &mut order);
    }
    let mut area: Vec<u32> = alloc(n)?;
    area.fill(1);
    for &i in &order {
        let i = i as usize;
        let r = receiver_index(i, w, h, rcv[i]);
        if r != i {
            area[r] = area[r].saturating_add(area[i]);
        }
    }
    Ok(Flow {
        rcv,
        area,
        open,
        order,
        cell_m2,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flats_route_through_to_the_sea() {
        // A flat 1 m plain walled by hills on three sides, open to the sea
        // in the east.
        let (w, h) = (40, 41);
        let mut g = Lattice::new(w, h, 39_062_500).unwrap();
        for (i, z) in g.z.iter_mut().enumerate() {
            let hill = !(10..=30).contains(&(i / w));
            *z = match i % w {
                35.. => -1_000,
                0..=4 => 10_000,
                _ if hill => 10_000,
                _ => 1_000,
            };
        }
        let f = route(&g).unwrap();
        let mut c = 20 * w + 8;
        for _ in 0..100 {
            if f.is_open_sea(c, &g.z) {
                break;
            }
            c = f.receiver(c, w, h);
        }
        assert!(f.is_open_sea(c, &g.z), "the plain reaches the sea");
        assert!(c % w >= 35, "through the coast, not the rim: {}", c % w);
        let coast: u32 = (10..=30).map(|y| f.area[y * w + 34]).sum();
        assert!(coast >= 21 * 30, "the whole plain drains out: {coast}");
    }
}
