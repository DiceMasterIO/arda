//! Class weights at every vertex: the cell's cover, forest density,
//! wetness and temperature, interpolated between cell centres so a density
//! change between neighbouring cells thins across the block rather than at
//! its edge, then shaped by the local slope and aspect and broken into
//! patches by one rotated noise field per class.

use crate::classes::{Class, COUNT};
use crate::context::Ctx;
use crate::noise::{fbm, smoothstep};
use arda::{Cell, Cover, TerrainKind};
use arda_core::biome::aspect::sunward;
use arda_core::biome::ground::rock;
use arda_core::biome::{climate, Lines};

/// Per-class weights.
pub type Weights = [f64; COUNT];

/// How strongly the WFC follows the prior (exponent on the weights).
pub const SHARPNESS: f64 = 5.5;
/// Patch-noise amplitude in log-weight units: enough to break up uniform
/// ground, small enough that the landform terms below decide where each
/// ground lies (hollows wetter, knolls rockier), not noise blobs.
const PATCH: f64 = 1.0;

/// Ground weights the shoreline ([`crate::biome::shoreline`]) adds at full
/// strength, calibrated with `arda-refine coastwet` (logic/09 §biome signals).
struct ShoreWeights {
    sand: f64,
    gravel: f64,
    mud: f64,
    marsh: f64,
    reed: f64,
    rock: f64,
    scree: f64,
}

const SHORE: ShoreWeights = ShoreWeights {
    sand: 1.0,
    gravel: 0.8,
    mud: 0.5,
    marsh: 0.8,
    reed: 0.3,
    rock: 0.5,
    scree: 0.1,
};

/// Base weights for one cell under the tree and snow lines of its row.
#[must_use]
pub fn cell_weights(c: &Cell, coast: crate::biome::Coast, lines: Lines) -> Weights {
    use Class::*;
    let mut w = [0.0; COUNT];
    let fd = f64::from(c.forest_density) / 255.0;
    let wet = f64::from(c.wetness) / 255.0;
    let moist = f64::from(c.moisture) / 255.0;
    let hab = f64::from(c.height_above_river_dm) / 10.0;
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
        biome_weights(c, coast, open, lines, &mut w);
    }
    w
}

/// Ground of the cell's biome signals ([`crate::biome`]), whatever its
/// cover says: saturated ground is marsh with reed beds and mud; dry
/// country trades meadow and moss for bare earth and scrub; sea-level land
/// is beach; above the tree line and on crags rock, scree and moss take
/// over from turf.
fn biome_weights(c: &Cell, coast: crate::biome::Coast, open: f64, lines: Lines, w: &mut Weights) {
    use crate::biome::{alpine, arid, craggy, marshy, shoreline};
    use Class::*;
    let bog = if c.cover == Cover::Marsh {
        0.0
    } else {
        marshy(c)
    };
    let sh = shoreline(c, coast);
    let (dry, coast, alp, crag) = (arid(c), sh.strip, alpine(c, lines), craggy(c, lines));
    let mut add = |k: Class, v: f64| w[k.index()] += v;
    add(Marsh, 1.4 * bog * open);
    add(ReedBed, 1.0 * bog * open);
    add(Mud, 0.5 * bog);
    add(Dirt, 0.45 * dry);
    add(Scrub, 0.12 * dry);
    add(Sand, 0.06 * dry * dry + SHORE.sand * sh.sand);
    add(Gravel, 0.04 * dry + SHORE.gravel * sh.shingle);
    add(Mud, SHORE.mud * sh.mud);
    add(Marsh, SHORE.marsh * sh.mud);
    add(ReedBed, SHORE.reed * sh.mud);
    add(Rock, SHORE.rock * sh.rock);
    add(Scree, SHORE.scree * sh.rock);
    add(Rock, 0.35 * alp + 0.2 * crag);
    add(Scree, 0.3 * alp + 0.3 * crag);
    add(Moss, 0.2 * alp * (1.0 - dry));
    add(Heath, 0.15 * alp);
    let mut scale = |k: Class, f: f64| w[k.index()] *= f.clamp(0.0, 1.0);
    scale(
        Grass,
        (1.0 - 0.8 * bog)
            * (1.0 - 0.3 * dry)
            * (1.0 - sh.turf)
            * (1.0 - 0.4 * alp)
            * (1.0 - 0.5 * crag),
    );
    scale(
        Meadow,
        (1.0 - 0.6 * bog) * (1.0 - 0.85 * dry) * (1.0 - 0.7 * coast) * (1.0 - 0.5 * alp),
    );
    scale(Moss, 1.0 - 0.9 * dry);
    // Sand, shingle and salt marsh are open ground: woods stop at the beach.
    let open_shore = 1.0 - 0.8 * sh.open;
    scale(ForestFloor, (1.0 - 0.9 * alp) * open_shore);
    scale(LeafLitter, (1.0 - 0.9 * alp) * open_shore);
    scale(Scrub, (1.0 - 0.7 * alp) * (1.0 - 0.5 * sh.open));
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
                cells.push((
                    k,
                    cell_weights(ctx.cell_at(k), ctx.coast_at(k), ctx.lines_of(k)),
                ));
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
        // Bare rock breaks out where the slope is too steep for the
        // vegetation the world shows to hold soil: the shared exposure
        // (`arda_core::biome::ground::rock`) of the square's slope under the
        // cells' forest density and tree growth, so a wooded mountainside
        // stays wooded where an open one turns to rock bands among the scree
        // (review 3.4-2). On open, frost-free ground the ramps are the old
        // ones: scree from about 26°, rock from 35°, cliff from 38°.
        let fd = ctx.bilinear(u, v, |c, _| f64::from(c.forest_density) / 255.0);
        let growth = ctx
            .lines_at(v)
            .growth(ctx.bilinear(u, v, |c, _| f64::from(c.temperature.raw()) / 100.0));
        let steep = rock(slope + 10.0, fd, growth);
        let crag = rock(slope, fd, growth);
        w[Scree.index()] = w[Scree.index()] * (1.0 + 3.0 * steep) + 0.35 * steep;
        w[Rock.index()] = w[Rock.index()] * (1.0 + 4.0 * crag) + 0.9 * crag;
        w[Cliff.index()] = 0.9 * rock(slope - 3.0, fd, growth);
        for k in [Grass, Meadow, Dirt, Marsh, Mud, ReedBed, Sand] {
            w[k.index()] *= 1.0 - 0.8 * steep;
        }
        w[ForestFloor.index()] *= 1.0 - 0.6 * crag;
        // Shade is the side away from the sun: north-facing in the northern
        // hemisphere, south-facing in the southern (review 3.2-7).
        let pole = -sunward(north, ctx.lat_deg(v));
        let facing = pole * smoothstep(3.0, 14.0, slope);
        w[Moss.index()] *= 1.0 + 1.2 * facing.max(0.0);
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
    // Snow: the world's snow line at an effective temperature corrected for
    // sun, hollows and wind-scoured ridges (the Atlas's rule). Its weight is
    // set so that, under the WFC's sharpened choice (weights to the power
    // `SHARPNESS`), snow wins at odds `p : 1 - p` against every other land
    // ground: the share of squares under snow follows the world's snow
    // fraction instead of a fixed bonus (review 3.4-9).
    let p = lie(ctx, u, v, slope, north, shape).min(0.98);
    if p > 1e-6 {
        let others: f64 = (0..COUNT)
            .filter(|&i| ![Snow.index(), Ice.index(), Water.index()].contains(&i))
            .filter(|&i| w[i] > 1e-4)
            .map(|i| crate::math::exp(SHARPNESS * crate::math::ln(w[i])))
            .sum();
        if others > 0.0 {
            let snow = crate::math::exp(
                (crate::math::ln(others) + crate::math::ln(p / (1.0 - p))) / SHARPNESS,
            );
            w[Snow.index()] += snow;
            w[Ice.index()] += 0.1 * p * snow;
        }
    }
}

