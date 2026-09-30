//! Belt relief: the tectonic relief a formation level uses for its masks
//! (logic/02 §fine-formation masks).
//!
//! Local standard deviation of height measures the regional *gradient*, so
//! it dips to a minimum along every broad crest and valley axis of a
//! mountain belt. Used directly, those axes were classed as low hills: a
//! several-times-higher channel-initiation area, less relative uplift and a
//! gentler talus cap left each main divide as a smooth, uneroded band
//! following the (often near-straight) macro crest. Relief belongs to the
//! belt, not to the slope position, so every node takes the highest local
//! relief within [`BELT_RADIUS_CELLS`] (a disc, so no lattice direction is
//! preferred), softened by a short blur.

use rayon::prelude::*;

use super::lattice::{alloc, blur_into, Lattice};
use super::FormationError;

/// Disc radius of the belt maximum, in macro (1 km) cells. It matches the
/// half-width of the standard-deviation window, so a crest takes the
/// relief of the flanks that measured it.
pub const BELT_RADIUS_CELLS: i64 = 12;
/// Box radius of the softening blur (two passes), in macro cells.
const BELT_BLUR_CELLS: usize = 4;

/// Disc maximum of `relief` (metres) over [`BELT_RADIUS_CELLS`], softened.
///
/// # Errors
/// Allocation failure.
pub fn belt_relief(relief: &Lattice) -> Result<Lattice, FormationError> {
    let (w, h) = (relief.width, relief.height);
    let r = BELT_RADIUS_CELLS;
    let offsets: Vec<(i64, i64)> = (-r..=r)
        .flat_map(|dy| (-r..=r).map(move |dx| (dx, dy)))
        .filter(|&(dx, dy)| dx * dx + dy * dy <= r * r)
        .collect();
    let mut peak: Vec<i32> = alloc(w * h)?;
    let src = &relief.z;
    peak.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for (x, out) in row.iter_mut().enumerate() {
            let mut m = src[y * w + x];
            for &(dx, dy) in &offsets {
                let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 {
                    continue;
                }
                m = m.max(src[ny as usize * w + nx as usize]);
            }
            *out = m;
        }
    });
    let mut out = Lattice::new(w, h, relief.spacing_um)?;
    let mut tmp: Vec<i32> = alloc(w * h)?;
    blur_into(&peak, w, h, BELT_BLUR_CELLS, &mut tmp, &mut out.z);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_crest_takes_the_relief_of_its_flanks() {
        // Relief measured on a belt: high on both flanks, dipping to a
        // quarter along the straight crest between them.
        let (w, h) = (101, 41);
        let mut rel = Lattice::new(w, h, 1_000_000_000).unwrap();
        for y in 0..h {
            for x in 0..w {
                let d = (x as i64 - 50).abs();
                rel.z[y * w + x] =
                    i32::try_from(if d > 20 { 0 } else { 100 + 30 * d.min(10) }).unwrap();
            }
        }
        let belt = belt_relief(&rel).unwrap();
        let at = |x: usize| belt.z[20 * w + x];
        assert!(at(50) >= 380, "crest relief {}", at(50));
        // Far from the belt it stays low; it never exceeds the belt peak.
        assert!(belt.z.iter().all(|&v| v <= 400));
        assert!(at(0) < 100, "outside the belt {}", at(0));
    }
}
