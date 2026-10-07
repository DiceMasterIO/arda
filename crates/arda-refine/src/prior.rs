//! Class weights at every vertex: the cell's cover, forest density,
//! wetness and temperature, interpolated between cell centres so a density
//! change between neighbouring cells thins across the block rather than at
//! its edge, then shaped by the local slope and aspect and broken into
//! patches by one rotated noise field per class.

use crate::classes::{Class, COUNT};
use crate::context::Ctx;
use crate::noise::{fbm, smoothstep};
use arda::{Cell, Cover, TerrainKind};

/// Per-class weights.
pub type Weights = [f64; COUNT];

/// How strongly the WFC follows the prior (exponent on the weights).
pub const SHARPNESS: f64 = 5.5;
/// Patch-noise amplitude in log-weight units: enough to break up uniform
/// ground, small enough that the landform terms below decide where each
/// ground lies (hollows wetter, knolls rockier), not noise blobs.
const PATCH: f64 = 1.0;

/// Base weights for one cell.
#[must_use]
pub fn cell_weights(c: &Cell) -> Weights {
    use Class::*;
    let mut w = [0.0; COUNT];
    let fd = f64::from(c.forest_density) / 255.0;
    let wet = f64::from(c.wetness) / 255.0;
    let moist = f64::from(c.moisture) / 255.0;
    let hab = f64::from(c.height_above_river_dm) / 10.0;
    let temp = f64::from(c.temperature.raw()) / 100.0;
    let mut set = |k: Class, v: f64| w[k.index()] += v;
    if c.terrain != TerrainKind::Land {
        set(Sand, 0.6);
        set(Gravel, 0.4);
        set(Rock, 0.3);
        set(Grass, 0.3);
        set(Mud, 0.2);
    } else {
        match c.cover {
            Cover::Forest => {
                set(ForestFloor, 0.35 + 0.9 * fd);
                set(LeafLitter, 0.25 + 0.25 * (1.0 - fd));
                set(Moss, 0.08 + 0.25 * moist);
                set(Scrub, 0.12);
                set(Grass, 0.4 * (1.0 - fd));
                set(Meadow, 0.1 * (1.0 - fd));
            }
            Cover::Grass => {
                set(Grass, 1.0);
                set(Meadow, 0.2 + 0.35 * moist);
                set(Scrub, 0.08);
                set(Heath, 0.05);
                set(Dirt, 0.03);
                set(LeafLitter, 0.35 * fd);
                set(ForestFloor, 0.35 * fd);
            }
            Cover::Scrub => {
                set(Scrub, 1.0);
                set(Heath, 0.6);
                set(Grass, 0.45);
                set(Meadow, 0.1);
                set(Rock, 0.05);
                set(Dirt, 0.05);
            }
            Cover::Marsh => {
                set(Marsh, 1.0);
                set(ReedBed, 0.55 * (1.0 - 0.5 * fd));
                set(Mud, 0.35);
                set(Grass, 0.25 * (1.0 - fd));
                set(Moss, 0.1 + 0.3 * fd);
                set(ForestFloor, 0.35 * fd);
                set(Meadow, 0.1 * (1.0 - fd));
            }
            Cover::Rock => {
                set(Rock, 1.0);
                set(Scree, 0.6);
                set(Moss, 0.12);
                set(Gravel, 0.1);
                set(Heath, 0.1);
                set(Grass, 0.1);
            }
            Cover::Ice => {
                set(Snow, 1.0);
                set(Ice, 0.35);
                set(Rock, 0.2);
                set(Scree, 0.1);
            }
            Cover::Bare => {
                set(Dirt, 0.5);
                set(Sand, 0.3);
                set(Gravel, 0.3);
                set(Grass, 0.3);
                set(Rock, 0.15);
                set(Scrub, 0.1);
            }
        }
        // A channel cell's own height above the river is zero by definition,
        // so only off-channel cells read it as floodplain.
        let low = if c.watercourse_order == 0 {
            1.0 - smoothstep(0.3, 1.5, hab)
        } else {
            0.0
        };
        let open = if c.cover == Cover::Forest {
            1.0 - 0.6 * fd
        } else {
            1.0
        };
        let soggy = ((wet - 0.7).max(0.0) * 1.5 + 0.4 * low) * open;
        set(Marsh, 0.5 * soggy);
        set(Mud, 0.2 * soggy);
        set(ReedBed, 0.2 * soggy);
        biome_weights(c, open, &mut w);
    }
    let snow = smoothstep(1.5, -2.5, temp);
    for k in [
        Grass,
        Meadow,
        ForestFloor,
        LeafLitter,
        Heath,
        Scrub,
        Marsh,
        ReedBed,
        Dirt,
    ] {
        w[k.index()] *= 1.0 - 0.85 * snow;
    }
    w[Snow.index()] += 1.6 * snow;
    w[Ice.index()] += 0.2 * snow;
    w
}

