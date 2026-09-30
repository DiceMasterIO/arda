//! Alluvial valley floors (logic/02 §fine-formation floodplain).
//!
//! Each cell is compared with the channel it drains to (height above
//! nearest drainage). Where that height is below a discharge-scaled fill
//! depth, and the setting is lowland, the cell becomes a nearly flat
//! alluvial floor; the hillslope that rises above the fill keeps its shape
//! and reads as a bluff or terrace edge.

use super::drainage::{receiver_index, FIXED};
use super::incision::{isqrt, Scratch};
use super::lattice::Lattice;

/// Channel threshold: 2 km^2 in Q8 finest-cell units (1 cell = 1525.88 m^2).
pub const CHANNEL_AREA_Q8: u64 = 2_000_000 * 65_536 / 390_625;
/// Fill depth coefficient: `depth = 2.5 m * lowland * (A / km^2)^(1/4)`.
pub const DEPTH_MM_PER_ROOT4_KM2: i64 = 2_500;

/// Applies the floodplain pass using the drainage left in `scratch` by the
/// level solve. `lowland` is Q8 (256 = full lowland). `chan` is scratch.
/// Returns the number of cells changed.
pub fn apply(
    g: &mut Lattice,
    flags: &[u8],
    lowland: &[u8],
    scratch: &Scratch,
    chan: &mut [i32],
) -> usize {
    let (w, h) = (g.width, g.height);
    let n = g.z.len();
    for &i in scratch.order.iter().rev() {
        let i = i as usize;
        let r = receiver_index(i, w, h, scratch.rcv[i]);
        chan[i] = if scratch.area[i] >= CHANNEL_AREA_Q8 || flags[i] & FIXED != 0 || r == i {
            i32::try_from(i).unwrap_or(i32::MAX)
        } else {
            chan[r]
        };
    }
    let mut changed = 0;
    // Channel heights are read from the unmodified channel cells: a channel
    // cell is never itself rewritten because `chan[c] == c`.
    for i in 0..n {
        let Ok(c) = usize::try_from(chan[i]) else {
            continue;
        };
        if c == i || flags[i] & FIXED != 0 {
            continue;
        }
        let lw = i64::from(lowland[c]);
        if lw == 0 {
            continue;
        }
        let km2_q8 = scratch.area[c] * 390_625 / 256 / 1_000_000;
        let root4 = isqrt(isqrt(km2_q8.max(1) << 24)); // (A km^2)^(1/4), Q8
        let depth = DEPTH_MM_PER_ROOT4_KM2 * lw * i64::try_from(root4).unwrap_or(0) / 65_536;
        let zc = i64::from(g.z[c]);
        let hand = i64::from(g.z[i]) - zc;
        // Soft edge: full fill below half the depth, none above 1.5 x, a
        // smoothstep between. Monotone in `hand`, so no pits, and no sharp
        // grid-stepped bluff line (logic/02 §fine-formation floodplain).
        let upper = depth * 3 / 2;
        if hand > 0 && hand < upper && depth > 0 {
            let t = ((upper - hand) * 4096 / depth).clamp(0, 4096);
            let w = t * t / 4096 * (3 * 4096 - 2 * t) / 4096;
            let flat = hand * 4 / 100 + 50;
            let nz = zc + hand + (flat - hand) * w / 4096;
            g.z[i] = i32::try_from(nz).unwrap_or(g.z[i]);
            changed += 1;
        }
    }
    changed
}
