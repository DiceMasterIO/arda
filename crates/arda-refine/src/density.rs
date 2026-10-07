//! How dense each scatter layer is at a point, and which asset a kept point
//! becomes, from the point's ecology (goals 19 and 43).
//!
//! Open country gets copses (clumps of trees from a slow noise field),
//! tree lines along streams, rock outcrops on steep knolls and boulder
//! fields at the foot of crags; forests get groves and glades. Every field
//! here is a pure function of global position.

use crate::context::Ctx;
use crate::ecology::{low_plant, shrub, tree, Eco, Species};
use crate::noise::{fbm, smoothstep};
use crate::scatter::Kind;
use crate::terrain::{Phys, Water};

/// Woodland structure at a point.
#[derive(Debug, Clone, Copy)]
pub struct Woods {
    /// Expected canopy share of large trees, `0..=1`.
    pub canopy: f64,
    /// Margin of a wood, grove or copse, `0..=1`: young trees, undergrowth.
    pub edge: f64,
    /// A slow clumping field in `0..=1` for shrubs and low plants.
    pub drift: f64,
    /// A clumping field in `0..=1` for loose rocks.
    pub stony: f64,
}

/// Spread of the grove noise (its standard deviation, about).
const GROVE_SD: f64 = 0.23;

/// The grove-noise level exceeded over a share `fd` of the ground: a
/// logistic approximation of the normal quantile.
fn quantile(fd: f64) -> f64 {
    let p = fd.clamp(0.002, 0.998);
    -GROVE_SD * crate::math::ln(p / (1.0 - p)) / 1.7
}

/// The woodland structure at `(x, y)`. Trees grow in groves rather than
/// evenly: a slow noise field is cut at the level that leaves a share of
/// the ground equal to the forest density wooded, so a sparse cell has a
/// few copses and a dense one a few glades, and the expected canopy stays
/// proportional to density. Grassland gets a few copses of its own, and
/// streams and lake edges a broken line of trees.
#[must_use]
pub fn woods(ctx: &Ctx, e: &Eco, x: f64, y: f64) -> Woods {
    let open = (1.0 - e.rocky) * (1.0 - 0.7 * e.marsh);
    // Steppe keeps its copses few: dry country grows trees only by water.
    // A stand closes only to about four fifths (gaps between crowns), so
    // the wooded share runs a little over the density to give canopy cover
    // close to it.
    let target =
        ((e.fd * 1.15).min(0.995) + 0.07 * e.grass * (1.0 - e.fd)) * open * (1.0 - 0.8 * e.arid);
    let n = fbm(ctx.seed, 0xC1, x, y, 26.0, 2, 0.5);
    let q = quantile(target);
    let grove = smoothstep(-0.03, 0.03, n - q);
    let rim = smoothstep(-0.07, -0.03, n - q) * (1.0 - smoothstep(0.0, 0.04, n - q));
    // Tree lines along streams and lake edges, broken into stretches.
    let gap = smoothstep(-0.25, 0.05, fbm(ctx.seed, 0xC3, x, y, 14.0, 2, 0.5));
    let line = 0.95 * (1.0 - smoothstep(1.5, 4.5, e.water_d)) * gap * (1.0 - e.marsh);
    let grow = if e.growth < 0.04 {
        0.0
    } else {
        0.25 + 0.75 * e.growth
    };
    let drift = smoothstep(-0.08, 0.22, fbm(ctx.seed, 0xC5, x, y, 11.0, 2, 0.5));
    Woods {
        canopy: grove.max(line) * grow,
        edge: rim.max(0.5 * line) * grow.max(0.3),
        drift,
        stony: smoothstep(-0.12, 0.2, fbm(ctx.seed, 0xC6, x, y, 9.0, 2, 0.5)),
    }
}

/// Firm ground a trunk or log may stand on.
fn firm(p: &Phys) -> bool {
    p.water == Water::Dry && p.slope_deg < 38.0 && p.river_d > 0.6 && p.stand_v < -0.34
}

