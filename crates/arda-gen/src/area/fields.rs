//! Per-cell quantities derived from relief and drainage (artifact, Water).
//!
//! "Two derived quantities matter more than the rivers themselves for what
//! follows": height above the nearest downstream watercourse, which decides
//! floodplain, and a wetness index combining how much land drains through a
//! cell with how flat it is.
//!
//! Nothing here calls `ln`, `tan`, or `atan2`. Architecture §Q4 bars libm
//! variance from sim paths, and a float here would agree with itself on one
//! platform while breaking the cross-platform golden gate.

use super::water::WaterGrid;
use arda_core::{CellCoord, AREA_CELLS};

const N: i32 = AREA_CELLS as i32;

/// Floodplain band, from height above the nearest watercourse.
///
/// The artifact's bands: "under about a metre it is marsh, under two and a
/// half it floods, and the terrace two to fifteen metres up is dry, close to
/// water and flat".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Floodplain {
    /// Under 1 m above the watercourse.
    Marsh,
    /// 1 m to 2.5 m — floods.
    Floods,
    /// 2 m to 15 m — the dry terrace settlements stand on.
    Terrace,
    /// Above the terrace, or with no watercourse below it.
    Dry,
}

/// Millimetres above the watercourse at each band edge.
const MARSH_MM: u32 = 1_000;
const FLOODS_MM: u32 = 2_500;
const TERRACE_MM: u32 = 15_000;

/// Classifies a cell from its height above the nearest watercourse.
#[must_use]
pub const fn floodplain(hand_mm: u32) -> Floodplain {
    if hand_mm < MARSH_MM {
        Floodplain::Marsh
    } else if hand_mm < FLOODS_MM {
        Floodplain::Floods
    } else if hand_mm <= TERRACE_MM {
        Floodplain::Terrace
    } else {
        Floodplain::Dry
    }
}

fn coord(x: i32, y: i32) -> Option<CellCoord> {
    CellCoord::new(u16::try_from(x).ok()?, u16::try_from(y).ok()?)
}

/// Height above the nearest downstream watercourse, in millimetres.
///
/// Found by following the drainage tree, per the artifact. A channel cell
/// is itself at the watercourse and stores 0. A cell whose path leaves the
/// tile, or cycles, before meeting a channel stores [`u32::MAX`] — feature
/// 01 spec R9: "A cell whose path leaves the tile before meeting a channel
/// stores `u16::MAX`" (the `u32` here is millimetres, downshifted to
/// decimetres and clamped to `u16` at `compose`). Before discharge-driven
/// initiation (feature 03) this branch was unreachable in practice —
/// 300-cell channels made every tile dense with channels — so it returned
/// 0 like a channel cell; a genuinely arid tile can now have no channel at
/// all, and every one of its cells hit this path, so the distinction from
/// an actual channel cell (also 0) now matters: conflating the two made
/// `floodplain` classify boundless dry land as `Marsh`.
///
/// Clamped at zero when found: routing happens on the filled surface while
/// heights are raw, so a cell inside a basin can sit *below* the channel it
/// drains to.
#[must_use]
pub fn hand(heights: &[i32], water: &WaterGrid) -> Vec<u32> {
    let count = (N * N) as usize;
    let mut out = vec![0u32; count];

    for y in 0..N {
        for x in 0..N {
            let Some(at) = coord(x, y) else { continue };
            if water.is_channel(at) {
                continue;
            }
            let mut cursor = at;
            let mut hops = 0u32;
            let found = loop {
                match water.downstream_of(cursor) {
                    Some(next) => {
                        cursor = next;
                        hops += 1;
                        if water.is_channel(cursor) {
                            break true;
                        }
                        if hops > (N * N) as u32 {
                            break false; // cycle guard; the tree forbids it
                        }
                    }
                    None => break false,
                }
            };
            out[at.index()] = if found {
                let diff = i64::from(heights[at.index()]) - i64::from(heights[cursor.index()]);
                u32::try_from(diff.max(0)).unwrap_or(u32::MAX)
            } else {
                u32::MAX
            };
        }
    }
    out
}

