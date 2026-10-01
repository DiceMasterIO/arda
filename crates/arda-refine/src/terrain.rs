//! Per-square physical fields: continuous elevation, channels, standing
//! water, depth and slope. Each is a function of global square position
//! only, so a block and its neighbour agree on every square they both see.

use crate::classes::{Class, Mask};
use crate::context::{Ctx, SQUARE_M};
use crate::grid::Grid;
use crate::noise::{fbm, smoothstep};
use crate::rivers::Piece;
use crate::source::CellKey;
use arda::{Cover, TerrainKind};

/// Which water, if any, covers a square.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Water {
    /// Dry land.
    Dry,
    /// A river or stream channel.
    River,
    /// A lake.
    Lake,
    /// The sea.
    Sea,
    /// A shallow marsh pool.
    Pool,
}

/// The physical fields of one square.
#[derive(Debug, Clone, Copy)]
pub struct Phys {
    /// Terrain before water shaping, metres.
    pub land_m: f64,
    /// Final ground or water-surface elevation, metres.
    pub elev_m: f64,
    /// Water cover.
    pub water: Water,
    /// Water depth, metres (zero when dry).
    pub depth_m: f64,
    /// Signed distance to the nearest river bank, squares (negative inside).
    pub river_d: f64,
    /// Standing-water indicator: positive is water.
    pub stand_v: f64,
    /// Which standing water the indicator belongs to.
    pub stand_kind: Water,
    /// Local slope, degrees.
    pub slope_deg: f64,
    /// How much the slope faces north, `-1..1`, zero on the flat.
    pub north: f64,
}

impl Default for Phys {
    fn default() -> Self {
        Self {
            land_m: 0.0,
            elev_m: 0.0,
            water: Water::Dry,
            depth_m: 0.0,
            river_d: f64::INFINITY,
            stand_v: -1.0,
            stand_kind: Water::Dry,
            slope_deg: 0.0,
            north: 0.0,
        }
    }
}

/// Detail-noise amplitude in metres for a cell: rough on steep and rocky
/// ground, smooth on marsh and meadow (goal 43).
fn roughness(c: &arda::Cell) -> f64 {
    let slope = f64::from(c.slope_milli_deg) / 1000.0;
    let cover = match c.cover {
        Cover::Rock => 0.35,
        Cover::Ice => 0.2,
        Cover::Bare => 0.12,
        Cover::Scrub => 0.08,
        Cover::Forest => 0.05,
        Cover::Grass => 0.0,
        Cover::Marsh => -0.08,
    };
    (0.2 + 0.6 * smoothstep(0.0, 35.0, slope) + cover).max(0.12)
}

/// A smooth, rotated domain warp of up to `amp` squares, so iso-lines of
/// separable (lattice-aligned) interpolation never run straight along the
/// grid axes.
#[must_use]
pub fn warp(ctx: &Ctx, tag: u64, u: f64, v: f64, amp: f64, wavelength: f64) -> (f64, f64) {
    (
        u + amp * fbm(ctx.seed, tag, u, v, wavelength, 2, 0.5),
        v + amp * fbm(ctx.seed, tag ^ 0x55, u, v, wavelength, 2, 0.5),
    )
}

/// Continuous land elevation at global square position `(u, v)`, metres.
#[must_use]
pub fn land_height(ctx: &Ctx, u: f64, v: f64) -> f64 {
    let amp = ctx.bilinear(u, v, |c, _| roughness(c));
    let (wu, wv) = warp(ctx, 0xA3A3, u, v, 4.0, 45.0);
    ctx.base_height(wu, wv) + amp * fbm(ctx.seed, 0xE1E7, u, v, 26.0, 3, 0.5)
}

/// Standing-water indicator: Catmull-Rom of ±1 over cells, bent by noise so
/// shores never follow cell edges.
fn standing(ctx: &Ctx, u: f64, v: f64, level: f64, land: f64) -> f64 {
    let (wu, wv) = warp(ctx, 0x5A5A, u, v, 9.0, 60.0);
    let base = ctx.bicubic(wu, wv, |c, _| {
        if c.terrain == TerrainKind::Land {
            -1.0
        } else {
            1.0
        }
    });
    // Where the terrain knows the shore, follow its water-level contour.
    let terrain = ((level - land) / 3.0).clamp(-1.0, 1.0);
    0.6 * base + 0.4 * terrain + 0.25 * fbm(ctx.seed, 0x5708, u, v, 20.0, 3, 0.5)
}

