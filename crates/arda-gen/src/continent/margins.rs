//! Tectonic margin context for recipe-5 coasts and basins (logic/02
//! §fine-formation margins, goals 6, 17 and 18).
//!
//! Replays the recipe-5 plate drift ([`super::tectonics::run_tectonics_formed`])
//! without its uplift and records, per 4 km simulation cell, how close each
//! kind of plate boundary has been over the run:
//! - **collision**: continent against continent, closing;
//! - **arc**: ocean against continent, closing (Andean arc on the
//!   continental side, trench on the oceanic side);
//! - **island arc**: ocean against ocean, closing, on the overriding plate;
//! - **rift**: any pair, opening.
//!
//! Formation reads it to decide active versus passive margins (shelf width),
//! where volcanic island arcs may rise, and which closed basins have a
//! tectonic cause. The macro heights are untouched, so every existing
//! recipe stays byte-identical.

use super::plates::{plate_of_warped_round, seed_plates_round, CrustType, Plate, SimExtent};
use super::tectonics::round_drift_q4;
use super::{SIM_CELL_KM, SKELETON_STEPS};

/// Belt half-width, in 4 km cells, over which a boundary counts as near.
const NEAR_CELLS: i32 = 6;

/// Proximity of each boundary kind, 0..=255 (255 = on the boundary now).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MarginSample {
    /// Continent-continent convergence.
    pub collision: u8,
    /// Ocean-continent convergence (Andean arc and trench).
    pub arc: u8,
    /// Ocean-ocean convergence on the overriding plate.
    pub island_arc: u8,
    /// Divergence.
    pub rift: u8,
}

/// Per-cell margin history on the 4 km simulation grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarginContext {
    sim: SimExtent,
    offset_km: (i32, i32),
    cells: Vec<MarginSample>,
    /// Final-step volcanic-arc axis cells (4 km grid), overriding side:
    /// ocean-ocean arcs and the continental edge of ocean-continent arcs.
    volcanic_axis: Vec<(i32, i32)>,
}

/// Bit flags of one step's boundary classification.
const COLLISION: u8 = 1;
const ARC: u8 = 2;
const ISLAND_ARC: u8 = 4;
const RIFT: u8 = 8;

/// Which plate of an ocean-ocean pair overrides: the one whose hash is
/// larger (plate age is not modelled; the choice is fixed per pair).
fn overrides(seed: u64, a: &Plate, b: &Plate) -> bool {
    let ha = crate::noise::hash_2d(seed ^ 0x0A2C, i32::from(a.id), i32::from(b.id));
    let hb = crate::noise::hash_2d(seed ^ 0x0A2C, i32::from(b.id), i32::from(a.id));
    ha > hb
}

/// Boundary flags of a cell owned by `a` against its distinct neighbours.
fn classify<'a>(seed: u64, a: &Plate, neighbours: impl IntoIterator<Item = &'a Plate>) -> u8 {
    let mut flags = 0;
    for b in neighbours {
        if a.id == b.id {
            continue;
        }
        let nx = i64::from(b.centre_x) - i64::from(a.centre_x);
        let ny = i64::from(b.centre_y) - i64::from(a.centre_y);
        let vx = i64::from(b.drift_x) - i64::from(a.drift_x);
        let vy = i64::from(b.drift_y) - i64::from(a.drift_y);
        let closing = vx * nx + vy * ny;
        flags |= match (a.crust, b.crust) {
            _ if closing > 0 => RIFT,
            (CrustType::Continental, CrustType::Continental) if closing < 0 => COLLISION,
            (CrustType::Oceanic, CrustType::Oceanic) if closing < 0 && overrides(seed, a, b) => {
                ISLAND_ARC
            }
            (CrustType::Continental, CrustType::Oceanic)
            | (CrustType::Oceanic, CrustType::Continental)
                if closing < 0 =>
            {
                ARC
            }
            _ => 0,
        };
    }
    flags
}

