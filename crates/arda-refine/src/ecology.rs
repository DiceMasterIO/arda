//! Ecology at one point: the world's climate and cover interpolated between
//! cells, corrected for the local slope, aspect and landform, and the
//! species that follow from it (goals 19, 20 and 43).
//!
//! - Effective temperature falls on shaded (north-facing) slopes and rises
//!   on sunny ones, so conifers climb lower on the shade side.
//! - Growth fades to zero at the tree line (the world's canopy limit runs
//!   from -4 °C to +5 °C), where dead and stunted trees take over.
//! - Wetness rises in hollows and by water, where willow and alder grow;
//!   knolls and ridges are dry, where pine and heather grow.
//!
//! Every input is a function of global position, so species agree across
//! block edges.

use crate::context::Ctx;
use crate::noise::smoothstep;
use crate::shape::Shape;
use crate::terrain::{Phys, Water};
use arda::Cover;

/// Ecology at one point.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Eco {
    /// Effective mean temperature, °C, after aspect.
    pub temp: f64,
    /// Tree growth, `0` above the tree line to `1` well below it.
    pub growth: f64,
    /// Conifer share of the stand, `0..=1`.
    pub conifer: f64,
    /// Ground wetness, `0..=1`.
    pub wet: f64,
    /// Distance to open water, squares (large when far).
    pub water_d: f64,
    /// Forest density of the cells, `0..=1`.
    pub fd: f64,
    /// Scrub-cover share.
    pub scrub: f64,
    /// Marsh-cover share.
    pub marsh: f64,
    /// Grass-cover share.
    pub grass: f64,
    /// Rock- and ice-cover share.
    pub rocky: f64,
    /// Soil moisture of the cells, `0..=1`.
    pub moist: f64,
    /// Sunniness: `1` on a steep south-facing slope, `-1` on a north one.
    pub sun: f64,
    /// Local landform.
    pub shape: Shape,
    /// Local slope, degrees.
    pub slope: f64,
    /// Whether the cells are settled or crossed by roads (`0..=1`).
    pub settled: f64,
    /// Snow lying on the ground here, `0..=1`: the cells' snow cover or a
    /// drift in cold shade (the ground prior's snow rule).
    pub snow: f64,
    /// Aridity of the cells ([`crate::biome::arid`]), `0..=1`.
    pub arid: f64,
    /// Height above the tree line ([`crate::biome::alpine`]), `0..=1`.
    pub alpine: f64,
    /// Beach: dry ground in the sea's shore band, `0..=1`.
    pub beach: f64,
    /// Dunes: dry ground just behind a sea beach, `0..=1`.
    pub dune: f64,
    /// How rocky the shore is (steep or rocky ground), `0..=1`.
    pub shore_rock: f64,
    /// Whether the point lies in the sea.
    pub in_sea: bool,
    /// How craggy the cells are ([`crate::biome::craggy`]), `0..=1`.
    pub crag: f64,
}

fn share(ctx: &Ctx, u: f64, v: f64, f: impl Fn(Cover) -> f64) -> f64 {
    ctx.bilinear(u, v, |c, _| f(c.cover))
}

