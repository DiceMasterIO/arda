//! Road bands: surface, verge, ditch and graded shoulder per class; then
//! retaining walls, milestones, junction signposts and bank reeds.

use super::{Cell, Grid};
use crate::input::{m_to_ft, RoadClass, Terrain, SQUARE_M};
use crate::plan::{square_centre, Plan, WayPlan};
use crate::sidecar::{EdgeRole, Feature};
use arda_tactical::layout::EdgeAxis;
use arda_tactical::WallRole;

/// Milestone spacing along highways and roads, metres.
pub const MILESTONE_M: f64 = 1000.0;
/// A drop of at least this many feet off a road edge gets a retaining wall.
pub const RETAIN_FT: i16 = 4;

const PRIO_ROAD: u8 = 5;
const PRIO_VERGE: u8 = 4;
const PRIO_DITCH: u8 = 3;
const PRIO_SHOULDER: u8 = 1;

/// Surface ground for a class and wealth (`h` is a per-square hash).
fn surface(way: &WayPlan, h: f64, edge: bool) -> &'static str {
    match way.class {
        RoadClass::Highway if way.wealth >= 96 => {
            if edge && h < 0.22 {
                "gravel"
            } else {
                "cobbles"
            }
        }
        RoadClass::Highway => "gravel",
        RoadClass::Road if way.wealth >= 170 => "cobbles",
        RoadClass::Road if way.wealth >= 60 => {
            if edge && h < 0.3 {
                "dirt"
            } else {
                "gravel"
            }
        }
        RoadClass::Road | RoadClass::Track | RoadClass::Footpath | RoadClass::None => {
            if way.wealth < 60 && h < 0.18 {
                "mud"
            } else {
                "dirt"
            }
        }
    }
}

fn claim(c: &mut Cell, prio: u8, class: RoadClass) -> bool {
    let rank = (prio, class.hierarchy());
    if c.water || rank <= c.rank {
        return false;
    }
    c.rank = rank;
    // A band that takes a square over replaces what the lower band left:
    // a ditch's pool or a cut face's difficulty must not survive under a
    // crossing highway's surface.
    c.difficult = false;
    c.water_ft = None;
    true
}

/// Paints every way, lowest class first so higher classes win overlaps.
pub fn paint(g: &mut Grid, plan: &Plan, terrain: &dyn Terrain) {
    let mut order: Vec<usize> = (0..plan.ways.len()).collect();
    order.sort_by_key(|&i| {
        let w = &plan.ways[i];
        (w.class.hierarchy(), w.road_id, w.segment)
    });
    let squares: Vec<(i64, i64)> = g.squares().collect();
    for wi in order {
        let way = &plan.ways[wi];
        let half = f64::from(way.spec.width_sq) / 2.0;
        let verge = half + way.spec.verge_sq;
        let ditch = verge + if way.spec.ditches { 1.0 } else { 0.0 };
        let shoulder = ditch + 2.0;
        for &(gx, gy) in &squares {
            let p = square_centre(gx, gy);
            let Some(hit) = way.dense.nearest(p, shoulder * SQUARE_M) else {
                continue;
            };
            let side = hit.side / SQUARE_M;
            let d = side.abs();
            let road_ft = m_to_ft(hit.z);
            let h = g.hash(0x5EF, gx, gy);
            let wet = plan
                .channels
                .iter()
                .any(|ch| ch.dist(p, ch.width_m / 2.0 + 4.0 * SQUARE_M).is_some());
            #[allow(clippy::cast_possible_truncation)] // bounded stretch index
            let stretch = (hit.s / 4.0).floor() as i64;
            let rut = g.hash(0x12F7, stretch, i64::from(side > 0.0)) < 0.3;
            // Ditches hold standing water only on wet ground.
            #[allow(clippy::cast_possible_truncation)]
            let pool = d > verge
                && d <= ditch
                && g.hash(0xD17C, (hit.s / 3.0).floor() as i64, i64::from(side > 0.0)) < 0.2
                && terrain.wet_ground(gx, gy);
            let Some(c) = g.get_mut(gx, gy) else { continue };
            // Half-open (-W/2, W/2]: exactly W squares across a straight road.
            if side > -half && side <= half {
                if !claim(c, PRIO_ROAD, way.class) {
                    continue;
                }
                let mut ground = surface(way, h, d > half - 1.0);
                let mut feature = Feature::Road;
                if way.class == RoadClass::Track {
                    feature = Feature::Ruts;
                    if rut || wet {
                        ground = "mud";
                    }
                } else if way.class == RoadClass::Footpath && wet {
                    ground = "mud";
                }
                c.ground = Some(ground);
                c.feature = feature;
                c.class = Some(way.class);
                c.elev_ft = Some(road_ft);
            } else if d <= verge {
                if !claim(c, PRIO_VERGE, way.class) {
                    continue;
                }
                c.ground = (h < 0.35).then_some("dirt");
                c.feature = Feature::Verge;
                c.class = Some(way.class);
                c.elev_ft = Some(road_ft);
            } else if d <= ditch {
                if !claim(c, PRIO_DITCH, way.class) {
                    continue;
                }
                c.ground = Some("mud");
                c.feature = Feature::Ditch;
                c.class = Some(way.class);
                c.elev_ft = Some(road_ft - 1);
                c.water_ft = pool.then_some(1);
                c.difficult = true;
            } else {
                if !claim(c, PRIO_SHOULDER, way.class) {
                    continue;
                }
                let f = ((d - ditch) / 2.0).clamp(0.0, 1.0);
                let e = f64::from(road_ft) + (f64::from(c.terrain_ft) - f64::from(road_ft)) * f;
                c.elev_ft = Some(m_to_ft(e * 0.3048));
                if c.terrain_ft - road_ft >= RETAIN_FT {
                    c.ground = Some("scree");
                    c.feature = Feature::CutFace;
                    c.difficult = true;
                } else {
                    c.feature = Feature::Shoulder;
                }
            }
        }
    }
}

