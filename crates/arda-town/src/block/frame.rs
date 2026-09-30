//! The canonical building frame. Interiors are authored with the street
//! front to the north (row 0, door on the top edge), `w` squares wide and
//! `d` deep, then rotated clockwise onto the building's real footprint. So
//! one programme serves every orientation, and a building's walls and props
//! depend only on its id, never on the window being cut (goal 46: seams).

use crate::plan::grid::SquareRect;
use arda_tactical::catalog::WallRole;
use arda_tactical::layout::EdgeAxis;
use std::collections::BTreeMap;

/// What a placement asks for: a vocabulary id or a `key:value` tag query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Want {
    /// A vocabulary asset id such as `prop.anvil`.
    Id(&'static str),
    /// A tag query, e.g. `["function:warehouse", "container"]`.
    Query(Vec<String>),
}

/// A prop in canonical cells: occupied rectangle and rotation.
#[derive(Debug, Clone)]
pub struct CProp {
    /// Asset wanted.
    pub want: Want,
    /// Occupied cells `x0..x1`.
    pub x0: i64,
    /// Occupied cells `y0..y1`.
    pub y0: i64,
    /// End column.
    pub x1: i64,
    /// End row.
    pub y1: i64,
    /// Rotation in degrees, canonical frame.
    pub rot: u16,
}

/// A light in canonical coordinates, optionally tied to a prop.
#[derive(Debug, Clone, Copy)]
pub struct CLight {
    /// Position, squares.
    pub x: f64,
    /// Position, squares.
    pub y: f64,
    /// Radius in feet.
    pub radius_ft: u16,
    /// Colour.
    pub colour: [u8; 3],
    /// Index of the prop that emits it.
    pub owner: Option<usize>,
}

/// A room: kind name and canonical cell rectangle.
#[derive(Debug, Clone)]
pub struct CRoom {
    /// Room kind, e.g. `common_room`.
    pub kind: &'static str,
    /// Cells `[x0, y0, x1, y1)`.
    pub rect: [i64; 4],
}

/// An interior under construction in the canonical frame.
#[derive(Debug, Clone)]
pub struct Canon {
    /// Width (frontage) in squares.
    pub w: i64,
    /// Depth in squares.
    pub d: i64,
    /// Floor key per cell.
    pub floor: Vec<&'static str>,
    /// Wall edges keyed by `(x, y, horizontal)`; later writes win.
    pub walls: BTreeMap<(i64, i64, bool), (WallRole, &'static str)>,
    /// Props.
    pub props: Vec<CProp>,
    /// Lights.
    pub lights: Vec<CLight>,
    /// Rooms.
    pub rooms: Vec<CRoom>,
    occ: Vec<u8>,
}

impl Canon {
    /// An empty interior with one floor key.
    #[must_use]
    pub fn new(w: i64, d: i64, floor: &'static str) -> Self {
        let n = usize::try_from((w * d).max(0)).unwrap_or(0);
        Self {
            w,
            d,
            floor: vec![floor; n],
            walls: BTreeMap::new(),
            props: Vec::new(),
            lights: Vec::new(),
            rooms: Vec::new(),
            occ: vec![0; n],
        }
    }

