//! The plan grid: one cell per tactical square, in global square
//! coordinates, so plots, buildings and tactical blocks share one lattice.
//!
//! A square is `SQUARE_M` = 100 m / 64, so a 64 × 64 block is exactly one
//! 100 m area cell (goal 42). Global square `(gx, gy)` covers world metres
//! `[gx·S, (gx+1)·S) × [gy·S, (gy+1)·S)`.

use crate::geom::{v2, Vec2};
use crate::num::floor_i;
use crate::site::TerrainInput;
use rayon::prelude::*;

/// Edge length of one tactical square in metres.
pub const SQUARE_M: f64 = 100.0 / 64.0;
/// Squares per block edge (one 100 m cell).
pub const BLOCK: i64 = 64;
/// Steepest ground that takes a building, degrees.
pub const MAX_BUILD_SLOPE: f64 = 22.0;

/// What occupies a plan cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// Countryside, unclaimed.
    Open,
    /// River, lake or sea.
    Water,
    /// A street or lane.
    Street,
    /// The market square (paved).
    Square,
    /// A bridge deck over water.
    Bridge,
    /// The town wall band.
    Wall,
    /// A gate passage through the wall.
    Gate,
    /// A water gate where the wall crosses a river.
    WaterGate,
    /// A plot's front strip between street and building.
    Front,
    /// A working yard behind a building.
    Yard,
    /// A garden or orchard at the back of a plot.
    Garden,
    /// Inside a building footprint.
    Building,
    /// A castle bailey.
    Bailey,
    /// A village green.
    Green,
    /// A churchyard.
    Churchyard,
    /// Plot cells not yet zoned (transient during planning).
    Plot,
    /// Open ground inside the built-up footprint: paddock, orchard,
    /// kitchen garden, small green or work yard ([`super::croft`]).
    Croft,
}

impl Kind {
    /// Whether people can walk here without passing a wall or door.
    #[must_use]
    pub fn walkable(self) -> bool {
        !matches!(self, Self::Water | Self::Wall | Self::Building)
    }

    /// Street-like public ground.
    #[must_use]
    pub fn public(self) -> bool {
        matches!(
            self,
            Self::Street | Self::Square | Self::Bridge | Self::Gate | Self::Green
        )
    }
}

/// The square-resolution plan index.
#[derive(Debug, Clone, Default)]
pub struct PlanGrid {
    /// Global square column of local column 0 (a multiple of [`BLOCK`]).
    pub gx0: i64,
    /// Global square row of local row 0 (a multiple of [`BLOCK`]).
    pub gy0: i64,
    /// Width in squares.
    pub w: i64,
    /// Height in squares.
    pub h: i64,
    /// Cell kinds, row-major.
    pub kind: Vec<Kind>,
    /// Street index + 1 for street, bridge and gate cells.
    pub street: Vec<u16>,
    /// Plot index + 1.
    pub plot: Vec<u32>,
    /// Building index + 1.
    pub building: Vec<u32>,
    /// Ground height relative to the focal point, metres.
    pub height: Vec<f32>,
    /// Ground slope, whole degrees.
    pub slope: Vec<u8>,
    /// Water depth proxy: squares to the nearest bank (0 on land).
    pub depth: Vec<u8>,
}

