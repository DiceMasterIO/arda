//! Compounds: farmsteads, watermills, mines and quarries.
//!
//! Each compound is built from its own 100 m cell alone (cell coordinates,
//! seed, terrain and roads), in a local frame that is then turned and
//! mirrored onto the global lattice. That keeps compounds identical in every
//! window that sees them.

pub mod farmstead;
pub mod mill;
pub mod mine;

use crate::geom::{Edge, Sq, CELL_SQUARES};
use crate::input::{FieldInputs, LandUse};
use crate::linear::RoadNet;
use arda_tactical::catalog::{AssetClass, WallRole};
use arda_tactical::layout::{AssetRef, EdgeAxis};
use std::collections::BTreeMap;

/// A placement on the global lattice, in square units.
#[derive(Debug, Clone, PartialEq)]
pub struct GPlacement {
    /// What to place.
    pub asset: AssetRef,
    /// Anchor position, squares east.
    pub x: f64,
    /// Anchor position, squares south.
    pub y: f64,
    /// Clockwise rotation in degrees.
    pub rotation: u16,
    /// Horizontal mirror.
    pub mirror: bool,
}

/// One square a compound claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CSquare {
    /// Ground key.
    pub ground: &'static str,
    /// Water depth in feet (leats, wheel pits).
    pub water_ft: u8,
    /// Elevation added to the terrain, in feet (pits, spoil heaps).
    pub elev_ft: i16,
}

/// A wall, door, window or gate edge of a compound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CWall {
    /// The edge.
    pub edge: Edge,
    /// Its role.
    pub role: WallRole,
    /// Wall kit.
    pub kit: &'static str,
}

/// What a compound is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompoundKind {
    /// Farmhouse, barn and yard.
    Farmstead,
    /// Watermill with wheel and leat.
    Mill,
    /// Mine adit with spoil heaps.
    Mine,
    /// Open quarry with spoil heaps.
    Quarry,
}

/// A built compound on the global lattice.
#[derive(Debug, Clone)]
pub struct Compound {
    /// Kind.
    pub kind: CompoundKind,
    /// Land-use cell it was built from.
    pub cell: (i64, i64),
    /// Squares it claims, with their ground.
    pub squares: BTreeMap<Sq, CSquare>,
    /// Wall edges.
    pub walls: Vec<CWall>,
    /// Props and vegetation.
    pub placements: Vec<GPlacement>,
    /// Free light sources: position, radius in feet, colour.
    pub lights: Vec<([f64; 2], u16, [u8; 3])>,
    /// Whether its outline is walled, so fields add no wall against it.
    pub walled: bool,
    /// The square outside its gate or door where its lane starts.
    pub lane_start: Option<Sq>,
    /// The direction the lane leaves in, a unit axis vector.
    pub lane_out: [i64; 2],
}

/// A local frame: local `+x` maps to `ex`, local `+y` to `ey`, and local
/// vertex `(0, 0)` to global vertex `o`.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    o: [i64; 2],
    ex: [i64; 2],
    ey: [i64; 2],
}

/// Clockwise quarter turns taking `(1, 0)` to `v` (y points south).
fn turns_of(v: [i64; 2]) -> u16 {
    match v {
        [0, 1] => 1,
        [-1, 0] => 2,
        [0, -1] => 3,
        _ => 0,
    }
}

impl Frame {
    /// A frame whose local vertex `(0, 0)` lands on global vertex `o`.
    #[must_use]
    pub fn new(o: [i64; 2], ex: [i64; 2], ey: [i64; 2]) -> Self {
        Self { o, ex, ey }
    }

    /// Maps a local point.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // map coordinates are far below 2^52
    pub fn pt(&self, l: [f64; 2]) -> [f64; 2] {
        [
            self.o[0] as f64 + self.ex[0] as f64 * l[0] + self.ey[0] as f64 * l[1],
            self.o[1] as f64 + self.ex[1] as f64 * l[0] + self.ey[1] as f64 * l[1],
        ]
    }

    fn vertex(&self, x: i64, y: i64) -> (i64, i64) {
        (
            self.o[0] + self.ex[0] * x + self.ey[0] * y,
            self.o[1] + self.ex[1] * x + self.ey[1] * y,
        )
    }

    /// Maps a local square.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // small local offsets
    pub fn sq(&self, x: i64, y: i64) -> Sq {
        Sq::containing(self.pt([x as f64 + 0.5, y as f64 + 0.5]))
    }

