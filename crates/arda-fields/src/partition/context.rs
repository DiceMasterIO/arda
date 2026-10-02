//! What a region of the hierarchy knows about its ground: its land use,
//! the nearest settlement, slope, rivers and enclosure style, and from
//! these the size its fields should be and, at a leaf, their use.
//!
//! Field sizes follow distance from the settlement (logic/17
//! §land-fields): small closes and paddocks by the houses, ploughland and
//! pasture fields growing to a few hectares at the edge of the township,
//! furlongs of open-field strips around villages that keep them, large
//! woods and wastes. Wealth makes fields larger, some cultures smaller,
//! planned enclosure larger and more regular.

use super::guide::river_distance;
use super::lattice::{ancient, vnoise};
use super::poly::{bbox, region_contains, P};
use crate::fields::{kind_of, open_field_radius, Crop, FieldKind};
use crate::geom::{h2, u01, CELL_SQUARES, SQUARE_M};
use crate::input::{LandUse, LandUseMap, RiverLine, Settlement, Terrain, Tier};

/// Squares per hectare.
pub const SQ_PER_HA: f64 = 10_000.0 / (SQUARE_M * SQUARE_M);
/// Settlements further than this from a region's centre do not shape it,
/// metres.
pub const INFLUENCE_M: f64 = 1_500.0;
/// Floodplain meadow reaches this far from a river bank, metres.
const MEADOW_M: f64 = 70.0;
/// Slope (rise over run) below which riverside ground floods.
const FLOOD_SLOPE: f64 = 0.05;
/// Slope above which grazing is left as open common.
const COMMON_SLOPE: f64 = 0.14;

/// Everything the partition reads about the world.
pub struct Ground<'a> {
    /// Seed.
    pub seed: u64,
    /// Land use per 100 m cell.
    pub landuse: &'a dyn LandUseMap,
    /// Terrain (only heights are read).
    pub terrain: &'a dyn Terrain,
    /// Settlements within reach (see [`super::INPUT_REACH_M`]).
    pub settlements: &'a [Settlement],
    /// River channels within reach.
    pub rivers: &'a [RiverLine],
    /// Enclosure style.
    pub style: Style,
}

/// The farming culture's enclosure habits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// Field size multiplier.
    pub size: f64,
    /// Bias towards old, irregular enclosure (`> 0`) or planned (`< 0`).
    pub bias: f64,
}

impl Style {
    /// The style of a culture at a wealth (0–255).
    #[must_use]
    pub fn of(culture: &str, wealth: u8) -> Self {
        let (size, bias) = match culture.to_ascii_lowercase().as_str() {
            "dwarf" | "dwarven" => (0.75, -0.25),
            "elf" | "elven" => (0.85, 0.35),
            "halfling" => (0.6, 0.3),
            "highland" => (1.1, 0.1),
            _ => (1.0, 0.0),
        };
        Self {
            size: size * (0.8 + 0.4 * f64::from(wealth) / 255.0),
            bias,
        }
    }
}

/// A region's view of its ground.
#[derive(Debug, Clone)]
pub struct Context {
    /// Land-use samples: position and class.
    pub samples: Vec<(P, Option<LandUse>)>,
    /// Most common class (the centre's on a tie).
    pub class: Option<LandUse>,
    /// Share of samples of the most common broad cover (farmed, wood or
    /// waste).
    pub purity: f64,
    /// Metres from the nearest settlement's built edge (capped at
    /// [`INFLUENCE_M`]).
    pub settle_m: f64,
    /// Whether the centre lies in a village's open fields.
    pub open_field: bool,
    /// Unit downhill-to-uphill direction and slope (rise over run).
    pub grad: P,
    /// Slope.
    pub slope: f64,
    /// Metres to the nearest river bank.
    pub river_m: f64,
    /// Old (1) or planned (0) enclosure.
    pub ancient: f64,
}

