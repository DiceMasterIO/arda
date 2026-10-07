//! The zone grid both generators carve into, and its conversion to wall
//! segments and a rules sidecar.
//!
//! Every square belongs to a zone: [`ROCK`], [`CORRIDOR`] or a room, cell or
//! cavern id. A wall stands on every edge between rock and floor, and on
//! every edge between two different floor zones unless that edge is an
//! opening (a door, or an open archway). Corridors share one zone, so they
//! merge freely; a room entered by a corridor gets a door there and walls
//! everywhere else.

use arda_scene::{RulesCell, RulesSidecar, SIDECAR_FORMAT_VERSION};
use arda_tactical::layout::{EdgeAxis, WallSegment};
use arda_tactical::WallRole;
use std::collections::{BTreeMap, BTreeSet};

/// Solid rock.
pub(crate) const ROCK: u32 = 0;
/// The shared corridor zone.
pub(crate) const CORRIDOR: u32 = 1;
/// The first room, cell or cavern zone.
pub(crate) const FIRST_ROOM: u32 = 2;

/// A unit edge in layout terms: `(axis, x, y)` as in `WallSegment`.
pub(crate) type EdgeKey = (EdgeAxis, u32, u32);

/// What fills an opening between two floor zones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Opening {
    /// No wall at all.
    Arch,
    /// A wall-kit door or gate with free tags.
    Door(WallRole, Vec<String>),
}

/// The zone grid.
#[derive(Debug, Clone)]
pub(crate) struct Grid {
    pub(crate) w: u32,
    pub(crate) h: u32,
    pub(crate) zone: Vec<u32>,
}

impl Grid {
    pub(crate) fn new(w: u32, h: u32) -> Self {
        Self {
            w,
            h,
            zone: vec![ROCK; w as usize * h as usize],
        }
    }

    pub(crate) fn idx(&self, x: u32, y: u32) -> usize {
        y as usize * self.w as usize + x as usize
    }

    /// The zone at `(x, y)`; rock outside the map.
    pub(crate) fn get(&self, x: i64, y: i64) -> u32 {
        if x < 0 || y < 0 || x >= i64::from(self.w) || y >= i64::from(self.h) {
            return ROCK;
        }
        self.zone[usize::try_from(y * i64::from(self.w) + x).unwrap_or(0)]
    }

    pub(crate) fn set(&mut self, x: u32, y: u32, z: u32) {
        if x < self.w && y < self.h {
            let i = self.idx(x, y);
            self.zone[i] = z;
        }
    }

    pub(crate) fn floor(&self, x: i64, y: i64) -> bool {
        self.get(x, y) != ROCK
    }

    /// Every square, row-major.
    pub(crate) fn squares(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        (0..self.h).flat_map(move |y| (0..self.w).map(move |x| (x, y)))
    }
}

/// The edge between orthogonal neighbours `a` and `b`.
pub(crate) fn edge_between(a: (u32, u32), b: (u32, u32)) -> EdgeKey {
    if a.1 == b.1 {
        (EdgeAxis::Vertical, a.0.max(b.0), a.1)
    } else {
        (EdgeAxis::Horizontal, a.0, a.1.max(b.1))
    }
}

/// The two squares an edge separates, as signed coordinates (either may be
/// off the map).
pub(crate) fn sides((axis, x, y): EdgeKey) -> ((i64, i64), (i64, i64)) {
    let (x, y) = (i64::from(x), i64::from(y));
    match axis {
        EdgeAxis::Horizontal => ((x, y - 1), (x, y)),
        EdgeAxis::Vertical => ((x - 1, y), (x, y)),
    }
}

/// Every unit edge of the map, horizontal then vertical, in row order.
fn all_edges(w: u32, h: u32) -> impl Iterator<Item = EdgeKey> {
    let hz = (0..=h).flat_map(move |y| (0..w).map(move |x| (EdgeAxis::Horizontal, x, y)));
    let vt = (0..h).flat_map(move |y| (0..=w).map(move |x| (EdgeAxis::Vertical, x, y)));
    hz.chain(vt)
}

/// Wall segments of `kit` for the grid: rock faces, zone borders and the
/// given openings. Edges in `open` (a cave mouth on the map border) get no
/// wall.
pub(crate) fn walls(
    g: &Grid,
    openings: &BTreeMap<EdgeKey, Opening>,
    open: &BTreeSet<EdgeKey>,
    kit: &str,
) -> Vec<WallSegment> {
    let mut out = Vec::new();
    for e in all_edges(g.w, g.h) {
        let (a, b) = sides(e);
        let (za, zb) = (g.get(a.0, a.1), g.get(b.0, b.1));
        if za == zb || open.contains(&e) {
            continue;
        }
        let (kind, tags) = if za == ROCK || zb == ROCK {
            (WallRole::Run, Vec::new())
        } else {
            match openings.get(&e) {
                Some(Opening::Arch) => continue,
                Some(Opening::Door(role, tags)) => (*role, tags.clone()),
                None => (WallRole::Run, Vec::new()),
            }
        };
        out.push(WallSegment {
            x: e.1,
            y: e.2,
            axis: e.0,
            kind,
            kit: kit.to_string(),
            tags,
        });
    }
    out
}

/// The rules sidecar: rock squares impassable and opaque, `difficult`
/// squares difficult terrain.
pub(crate) fn rules(g: &Grid, difficult: &[bool]) -> RulesSidecar {
    let squares = g
        .squares()
        .map(|(x, y)| {
            let i = g.idx(x, y);
            let mut c = RulesCell::default();
            if g.zone[i] == ROCK {
                c.blocks_movement = Some(true);
                c.blocks_sight = Some(true);
            } else if difficult.get(i).copied().unwrap_or(false) {
                c.difficult = Some(true);
            }
            c
        })
        .collect();
    RulesSidecar {
        format_version: SIDECAR_FORMAT_VERSION,
        width: g.w,
        height: g.h,
        squares,
        edges: Vec::new(),
    }
}

/// Orthogonal neighbour offsets.
pub(crate) const N4: [(i64, i64); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

/// Squares reachable from `start` through squares where `pass` holds,
/// moving orthogonally; returns a visited mask.
pub(crate) fn flood(
    w: u32,
    h: u32,
    start: (u32, u32),
    pass: impl Fn(u32, u32) -> bool,
) -> Vec<bool> {
    let mut seen = vec![false; w as usize * h as usize];
    if start.0 >= w || start.1 >= h || !pass(start.0, start.1) {
        return seen;
    }
    let mut stack = vec![start];
    seen[start.1 as usize * w as usize + start.0 as usize] = true;
    while let Some((x, y)) = stack.pop() {
        for (dx, dy) in N4 {
            let (nx, ny) = (i64::from(x) + dx, i64::from(y) + dy);
            if nx < 0 || ny < 0 || nx >= i64::from(w) || ny >= i64::from(h) {
                continue;
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let (nx, ny) = (nx as u32, ny as u32);
            let i = ny as usize * w as usize + nx as usize;
            if !seen[i] && pass(nx, ny) {
                seen[i] = true;
                stack.push((nx, ny));
            }
        }
    }
    seen
}