/// Density of a layer at a point, `0..=1`.
#[must_use]
pub fn density(kind: Kind, e: &Eco, w: &Woods, p: &Phys) -> f64 {
    let dry = p.water == Water::Dry;
    let s = &e.shape;
    // Nothing grows through lying snow; above the tree line plants thin
    // out to sparse cushions and only krummholz holds on near its edge.
    let bare = 1.0 - smoothstep(0.3, 0.65, e.snow);
    let alive = smoothstep(0.0, 0.2, e.growth);
    match kind {
        // The sea shore (logic/09 §shores): driftwood and wrack on the
        // beach, dune grass behind it, tide pools on rocky strands, sea
        // stacks and rocks standing in the shallows.
        Kind::Log if dry && e.beach > 0.2 => 0.1 * e.beach,
        Kind::Low if dry && e.dune > 0.2 => 0.55 * e.dune * bare,
        Kind::Rock if dry && e.beach > 0.3 => 0.04 + 0.3 * e.shore_rock * e.beach,
        Kind::Boulder if p.water == Water::Sea && p.depth_m < 2.5 => 0.04 + 0.35 * e.shore_rock,
        Kind::TreeLarge if firm(p) => w.canopy,
        Kind::TreeSmall if firm(p) => {
            (0.5 * w.canopy + 0.4 * w.edge + 0.1 * e.scrub * w.drift).min(1.0)
        }
        Kind::Log if firm(p) => 0.2 * w.canopy * e.growth.max(0.3),
        Kind::Undergrowth if firm(p) => {
            let juniper = 0.3 * smoothstep(7.0, 2.0, e.temp) * (1.0 - e.rocky) * alive;
            let steppe = 0.15 * e.arid;
            let open = (0.4 * e.scrub + juniper + steppe) * w.drift;
            ((0.12 * w.canopy + 0.55 * w.edge + open + 0.25 * s.talus * alive).min(1.0)) * bare
        }
        Kind::Outcrop if dry && p.river_d > 1.5 => {
            let knoll = smoothstep(0.25, 0.7, s.ridge()) * smoothstep(10.0, 28.0, p.slope_deg);
            // Crags: outcrops break out of steep mountainsides even off
            // the knolls.
            let crag = smoothstep(24.0, 40.0, p.slope_deg) * (0.35 + 0.65 * s.ridge());
            ((0.45 + e.rocky) * knoll + 0.08 * e.rocky + 0.3 * crag + 0.06 * e.alpine).min(0.9)
        }
        Kind::Boulder | Kind::Rock if dry && p.river_d > 0.3 => {
            (0.35 + 0.9 * w.stony).min(1.0)
                * (0.02
                    + 0.35 * e.rocky
                    + 0.45 * smoothstep(20.0, 42.0, p.slope_deg)
                    + 0.9 * s.talus
                    + 0.2 * s.ridge() * smoothstep(8.0, 25.0, p.slope_deg)
                    + 0.25 * e.alpine)
                    .min(0.92)
        }
        Kind::Reeds => {
            let shore = if dry {
                1.0 - smoothstep(0.3, 2.0, e.water_d)
            } else if p.depth_m < 0.6 {
                0.8
            } else {
                0.0
            };
            let calm = if p.water == Water::Sea || p.stand_kind == Water::Sea {
                0.1
            } else {
                1.0
            };
            (shore * calm * (0.35 + 0.6 * e.marsh) + 0.5 * e.marsh * f64::from(u8::from(dry)))
                .min(0.95)
                * (1.0 - smoothstep(4.0, 1.0, e.temp + 3.0))
        }
        Kind::Lilies => match p.water {
            Water::Lake | Water::Pool if p.depth_m < 1.8 => 0.35,
            _ => 0.0,
        },
        Kind::Low if firm(p) => {
            // Drifts: flowers and tall grass gather in patches, ferns under
            // trees and along wood edges.
            let open = (0.05 + 0.5 * e.grass + 0.3 * e.scrub) * w.drift * (1.0 - 0.6 * w.canopy);
            let wood = 0.3 * w.canopy + 0.4 * w.edge;
            let base = (open + wood + 0.15 * s.bowl()) * (1.0 - 0.8 * e.rocky);
            // Marsh sedges stand thick; alpine cushions are sparse.
            let marsh = 0.35 * e.marsh * w.drift;
            ((base + marsh) * (1.0 - 0.65 * e.alpine) * (1.0 - 0.6 * e.crag) * bare).min(0.7)
        }
        // Crevices and ledges: grass tufts and shrubs in the hollows of
        // rocky ground too steep for anything else.
        Kind::Undergrowth | Kind::Low if dry && p.slope_deg < 48.0 && p.river_d > 0.6 => {
            0.5 * e.rocky * s.bowl() * e.growth * bare
        }
        _ => 0.0,
    }
}