/// The ecology at global square position `(u, v)` over physical square `p`
/// with landform `s`.
#[must_use]
pub fn eco(ctx: &Ctx, p: &Phys, s: Shape, u: f64, v: f64) -> Eco {
    let cell_temp = ctx.bilinear(u, v, |c, _| f64::from(c.temperature.raw()) / 100.0);
    let steep = smoothstep(4.0, 24.0, p.slope_deg);
    let sun = -p.north * steep;
    let temp = cell_temp + 1.8 * sun;
    let water_d = if p.water == Water::Dry {
        p.river_d.min(-p.stand_v * 30.0).max(0.0)
    } else {
        0.0
    };
    let near_water = 1.0 - smoothstep(0.5, 6.0, water_d);
    let cell_wet = ctx.bilinear(u, v, |c, _| f64::from(c.wetness) / 255.0);
    let moist = ctx.bilinear(u, v, |c, _| f64::from(c.moisture) / 255.0);
    let wet = (0.35 * cell_wet + 0.25 * moist + 0.45 * s.bowl() - 0.25 * s.ridge()
        + 0.55 * near_water)
        .clamp(0.0, 1.0);
    // The sea's shore band: the beach where the shore rule draws sand,
    // dunes just behind it.
    let (beach, dune) = if p.water == Water::Dry && p.stand_kind == Water::Sea {
        let sv = p.stand_v;
        (
            smoothstep(-0.42, -0.25, sv),
            smoothstep(-0.8, -0.55, sv) * (1.0 - smoothstep(-0.38, -0.25, sv)),
        )
    } else {
        (0.0, 0.0)
    };
    // Cold, shade and thin dry soils favour conifers; warmth favours
    // broadleaves.
    let conifer = (smoothstep(9.5, 3.5, temp) + 0.25 * s.ridge() - 0.15 * wet).clamp(0.0, 1.0);
    Eco {
        temp,
        growth: smoothstep(-1.5, 3.5, temp),
        conifer,
        wet,
        water_d,
        fd: ctx.bilinear(u, v, |c, _| f64::from(c.forest_density) / 255.0),
        scrub: share(ctx, u, v, |c| f64::from(u8::from(c == Cover::Scrub))),
        marsh: ctx.bilinear(u, v, |c, _| crate::biome::marshy(c)),
        grass: share(ctx, u, v, |c| f64::from(u8::from(c == Cover::Grass))),
        rocky: crate::fixed::rocky(ctx, u, v),
        moist,
        sun,
        shape: s,
        slope: p.slope_deg,
        settled: ctx.bilinear(u, v, |c, _| {
            f64::from(u8::from(c.built_by != 0 || c.road != arda::RoadClass::None))
        }),
        snow: ctx
            .bilinear(u, v, |c, _| crate::biome::snowy(c))
            .max(smoothstep(
                0.3,
                0.8,
                crate::prior::snow_lie(ctx, u, v, p, s),
            )),
        arid: ctx.bilinear(u, v, |c, _| crate::biome::arid(c)),
        alpine: ctx.bilinear(u, v, |c, _| crate::biome::alpine(c)),
        beach,
        dune,
        shore_rock: smoothstep(6.0, 18.0, p.slope_deg).max(crate::fixed::rocky(ctx, u, v)),
        in_sea: p.water == Water::Sea,
        crag: ctx.bilinear(u, v, |c, _| crate::biome::craggy(c)),
    }
}

/// A tree species: vocabulary id and semantic tag.
pub type Species = (&'static str, &'static str);

fn sized(large: bool, big: &'static str, small: &'static str) -> &'static str {
    if large {
        big
    } else {
        small
    }
}

/// The tree for a stand with ecology `e`; `r` and `q` are independent
/// uniform draws in `[0, 1)`.
#[must_use]
pub fn tree(e: &Eco, large: bool, r: f64, q: f64) -> Species {
    let limit = 1.0 - e.growth;
    // Near the tree line trees die back and stunt (krummholz).
    let dead = 0.015 + 0.12 * limit;
    if r < dead {
        return ("veg.tree_dead", "tree:dead");
    }
    if r < dead + 0.85 * limit * limit + 0.3 * limit * f64::from(u8::from(!large)) {
        return ("veg.tree_stunted", "tree:conifer:stunted");
    }
    // Wet banks and hollows: willow in the warm lowlands, alder elsewhere.
    let riparian = 0.8 * (1.0 - smoothstep(0.8, 4.5, e.water_d)) + 0.4 * (e.wet - 0.7).max(0.0);
    if q < riparian {
        return if e.temp > 8.5 && (q * 7.0).fract() < 0.55 {
            (
                "veg.tree_willow",
                sized(large, "tree:broadleaf:large", "tree:broadleaf:small"),
            )
        } else {
            (
                "veg.tree_alder",
                sized(large, "tree:broadleaf:large", "tree:broadleaf:small"),
            )
        };
    }
    let t = (q - riparian) / (1.0 - riparian).max(1e-9);
    if t < e.conifer {
        // Spruce on moist and shaded ground, pine on dry, sunny knolls.
        let spruce =
            (0.5 + 0.6 * (e.wet - 0.4) - 0.5 * e.sun - 0.4 * e.shape.ridge()).clamp(0.1, 0.9);
        let id = if (t / e.conifer.max(1e-9)) < spruce {
            "veg.tree_spruce"
        } else {
            "veg.tree_pine"
        };
        return (id, sized(large, "tree:conifer:large", "tree:conifer:small"));
    }
    let b = (t - e.conifer) / (1.0 - e.conifer).max(1e-9);
    if !large || b > 0.82 - 0.25 * smoothstep(9.0, 5.0, e.temp) {
        // Birch: the pioneer of edges, clearings and cool ground.
        ("veg.tree_birch", "tree:broadleaf:small")
    } else if b < 0.5 + 0.25 * e.sun - 0.3 * (e.wet - 0.4) {
        // Oak on warm, well-drained ground.
        ("veg.tree_oak", "tree:broadleaf:large")
    } else {
        // Elm on moist, rich lowland soil.
        ("veg.tree_elm", "tree:broadleaf:large")
    }
}