/// The standing water nearest `(u, v)`: kind, level and full depth.
fn nearest_standing(ctx: &Ctx, u: f64, v: f64) -> Option<(Water, f64, f64)> {
    let home = Ctx::cell_of(u, v);
    let mut best: Option<(f64, CellKey)> = None;
    for dy in -1..=1 {
        for dx in -1..=1 {
            let k = home.offset(dx, dy);
            if !ctx.is_water_cell(k) {
                continue;
            }
            let (cx, cy) = crate::rivers::centre(k);
            let d = (cx - u) * (cx - u) + (cy - v) * (cy - v);
            if best.is_none_or(|b| d < b.0) {
                best = Some((d, k));
            }
        }
    }
    let k = best?.1;
    let cell = ctx.cell_at(k);
    if cell.terrain == TerrainKind::Sea {
        let depth = (-f64::from(cell.height.raw()) / 1000.0).clamp(1.0, 40.0);
        return Some((Water::Sea, 0.0, depth));
    }
    let (level, depth) = ctx
        .lake_at(k)
        .map_or((f64::from(cell.height.raw()) / 1000.0, 2.0), |l| {
            (
                f64::from(l.surface_mm) / 1000.0,
                f64::from(l.depth_mm) / 1000.0,
            )
        });
    Some((Water::Lake, level, depth.clamp(0.5, 40.0)))
}

/// Standing water near a point (logic/09 §linear-features, Shores).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Standing {
    /// Lake or sea.
    pub kind: Water,
    /// Water level, metres.
    pub level: f64,
    /// Full depth of the nearest water cell, metres.
    pub full: f64,
    /// Indicator: positive is water, the bank shaping runs above -0.6.
    pub v: f64,
}

impl Standing {
    /// Water depth where the indicator is positive, metres.
    #[must_use]
    pub fn depth_m(&self) -> f64 {
        (self.full * smoothstep(0.0, 0.7, self.v)).max(0.3)
    }

    /// Whether the point is standing water.
    #[must_use]
    pub fn is_water(&self) -> bool {
        self.v > 0.0
    }
}

/// The standing water the shore rule sees at `(u, v)` over land at
/// `land()` metres, or `None` where no water cell is near enough to shape
/// the ground. The one rule blocks and relief tiles share.
#[must_use]
pub fn standing_water(ctx: &Ctx, u: f64, v: f64, land: impl FnOnce() -> f64) -> Option<Standing> {
    let (kind, level, full) = nearest_standing(ctx, u, v)?;
    let sv = standing(ctx, u, v, level, land());
    (sv > -0.6).then_some(Standing {
        kind,
        level,
        full,
        v: sv,
    })
}

/// How far beyond the bank the valley is shaped, squares.
const CARVE_REACH: f64 = 20.0;
/// Bank rise per square away from the water, metres (about 18 degrees).
const BANK_RISE: f64 = 0.5;

/// River hit at one square: signed bank distance, half-width, surface and
/// thalweg depth.
#[derive(Debug, Clone, Copy)]
struct Hit {
    d: f64,
    half: f64,
    surface: f64,
    depth: f64,
}

fn rasterise(pieces: &[Piece], grid: &mut Grid<Option<Hit>>) {
    for p in pieces {
        for i in 1..p.pts.len() {
            let (a, b) = (p.pts[i - 1], p.pts[i]);
            let reach = p.half[i].max(p.half[i - 1]) + CARVE_REACH;
            #[allow(clippy::cast_possible_truncation)]
            let (x0, x1, y0, y1) = (
                (a.0.min(b.0) - reach).floor() as i64,
                (a.0.max(b.0) + reach).ceil() as i64,
                (a.1.min(b.1) - reach).floor() as i64,
                (a.1.max(b.1) + reach).ceil() as i64,
            );
            for y in y0.max(grid.y0)..=y1.min(grid.y0 + crate::grid::span(grid.h) - 1) {
                for x in x0.max(grid.x0)..=x1.min(grid.x0 + crate::grid::span(grid.w) - 1) {
                    let (d, t) =
                        crate::rivers::bank_distance(p, i, (x as f64 + 0.5, y as f64 + 0.5));
                    let half = p.half[i - 1] + (p.half[i] - p.half[i - 1]) * t;
                    let hit = Hit {
                        d,
                        half,
                        surface: p.surface[i - 1] + (p.surface[i] - p.surface[i - 1]) * t,
                        depth: p.depth[i - 1] + (p.depth[i] - p.depth[i - 1]) * t,
                    };
                    if let Some(slot) = grid.get_mut(x, y) {
                        if slot.is_none_or(|h| hit.d < h.d) {
                            *slot = Some(hit);
                        }
                    }
                }
            }
        }
    }
}

/// Polynomial smooth minimum with blending width `k`.
fn smooth_min(a: f64, b: f64, k: f64) -> f64 {
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    b + (a - b) * h - k * h * (1.0 - h)
}

