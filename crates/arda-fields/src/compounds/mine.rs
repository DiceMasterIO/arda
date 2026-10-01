//! Mines and quarries (goal 39: "mines in mountains"): an adit driven into
//! the hillside or an open pit, with an apron, a crane or ore cart and spoil
//! heaps of scree tipped downhill.
//!
//! Local frame: downhill is local south (+y); the working face is uphill.

use super::{cell_centre, facing, CSquare, Compound, CompoundKind, Local};
use crate::geom::{h2, h3, s11, u01, Sq};
use crate::input::FieldInputs;
use crate::linear::RoadNet;
use arda_tactical::catalog::WallRole;
use arda_tactical::layout::EdgeAxis::{Horizontal as H, Vertical as V};
use arda_tactical::noise::fbm;

const SALT: u64 = 0x4133;
/// Slope (rise over run) from which a resource site is worked by an adit
/// driven into the hillside rather than an open quarry.
pub const ADIT_SLOPE: f64 = 0.1;

/// Slope at the centre of a cell.
#[must_use]
pub fn cell_slope(inputs: &FieldInputs<'_>, cell: (i64, i64)) -> f64 {
    let m = cell_centre(cell).centre_m();
    let g = crate::partition::gradient(inputs.terrain, m);
    (g[0] * g[0] + g[1] * g[1]).sqrt()
}

/// The downhill axis at a square, from a central difference over 8 m.
fn downhill(inputs: &FieldInputs<'_>, s: Sq, h: u64) -> [i64; 2] {
    let m = s.centre_m();
    let d = 8.0;
    let t = |dx: f64, dy: f64| inputs.terrain.sample(m[0] + dx, m[1] + dy).height_m;
    let g = [t(d, 0.0) - t(-d, 0.0), t(0.0, d) - t(0.0, -d)];
    if g[0].abs() + g[1].abs() < 1e-6 {
        return [[0, 1], [1, 0], [0, -1], [-1, 0]][crate::geom::pick4(h >> 30)];
    }
    super::axis_towards([-g[0], -g[1]])
}

/// Shifts, in squares, tried in turn when a mine's footprint would cut a
/// road; the first is its usual site.
const SHIFTS: [(i64, i64); 9] = [
    (0, 0),
    (16, 0),
    (-16, 0),
    (0, 16),
    (0, -16),
    (16, 16),
    (-16, 16),
    (16, -16),
    (-16, -16),
];

/// Whether square `s` lies on a carriageway or its edge square.
fn on_road(net: &RoadNet, s: Sq) -> bool {
    net.near(s.centre(), 0.5)
}

/// Builds the mine (or quarry) of a cell; `None` if its centre is under water.
///
/// Roads win (review round 2 #37: compounds are laid over every cover, so a
/// pit or spoil heap cut a road that arda-ways still painted): the mine
/// moves to the first of [`SHIFTS`] whose squares are all off the road
/// net, and if none is, keeps its site and gives up the squares and props
/// on the road.
#[must_use]
pub fn build(
    inputs: &FieldInputs<'_>,
    net: &RoadNet,
    seed: u64,
    cell: (i64, i64),
    quarry: bool,
) -> Option<Compound> {
    let first = build_at(inputs, seed, cell, quarry, SHIFTS[0])?;
    let clear = |c: &Compound| !c.squares.keys().any(|&s| on_road(net, s));
    if clear(&first) {
        return Some(first);
    }
    for &shift in &SHIFTS[1..] {
        if let Some(c) = build_at(inputs, seed, cell, quarry, shift).filter(clear) {
            return Some(c);
        }
    }
    let mut c = first;
    c.squares.retain(|&s, _| !on_road(net, s));
    #[allow(clippy::cast_possible_truncation)] // anchors are within the world
    c.placements
        .retain(|p| !on_road(net, Sq::new(p.x.floor() as i64, p.y.floor() as i64)));
    c.walls.retain(|w| {
        let (a, b) = w.edge.sides();
        !on_road(net, a) && !on_road(net, b)
    });
    Some(c)
}

fn build_at(
    inputs: &FieldInputs<'_>,
    seed: u64,
    cell: (i64, i64),
    quarry: bool,
    shift: (i64, i64),
) -> Option<Compound> {
    let h = h2(seed, SALT, cell.0, cell.1);
    #[allow(clippy::cast_possible_truncation)] // jitter of a few squares
    let c = cell_centre(cell).offset(
        (s11(h) * 6.0).round() as i64 + shift.0,
        (s11(h >> 9) * 6.0).round() as i64 + shift.1,
    );
    let m = c.centre_m();
    if inputs.terrain.sample(m[0], m[1]).water_depth_m > 0.0 {
        return None;
    }
    let down = downhill(inputs, c, h);
    let mut l = Local::new(facing([c.x, c.y], down, u01(h >> 5) < 0.5));
    let noise = |l: &Local, x: i64, y: i64| {
        let g = l.global(x, y);
        #[allow(clippy::cast_possible_truncation)] // noise input precision
        let v = fbm(
            seed ^ 0x5B01,
            (g.x as f32) * 0.11,
            (g.y as f32) * 0.11,
            2,
            None,
        );
        f64::from(v) - 0.5
    };
    let kind = if quarry {
        pit(&mut l, &noise);
        CompoundKind::Quarry
    } else {
        adit(&mut l);
        CompoundKind::Mine
    };
    let heaps: [(i64, i64, i64); 3] = if quarry {
        [(-13, 13, 6), (12, 15, 5), (1, 19, 4)]
    } else {
        [(-8, 6, 5), (8, 8, 6), (-1, 12, 3)]
    };
    for (i, (hx, hy, r)) in heaps.into_iter().enumerate() {
        spoil(&mut l, seed, (hx, hy, r), i, &noise);
    }
    let start = if quarry { (0, 12) } else { (0, 3) };
    Some(l.finish(kind, cell, false, Some(start), [0, 1]))
}