/// Perennial snow at a square (`0..=1`): the snow line of its latitude at
/// the cells' temperature, warmer on sunny slopes and ridges and colder in
/// hollows (`arda_core::biome::climate`, the corrections the Atlas paints
/// couloirs with), shed by steep faces.
fn lie(ctx: &Ctx, u: f64, v: f64, slope: f64, north: f64, shape: crate::shape::Shape) -> f64 {
    let temp = ctx.bilinear(u, v, |c, _| f64::from(c.temperature.raw()) / 100.0);
    let lines = ctx.lines_at(v);
    let sun = sunward(north, ctx.lat_deg(v)) * smoothstep(4.0, 24.0, slope);
    let t_eff = temp + f64::from(climate::SNOW_SUN_CENTI) / 100.0 * sun
        - f64::from(climate::SNOW_HOLLOW_CENTI) / 100.0 * shape.bowl()
        + f64::from(climate::SNOW_RIDGE_CENTI) / 100.0 * shape.ridge();
    lines.snow(t_eff, slope)
}

/// How much snow lies at a square (`0..=1`), the term the ground prior
/// adds to snow; scatter reads it so no flower stands on a drift.
#[must_use]
pub fn snow_lie(
    ctx: &Ctx,
    u: f64,
    v: f64,
    p: &crate::terrain::Phys,
    s: crate::shape::Shape,
) -> f64 {
    lie(ctx, u, v, p.slope_deg, p.north, s)
}

fn split(t: f64) -> (i64, f64) {
    let f = t.floor();
    #[allow(clippy::cast_possible_truncation)]
    let i = f as i64;
    (i, t - f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biome::Coast;
    use arda::{Cover, TerrainKind};
    use arda_core::{HeightMm, ShoreClass, TempCentiC};

    fn plain() -> Cell {
        Cell {
            terrain: TerrainKind::Land,
            cover: Cover::Grass,
            height: HeightMm::new(1),
            temperature: TempCentiC::new(1000),
            wetness: 40,
            height_above_river_dm: 60,
            ..Cell::default()
        }
    }

    #[test]
    fn a_world_without_a_shore_layer_gives_sea_level_land_the_old_beach() {
        let w = cell_weights(&plain(), Coast::default(), Lines::MID);
        assert!((w[Class::Sand.index()] - 0.45).abs() < 1e-9);
        assert!((w[Class::Gravel.index()] - 0.15).abs() < 1e-9);
    }

    #[test]
    fn beach_cliff_and_salt_marsh_take_their_own_ground() {
        let at = |class| {
            cell_weights(
                &plain(),
                Coast {
                    dist: 1,
                    shore: class,
                    layer: true,
                },
                Lines::MID,
            )
        };
        let sand = at(ShoreClass::SandBeach);
        assert!(sand[Class::Sand.index()] > sand[Class::Grass.index()]);
        let cliff = at(ShoreClass::Cliff);
        assert!(cliff[Class::Sand.index()] < 1e-9 && cliff[Class::Rock.index()] > 0.4);
        let marsh = at(ShoreClass::Marsh);
        assert!(marsh[Class::Mud.index()] > 0.4 && marsh[Class::Marsh.index()] > 0.4);
        let shingle = at(ShoreClass::ShingleBeach);
        assert!(shingle[Class::Gravel.index()] > shingle[Class::Sand.index()]);
    }
}