fn is_way(f: Feature) -> bool {
    matches!(
        f,
        Feature::Road | Feature::Ruts | Feature::Verge | Feature::Abutment | Feature::Bridge
    )
}

/// Retaining walls, milestones, signposts and reeds.
pub fn finish(g: &mut Grid, plan: &Plan) {
    retaining_walls(g);
    milestones(g, plan);
    signposts(g, plan);
    reeds(g);
}

fn retaining_walls(g: &mut Grid) {
    let mut walls = Vec::new();
    for (gx, gy) in g.squares() {
        let Some(c) = g.get(gx, gy) else { continue };
        if !matches!(c.feature, Feature::Road | Feature::Ruts | Feature::Verge) {
            continue;
        }
        let e = c.elevation();
        // (neighbour, edge square, axis): the edge between the two squares.
        for (nx, ny, ex, ey, axis) in [
            (gx + 1, gy, gx + 1, gy, EdgeAxis::Vertical),
            (gx - 1, gy, gx, gy, EdgeAxis::Vertical),
            (gx, gy + 1, gx, gy + 1, EdgeAxis::Horizontal),
            (gx, gy - 1, gx, gy, EdgeAxis::Horizontal),
        ] {
            let Some(n) = g.get(nx, ny) else { continue };
            if is_way(n.feature) || n.water || n.feature == Feature::Building {
                continue;
            }
            if e - n.elevation() >= RETAIN_FT {
                walls.push((ex, ey, axis));
            }
        }
    }
    for (x, y, axis) in walls {
        g.wall(x, y, axis, WallRole::Run, "drystone", EdgeRole::Retaining);
    }
}

fn milestones(g: &mut Grid, plan: &Plan) {
    for way in plan
        .ways
        .iter()
        .filter(|w| w.class.hierarchy() >= RoadClass::Road.hierarchy())
    {
        let off = (f64::from(way.spec.width_sq) / 2.0 + way.spec.verge_sq * 0.5 + 0.5) * SQUARE_M;
        for run in &way.dense.runs {
            for w in run.windows(2) {
                let k = (w[1].s / MILESTONE_M).floor();
                if k < 1.0 || (w[0].s / MILESTONE_M).floor() == k {
                    continue;
                }
                let dir = crate::curve::unit([w[1].p[0] - w[0].p[0], w[1].p[1] - w[0].p[1]]);
                let p = [w[1].p[0] - dir[1] * off, w[1].p[1] + dir[0] * off];
                g.prop(p[0] / SQUARE_M, p[1] / SQUARE_M, "prop.milestone", 0);
            }
        }
    }
}

fn signposts(g: &mut Grid, plan: &Plan) {
    for j in &plan.junctions {
        let main = &plan.ways[j.main];
        let Some(hit) = main.dense.nearest(j.at, 5.0) else {
            continue;
        };
        let d = hit.dir;
        let right = d[0] * j.branch_dir[1] - d[1] * j.branch_dir[0] > 0.0;
        // Opposite the branch, beyond the main road's verge.
        let off = (f64::from(main.spec.width_sq) / 2.0 + main.spec.verge_sq + 0.5) * SQUARE_M;
        let s = if right { -1.0 } else { 1.0 };
        let p = [j.at[0] - d[1] * off * s, j.at[1] + d[0] * off * s];
        g.prop(p[0] / SQUARE_M, p[1] / SQUARE_M, "prop.signpost", 0);
    }
}

#[allow(clippy::cast_precision_loss)] // small grid indices
fn reeds(g: &mut Grid) {
    let mut add = Vec::new();
    for (gx, gy) in g.squares() {
        let Some(c) = g.get(gx, gy) else { continue };
        if c.feature == Feature::Bank && g.hash(0x2EED, gx, gy) < 0.12 {
            add.push((gx as f64 + 0.5, gy as f64 + 0.5));
        }
    }
    for (x, y) in add {
        g.prop(x, y, "veg.reeds", 0);
    }
}
