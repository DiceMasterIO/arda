//! Props and vegetation by land use, each drawn from a global jittered
//! lattice so placements agree across windows.

use super::PAD;
use crate::boundary::Boundaries;
use crate::compounds::GPlacement;
use crate::fields::{Crop, Field, FieldKind};
use crate::geom::{h2, h3, s11, u01, Grid, Sq};
use crate::plan::{Cover, Plan};
use arda_tactical::catalog::{AssetClass, WallRole};
use arda_tactical::layout::{AssetRef, EdgeAxis};
use std::collections::BTreeMap;

/// Hedgerow-tree lattice spacing, in squares.
const HEDGE_TREE_CELL: i64 = 14;

fn id(asset: &str, p: [f64; 2], h: u64) -> GPlacement {
    GPlacement {
        asset: AssetRef::Id(asset.to_string()),
        x: p[0],
        y: p[1],
        rotation: [0, 90, 180, 270][crate::geom::pick4(h >> 50)],
        mirror: (h >> 45) & 1 == 1,
    }
}

fn query(tags: &[&str], p: [f64; 2], h: u64) -> GPlacement {
    GPlacement {
        asset: AssetRef::Query {
            class: Some(AssetClass::Prop),
            tags: tags.iter().map(|t| (*t).to_string()).collect(),
        },
        x: p[0],
        y: p[1],
        rotation: [0, 90, 180, 270][crate::geom::pick4(h >> 50)],
        mirror: (h >> 45) & 1 == 1,
    }
}

/// A lattice generator: calls `f(node hash, jittered point, square)` for
/// every node of spacing `step` whose point lands in the window.
#[allow(clippy::cast_precision_loss)] // lattice coordinates
fn lattice(plan: &Plan, salt: u64, step: i64, jitter: f64, mut f: impl FnMut(u64, [f64; 2], Sq)) {
    let (x0, y0, w, h) = plan.win;
    let (i0, j0) = ((x0 - 2).div_euclid(step), (y0 - 2).div_euclid(step));
    let (i1, j1) = ((x0 + w + 2).div_euclid(step), (y0 + h + 2).div_euclid(step));
    let s = step as f64;
    for j in j0..=j1 {
        for i in i0..=i1 {
            let hh = h2(plan.seed, salt, i, j);
            let p = [
                (i as f64 + 0.5) * s + jitter * s11(hh),
                (j as f64 + 0.5) * s + jitter * s11(hh >> 21),
            ];
            let sq = Sq::containing(p);
            if plan.in_window(sq) {
                f(hh, p, sq);
            }
        }
    }
}

fn field_of(plan: &Plan, s: Sq) -> Option<(usize, &Field)> {
    plan.field_at(s)
}

