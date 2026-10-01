//! Water over any region of global squares: the one source of truth for
//! where rivers, lakes and the sea lie at every zoom (logic/09
//! §linear-features; logic/17 §water).
//!
//! A [`WaterRegion`] gathers the same global cells, channel edges and fine
//! lattice nodes a block reads ([`Ctx::gather_region`]) and answers point
//! queries with the blocks' own rules: [`standing_water`] for lake and sea
//! shores, the centreline pieces of [`crate::rivers`] for channels and
//! [`crate::pools`] for marsh pools. At a square's centre,
//! [`WaterRegion::water`] is the water a refined block marks there, so
//! relief tiles drawn from a region and tactical maps agree square for
//! square.

use crate::context::{Ctx, EDGE_R};
use crate::error::RefineError;
use crate::pools::pool;
use crate::rivers::{bank_distance, cell_pieces, Piece};
use crate::source::{CellKey, Source};
use crate::terrain::{land_height, standing_water, Water};
use std::collections::HashMap;

/// How a caller draws channels: half-width `max(half · gain, min_half)`
/// squares. The default (`gain` 1, `min_half` 0) is the blocks' own banks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiverStyle {
    /// Drawn over physical half-width, at least 1.
    pub gain: f64,
    /// Narrowest drawn half-width, squares.
    pub min_half: f64,
}

impl Default for RiverStyle {
    fn default() -> Self {
        Self {
            gain: 1.0,
            min_half: 0.0,
        }
    }
}

/// The water at a point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterAt {
    /// Which water, [`Water::Dry`] on land.
    pub kind: Water,
    /// Depth, metres (zero when dry).
    pub depth_m: f64,
}

/// The nearest drawn channel bank at a point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiverHit {
    /// Signed distance to the drawn bank, squares (negative inside).
    pub d: f64,
    /// Saved discharge of that channel, milli-m³/s.
    pub discharge_milli: u64,
}

/// Side of the bins segments are indexed by, squares.
const BIN: f64 = 8.0;

#[allow(clippy::cast_possible_truncation)] // square coordinates fit i64
fn bin(v: f64) -> i64 {
    (v / BIN).floor() as i64
}

/// Water geometry over a region of global squares.
#[derive(Debug, Clone)]
pub struct WaterRegion {
    ctx: Ctx,
    pieces: Vec<Piece>,
    bins: HashMap<(i64, i64), Vec<(u32, u32)>>,
    style: RiverStyle,
}

impl WaterRegion {
    /// Gathers the water of global squares `[sx0, sx1] × [sy0, sy1]`
    /// (inclusive), with channels drawn in `style`.
    ///
    /// # Errors
    /// A world layer failed to read.
    pub fn gather(
        src: &dyn Source,
        squares: (i64, i64, i64, i64),
        style: RiverStyle,
    ) -> Result<Self, RefineError> {
        let style = RiverStyle {
            gain: style.gain.max(1.0),
            min_half: style.min_half.max(0.0),
        };
        let ctx = Ctx::gather_region(src, squares)?;
        let (sx0, sy0, sx1, sy1) = squares;
        let lo = Ctx::cell_of(sx0 as f64, sy0 as f64).offset(-EDGE_R, -EDGE_R);
        let hi = Ctx::cell_of(sx1 as f64, sy1 as f64).offset(EDGE_R, EDGE_R);
        let mut cells: Vec<CellKey> = ctx
            .edges
            .iter()
            .flat_map(|e| [e.from, e.to])
            .filter(|c| c.x >= lo.x && c.x <= hi.x && c.y >= lo.y && c.y <= hi.y)
            .collect();
        cells.sort();
        cells.dedup();
        let (w, h) = src.cells_wide_high();
        let mut pieces = Vec::new();
        for c in cells {
            if c.x >= 0 && c.y >= 0 && c.x < w && c.y < h {
                pieces.extend(
                    cell_pieces(&ctx, c)
                        .into_iter()
                        .filter(|p| p.pts.len() >= 2),
                );
            }
        }
        let mut bins: HashMap<(i64, i64), Vec<(u32, u32)>> = HashMap::new();
        for (k, p) in pieces.iter().enumerate() {
            let k = u32::try_from(k).unwrap_or(u32::MAX);
            for i in 1..p.pts.len() {
                let (a, b) = (p.pts[i - 1], p.pts[i]);
                let r = drawn(&style, p.half[i - 1].max(p.half[i])) + 1.0;
                let i = u32::try_from(i).unwrap_or(u32::MAX);
                for by in bin(a.1.min(b.1) - r)..=bin(a.1.max(b.1) + r) {
                    for bx in bin(a.0.min(b.0) - r)..=bin(a.0.max(b.0) + r) {
                        bins.entry((bx, by)).or_default().push((k, i));
                    }
                }
            }
        }
        Ok(Self {
            ctx,
            pieces,
            bins,
            style,
        })
    }

    /// The gathered world context.
    #[must_use]
    pub const fn ctx(&self) -> &Ctx {
        &self.ctx
    }

    /// The channel pieces near the region.
    #[must_use]
    pub fn pieces(&self) -> &[Piece] {
        &self.pieces
    }

    fn segments(&self, q: (f64, f64)) -> impl Iterator<Item = (&Piece, usize)> {
        self.bins
            .get(&(bin(q.0), bin(q.1)))
            .into_iter()
            .flatten()
            .filter_map(|&(k, i)| {
                let p = self.pieces.get(usize::try_from(k).ok()?)?;
                Some((p, usize::try_from(i).ok()?))
            })
    }