/// Ground of the cell's biome signals ([`crate::biome`]), whatever its
/// cover says: saturated ground is marsh with reed beds and mud; dry
/// country trades meadow and moss for bare earth and scrub; sea-level land
/// is beach; above the tree line and on crags rock, scree and moss take
/// over from turf.
fn biome_weights(c: &Cell, open: f64, w: &mut Weights) {
    use crate::biome::{alpine, arid, coastal, craggy, marshy};
    use Class::*;
    let bog = if c.cover == Cover::Marsh {
        0.0
    } else {
        marshy(c)
    };
    let (dry, coast, alp, crag) = (arid(c), coastal(c), alpine(c), craggy(c));
    let mut add = |k: Class, v: f64| w[k.index()] += v;
    add(Marsh, 1.4 * bog * open);
    add(ReedBed, 1.0 * bog * open);
    add(Mud, 0.5 * bog);
    add(Dirt, 0.45 * dry);
    add(Scrub, 0.12 * dry);
    add(Sand, 0.06 * dry * dry + 0.45 * coast);
    add(Gravel, 0.04 * dry + 0.15 * coast);
    add(Rock, 0.35 * alp + 0.2 * crag);
    add(Scree, 0.3 * alp + 0.3 * crag);
    add(Moss, 0.2 * alp * (1.0 - dry));
    add(Heath, 0.15 * alp);
    let mut scale = |k: Class, f: f64| w[k.index()] *= f.clamp(0.0, 1.0);
    scale(
        Grass,
        (1.0 - 0.8 * bog)
            * (1.0 - 0.3 * dry)
            * (1.0 - 0.5 * coast)
            * (1.0 - 0.4 * alp)
            * (1.0 - 0.5 * crag),
    );
    scale(
        Meadow,
        (1.0 - 0.6 * bog) * (1.0 - 0.85 * dry) * (1.0 - 0.7 * coast) * (1.0 - 0.5 * alp),
    );
    scale(Moss, 1.0 - 0.9 * dry);
    scale(ForestFloor, 1.0 - 0.9 * alp);
    scale(LeafLitter, 1.0 - 0.9 * alp);
    scale(Scrub, 1.0 - 0.7 * alp);
}

/// Precomputed cell weights for a block's neighbourhood.
#[derive(Debug, Clone)]
pub struct Prior {
    cells: Vec<(crate::source::CellKey, Weights)>,
}

impl Prior {
    /// Computes the weights of the gathered cells.
    #[must_use]
    pub fn new(ctx: &Ctx) -> Self {
        let r = crate::context::R;
        let mut cells = Vec::new();
        for dy in -r..=r {
            for dx in -r..=r {
                let k = ctx.cell.offset(dx, dy);
                cells.push((k, cell_weights(ctx.cell_at(k))));
            }
        }
        Self { cells }
    }

    fn at(&self, ctx: &Ctx, k: crate::source::CellKey) -> &Weights {
        let r = crate::context::R;
        let dx = (k.x - ctx.cell.x).clamp(-r, r) + r;
        let dy = (k.y - ctx.cell.y).clamp(-r, r) + r;
        let i = usize::try_from(dy * (2 * r + 1) + dx).unwrap_or(0);
        &self.cells[i.min(self.cells.len() - 1)].1
    }

