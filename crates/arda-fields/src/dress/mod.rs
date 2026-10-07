//! Ground and dressing: crops, furrows, headlands, woodland fringes, and
//! the props and vegetation of each land use.

mod props;

use crate::boundary::Boundaries;
use crate::compounds::GPlacement;
use crate::fields::{Crop, FieldKind};
use crate::geom::{Grid, Sq};
use crate::plan::{Cover, Plan};
use crate::strips;
use arda_tactical::noise::fbm;

/// Squares beyond the window whose zone distances are computed.
pub(crate) const PAD: i64 = 18;
/// Farthest zone distance tracked.
const MAX_DIST: u8 = 5;

/// Dressing of one window.
#[derive(Debug, Clone)]
pub struct Dressed {
    /// Ground key per window square.
    pub ground: Grid<&'static str>,
    /// Crop per window square, where farmed.
    pub crop: Grid<Option<Crop>>,
    /// Furrow direction per window square, where ploughed.
    pub furrow: Grid<Option<[f32; 2]>>,
    /// Props and vegetation with anchors in the window, global squares.
    pub placements: Vec<GPlacement>,
}

/// A zone for distance-to-edge purposes: woodland counts as one zone.
fn zone(plan: &Plan, s: Sq) -> u64 {
    match plan.at(s) {
        Cover::Field(i) => match plan.fields.get(i as usize).map(|f| f.kind) {
            Some(FieldKind::Woodland) => 1 << 60,
            _ => 1000 + u64::from(i),
        },
        Cover::Wild => 1,
        Cover::Rough => 2,
        Cover::Apron => 7,
        Cover::Water => 3,
        Cover::Road(_) | Cover::Lane => 4,
        Cover::Built => 5,
        Cover::Compound(c) => (1 << 61) + u64::from(c),
    }
}

/// Chebyshev distance (1 = touching) to the nearest square of another
/// zone, capped at [`MAX_DIST`], over the window plus [`PAD`].
#[must_use]
pub fn zone_distance(plan: &Plan) -> Grid<u8> {
    let (x0, y0, w, h) = plan.win;
    let mut d = Grid::new(x0 - PAD, y0 - PAD, w + 2 * PAD, h + 2 * PAD, MAX_DIST);
    let r = i64::from(MAX_DIST) - 1;
    for s in d.squares().collect::<Vec<_>>() {
        let z = zone(plan, s);
        let mut best = MAX_DIST;
        for dy in -r..=r {
            for dx in -r..=r {
                let k = u8::try_from(dx.abs().max(dy.abs())).unwrap_or(MAX_DIST);
                if k == 0 || k >= best {
                    continue;
                }
                if zone(plan, s.offset(dx, dy)) != z {
                    best = k;
                }
            }
        }
        d.set(s, best);
    }
    d
}

/// Smooth patch noise in `[0, 1)` at a square, for ground variety.
#[must_use]
#[allow(clippy::cast_precision_loss)] // noise input precision
pub fn patch(seed: u64, salt: u64, s: Sq, scale: f32) -> f64 {
    f64::from(fbm(
        seed ^ salt,
        s.x as f32 * scale,
        s.y as f32 * scale,
        3,
        None,
    ))
}

/// Dresses the window of a plan.
#[must_use]
pub fn dress(plan: &Plan, bounds: &Boundaries) -> Dressed {
    let (x0, y0, w, h) = plan.win;
    let dist = zone_distance(plan);
    let mut ground = Grid::new(x0, y0, w, h, "grass");
    let mut crop = Grid::new(x0, y0, w, h, None);
    let mut furrow = Grid::new(x0, y0, w, h, None);
    for s in ground.squares().collect::<Vec<_>>() {
        let d = dist.get(s).copied().unwrap_or(MAX_DIST);
        let (g, c, f) = square_ground(plan, s, d);
        ground.set(s, g);
        crop.set(s, c);
        #[allow(clippy::cast_possible_truncation)] // unit vector components
        furrow.set(s, f.map(|v| [v[0] as f32, v[1] as f32]));
    }
    let placements = props::place(plan, bounds, &dist);
    Dressed {
        ground,
        crop,
        furrow,
        placements,
    }
}