    /// Maps a local edge given in the layout convention.
    #[must_use]
    pub fn edge(&self, axis: EdgeAxis, x: i64, y: i64) -> Edge {
        let (a, b) = match axis {
            EdgeAxis::Horizontal => ((x, y), (x + 1, y)),
            EdgeAxis::Vertical => ((x, y), (x, y + 1)),
        };
        Edge::from_vertices(self.vertex(a.0, a.1), self.vertex(b.0, b.1))
    }

    /// Maps a local rotation and mirror.
    #[must_use]
    pub fn orient(&self, rotation: u16, mirror: bool) -> (u16, bool) {
        let det = self.ex[0] * self.ey[1] - self.ex[1] * self.ey[0];
        if det > 0 {
            ((rotation + 90 * turns_of(self.ex)) % 360, mirror)
        } else {
            let t = turns_of([-self.ex[0], -self.ex[1]]);
            ((360 + 90 * t - rotation % 360) % 360, !mirror)
        }
    }
}

/// Builds a compound in local coordinates.
pub struct Local {
    frame: Frame,
    squares: BTreeMap<(i64, i64), CSquare>,
    walls: BTreeMap<(EdgeAxis, i64, i64), (WallRole, &'static str)>,
    placements: Vec<GPlacement>,
    lights: Vec<([f64; 2], u16, [u8; 3])>,
}

impl Local {
    /// An empty builder in a frame.
    #[must_use]
    pub fn new(frame: Frame) -> Self {
        Self {
            frame,
            squares: BTreeMap::new(),
            walls: BTreeMap::new(),
            placements: Vec::new(),
            lights: Vec::new(),
        }
    }

    /// Sets the ground of the half-open rectangle `x0..x1 × y0..y1`.
    pub fn ground(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, g: &'static str) {
        for y in y0..y1 {
            for x in x0..x1 {
                let e = self.squares.entry((x, y)).or_insert(CSquare {
                    ground: g,
                    water_ft: 0,
                    elev_ft: 0,
                });
                e.ground = g;
            }
        }
    }

    /// Sets one square completely.
    pub fn square(&mut self, x: i64, y: i64, s: CSquare) {
        self.squares.insert((x, y), s);
    }

    /// Whether a local square is claimed.
    #[must_use]
    pub fn has(&self, x: i64, y: i64) -> bool {
        self.squares.contains_key(&(x, y))
    }

    /// The global square of a local one.
    #[must_use]
    pub fn global(&self, x: i64, y: i64) -> Sq {
        self.frame.sq(x, y)
    }

    /// Walls the outline of a rectangle with `Run` pieces.
    pub fn rect_walls(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, kit: &'static str) {
        for x in x0..x1 {
            self.walls
                .insert((EdgeAxis::Horizontal, x, y0), (WallRole::Run, kit));
            self.walls
                .insert((EdgeAxis::Horizontal, x, y1), (WallRole::Run, kit));
        }
        for y in y0..y1 {
            self.walls
                .insert((EdgeAxis::Vertical, x0, y), (WallRole::Run, kit));
            self.walls
                .insert((EdgeAxis::Vertical, x1, y), (WallRole::Run, kit));
        }
    }

    /// Sets the role of one wall edge (door, window, gate), adding it if absent.
    pub fn opening(&mut self, axis: EdgeAxis, x: i64, y: i64, role: WallRole, kit: &'static str) {
        self.walls.insert((axis, x, y), (role, kit));
    }

    /// Places an asset by id at a local point.
    pub fn id(&mut self, id: &str, x: f64, y: f64, rotation: u16) {
        self.put(AssetRef::Id(id.to_string()), x, y, rotation);
    }

    /// Places an asset by tag query at a local point.
    pub fn query(&mut self, class: AssetClass, tags: &[&str], x: f64, y: f64, rotation: u16) {
        let tags = tags.iter().map(|t| (*t).to_string()).collect();
        self.put(
            AssetRef::Query {
                class: Some(class),
                tags,
            },
            x,
            y,
            rotation,
        );
    }

    fn put(&mut self, asset: AssetRef, x: f64, y: f64, rotation: u16) {
        let p = self.frame.pt([x, y]);
        let (rotation, mirror) = self.frame.orient(rotation, false);
        self.placements.push(GPlacement {
            asset,
            x: p[0],
            y: p[1],
            rotation,
            mirror,
        });
    }

    /// Adds a free light at a local point.
    pub fn light(&mut self, x: f64, y: f64, radius_ft: u16, colour: [u8; 3]) {
        self.lights.push((self.frame.pt([x, y]), radius_ft, colour));
    }