    fn i(&self, x: i64, y: i64) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.w && y < self.d)
            .then(|| usize::try_from(y * self.w + x).unwrap_or(0))
    }

    /// Outer shell: runs on every boundary edge.
    pub fn shell(&mut self, kit: &'static str) {
        for x in 0..self.w {
            self.walls.insert((x, 0, true), (WallRole::Run, kit));
            self.walls.insert((x, self.d, true), (WallRole::Run, kit));
        }
        for y in 0..self.d {
            self.walls.insert((0, y, false), (WallRole::Run, kit));
            self.walls.insert((self.w, y, false), (WallRole::Run, kit));
        }
    }

    /// Sets one horizontal edge (north edge of cell `(x, y)`).
    pub fn h(&mut self, x: i64, y: i64, role: WallRole, kit: &'static str) {
        self.walls.insert((x, y, true), (role, kit));
    }

    /// Sets one vertical edge (west edge of cell `(x, y)`).
    pub fn v(&mut self, x: i64, y: i64, role: WallRole, kit: &'static str) {
        self.walls.insert((x, y, false), (role, kit));
    }

    /// A partition along row line `y` from `x0` to `x1`, with a door.
    pub fn hwall(&mut self, y: i64, x0: i64, x1: i64, door: Option<i64>, kit: &'static str) {
        for x in x0..x1 {
            let role = if Some(x) == door {
                WallRole::Door
            } else {
                WallRole::Run
            };
            self.h(x, y, role, kit);
        }
        if let Some(dx) = door {
            self.reserve(dx, y - 1);
            self.reserve(dx, y);
        }
    }

    /// A partition along column line `x` from `y0` to `y1`, with a door.
    pub fn vwall(&mut self, x: i64, y0: i64, y1: i64, door: Option<i64>, kit: &'static str) {
        for y in y0..y1 {
            let role = if Some(y) == door {
                WallRole::Door
            } else {
                WallRole::Run
            };
            self.v(x, y, role, kit);
        }
        if let Some(dy) = door {
            self.reserve(x - 1, dy);
            self.reserve(x, dy);
        }
    }

    /// Windows every `step` squares on the side and back walls.
    pub fn windows(&mut self, step: i64, kit: &'static str) {
        let step = step.max(2);
        for y in (1..self.d - 1).step_by(usize::try_from(step).unwrap_or(3)) {
            for x in [0, self.w] {
                if self
                    .walls
                    .get(&(x, y, false))
                    .is_some_and(|w| w.0 == WallRole::Run)
                {
                    self.v(x, y, WallRole::Window, kit);
                }
            }
        }
        for x in (1..self.w - 1).step_by(usize::try_from(step).unwrap_or(3)) {
            for y in [0, self.d] {
                if self
                    .walls
                    .get(&(x, y, true))
                    .is_some_and(|w| w.0 == WallRole::Run)
                {
                    self.h(x, y, WallRole::Window, kit);
                }
            }
        }
    }

    /// Paints a floor rectangle.
    pub fn floor_rect(&mut self, r: [i64; 4], key: &'static str) {
        for y in r[1]..r[3] {
            for x in r[0]..r[2] {
                if let Some(k) = self.i(x, y) {
                    self.floor[k] = key;
                }
            }
        }
    }

    /// Keeps a cell clear of props (doorways, aisles).
    pub fn reserve(&mut self, x: i64, y: i64) {
        if let Some(k) = self.i(x, y) {
            self.occ[k] = self.occ[k].max(2);
        }
    }

    /// Keeps a rectangle clear of props.
    pub fn reserve_rect(&mut self, r: [i64; 4]) {
        for y in r[1]..r[3] {
            for x in r[0]..r[2] {
                self.reserve(x, y);
            }
        }
    }

    /// Whether every cell of the rectangle is free.
    #[must_use]
    pub fn free(&self, r: [i64; 4]) -> bool {
        (r[1]..r[3]).all(|y| (r[0]..r[2]).all(|x| self.i(x, y).is_some_and(|k| self.occ[k] == 0)))
    }

    /// Places a prop on a free rectangle; returns its index.
    pub fn put(&mut self, want: Want, r: [i64; 4], rot: u16) -> Option<usize> {
        if r[2] <= r[0] || r[3] <= r[1] || !self.free(r) {
            return None;
        }
        for y in r[1]..r[3] {
            for x in r[0]..r[2] {
                if let Some(k) = self.i(x, y) {
                    self.occ[k] = 1;
                }
            }
        }
        self.props.push(CProp {
            want,
            x0: r[0],
            y0: r[1],
            x1: r[2],
            y1: r[3],
            rot,
        });
        Some(self.props.len() - 1)
    }

    /// Shorthand for an id placement.
    pub fn put_id(&mut self, id: &'static str, r: [i64; 4], rot: u16) -> Option<usize> {
        self.put(Want::Id(id), r, rot)
    }

    /// Adds a light, optionally owned by a prop.
    pub fn light(&mut self, x: f64, y: f64, radius_ft: u16, colour: [u8; 3], owner: Option<usize>) {
        self.lights.push(CLight {
            x,
            y,
            radius_ft,
            colour,
            owner,
        });
    }

    /// Records a room.
    pub fn room(&mut self, kind: &'static str, rect: [i64; 4]) {
        self.rooms.push(CRoom { kind, rect });
    }
}