type GroundOut = (&'static str, Option<Crop>, Option<[f64; 2]>);

fn square_ground(plan: &Plan, s: Sq, d: u8) -> GroundOut {
    let seed = plan.seed;
    match plan.at(s) {
        Cover::Water => ("mud", None, None),
        Cover::Road(c) => (c.ground(), None, None),
        Cover::Lane => ("dirt", None, None),
        Cover::Compound(c) => {
            let g = plan
                .compounds
                .get(c as usize)
                .and_then(|c| c.squares.get(&s))
                .map_or("dirt", |cs| cs.ground);
            (g, None, None)
        }
        Cover::Rough => ("scrub", None, None),
        Cover::Built => ("grass", None, None),
        Cover::Wild | Cover::Apron => wild_ground(seed, s),
        Cover::Field(i) => {
            let Some(f) = plan.fields.get(i as usize) else {
                return ("grass", None, None);
            };
            let site = &plan.partition.sites[f.site];
            match f.kind {
                FieldKind::Strips => {
                    if d <= 1 {
                        return ("grass", None, None);
                    }
                    // Balks stay unploughed but are not painted as grass: a
                    // one-square line of grass between strips breaks into
                    // ragged blobs once ground borders blend; each strip's
                    // own crop marks it off from its neighbours.
                    let at = strips::locate(seed, site, s.centre());
                    let c = strips::crop(seed, site, f.crop, at.index);
                    let fur = (c == Crop::Ploughed).then_some(at.along);
                    (c.ground(), Some(c), fur)
                }
                FieldKind::Arable => {
                    if d <= 1 {
                        return ("grass", None, None);
                    }
                    let fur = (f.crop == Crop::Ploughed).then_some(site.along);
                    (f.crop.ground(), Some(f.crop), fur)
                }
                FieldKind::Pasture => {
                    let g = if patch(seed, 0x9A57, s, 0.09) > 0.68 {
                        "meadow"
                    } else {
                        "pasture"
                    };
                    (g, Some(Crop::Grazed), None)
                }
                FieldKind::Common => {
                    // Open grazing on poor ground: tussocks and scrub.
                    let n = patch(seed, 0xC0A1, s, 0.1);
                    let g = if n > 0.74 {
                        "scrub"
                    } else if n > 0.56 {
                        "meadow"
                    } else {
                        "pasture"
                    };
                    (g, Some(Crop::Grazed), None)
                }
                FieldKind::Meadow => ("meadow", Some(f.crop), None),
                // One fallow ground to the hedge, not scrub blots.
                FieldKind::Fallow => ("fallow", Some(Crop::Fallow), None),
                FieldKind::Orchard => ("grass", Some(Crop::Grazed), None),
                FieldKind::Woodland => {
                    if d <= woodland_edge(seed, s) {
                        let g = if d <= 1 || patch(seed, 0xED6F, s, 0.2) < 0.4 {
                            "meadow"
                        } else {
                            "scrub"
                        };
                        (g, None, None)
                    } else if patch(seed, 0x1EAF, s, 0.08) > 0.6 {
                        ("leaf_litter", None, None)
                    } else {
                        ("forest_floor", None, None)
                    }
                }
                FieldKind::Wild => wild_ground(seed, s),
            }
        }
    }
}

/// How far into a wood its scrubby margin reaches, squares: a noise
/// field, so the wood's edge is organic rather than a band of even width.
#[must_use]
pub fn woodland_edge(seed: u64, s: Sq) -> u8 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..3
    let extra = (3.0 * patch(seed, 0xED6E, s, 0.08).clamp(0.0, 0.99)) as u8;
    1 + extra
}

fn wild_ground(seed: u64, s: Sq) -> GroundOut {
    let n = patch(seed, 0x3E47, s, 0.05);
    let g = if n > 0.64 {
        "heath"
    } else if n < 0.3 {
        "scrub"
    } else {
        "grass"
    };
    (g, None, None)
}