/// Chebyshev-style chamfer distance (in 4 km cells, Q2) to cells with `bit`.
fn distance_q2(flags: &[u8], bit: u8, w: usize, h: usize) -> Vec<i32> {
    let far = i32::MAX / 4;
    let mut d: Vec<i32> = flags
        .iter()
        .map(|&f| if f & bit != 0 { 0 } else { far })
        .collect();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let mut v = d[i];
            if x > 0 {
                v = v.min(d[i - 1] + 4);
            }
            if y > 0 {
                v = v.min(d[i - w] + 4);
                if x > 0 {
                    v = v.min(d[i - w - 1] + 6);
                }
                if x + 1 < w {
                    v = v.min(d[i - w + 1] + 6);
                }
            }
            d[i] = v;
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            let mut v = d[i];
            if x + 1 < w {
                v = v.min(d[i + 1] + 4);
            }
            if y + 1 < h {
                v = v.min(d[i + w] + 4);
                if x + 1 < w {
                    v = v.min(d[i + w + 1] + 6);
                }
                if x > 0 {
                    v = v.min(d[i + w - 1] + 6);
                }
            }
            d[i] = v;
        }
    }
    d
}

/// Replays the recipe-5 plate drift for a `vis_w x vis_h` km continent.
#[must_use]
pub fn margin_context_formed(seed: u64, vis_w: i32, vis_h: i32, attempt: u8) -> MarginContext {
    let sim = SimExtent {
        width: (vis_w * 2 / SIM_CELL_KM).max(8),
        height: (vis_h * 2 / SIM_CELL_KM).max(8),
    };
    let plates = seed_plates_round(seed, sim, attempt);
    let (w, h) = (
        usize::try_from(sim.width).unwrap_or(8),
        usize::try_from(sim.height).unwrap_or(8),
    );
    let n = w * h;
    // Accumulated proximity, weighted towards recent steps (step + 1).
    let mut acc = vec![[0_i64; 4]; n];
    let mut total_weight = 0_i64;
    let mut last_flags = vec![0_u8; n];
    let mut last_continental = vec![false; n];
    for step in 0..SKELETON_STEPS {
        let moved: Vec<Plate> = plates
            .iter()
            .map(|p| {
                let (vx, vy) = round_drift_q4(seed, p);
                Plate {
                    centre_x: p.centre_x + (vx * i32::from(step) + 8).div_euclid(16),
                    centre_y: p.centre_y + (vy * i32::from(step) + 8).div_euclid(16),
                    drift_x: vx,
                    drift_y: vy,
                    ..*p
                }
            })
            .collect();
        let owner: Vec<u8> = (0..sim.height)
            .flat_map(|y| (0..sim.width).map(move |x| (x, y)))
            .map(|(x, y)| plate_of_warped_round(seed, &moved, x, y))
            .collect();
        let find = |id: u8| moved.iter().find(|p| p.id == id);
        let mut flags = vec![0_u8; n];
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let i = y * w + x;
                let Some(a) = find(owner[i]) else {
                    continue;
                };
                flags[i] = classify(
                    seed,
                    a,
                    [i - 1, i + 1, i - w, i + w]
                        .into_iter()
                        .filter(|&j| owner[j] != owner[i])
                        .filter_map(|j| find(owner[j])),
                );
            }
        }
        let weight = i64::from(step) + 1;
        total_weight += weight;
        for (k, bit) in [COLLISION, ARC, ISLAND_ARC, RIFT].into_iter().enumerate() {
            let d = distance_q2(&flags, bit, w, h);
            for (a, &di) in acc.iter_mut().zip(&d) {
                let near = i64::from(NEAR_CELLS) * 4;
                if i64::from(di) < near {
                    // Cubic belt, 1 on the boundary to 0 at NEAR_CELLS.
                    let t = (near - i64::from(di)) * 1024 / near;
                    a[k] += weight * t * t / 1024 * t / 1024;
                }
            }
        }
        last_continental = owner
            .iter()
            .map(|&id| find(id).is_some_and(|p| p.crust == CrustType::Continental))
            .collect();
        last_flags = flags;
    }
    let cells = acc
        .iter()
        .map(|a| {
            let q = |v: i64| u8::try_from((v * 255 / (total_weight * 1024).max(1)).clamp(0, 255));
            MarginSample {
                collision: q(a[0]).unwrap_or(0),
                arc: q(a[1]).unwrap_or(0),
                island_arc: q(a[2]).unwrap_or(0),
                rift: q(a[3]).unwrap_or(0),
            }
        })
        .collect();
    let volcanic_axis = (0..n)
        .filter(|&i| last_flags[i] & ISLAND_ARC != 0 || last_flags[i] & ARC != 0)
        // Ocean-continent arcs erupt on the continental (overriding) side.
        .filter(|&i| last_flags[i] & ISLAND_ARC != 0 || last_continental[i])
        .map(|i| {
            (
                i32::try_from(i % w).unwrap_or(0),
                i32::try_from(i / w).unwrap_or(0),
            )
        })
        .collect();
    MarginContext {
        sim,
        offset_km: (
            (sim.width * SIM_CELL_KM - vis_w) / 2,
            (sim.height * SIM_CELL_KM - vis_h) / 2,
        ),
        cells,
        volcanic_axis,
    }
}