impl PlanGrid {
    /// A grid covering world rectangle `lo..hi`, snapped outwards to blocks,
    /// sampling the terrain at every cell centre.
    #[must_use]
    pub fn new(lo: Vec2, hi: Vec2, terrain: &TerrainInput, base_height: f64) -> Self {
        let snap = |v: f64, up: bool| {
            let b = floor_i(v / SQUARE_M).div_euclid(BLOCK);
            (b + i64::from(up)) * BLOCK
        };
        let (gx0, gy0) = (snap(lo.x, false), snap(lo.y, false));
        let (gx1, gy1) = (snap(hi.x, true), snap(hi.y, true));
        let (w, h) = (gx1 - gx0, gy1 - gy0);
        let n = usize::try_from(w * h).unwrap_or(0);
        let mut g = Self {
            gx0,
            gy0,
            w,
            h,
            kind: vec![Kind::Open; n],
            street: vec![0; n],
            plot: vec![0; n],
            building: vec![0; n],
            height: vec![0.0; n],
            slope: vec![0; n],
            depth: vec![0; n],
        };
        // Every cell samples the terrain on its own, so rows run in
        // parallel without changing a value (goal 50).
        let rows: Vec<Vec<(bool, f32, u8)>> = (0..h)
            .into_par_iter()
            .map(|j| {
                (0..w)
                    .map(|i| {
                        let p = g.centre(i, j);
                        (
                            (terrain.water)(p),
                            crate::num::f32_of((terrain.height)(p) - base_height),
                            crate::num::round_u8((terrain.slope)(p)),
                        )
                    })
                    .collect()
            })
            .collect();
        for (k, (wet, height, slope)) in rows.into_iter().flatten().enumerate() {
            if wet {
                g.kind[k] = Kind::Water;
            }
            g.height[k] = height;
            g.slope[k] = slope;
        }
        g.compute_depth();
        g
    }

    fn lin(&self, i: i64, j: i64) -> usize {
        usize::try_from(j * self.w + i).unwrap_or(0)
    }

    /// Linear index of local cell `(i, j)`, if inside.
    #[must_use]
    pub fn idx(&self, i: i64, j: i64) -> Option<usize> {
        (i >= 0 && j >= 0 && i < self.w && j < self.h).then(|| self.lin(i, j))
    }

    /// Linear index of global square `(gx, gy)`, if inside.
    #[must_use]
    pub fn gidx(&self, gx: i64, gy: i64) -> Option<usize> {
        self.idx(gx - self.gx0, gy - self.gy0)
    }

    /// Local `(i, j)` of a linear index.
    #[must_use]
    pub fn ij(&self, k: usize) -> (i64, i64) {
        let k = i64::try_from(k).unwrap_or(0);
        (k % self.w, k / self.w)
    }

    /// World centre of local cell `(i, j)`.
    #[must_use]
    pub fn centre(&self, i: i64, j: i64) -> Vec2 {
        #[allow(clippy::cast_precision_loss)]
        let (x, y) = ((self.gx0 + i) as f64, (self.gy0 + j) as f64);
        v2((x + 0.5) * SQUARE_M, (y + 0.5) * SQUARE_M)
    }

    /// World position of local lattice vertex `(i, j)`.
    #[must_use]
    pub fn vertex(&self, i: i64, j: i64) -> Vec2 {
        #[allow(clippy::cast_precision_loss)]
        let (x, y) = ((self.gx0 + i) as f64, (self.gy0 + j) as f64);
        v2(x * SQUARE_M, y * SQUARE_M)
    }

    /// Local cell containing world point `p` (may be outside the grid).
    #[must_use]
    pub fn cell_of(&self, p: Vec2) -> (i64, i64) {
        (
            floor_i(p.x / SQUARE_M) - self.gx0,
            floor_i(p.y / SQUARE_M) - self.gy0,
        )
    }

    /// Kind at local `(i, j)`; outside the grid is open country.
    #[must_use]
    pub fn kind_at(&self, i: i64, j: i64) -> Kind {
        self.idx(i, j).map_or(Kind::Open, |k| self.kind[k])
    }

    /// Whether a plot may claim local `(i, j)`: open, flat enough, not
    /// reserved, and not against the wall band (a one-square pomerium).
    #[must_use]
    pub fn claimable(&self, i: i64, j: i64) -> bool {
        let wall = |a: i64, b: i64| {
            matches!(
                self.kind_at(a, b),
                Kind::Wall | Kind::Gate | Kind::WaterGate
            )
        };
        self.idx(i, j).is_some_and(|k| {
            self.kind[k] == Kind::Open
                && self.building[k] == 0
                && f64::from(self.slope[k]) <= MAX_BUILD_SLOPE
        }) && !(wall(i + 1, j) || wall(i - 1, j) || wall(i, j + 1) || wall(i, j - 1))
    }