/// The asset of a kept point; `r` and `q` are independent uniform draws.
/// Returns the kind the item really is (a boulder layer point may become a
/// small rock) with its asset and tag.
#[must_use]
pub fn choose(kind: Kind, e: &Eco, w: &Woods, r: f64, q: f64) -> (Kind, Species) {
    match kind {
        Kind::TreeLarge | Kind::TreeSmall => {
            let t = tree(e, kind == Kind::TreeLarge, r, q);
            // A stunted or dead tree is small whichever layer drew it.
            let k = if t.0 == "veg.tree_stunted" {
                Kind::TreeSmall
            } else {
                kind
            };
            (k, t)
        }
        Kind::Undergrowth => (kind, shrub(e, r)),
        Kind::Log if e.beach > 0.2 => (kind, ("veg.driftwood", "deadwood:driftwood")),
        Kind::Rock if e.beach > 0.3 => (Kind::Low, ("veg.tide_pool", "shore:tide_pool")),
        Kind::Boulder if e.in_sea => (Kind::Boulder, ("veg.sea_rock", "rock:sea")),
        Kind::Outcrop => {
            if r < 0.65 {
                (Kind::Outcrop, ("veg.rock_outcrop", "rock:outcrop"))
            } else {
                (Kind::Boulder, ("veg.rock_large", "rock:large"))
            }
        }
        Kind::Boulder | Kind::Rock => {
            let s = &e.shape;
            if e.snow > 0.5 && r < 0.5 {
                (Kind::Boulder, ("veg.rock_snow", "rock:snow"))
            } else if e.alpine > 0.4 && r > 1.0 - 0.45 * e.alpine {
                (Kind::Rock, ("veg.lichen_rock", "rock:lichen"))
            } else if s.talus > 0.35 {
                // A boulder field under a crag: big blocks among smaller.
                if r < 0.35 {
                    (Kind::Boulder, ("veg.boulder", "rock:boulder"))
                } else if r < 0.7 {
                    (Kind::Rock, ("veg.rock_small", "rock:small"))
                } else {
                    (Kind::Rock, ("veg.stones", "rock:stones"))
                }
            } else if r < 0.18 {
                (Kind::Boulder, ("veg.boulder", "rock:boulder"))
            } else if r < 0.36 {
                (Kind::Boulder, ("veg.rock_large", "rock:large"))
            } else if e.slope > 26.0 && r < 0.62 {
                (Kind::Rock, ("veg.scree_patch", "rock:scree"))
            } else if r < 0.8 {
                (Kind::Rock, ("veg.rock_small", "rock:small"))
            } else {
                (Kind::Rock, ("veg.stones", "rock:stones"))
            }
        }
        Kind::Log => {
            // Near settlements logs are cut: more stumps.
            if r < 0.65 - 0.35 * e.settled {
                (kind, ("veg.fallen_log", "deadwood:log"))
            } else {
                (kind, ("veg.stump", "deadwood:stump"))
            }
        }
        Kind::Reeds => {
            if r < 0.35 {
                (kind, ("veg.cattail", "water_plant:cattail"))
            } else {
                (kind, ("veg.reeds", "water_plant:reeds"))
            }
        }
        Kind::Lilies => (kind, ("veg.lily_pads", "water_plant:lily")),
        Kind::Low => (
            kind,
            low_plant(e, w.canopy.max(w.edge), 0.4 * r + 0.6 * w.drift),
        ),
    }
}