/// Topographic wetness index, 0-255.
///
/// The artifact's description is "how much land drains through a cell with
/// how flat it is" — the standard `ln(A / tan β)`, evaluated here as an
/// integer log2 of drainage area over a fixed-point slope, so no libm call
/// is involved.
#[must_use]
pub fn wetness(drainage: u32, slope_milli_deg: u16) -> u8 {
    // log2(A) in 0..=18 for a 512x512 tile, scaled to give headroom.
    let log_a = i64::from(drainage.max(1).ilog2()) * 16;
    // Flatter ground is wetter. Guard the flat case rather than dividing by
    // zero; one milli-degree is the smallest slope we represent.
    let slope = i64::from(slope_milli_deg.max(1));
    let twi = log_a * 1000 / slope.max(1);
    u8::try_from(twi.clamp(0, 255)).unwrap_or(255)
}

/// Slope in thousandths of a degree and downslope bearing in degrees.
///
/// Uses the steepest of the eight neighbours. `slope_milli_deg` saturates at
/// 65.535 degrees, so cliffs past that clamp — steeper ground than the
/// artifact's 35-degree repose angle should not survive erosion anyway.
#[must_use]
#[allow(clippy::cast_sign_loss)] // callers bounds-check 0 <= x,y < N
pub fn slope_and_aspect(heights: &[i32], x: i32, y: i32) -> (u16, u16) {
    let here = heights[(y * N + x) as usize];
    let mut best: Option<(i64, i32, i32)> = None;

    for (dx, dy) in super::fill::NEIGHBOURS {
        let (nx, ny) = (x + dx, y + dy);
        if nx < 0 || ny < 0 || nx >= N || ny >= N {
            continue;
        }
        let drop = i64::from(here - heights[usize::try_from(ny * N + nx).unwrap_or(0)]);
        if drop <= 0 {
            continue;
        }
        let run = if dx != 0 && dy != 0 { 141_421 } else { 100_000 };
        // Gradient scaled by 10_000 so the comparison stays integral.
        let grad = drop * 10_000 / run;
        if best.is_none_or(|(bg, _, _)| grad > bg) {
            best = Some((grad, dx, dy));
        }
    }

    let Some((grad, dx, dy)) = best else {
        return (0, 0);
    };
    (atan_milli_deg(grad), bearing_deg(dx, dy))
}

/// `atan` of a gradient scaled by 10_000, returned in thousandths of a
/// degree. A 16-entry table with linear interpolation between entries; no
/// libm, and identical on every platform.
fn atan_milli_deg(grad_10k: i64) -> u16 {
    // atan(i/8) in milli-degrees, for i = 0..=16 (gradient 0 to 2.0).
    const TABLE: [i64; 17] = [
        0, 7_125, 14_036, 20_556, 26_565, 32_005, 36_870, 41_186, 45_000, 48_366, 51_340, 53_973,
        56_310, 58_392, 60_255, 61_928, 63_435,
    ];
    let step: i64 = 10_000 / 8; // gradient units per table entry
    let slot = (grad_10k / step).clamp(0, 16);
    let i = usize::try_from(slot).unwrap_or(0);
    if i >= 16 {
        return u16::try_from(TABLE[16].min(65_535)).unwrap_or(u16::MAX);
    }
    let frac = grad_10k - slot * step;
    let v = TABLE[i] + (TABLE[i + 1] - TABLE[i]) * frac / step;
    u16::try_from(v.clamp(0, 65_535)).unwrap_or(u16::MAX)
}