impl MarginContext {
    /// Bilinear margin proximity at a visible-continent position in metres.
    #[must_use]
    pub fn sample_m(&self, x_m: i64, y_m: i64) -> MarginSample {
        let cell_m = i64::from(SIM_CELL_KM) * 1_000;
        let sx = x_m + i64::from(self.offset_km.0) * 1_000 - cell_m / 2;
        let sy = y_m + i64::from(self.offset_km.1) * 1_000 - cell_m / 2;
        let (w, h) = (i64::from(self.sim.width), i64::from(self.sim.height));
        let x0 = sx.div_euclid(cell_m).clamp(0, w - 1);
        let y0 = sy.div_euclid(cell_m).clamp(0, h - 1);
        let fx = (sx.rem_euclid(cell_m) * 256 / cell_m).clamp(0, 256);
        let fy = (sy.rem_euclid(cell_m) * 256 / cell_m).clamp(0, 256);
        let at = |x: i64, y: i64| {
            let i = usize::try_from(y.clamp(0, h - 1) * w + x.clamp(0, w - 1)).unwrap_or(0);
            self.cells.get(i).copied().unwrap_or_default()
        };
        let c = [
            at(x0, y0),
            at(x0 + 1, y0),
            at(x0, y0 + 1),
            at(x0 + 1, y0 + 1),
        ];
        let mix = |f: fn(&MarginSample) -> u8| {
            let v = |s: &MarginSample| i64::from(f(s));
            let top = v(&c[0]) * (256 - fx) + v(&c[1]) * fx;
            let bot = v(&c[2]) * (256 - fx) + v(&c[3]) * fx;
            u8::try_from((top * (256 - fy) + bot * fy) / 65_536).unwrap_or(255)
        };
        MarginSample {
            collision: mix(|s| s.collision),
            arc: mix(|s| s.arc),
            island_arc: mix(|s| s.island_arc),
            rift: mix(|s| s.rift),
        }
    }

    /// Final-step volcanic-arc axis cells as visible-continent positions
    /// (metres, cell centres), in deterministic row-major order.
    #[must_use]
    pub fn volcanic_axis_m(&self) -> Vec<(i64, i64)> {
        let cell_m = i64::from(SIM_CELL_KM) * 1_000;
        self.volcanic_axis
            .iter()
            .map(|&(x, y)| {
                (
                    i64::from(x) * cell_m + cell_m / 2 - i64::from(self.offset_km.0) * 1_000,
                    i64::from(y) * cell_m + cell_m / 2 - i64::from(self.offset_km.1) * 1_000,
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_is_deterministic_and_marks_some_margins() {
        let a = margin_context_formed(42, 102, 204, 0);
        let b = margin_context_formed(42, 102, 204, 0);
        assert_eq!(a, b);
        let any = a
            .cells
            .iter()
            .any(|c| c.collision > 0 || c.arc > 0 || c.island_arc > 0 || c.rift > 0);
        assert!(any, "a drifting plate set must have some active margins");
    }

    #[test]
    fn classification_follows_crust_and_closing_sign() {
        let p = |id, crust, cx, dx| Plate {
            id,
            centre_x: cx,
            centre_y: 0,
            crust,
            drift_x: dx,
            drift_y: 0,
        };
        let c = CrustType::Continental;
        let o = CrustType::Oceanic;
        // b to the east moving west: closing.
        assert_eq!(classify(1, &p(0, c, 0, 0), [&p(1, c, 10, -2)]), COLLISION);
        assert_eq!(classify(1, &p(0, c, 0, 0), [&p(1, o, 10, -2)]), ARC);
        assert_eq!(classify(1, &p(0, c, 0, 0), [&p(1, o, 10, 2)]), RIFT);
        let (a, b) = (p(0, o, 0, 0), p(1, o, 10, -2));
        let one = classify(1, &a, [&b]) == ISLAND_ARC;
        let other = classify(1, &b, [&a]) == ISLAND_ARC;
        assert!(one ^ other, "exactly one side of an ocean arc overrides");
    }
}