/// People per hectare of a built-up core (as the plan reserves them).
fn core_radius_m(s: &Settlement) -> f64 {
    let density = match s.tier {
        Tier::Hamlet | Tier::Village => 60.0,
        Tier::Town | Tier::City => 150.0,
    };
    (f64::from(s.population) / density * 10_000.0 / std::f64::consts::PI).sqrt()
}

/// Settlements within [`INFLUENCE_M`] of a point (square units).
#[must_use]
pub fn near(settlements: &[Settlement], p: P) -> Vec<Settlement> {
    let m = [p[0] * SQUARE_M, p[1] * SQUARE_M];
    settlements
        .iter()
        .filter(|s| (s.x_m - m[0]).hypot(s.y_m - m[1]) <= INFLUENCE_M)
        .cloned()
        .collect()
}

#[allow(clippy::cast_possible_truncation)] // floors of map positions
fn class_at(g: &Ground<'_>, p: P) -> Option<LandUse> {
    // The raster is read through a smooth displacement (up to 70 m, at
    // 280 and 90 m), so the outline of farmland against wood and waste
    // wanders with the field edges instead of stepping along 100 m cells.
    let n = |salt: u64| {
        36.0 * vnoise(g.seed, salt, p, 180.0) + 10.0 * vnoise(g.seed, salt ^ 0x77, p, 58.0)
    };
    let q = [p[0] + n(0x1A5D), p[1] + n(0x1A5E)];
    let cell = |v: f64| (v.floor() as i64).div_euclid(CELL_SQUARES);
    g.landuse.class_at(cell(q[0]), cell(q[1]))
}

impl Context {
    /// The context of a region with centroid `c` and area `area`.
    #[must_use]
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    pub fn new(g: &Ground<'_>, region: &[Vec<P>], c: P, area: f64) -> Self {
        // About 150 land-use samples, never finer than 8 squares.
        let step = (area.sqrt() / 12.0).clamp(8.0, 64.0);
        let b = bbox(region);
        let mut samples = vec![(c, class_at(g, c))];
        let (nx, ny) = (
            ((b[2] - b[0]) / step).ceil() as i64,
            ((b[3] - b[1]) / step).ceil() as i64,
        );
        for j in 0..ny {
            for i in 0..nx {
                let p = [
                    b[0] + (i as f64 + 0.5) * step,
                    b[1] + (j as f64 + 0.5) * step,
                ];
                if region_contains(region, p) {
                    samples.push((p, class_at(g, p)));
                }
            }
        }
        let class = majority(&samples, |c| c).0.flatten();
        let (_, purity) = majority(&samples, cover);
        let m = [c[0] * SQUARE_M, c[1] * SQUARE_M];
        let mut settle_m = INFLUENCE_M;
        let mut open_field = false;
        for s in g.settlements {
            let d = (s.x_m - m[0]).hypot(s.y_m - m[1]);
            if d > INFLUENCE_M {
                continue;
            }
            settle_m = settle_m.min((d - core_radius_m(s)).max(0.0));
            let r = open_field_radius(s) * SQUARE_M;
            open_field |= s.tier >= Tier::Village && d <= r;
        }
        let gr = super::gradient(g.terrain, m);
        let slope = gr[0].hypot(gr[1]);
        let river_m = river_distance(g.rivers, c).map_or(f64::INFINITY, |r| r.0);
        Self {
            samples,
            class,
            purity,
            settle_m,
            open_field,
            grad: super::poly::unit(gr),
            slope,
            river_m,
            ancient: 0.0,
        }
    }