/// Computes the physical fields over `grid`'s squares; slopes are left for
/// [`slopes`].
#[must_use]
pub fn physical(ctx: &Ctx, pieces: &[Piece], x0: i64, y0: i64, side: usize) -> Grid<Phys> {
    let mut hits: Grid<Option<Hit>> = Grid::new(x0, y0, side, side, None);
    rasterise(pieces, &mut hits);
    let mut out = Grid::new(x0, y0, side, side, Phys::default());
    for (i, (x, y)) in hits.coords().collect::<Vec<_>>().into_iter().enumerate() {
        let (u, v) = (x as f64 + 0.5, y as f64 + 0.5);
        let land = land_height(ctx, u, v);
        let mut p = Phys {
            land_m: land,
            elev_m: land,
            ..Phys::default()
        };
        if let Some(st) = standing_water(ctx, u, v, || land) {
            let (sv, kind, level) = (st.v, st.kind, st.level);
            p.stand_v = sv;
            p.stand_kind = kind;
            if sv > 0.0 {
                p.water = kind;
                p.elev_m = level;
                p.depth_m = st.depth_m();
            } else {
                let t = smoothstep(-0.45, 0.0, sv);
                p.elev_m = land + (level + 0.3 - land) * t;
            }
        }
        if let Some(h) = hits.data[i] {
            p.river_d = h.d;
            if p.water == Water::Dry {
                let bank = 2.0 + h.half;
                if h.d <= 0.0 {
                    let rel = ((h.d + h.half) / h.half).clamp(0.0, 1.0);
                    p.water = Water::River;
                    p.elev_m = h.surface;
                    p.depth_m = (h.depth * (1.0 - rel * rel)).max(0.12);
                } else {
                    // Banks: lower the terrain onto a ramp rising from the
                    // water surface, so a channel never sits in a trench
                    // with cliff walls; raise it to bank level right at the
                    // edge; fade the shaping out smoothly.
                    let ramp = h.surface + 0.25 + BANK_RISE * h.d;
                    let fade = 1.0 - smoothstep(CARVE_REACH * 0.6, CARVE_REACH, h.d);
                    let shaped = if p.elev_m > ramp {
                        smooth_min(p.elev_m, ramp, 1.5)
                    } else {
                        p.elev_m + (ramp - p.elev_m) * (1.0 - smoothstep(0.0, bank, h.d))
                    };
                    p.elev_m += (shaped - p.elev_m) * fade;
                }
            }
        }
        if p.water == Water::Dry && crate::pools::pool(ctx, u, v) > 0.0 {
            p.water = Water::Pool;
            p.depth_m = 0.3;
        }
        out.data[i] = p;
    }
    out
}

/// Fills slope and northness from central differences of the final
/// elevation; edge squares reuse their inner neighbour's slope.
pub fn slopes(grid: &mut Grid<Phys>) {
    let snapshot: Vec<f64> = grid.data.iter().map(|p| p.elev_m).collect();
    let (w, h) = (grid.w, grid.h);
    for j in 0..h {
        for i in 0..w {
            let at = |a: usize, b: usize| snapshot[b.min(h - 1) * w + a.min(w - 1)];
            let (l, r) = (i.saturating_sub(1), i + 1);
            let (t, b) = (j.saturating_sub(1), j + 1);
            let span_x = (r.min(w - 1) - l) as f64 * SQUARE_M;
            let span_y = (b.min(h - 1) - t) as f64 * SQUARE_M;
            let gx = (at(r, j) - at(l, j)) / span_x.max(1e-9);
            let gy = (at(i, b) - at(i, t)) / span_y.max(1e-9);
            let g = (gx * gx + gy * gy).sqrt();
            let p = &mut grid.data[j * w + i];
            p.slope_deg = atan_deg(g);
            // Elevation rising southward means the slope faces north.
            p.north = if g > 1e-9 { gy / g } else { 0.0 };
        }
    }
}

/// `atan(x)` in degrees for `x >= 0`, by a rational approximation (max
/// error about 0.1 degree) so results are identical on every platform.
#[must_use]
pub fn atan_deg(x: f64) -> f64 {
    let (x, flip) = if x > 1.0 { (1.0 / x, true) } else { (x, false) };
    let a = x * (std::f64::consts::FRAC_PI_4 + 0.273 * (1.0 - x)) * 57.295_779_513;
    if flip {
        90.0 - a
    } else {
        a
    }
}

/// Bank classes a water square may show at its margins (shallows and bank
/// transitions), by water kind, slope and cover.
#[must_use]
pub fn bank_classes(p: &Phys, rocky: f64) -> Mask {
    use Class::{Grass, Gravel, Marsh, Moss, Mud, ReedBed, Rock, Sand};
    let steep = p.slope_deg > 22.0 || rocky > 0.5;
    let bits = match p.water {
        Water::Sea if steep => Rock.bit() | Gravel.bit(),
        Water::Sea if p.slope_deg > 9.0 => Gravel.bit() | Sand.bit() | Rock.bit(),
        Water::Sea => Sand.bit(),
        Water::Lake if steep => Rock.bit() | Gravel.bit(),
        Water::Lake => Sand.bit() | Gravel.bit() | Mud.bit() | ReedBed.bit() | Grass.bit(),
        Water::River if steep => Rock.bit() | Gravel.bit() | Grass.bit(),
        Water::River => Mud.bit() | Gravel.bit() | Grass.bit() | Sand.bit() | ReedBed.bit(),
        Water::Pool => ReedBed.bit() | Mud.bit() | Marsh.bit() | Moss.bit(),
        Water::Dry => 0,
    };
    bits | Class::Water.bit()
}