/// Compass bearing of a neighbour offset, degrees clockwise from north.
const fn bearing_deg(dx: i32, dy: i32) -> u16 {
    match (dx, dy) {
        (0, -1) => 0,
        (1, -1) => 45,
        (1, 0) => 90,
        (1, 1) => 135,
        (0, 1) => 180,
        (-1, 1) => 225,
        (-1, 0) => 270,
        (-1, -1) => 315,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floodplain_bands_match_the_artifact() {
        // "under about a metre it is marsh, under two and a half it floods,
        // and the terrace two to fifteen metres up is dry"
        assert_eq!(floodplain(0), Floodplain::Marsh);
        assert_eq!(floodplain(999), Floodplain::Marsh);
        assert_eq!(floodplain(1_000), Floodplain::Floods);
        assert_eq!(floodplain(2_499), Floodplain::Floods);
        assert_eq!(floodplain(2_500), Floodplain::Terrace);
        assert_eq!(floodplain(15_000), Floodplain::Terrace);
        assert_eq!(floodplain(15_001), Floodplain::Dry);
        // Fix 1 (round-1 review): `hand`'s no-channel-below sentinel must
        // fall through every band into `Dry`, not wrap or land in `Marsh`.
        assert_eq!(floodplain(u32::MAX), Floodplain::Dry);
    }

    #[test]
    fn atan_table_matches_the_float_result() {
        // Within a tenth of a degree across the whole range.
        for i in 0..=20 {
            let grad = i * 1000;
            let want = (f64::from(grad) / 10_000.0).atan().to_degrees() * 1000.0;
            let got = f64::from(atan_milli_deg(i64::from(grad)));
            assert!(
                (got - want).abs() < 120.0,
                "grad {grad}: got {got}, want {want}"
            );
        }
    }

    #[test]
    fn a_flat_cell_has_no_slope_and_no_aspect() {
        let heights = vec![1_000; (N * N) as usize];
        assert_eq!(slope_and_aspect(&heights, 100, 100), (0, 0));
    }

    #[test]
    fn aspect_points_downhill() {
        let mut heights = vec![1_000; (N * N) as usize];
        // Drop the eastern neighbour: downslope bearing should be 90.
        heights[(100 * N + 101) as usize] = 0;
        let (slope, aspect) = slope_and_aspect(&heights, 100, 100);
        assert_eq!(aspect, 90);
        assert!(slope > 0);
    }

    #[test]
    fn wetness_rises_with_drainage_and_falls_with_slope() {
        assert!(wetness(10_000, 500) > wetness(10, 500));
        assert!(wetness(10_000, 500) > wetness(10_000, 5_000));
    }

    #[test]
    fn wetness_survives_a_flat_cell() {
        // Slope zero must not divide by zero.
        let _ = wetness(1_000, 0);
    }

    #[test]
    fn hand_stores_the_sentinel_when_no_cell_ever_reaches_a_channel() {
        // Fix 1 (round-1 review of feature 03): before discharge-driven
        // initiation, "path never meets a channel" was unreachable in
        // practice (300-cell channels made every tile dense with them), so
        // `hand` returning 0 for it was indistinguishable from a real
        // channel cell. Discharge-driven initiation (§Q4) makes a
        // channel-less tile possible by design (an arid tile with
        // `discharge < 40` L/s everywhere) — synthesized here by reusing a
        // real seed's relief and drainage tree (not hand-rolled) but
        // zeroing its rainfall patch and dropping any entering river, so
        // no cell anywhere in the tile ever reaches the 40 L/s threshold.
        use crate::area::{area_rainfall, fill, water};
        use crate::continent::build_continent;
        use crate::continent::bundles::{bundle_for, PATCH_KM};
        use arda_core::{AreaCoord, GenerateConfig};

        let c = build_continent(42, GenerateConfig::MICRO, 0);
        let mut b = bundle_for(42, &c, AreaCoord::new(0, 1));
        b.entering.clear();
        b.rainfall_km = vec![0u16; PATCH_KM * PATCH_KM];

        let r = crate::area::relief::relief(42, &c.grid, &b);
        let heights: Vec<i32> = (0..(N * N) as usize)
            .filter_map(|i| {
                let i = i32::try_from(i).ok()?;
                Some(r.get(coord(i % N, i / N)?))
            })
            .collect();
        assert!(heights.iter().any(|&h| h > 0), "fixture must have land");

        let filled = fill::fill(&heights, &b);
        let rain = area_rainfall(&b);
        assert!(
            rain.iter().all(|&mm| mm == 0),
            "a zeroed rainfall patch must resample to zero everywhere"
        );
        let w = water::water(&filled, &b, &rain);
        assert!(
            (0..N)
                .flat_map(|y| (0..N).map(move |x| (x, y)))
                .filter_map(|(x, y)| coord(x, y))
                .all(|at| !w.is_channel(at)),
            "zero rainfall and no entering river must leave the whole tile without a channel"
        );

        let out = hand(&heights, &w);
        assert!(
            out.iter().all(|&mm| mm == u32::MAX),
            "every cell's path leaves the tile without meeting a channel, so every cell — \
             channel or not — must store the sentinel, never the old 0"
        );

        // And the consequence this fix restores: such a cell must not be
        // misclassified as Marsh.
        assert_eq!(floodplain(out[0]), Floodplain::Dry);
    }
}