/// Every placement anchored inside the window.
pub(super) fn place(plan: &Plan, bounds: &Boundaries, dist: &Grid<u8>) -> Vec<GPlacement> {
    let mut out = Vec::new();
    let d = |s: Sq| dist.get(s).copied().unwrap_or(0);
    let kind = |s: Sq| field_of(plan, s).map(|(_, f)| f.kind);
    // Orchards: rows of fruit trees with a few gaps.
    lattice(plan, 0x0C4A, 5, 0.35, |hh, p, s| {
        if kind(s) == Some(FieldKind::Orchard) && d(s) >= 2 && u01(hh >> 3) > 0.08 {
            out.push(id("veg.tree_fruit", p, hh));
        }
    });
    // Woodland: a closed canopy in stands of one species, thinning to
    // scrub and birch on the fringe, with glades where the noise dips.
    lattice(plan, 0x3000, 3, 1.5, |hh, p, s| {
        if kind(s) != Some(FieldKind::Woodland) {
            return;
        }
        let u = u01(hh >> 3);
        let stand = super::patch(plan.seed, 0x57A4, s, 0.035);
        let glade = super::patch(plan.seed, 0x61AD, s, 0.06) < 0.22;
        if d(s) > super::woodland_edge(plan.seed, s) && !glade {
            if u > 0.88 {
                return;
            }
            let tree = if stand < 0.42 {
                if u < 0.7 {
                    "veg.tree_oak"
                } else {
                    "veg.tree_elm"
                }
            } else if stand < 0.62 {
                if u < 0.6 {
                    "veg.tree_elm"
                } else {
                    "veg.tree_oak"
                }
            } else if u < 0.75 {
                "veg.tree_birch"
            } else {
                "veg.tree_oak"
            };
            out.push(id(tree, p, hh));
        } else if u < 0.45 {
            out.push(id("veg.bush", p, hh));
        } else if u < 0.6 {
            out.push(id("veg.tree_birch", p, hh));
        }
    });
    lattice(plan, 0x3001, 7, 2.5, |hh, p, s| {
        if kind(s) != Some(FieldKind::Woodland) || d(s) < 3 {
            return;
        }
        let u = u01(hh >> 3);
        let what = if u < 0.3 {
            "veg.fern"
        } else if u < 0.38 {
            "veg.fallen_log"
        } else if u < 0.44 {
            "veg.stump"
        } else if u < 0.46 {
            "veg.mushroom_ring"
        } else {
            return;
        };
        out.push(id(what, p, hh));
    });
    livestock(plan, &d, &mut out);
    lattice(plan, 0x4E4D, 4, 2.0, |hh, p, s| {
        let Some((_, f)) = field_of(plan, s) else {
            return;
        };
        let u = u01(hh >> 3);
        match (f.kind, f.crop) {
            (FieldKind::Meadow, Crop::Hay) if d(s) >= 2 && u < 0.3 => {
                out.push(id("veg.tall_grass", p, hh));
            }
            (FieldKind::Meadow, Crop::Hay) if d(s) >= 2 && u < 0.4 => {
                out.push(id("veg.flower_patch", p, hh));
            }
            (FieldKind::Fallow, _) if u < 0.2 => out.push(id("veg.tall_grass", p, hh)),
            (FieldKind::Fallow, _) if u < 0.26 => out.push(id("veg.bush", p, hh)),
            _ => {}
        }
    });
    // Hay: bales in rows on mown meadows, stooks on stubble.
    lattice(plan, 0x4A7B, 7, 0.4, |hh, p, s| {
        let Some((_, f)) = field_of(plan, s) else {
            return;
        };
        let u = u01(hh >> 3);
        let stubble = match f.kind {
            FieldKind::Arable => f.crop == Crop::Stubble,
            FieldKind::Strips => {
                let site = &plan.partition.sites[f.site];
                let at = crate::strips::locate(plan.seed, site, s.centre());
                !at.balk && crate::strips::crop(plan.seed, site, f.crop, at.index) == Crop::Stubble
            }
            _ => false,
        };
        let mown = f.kind == FieldKind::Meadow && f.crop == Crop::Mown;
        if (stubble && u < 0.4 || mown && u < 0.55) && d(s) >= 3 {
            let r = if (f.id >> 7) & 1 == 0 { 0 } else { 90 };
            out.push(GPlacement {
                rotation: r,
                ..id("prop.hay_bale", p, hh)
            });
        }
    });
    // Rough grazing and slivers.
    lattice(plan, 0x3171, 6, 3.0, |hh, p, s| {
        let u = u01(hh >> 3);
        let wild =
            matches!(plan.at(s), Cover::Wild | Cover::Apron) || kind(s) == Some(FieldKind::Wild);
        if plan.at(s) == Cover::Rough && u < 0.6 || wild && u < 0.14 {
            out.push(id("veg.bush", p, hh));
        } else if wild && u < 0.2 {
            out.push(id("veg.heather", p, hh));
        } else if wild && u < 0.24 {
            out.push(id("veg.stones", p, hh));
        } else if kind(s) == Some(FieldKind::Pasture) && d(s) >= 2 && u < 0.05 {
            out.push(id("veg.bush", p, hh));
        } else if kind(s) == Some(FieldKind::Common) && d(s) >= 2 && u < 0.12 {
            let v = if u < 0.07 { "veg.bush" } else { "veg.heather" };
            out.push(id(v, p, hh));
        }
    });
    hedgerow_trees(plan, bounds, &mut out);
    troughs(plan, bounds, &mut out);
    for c in &plan.compounds {
        out.extend(
            c.placements
                .iter()
                .filter(|p| plan.in_window(Sq::containing([p.x, p.y])))
                .cloned(),
        );
    }
    out
}