    /// The field size this region aims at, squares.
    #[must_use]
    pub fn target_sq(&self, style: Style) -> f64 {
        // Enclosed fields: closes by the houses, larger fields further out.
        let t = ((self.settle_m - 80.0) / 1_300.0).clamp(0.0, 1.0);
        let enclosure = 0.4 + 3.0 * t * t * (3.0 - 2.0 * t);
        let ha = match self.class {
            Some(LandUse::Arable | LandUse::Fallow) if self.open_field => 6.0,
            Some(LandUse::Arable | LandUse::Fallow) => enclosure,
            Some(LandUse::Pasture) => enclosure * 1.25,
            Some(LandUse::Meadow | LandUse::Mill) => 1.4,
            Some(LandUse::Orchard) => 0.7,
            Some(LandUse::Farmstead) => 0.45,
            Some(LandUse::Woodland) => 7.0,
            Some(LandUse::MineQuarry) | None => 9.0,
        };
        let planned = 1.0 + 0.25 * (1.0 - self.ancient);
        ha * style.size * planned * SQ_PER_HA
    }
}

/// The broad cover of a class: farmed land (`0`, whatever its use this
/// year), woodland (`1`) or waste (`2`). Fields of one hedged block change
/// use between ploughland and grass; woods and wastes keep their own
/// outlines.
fn cover(c: Option<LandUse>) -> u8 {
    match c {
        Some(LandUse::Woodland) => 1,
        None | Some(LandUse::MineQuarry) => 2,
        Some(_) => 0,
    }
}

/// The most common value of `key` over samples and its share (ties to the
/// earliest sample's, the centre).
fn majority<K: Copy + PartialEq>(
    samples: &[(P, Option<LandUse>)],
    key: impl Fn(Option<LandUse>) -> K,
) -> (Option<K>, f64) {
    let mut counts: Vec<(K, usize)> = Vec::new();
    for (_, c) in samples {
        let k = key(*c);
        match counts.iter_mut().find(|(x, _)| *x == k) {
            Some(e) => e.1 += 1,
            None => counts.push((k, 1)),
        }
    }
    let best = counts.iter().fold(
        (None, 0),
        |acc, &(k, n)| if n > acc.1 { (Some(k), n) } else { acc },
    );
    #[allow(clippy::cast_precision_loss)] // sample counts
    let share = best.1 as f64 / samples.len().max(1) as f64;
    (best.0, share)
}

/// How well a candidate split separates broad cover: the samples agreeing
/// with their side's majority.
#[must_use]
pub fn separation(
    samples: &[(P, Option<LandUse>)],
    side: impl Fn(P) -> usize,
) -> (usize, [usize; 2]) {
    let mut sides: [Vec<(P, Option<LandUse>)>; 2] = [Vec::new(), Vec::new()];
    for s in samples {
        sides[side(s.0) & 1].push(*s);
    }
    let agree = |v: &[(P, Option<LandUse>)]| {
        let (_, share) = majority(v, cover);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let n = (share * v.len() as f64).round() as usize;
        n
    };
    (
        agree(&sides[0]) + agree(&sides[1]),
        [sides[0].len(), sides[1].len()],
    )
}

/// The use and crop of a leaf: [`kind_of`] on its majority class, with
/// floodplain meadow by rivers and open commons on poor grazing.
#[must_use]
pub fn leaf_use(
    ctx: &Context,
    c: P,
    g: &Ground<'_>,
    seed: u64,
    key: (i64, i64),
) -> (FieldKind, Crop) {
    let mut class = ctx.class;
    let farmed = matches!(
        class,
        Some(LandUse::Arable | LandUse::Fallow | LandUse::Pasture)
    );
    if farmed && ctx.river_m <= MEADOW_M && ctx.slope < FLOOD_SLOPE {
        class = Some(LandUse::Meadow);
    }
    let near = near(g.settlements, c);
    let (kind, crop) = kind_of(class, c, &near, seed, key);
    let u = u01(h2(seed, 0xC0AA, key.0, key.1));
    let poor = ctx.slope > COMMON_SLOPE || (ctx.settle_m >= 900.0 && u < 0.3);
    if kind == FieldKind::Pasture && poor {
        return (FieldKind::Common, Crop::Grazed);
    }
    (kind, crop)
}

/// The style weight at a point.
#[must_use]
pub fn ancient_at(seed: u64, c: P, style: Style) -> f64 {
    ancient(seed, c, style.bias)
}