    /// The nearest drawn channel bank at `q` (global square units), within
    /// one square of a drawn channel; `None` farther out.
    #[must_use]
    pub fn river(&self, q: (f64, f64)) -> Option<RiverHit> {
        let mut best: Option<RiverHit> = None;
        for (p, i) in self.segments(q) {
            let (d, t) = bank_distance(p, i, q);
            let half = p.half[i - 1] + (p.half[i] - p.half[i - 1]) * t;
            let d = d + half - drawn(&self.style, half);
            if best.is_none_or(|b| d < b.d) {
                best = Some(RiverHit {
                    d,
                    discharge_milli: p.discharge_milli,
                });
            }
        }
        best.filter(|b| b.d <= 1.0)
    }

    /// Whether `q` lies within a channel's physical banks.
    fn in_channel(&self, q: (f64, f64)) -> bool {
        self.segments(q)
            .any(|(p, i)| bank_distance(p, i, q).0 <= 0.0)
    }

    /// Standing water at global square position `(u, v)`: lake or sea
    /// where the shore rule puts water, then (with `pools`) marsh pools;
    /// `None` on land and in channels.
    #[must_use]
    pub fn standing(&self, u: f64, v: f64, pools: bool) -> Option<WaterAt> {
        if let Some(st) = standing_water(&self.ctx, u, v, || land_height(&self.ctx, u, v))
            .filter(|s| s.is_water())
        {
            return Some(WaterAt {
                kind: st.kind,
                depth_m: st.depth_m(),
            });
        }
        (pools && !self.in_channel((u, v)) && pool(&self.ctx, u, v) > 0.0).then_some(WaterAt {
            kind: Water::Pool,
            depth_m: 0.3,
        })
    }

    /// The water a refined block marks at global square position `(u, v)`
    /// (logic/09 §linear-features): standing water first, then channels,
    /// then (with `pools`) marsh pools.
    #[must_use]
    pub fn water(&self, u: f64, v: f64, pools: bool) -> WaterAt {
        if let Some(at) = self.standing(u, v, false) {
            return at;
        }
        if self.in_channel((u, v)) {
            return WaterAt {
                kind: Water::River,
                depth_m: 0.0,
            };
        }
        if pools && pool(&self.ctx, u, v) > 0.0 {
            return WaterAt {
                kind: Water::Pool,
                depth_m: 0.3,
            };
        }
        WaterAt {
            kind: Water::Dry,
            depth_m: 0.0,
        }
    }
}

fn drawn(style: &RiverStyle, half: f64) -> f64 {
    (half * style.gain).max(style.min_half)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::refine;
    use crate::context::N;
    use crate::synthetic::{world, MARSH, RIVER_CELL, SEA_ROW};

    /// logic/17 §water: a region's water is the blocks' water, square for
    /// square, over rivers, a lake shore and the sea.
    #[test]
    fn region_water_matches_the_refined_blocks() {
        let src = world(7);
        let (mut wet, mut dry) = (0, 0);
        let cells = [
            RIVER_CELL,
            RIVER_CELL.offset(0, -1),
            CellKey::new(11, 2),
            CellKey::new(12, 3),
            CellKey::new(5, SEA_ROW),
            CellKey::new(5, SEA_ROW - 1),
            CellKey::new(MARSH[0].0, MARSH[0].1),
        ];
        for c in cells {
            let block = refine(&src, c).unwrap();
            let region = WaterRegion::gather(
                &src,
                (c.x * N - 5, c.y * N - 3, c.x * N + 70, c.y * N + 64),
                RiverStyle::default(),
            )
            .unwrap();
            for y in 0..N {
                for x in 0..N {
                    let (gx, gy) = (c.x * N + x, c.y * N + y);
                    let k = usize::try_from(y * N + x).unwrap();
                    let at = region.water(gx as f64 + 0.5, gy as f64 + 0.5, true);
                    let river = region.river((gx as f64 + 0.5, gy as f64 + 0.5));
                    assert_eq!(
                        at.kind != Water::Dry,
                        block.depth_ft[k] > 0,
                        "square {gx},{gy}: region {at:?}"
                    );
                    if at.kind == Water::River {
                        assert!(river.is_some_and(|r| r.d <= 0.0));
                    }
                    if block.depth_ft[k] > 0 {
                        wet += 1;
                    } else {
                        dry += 1;
                    }
                }
            }
        }
        assert!(wet > 500 && dry > 500, "{wet} wet, {dry} dry squares");
    }

    #[test]
    fn drawn_banks_widen_with_the_style() {
        let src = world(7);
        let c = RIVER_CELL;
        let squares = (c.x * N, c.y * N, c.x * N + 63, c.y * N + 63);
        let exact = WaterRegion::gather(&src, squares, RiverStyle::default()).unwrap();
        let wide = WaterRegion::gather(
            &src,
            squares,
            RiverStyle {
                gain: 3.0,
                min_half: 2.0,
            },
        )
        .unwrap();
        let count = |r: &WaterRegion| {
            (0..N * N)
                .filter(|k| {
                    let q = (
                        (c.x * N + k % N) as f64 + 0.5,
                        (c.y * N + k / N) as f64 + 0.5,
                    );
                    r.river(q).is_some_and(|h| h.d <= 0.0)
                })
                .count()
        };
        assert!(count(&wide) > 2 * count(&exact), "{}", count(&exact));
        assert!(count(&exact) > 0);
    }
}