/// An open pit: a cliffed face uphill, a gravel floor, a ramp downhill.
#[allow(clippy::cast_precision_loss)] // small local offsets
fn pit(l: &mut Local, noise: &impl Fn(&Local, i64, i64) -> f64) {
    for y in -12..11 {
        for x in -15..15 {
            let q = (x as f64 / 12.0).powi(2) + (y as f64 / 9.0).powi(2) + 0.35 * noise(l, x, y);
            if q >= 1.0 {
                continue;
            }
            let (ground, elev) = if y < 0 && q > 0.72 {
                ("cliff", -4)
            } else if y < 0 && q > 0.5 {
                ("rock", -7)
            } else {
                ("gravel", -10)
            };
            l.square(
                x,
                y,
                CSquare {
                    ground,
                    water_ft: 0,
                    elev_ft: elev,
                },
            );
        }
    }
    for y in 0..12 {
        for x in -1..2 {
            if !l.has(x, y) || y > 5 {
                l.square(
                    x,
                    y,
                    CSquare {
                        ground: "gravel",
                        water_ft: 0,
                        elev_ft: -(10 - i16::try_from(y).unwrap_or(10)).max(0),
                    },
                );
            }
        }
    }
    for (x, y) in [
        (-6.5, -1.5),
        (-3.5, 2.5),
        (4.5, -2.5),
        (7.5, 1.5),
        (-8.5, 3.5),
    ] {
        l.id("veg.rock_large", x, y, 0);
    }
    for (x, y) in [(-1.5, -4.5), (2.5, 4.5), (5.5, 5.5), (-5.5, 5.5)] {
        l.id("veg.rock_small", x, y, 90);
    }
    l.id("prop.crane", 6.0, -11.0, 0);
    l.id("prop.cart", 1.5, 7.0, 0);
    l.id("prop.crate", 3.5, 9.5, 0);
    l.id("prop.crate", 3.5, 10.5, 90);
}

/// An adit: a timber-framed tunnel mouth in a rock face, with an apron.
fn adit(l: &mut Local) {
    let rock = |l: &mut Local, x0, y0, x1, y1, g: &'static str, e: i16| {
        for y in y0..y1 {
            for x in x0..x1 {
                l.square(
                    x,
                    y,
                    CSquare {
                        ground: g,
                        water_ft: 0,
                        elev_ft: e,
                    },
                );
            }
        }
    };
    rock(l, -7, -9, 8, -6, "cliff", 6);
    rock(l, -7, -6, 8, -3, "rock", 3);
    rock(l, -5, -3, 6, 3, "gravel", 0);
    rock(l, -1, -9, 2, -3, "stone_floor", 0);
    for y in -9..-3 {
        l.opening(V, -1, y, WallRole::Run, "timber");
        l.opening(V, 2, y, WallRole::Run, "timber");
    }
    for x in -1..2 {
        l.opening(H, x, -9, WallRole::Run, "stone");
    }
    l.opening(H, -1, -3, WallRole::Run, "timber");
    l.opening(H, 0, -3, WallRole::Door, "timber");
    l.opening(H, 1, -3, WallRole::Run, "timber");
    l.id("prop.cart", 0.5, -6.0, 0);
    l.id("prop.lantern", 1.5, -3.5, 0);
    l.id("prop.woodpile", -3.5, -1.5, 0);
    l.id("prop.woodpile", -4.5, -1.5, 90);
    l.id("prop.crate", 3.5, -1.5, 0);
    l.id("prop.barrel", 4.5, -1.5, 0);
    l.id("prop.cart", -2.0, 1.0, 90);
    l.id("veg.rock_large", -6.5, -4.5, 0);
    l.id("veg.rock_large", 7.5, -5.5, 90);
}

/// A tipped heap of scree with rocks on its flanks.
#[allow(clippy::cast_precision_loss)] // small local offsets
fn spoil(
    l: &mut Local,
    seed: u64,
    (cx, cy, r): (i64, i64, i64),
    i: usize,
    noise: &impl Fn(&Local, i64, i64) -> f64,
) {
    for y in (cy - r - 2)..=(cy + r + 2) {
        for x in (cx - r - 2)..=(cx + r + 2) {
            if l.has(x, y) {
                continue;
            }
            let d = (((x - cx).pow(2) + (y - cy).pow(2)) as f64).sqrt();
            let rr = r as f64 * (1.0 + 0.5 * noise(l, x + 40, y));
            if d >= rr {
                continue;
            }
            #[allow(clippy::cast_possible_truncation)] // a few feet
            let elev = ((rr - d) * 1.2).min(6.0) as i16;
            l.square(
                x,
                y,
                CSquare {
                    ground: if d < rr * 0.7 { "scree" } else { "gravel" },
                    water_ft: 0,
                    elev_ft: elev,
                },
            );
            let hh = h3(seed, 0x5E, x, y, i64::try_from(i).unwrap_or(0));
            let g = l.global(x, y);
            let hg = h2(seed, 0x5F, g.x, g.y);
            if u01(hh) < 0.12 {
                let id = if u01(hg) < 0.6 {
                    "veg.rock_small"
                } else {
                    "veg.scree_patch"
                };
                let rot = [0, 90, 180, 270][crate::geom::pick4(hg >> 40)];
                l.id(id, x as f64 + 0.5, y as f64 + 0.5, rot);
            }
        }
    }
}