/// The low plant at a point: ferns under trees and in shade, heather on
/// cold, dry, acid knolls, tall grass in damp swales and by water, flowers
/// on sunny meadow, mushroom rings now and then.
#[must_use]
pub fn low_plant(e: &Eco, canopy: f64, r: f64) -> Species {
    let shade = canopy.max(0.6 * (-e.sun).max(0.0) * e.moist);
    if r < 0.025 + 0.03 * canopy {
        return ("veg.mushroom_ring", "low:mushrooms");
    }
    let fern = shade * (0.4 + 0.6 * e.moist) * (1.0 - e.arid);
    let heather = (e.scrub + 0.6 * e.shape.ridge() + 0.5 * smoothstep(7.0, 2.0, e.temp))
        * (1.0 - e.wet)
        * (1.0 - canopy)
        * (1.0 - 0.7 * e.arid);
    let tall = (0.3 + e.wet + 0.4 * e.shape.bowl()) * (1.0 - canopy);
    let flowers =
        (0.5 + 0.6 * e.sun.max(0.0) + 0.4 * e.grass) * (1.0 - canopy) * (1.0 - e.wet * 0.5);
    let total = fern + heather + tall + flowers;
    let x = (r - 0.025) / 0.975 * total;
    let pick = if x < fern {
        ("veg.fern", "low:fern")
    } else if x < fern + heather {
        ("veg.heather", "low:heather")
    } else if x < fern + heather + tall {
        ("veg.tall_grass", "low:tall_grass")
    } else {
        ("veg.flower_patch", "low:flowers")
    };
    biome_low_plant(e, pick, r)
}

/// The biome's own low plants in place of the temperate pick (logic of
/// [`crate::biome`]): dune grass behind beaches, sedge and marsh flowers on
/// saturated ground, dry grass and tussocks on steppe, alpine flowers and
/// cushion plants above the tree line. `r` is the same draw as the pick.
fn biome_low_plant(e: &Eco, pick: Species, r: f64) -> Species {
    let q = (r * 13.0).fract();
    if e.dune > 0.3 {
        return ("veg.dune_grass", "low:dune_grass");
    }
    if e.marsh > 0.5 && pick.1 != "low:fern" {
        return if q < 0.3 {
            ("veg.marsh_flowers", "low:marsh_flowers")
        } else if q < 0.75 {
            ("veg.sedge", "low:sedge")
        } else {
            ("veg.tall_grass", "low:tall_grass")
        };
    }
    if e.alpine > 0.5 {
        return if q < 0.55 {
            ("veg.alpine_flowers", "low:alpine_flowers")
        } else {
            ("veg.tussock", "low:tussock")
        };
    }
    if q < e.arid && matches!(pick.1, "low:flowers" | "low:tall_grass" | "low:heather") {
        return if (q * 7.0).fract() < 0.6 {
            ("veg.dry_grass", "low:dry_grass")
        } else {
            ("veg.tussock", "low:tussock")
        };
    }
    pick
}

/// The shrub at a point: juniper on cold open ground, flowering bushes on
/// warm sunny edges, plain bushes elsewhere, ferns under a closed canopy.
#[must_use]
pub fn shrub(e: &Eco, r: f64) -> Species {
    if e.fd > 0.55 && r < 0.4 {
        return ("veg.fern", "undergrowth:fern");
    }
    // At the tree line shrubs are wind-cut krummholz; on steppe, sagebrush.
    if e.growth < 0.35 {
        return ("veg.krummholz", "undergrowth:krummholz");
    }
    if r < 0.8 * e.arid {
        return ("veg.sagebrush", "undergrowth:sagebrush");
    }
    let juniper = smoothstep(8.0, 2.0, e.temp) * (1.0 - e.wet) * (0.6 + 0.4 * e.shape.ridge());
    if r < 0.45 * juniper + 0.4 * (1.0 - e.growth) {
        ("veg.juniper", "undergrowth:juniper")
    } else if r > 0.82 - 0.12 * e.sun.max(0.0) {
        ("veg.bush_flowering", "undergrowth:bush")
    } else {
        ("veg.bush", "undergrowth:bush")
    }
}

#[cfg(test)]
mod tests;