/// Sheep or cattle grazing in a loose flock around a point in each pasture.
#[allow(clippy::cast_precision_loss)] // hash-derived offsets
fn livestock(plan: &Plan, d: &impl Fn(Sq) -> u8, out: &mut Vec<GPlacement>) {
    lattice(plan, 0x5EEF, 4, 2.0, |hh, p, s| {
        let Some((_, f)) = field_of(plan, s) else {
            return;
        };
        let orchard = f.kind == FieldKind::Orchard;
        let grazed = matches!(f.kind, FieldKind::Pasture | FieldKind::Common);
        if !(grazed || orchard) || d(s) < 2 {
            return;
        }
        let fh = h2(plan.seed, 0x5EF0, f.first.x, f.first.y);
        let c = f.centroid();
        let centre = [c[0] + 18.0 * s11(fh), c[1] + 18.0 * s11(fh >> 20)];
        let r = ((p[0] - centre[0]).powi(2) + (p[1] - centre[1]).powi(2)).sqrt();
        let sheep = orchard || u01(fh >> 40) < 0.65;
        let density = if sheep { 0.5 } else { 0.3 };
        if u01(hh >> 3) < density * (1.0 - r / 26.0).max(0.0) {
            let tag = if sheep {
                "livestock:sheep"
            } else {
                "livestock:cattle"
            };
            out.push(query(&[tag], p, hh));
        }
    });
}

/// Standard trees left in the hedgerows, at most one per lattice cell.
fn hedgerow_trees(plan: &Plan, bounds: &Boundaries, out: &mut Vec<GPlacement>) {
    let (x0, y0, w, h) = plan.win;
    let mut best: BTreeMap<(i64, i64), (u64, (i64, i64))> = BTreeMap::new();
    let lo = (x0 - PAD, y0 - PAD);
    let hi = (x0 + w + PAD, y0 + h + PAD);
    for (e, info) in bounds.walls.range(..) {
        if info.kit != "hedge" || info.role != WallRole::Run {
            continue;
        }
        if e.x < lo.0 || e.y < lo.1 || e.x > hi.0 || e.y > hi.1 {
            continue;
        }
        // The far vertex of a run: counting each vertex once per edge axis.
        let v = match e.axis {
            EdgeAxis::Horizontal => (e.x + 1, e.y),
            EdgeAxis::Vertical => (e.x, e.y + 1),
        };
        let cell = (
            v.0.div_euclid(HEDGE_TREE_CELL),
            v.1.div_euclid(HEDGE_TREE_CELL),
        );
        let hv = h2(plan.seed, 0x7EE5, v.0, v.1);
        best.entry(cell)
            .and_modify(|b| {
                if (hv, v) < *b {
                    *b = (hv, v);
                }
            })
            .or_insert((hv, v));
    }
    for (cell, (hv, v)) in best {
        let hc = h2(plan.seed, 0x7EE6, cell.0, cell.1);
        #[allow(clippy::cast_precision_loss)] // lattice vertices
        let p = [v.0 as f64, v.1 as f64];
        if u01(hc) < 0.6 && plan.in_window(Sq::containing(p)) {
            let tree = if u01(hc >> 20) < 0.6 {
                "veg.tree_oak"
            } else {
                "veg.tree_elm"
            };
            out.push(id(tree, p, hv));
        }
    }
}

/// A water trough just inside the gate of every pasture.
#[allow(clippy::cast_precision_loss)] // unit offsets
fn troughs(plan: &Plan, bounds: &Boundaries, out: &mut Vec<GPlacement>) {
    for (i, f) in plan.fields.iter().enumerate() {
        if f.kind != FieldKind::Pasture {
            continue;
        }
        let Some(e) = bounds.gates.get(i).and_then(|g| g.first()) else {
            continue;
        };
        let (a, b) = e.sides();
        let (inside, outside) = if plan.field_at(a).is_some_and(|(j, _)| j == i) {
            (a, b)
        } else {
            (b, a)
        };
        let n = [inside.x - outside.x, inside.y - outside.y];
        let c = inside.centre();
        let p = [c[0] + 1.5 * n[0] as f64, c[1] + 1.5 * n[1] as f64];
        if !plan.in_window(Sq::containing(p)) {
            continue;
        }
        let rotation = if e.axis == EdgeAxis::Horizontal {
            0
        } else {
            90
        };
        let hh = h3(plan.seed, 0x7206, e.x, e.y, 0);
        out.push(GPlacement {
            rotation,
            mirror: false,
            ..id("prop.trough", p, hh)
        });
    }
}