    /// Log-weights (scores) at a vertex, from interpolated cell weights,
    /// local slope and northness, and per-class patch noise.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn scores(
        &self,
        ctx: &Ctx,
        u: f64,
        v: f64,
        slope: f64,
        north: f64,
        river_d: f64,
        shape: crate::shape::Shape,
    ) -> Weights {
        use Class::*;
        let (ix, fx) = split((u - 32.0) / 64.0);
        let (iy, fy) = split((v - 32.0) / 64.0);
        let k = crate::source::CellKey::new(ix, iy);
        let (a, b, c, d) = (
            self.at(ctx, k),
            self.at(ctx, k.offset(1, 0)),
            self.at(ctx, k.offset(0, 1)),
            self.at(ctx, k.offset(1, 1)),
        );
        let mut w = [0.0; COUNT];
        for i in 0..COUNT {
            let top = a[i] + (b[i] - a[i]) * fx;
            let bot = c[i] + (d[i] - c[i]) * fx;
            w[i] = top + (bot - top) * fy;
        }
        let steep = smoothstep(16.0, 36.0, slope);
        let crag = smoothstep(26.0, 44.0, slope);
        w[Scree.index()] = w[Scree.index()] * (1.0 + 3.0 * steep) + 0.35 * steep;
        // Steep squares break out as bare rock and cliff faces, so a
        // mountainside reads as rock bands among the scree, not a flat
        // talus sheet.
        w[Rock.index()] = w[Rock.index()] * (1.0 + 4.0 * crag) + 0.9 * crag;
        w[Cliff.index()] = 0.9 * smoothstep(32.0, 44.0, slope);
        for k in [Grass, Meadow, Dirt, Marsh, Mud, ReedBed, Sand] {
            w[k.index()] *= 1.0 - 0.8 * steep;
        }
        w[ForestFloor.index()] *= 1.0 - 0.6 * crag;
        let facing = north * smoothstep(3.0, 14.0, slope);
        w[Moss.index()] *= 1.0 + 1.2 * facing.max(0.0);
        w[Snow.index()] *= 1.0 + 0.6 * facing;
        w[Heath.index()] *= 1.0 + 0.8 * (-facing).max(0.0);
        w[Scrub.index()] *= 1.0 + 0.4 * (-facing).max(0.0);
        w[Meadow.index()] *= 1.0 + 0.3 * (-facing).max(0.0);
        landform(ctx, &mut w, u, v, slope, north, shape);
        // Riparian strip: soft ground and reeds along slow channels.
        let strip = (1.0 - smoothstep(0.0, 5.0, river_d)) * (1.0 - steep);
        w[Mud.index()] += 0.25 * strip;
        w[ReedBed.index()] += 0.3 * strip;
        w[Grass.index()] += 0.15 * strip;
        w[Water.index()] = 0.35;
        let mut s = [f64::NEG_INFINITY; COUNT];
        for i in 0..COUNT {
            if w[i] > 1e-4 {
                let tag = 0x7A00 + i as u64;
                s[i] = crate::math::ln(w[i]) + PATCH * fbm(ctx.seed, tag, u, v, 19.0, 3, 0.5);
            } else {
                s[i] = -12.0;
            }
        }
        s
    }
}

/// Ground follows the landform (goals 19, 20 and 43): knolls and ridges
/// carry thin soil, rock and heath; hollows collect water, deeper meadow,
/// moss and marsh; in cold country snow lingers in shaded hollows and on
/// north-facing slopes long after it has left the sunny ones.
fn landform(
    ctx: &Ctx,
    w: &mut Weights,
    u: f64,
    v: f64,
    slope: f64,
    north: f64,
    shape: crate::shape::Shape,
) {
    use Class::*;
    let (ridge, bowl) = (shape.ridge(), shape.bowl());
    let land = 1.0 - w[Water.index()].min(1.0);
    let steepish = smoothstep(6.0, 24.0, slope);
    w[Rock.index()] += land * ridge * (0.12 + 0.45 * steepish);
    w[Scree.index()] += land * 0.3 * shape.talus;
    w[Heath.index()] += 0.25 * ridge * (w[Grass.index()] + w[Scrub.index()]).min(1.0);
    w[Dirt.index()] += 0.06 * ridge * w[Grass.index()].min(1.0);
    w[Grass.index()] *= 1.0 - 0.35 * ridge;
    w[Meadow.index()] *= 1.0 + 2.0 * bowl - 0.6 * ridge;
    w[Moss.index()] += 0.2 * bowl * (w[ForestFloor.index()] + w[Rock.index()]).min(1.0);
    let wet = ctx.bilinear(u, v, |c, _| f64::from(c.wetness) / 255.0);
    w[Marsh.index()] += 0.5 * bowl * smoothstep(0.35, 0.8, wet) * (1.0 - steepish);
    w[Mud.index()] += 0.12 * bowl * smoothstep(0.45, 0.9, wet) * (1.0 - steepish);
    // Snow patches: cold country only, in proportion to shade.
    w[Snow.index()] += 1.8 * lie(ctx, u, v, slope, north, bowl);
}

fn lie(ctx: &Ctx, u: f64, v: f64, slope: f64, north: f64, bowl: f64) -> f64 {
    let temp = ctx.bilinear(u, v, |c, _| f64::from(c.temperature.raw()) / 100.0);
    let cold = smoothstep(5.0, 0.0, temp);
    let shade = (north * smoothstep(4.0, 22.0, slope)).max(0.0);
    cold * smoothstep(0.45, 0.9, 0.55 * shade + 0.8 * bowl + 0.15 * north)
}

/// How strongly snow lingers at a square in cold shade and hollows
/// (`0..=1`), the term the ground prior adds to snow; scatter reads it so
/// no flower stands on a drift.
#[must_use]
pub fn snow_lie(
    ctx: &Ctx,
    u: f64,
    v: f64,
    p: &crate::terrain::Phys,
    s: crate::shape::Shape,
) -> f64 {
    lie(ctx, u, v, p.slope_deg, p.north, s.bowl())
}

fn split(t: f64) -> (i64, f64) {
    let f = t.floor();
    #[allow(clippy::cast_possible_truncation)]
    let i = f as i64;
    (i, t - f)
}