    /// Maps everything onto the global lattice.
    #[must_use]
    pub fn finish(
        self,
        kind: CompoundKind,
        cell: (i64, i64),
        walled: bool,
        lane_start: Option<(i64, i64)>,
        lane_out: [i64; 2],
    ) -> Compound {
        let f = self.frame;
        let out = [
            f.ex[0] * lane_out[0] + f.ey[0] * lane_out[1],
            f.ex[1] * lane_out[0] + f.ey[1] * lane_out[1],
        ];
        Compound {
            kind,
            cell,
            squares: self
                .squares
                .into_iter()
                .map(|((x, y), s)| (f.sq(x, y), s))
                .collect(),
            walls: self
                .walls
                .into_iter()
                .map(|((axis, x, y), (role, kit))| CWall {
                    edge: f.edge(axis, x, y),
                    role,
                    kit,
                })
                .collect(),
            placements: self.placements,
            lights: self.lights,
            walled,
            lane_start: lane_start.map(|(x, y)| f.sq(x, y)),
            lane_out: out,
        }
    }
}

/// The unit axis vector closest to `v`, preferring x on ties.
#[must_use]
pub fn axis_towards(v: [f64; 2]) -> [i64; 2] {
    if v[0].abs() >= v[1].abs() {
        [if v[0] >= 0.0 { 1 } else { -1 }, 0]
    } else {
        [0, if v[1] >= 0.0 { 1 } else { -1 }]
    }
}

/// The frame whose local `+y` points along `down` and whose local `+x` is
/// its clockwise-left perpendicular, optionally mirrored.
#[must_use]
pub fn facing(o: [i64; 2], down: [i64; 2], mirror: bool) -> Frame {
    // With y south, local +x = (down.y, -down.x) keeps the frame proper:
    // down = south gives the identity frame.
    let ex = [down[1], -down[0]];
    let ex = if mirror { [-ex[0], -ex[1]] } else { ex };
    Frame::new(o, ex, down)
}

/// Builds every compound whose cell lies in the half-open cell rectangle.
#[must_use]
pub fn enumerate(
    inputs: &FieldInputs<'_>,
    net: &RoadNet,
    seed: u64,
    cells: (i64, i64, i64, i64),
) -> Vec<Compound> {
    let (cx0, cy0, cx1, cy1) = cells;
    let mut out = Vec::new();
    for cy in cy0..cy1 {
        for cx in cx0..cx1 {
            let c = match inputs.landuse.class_at(cx, cy) {
                Some(LandUse::Farmstead) => farmstead::build(inputs, net, seed, (cx, cy)),
                Some(LandUse::Mill) => mill::build(inputs, net, seed, (cx, cy)),
                Some(LandUse::MineQuarry) => {
                    let quarry = mine::cell_slope(inputs, (cx, cy)) < mine::ADIT_SLOPE;
                    mine::build(inputs, net, seed, (cx, cy), quarry)
                }
                _ => None,
            };
            out.extend(c.filter(|c| !meets_core(inputs, c)));
        }
    }
    out
}

/// Whether a compound would stand on or against a settlement's built-up
/// core (the caller's town footprints): such a farmstead, mill or mine
/// is left out, since the town plan already builds that ground and its
/// walls would cross the town's buildings and fences.
fn meets_core(inputs: &FieldInputs<'_>, c: &Compound) -> bool {
    let Some(core) = inputs.cores else {
        return false;
    };
    c.squares
        .keys()
        .any(|s| (-1..=1).any(|dy| (-1..=1).any(|dx| core(s.x + dx, s.y + dy))))
}

/// The global centre square of a cell.
#[must_use]
pub fn cell_centre(cell: (i64, i64)) -> Sq {
    Sq::new(
        cell.0 * CELL_SQUARES + CELL_SQUARES / 2,
        cell.1 * CELL_SQUARES + CELL_SQUARES / 2,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_map_squares_edges_and_rotations_consistently() {
        let id = facing([0, 0], [0, 1], false);
        assert_eq!(id.sq(3, 4), Sq::new(3, 4));
        assert_eq!(id.orient(90, true), (90, true));
        let f = facing([10, 10], [-1, 0], false);
        // Local (0, 0) square sits just west-south of the origin vertex.
        let s = f.sq(0, 0);
        let e = f.edge(EdgeAxis::Horizontal, 0, 0);
        let (a, b) = e.sides();
        assert!(a == s || b == s, "{e:?} does not border {s:?}");
        let (r, m) = f.orient(0, false);
        assert!(!m);
        assert_eq!(r % 90, 0);
        let fm = facing([10, 10], [0, 1], true);
        assert!(fm.orient(90, false).1);
    }
}