/// Maps the canonical frame onto a footprint facing `turns` quarter turns
/// clockwise from north.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    /// Footprint in global squares.
    pub rect: SquareRect,
    /// Quarter turns (0 = front north).
    pub turns: u8,
    /// Canonical width.
    pub w: i64,
    /// Canonical depth.
    pub d: i64,
}

impl Frame {
    /// The frame for a footprint whose front faces `turns`.
    #[must_use]
    pub fn new(rect: SquareRect, turns: u8) -> Self {
        let (w, d) = if turns.is_multiple_of(2) {
            (rect.w(), rect.h())
        } else {
            (rect.h(), rect.w())
        };
        Self { rect, turns, w, d }
    }

    /// Canonical point to global squares (floats; vertices map exactly).
    #[must_use]
    pub fn point(&self, x: f64, y: f64) -> (f64, f64) {
        #[allow(clippy::cast_precision_loss)]
        let (x0, y0, w, d) = (
            self.rect.x0 as f64,
            self.rect.y0 as f64,
            self.w as f64,
            self.d as f64,
        );
        match self.turns % 4 {
            0 => (x0 + x, y0 + y),
            1 => (x0 + d - y, y0 + x),
            2 => (x0 + w - x, y0 + d - y),
            _ => (x0 + y, y0 + w - x),
        }
    }

    /// Canonical lattice vertex to global lattice vertex.
    #[must_use]
    pub fn vertex(&self, x: i64, y: i64) -> (i64, i64) {
        let (r, w, d) = (self.rect, self.w, self.d);
        match self.turns % 4 {
            0 => (r.x0 + x, r.y0 + y),
            1 => (r.x0 + d - y, r.y0 + x),
            2 => (r.x0 + w - x, r.y0 + d - y),
            _ => (r.x0 + y, r.y0 + w - x),
        }
    }

    /// Canonical cell to global cell.
    #[must_use]
    pub fn cell(&self, x: i64, y: i64) -> (i64, i64) {
        let (a, b) = self.vertex(x, y);
        let (c, e) = self.vertex(x + 1, y + 1);
        (a.min(c), b.min(e))
    }

    /// Canonical edge to a global `(x, y, axis)` edge key.
    #[must_use]
    pub fn edge(&self, x: i64, y: i64, horizontal: bool) -> (i64, i64, EdgeAxis) {
        let a = self.vertex(x, y);
        let b = if horizontal {
            self.vertex(x + 1, y)
        } else {
            self.vertex(x, y + 1)
        };
        if a.1 == b.1 {
            (a.0.min(b.0), a.1, EdgeAxis::Horizontal)
        } else {
            (a.0, a.1.min(b.1), EdgeAxis::Vertical)
        }
    }

    /// Canonical rotation to global rotation.
    #[must_use]
    pub fn rotation(&self, rot: u16) -> u16 {
        (rot + 90 * u16::from(self.turns % 4)) % 360
    }

    /// The canonical cell mapping to global cell `(gx, gy)`.
    #[must_use]
    pub fn inverse_cell(&self, gx: i64, gy: i64) -> Option<(i64, i64)> {
        (0..self.d)
            .flat_map(|y| (0..self.w).map(move |x| (x, y)))
            .find(|&(x, y)| self.cell(x, y) == (gx, gy))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_rotate_the_front_onto_the_right_side() {
        let r = SquareRect {
            x0: 10,
            y0: 20,
            x1: 16,
            y1: 24,
        };
        // Front east: canonical row 0 lies on the east column.
        let f = Frame::new(r, 1);
        assert_eq!((f.w, f.d), (4, 6));
        assert_eq!(f.cell(0, 0).0, 15);
        assert_eq!(f.edge(0, 0, true), (16, 20, EdgeAxis::Vertical));
        // Every canonical cell maps inside the footprint, one to one.
        for t in 0..4 {
            let f = Frame::new(r, t);
            let mut seen = std::collections::BTreeSet::new();
            for y in 0..f.d {
                for x in 0..f.w {
                    let c = f.cell(x, y);
                    assert!(r.contains(c.0, c.1));
                    assert!(seen.insert(c));
                }
            }
            assert_eq!(f.inverse_cell(f.cell(1, 2).0, f.cell(1, 2).1), Some((1, 2)));
        }
    }
}