    /// Recomputes the bank-distance depth proxy for water cells.
    pub fn compute_depth(&mut self) {
        let n = self.kind.len();
        let mut d = vec![u8::MAX; n];
        let mut frontier = Vec::new();
        for (k, kind) in self.kind.iter().enumerate() {
            if !matches!(kind, Kind::Water | Kind::Bridge | Kind::WaterGate) {
                d[k] = 0;
                frontier.push(k);
            }
        }
        let mut level = 0u8;
        while !frontier.is_empty() && level < 16 {
            level += 1;
            let mut next = Vec::new();
            for &k in &frontier {
                let (i, j) = self.ij(k);
                for (di, dj) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    if let Some(m) = self.idx(i + di, j + dj) {
                        if d[m] == u8::MAX {
                            d[m] = level;
                            next.push(m);
                        }
                    }
                }
            }
            frontier = next;
        }
        for v in &mut d {
            if *v == u8::MAX {
                *v = 16;
            }
        }
        self.depth = d;
    }
}

/// A side of a rectangle or the direction a front faces.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// Towards −y.
    North,
    /// Towards +x.
    East,
    /// Towards +y.
    South,
    /// Towards −x.
    West,
}

impl Side {
    /// Unit step in grid cells.
    #[must_use]
    pub const fn step(self) -> (i64, i64) {
        match self {
            Self::North => (0, -1),
            Self::East => (1, 0),
            Self::South => (0, 1),
            Self::West => (-1, 0),
        }
    }

    /// Quarter turns clockwise from north.
    #[must_use]
    pub const fn turns(self) -> u8 {
        match self {
            Self::North => 0,
            Self::East => 1,
            Self::South => 2,
            Self::West => 3,
        }
    }

    /// The opposite side.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::East => Self::West,
            Self::South => Self::North,
            Self::West => Self::East,
        }
    }

    /// The side nearest to a direction vector (y south).
    #[must_use]
    pub fn of_dir(d: Vec2) -> Self {
        if d.x.abs() >= d.y.abs() {
            if d.x >= 0.0 {
                Self::East
            } else {
                Self::West
            }
        } else if d.y >= 0.0 {
            Self::South
        } else {
            Self::North
        }
    }
}

/// A half-open rectangle of global squares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SquareRect {
    /// First column.
    pub x0: i64,
    /// First row.
    pub y0: i64,
    /// One past the last column.
    pub x1: i64,
    /// One past the last row.
    pub y1: i64,
}

impl SquareRect {
    /// Width in squares.
    #[must_use]
    pub const fn w(&self) -> i64 {
        self.x1 - self.x0
    }

    /// Height in squares.
    #[must_use]
    pub const fn h(&self) -> i64 {
        self.y1 - self.y0
    }

    /// Whether the rectangles share any square.
    #[must_use]
    pub const fn overlaps(&self, o: &Self) -> bool {
        self.x0 < o.x1 && o.x0 < self.x1 && self.y0 < o.y1 && o.y0 < self.y1
    }

    /// Whether global square `(x, y)` lies inside.
    #[must_use]
    pub const fn contains(&self, x: i64, y: i64) -> bool {
        x >= self.x0 && x < self.x1 && y >= self.y0 && y < self.y1
    }

    /// The rectangle grown by `m` squares on every side.
    #[must_use]
    pub const fn grown(&self, m: i64) -> Self {
        Self {
            x0: self.x0 - m,
            y0: self.y0 - m,
            x1: self.x1 + m,
            y1: self.y1 + m,
        }
    }

    /// The four corners as a world-metre polygon (clockwise on screen).
    #[must_use]
    pub fn polygon(&self) -> Vec<Vec2> {
        #[allow(clippy::cast_precision_loss)]
        let f = |x: i64, y: i64| v2(x as f64 * SQUARE_M, y as f64 * SQUARE_M);
        vec![
            f(self.x0, self.y0),
            f(self.x1, self.y0),
            f(self.x1, self.y1),
            f(self.x0, self.y1),
        ]
    }
}
