//! Invariant checks over generated levels, shared by the generator's own
//! tests and by callers that want to verify a level they were sent.

use crate::ROCK_GROUND;
use arda_scene::walls::unit_edges;
use arda_scene::{Movement, Scene};
use arda_tactical::layout::{EdgeAxis, TacticalLayout};
use arda_tactical::WallRole;
use std::collections::BTreeSet;

/// Squares that are not impassable in `scene` yet cannot be reached from
/// `start`, moving orthogonally, with every door, gate and secret door
/// treated as opened (locked ones too: they have a key somewhere). Plain
/// walls and windows that block movement stop movement.
#[must_use]
pub fn unreachable(scene: &Scene, start: (u32, u32)) -> Vec<(u32, u32)> {
    let (w, h) = (scene.width, scene.height);
    let mut blocked: BTreeSet<(EdgeAxis, u32, u32)> = BTreeSet::new();
    for wall in &scene.walls {
        if wall.blocks_movement && !wall.kind.opens() {
            blocked.extend(unit_edges(wall));
        }
    }
    let idx = |x: u32, y: u32| y as usize * w as usize + x as usize;
    let pass = |x: u32, y: u32| scene.movement.0[idx(x, y)] != Movement::Impassable;
    let mut seen = vec![false; w as usize * h as usize];
    let mut stack = Vec::new();
    if start.0 < w && start.1 < h && pass(start.0, start.1) {
        seen[idx(start.0, start.1)] = true;
        stack.push(start);
    }
    while let Some((x, y)) = stack.pop() {
        let steps = [
            (
                x.checked_sub(1).map(|nx| (nx, y)),
                (EdgeAxis::Vertical, x, y),
            ),
            (
                (x + 1 < w).then_some((x + 1, y)),
                (EdgeAxis::Vertical, x + 1, y),
            ),
            (
                y.checked_sub(1).map(|ny| (x, ny)),
                (EdgeAxis::Horizontal, x, y),
            ),
            (
                (y + 1 < h).then_some((x, y + 1)),
                (EdgeAxis::Horizontal, x, y + 1),
            ),
        ];
        for (next, edge) in steps {
            let Some((nx, ny)) = next else { continue };
            if !seen[idx(nx, ny)] && pass(nx, ny) && !blocked.contains(&edge) {
                seen[idx(nx, ny)] = true;
                stack.push((nx, ny));
            }
        }
    }
    (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|&(x, y)| pass(x, y) && !seen[idx(x, y)])
        .collect()
}

/// Whether `(x, y)` is walkable floor (not the rock ground).
#[must_use]
pub fn is_floor(layout: &TacticalLayout, x: i64, y: i64) -> bool {
    x >= 0
        && y >= 0
        && x < i64::from(layout.width)
        && y < i64::from(layout.height)
        && layout.square(x, y).ground != ROCK_GROUND
}

/// Door and gate segments that float: not between two floor squares, or
/// with an end vertex that no other wall segment touches (a door must hang
/// between walls).
#[must_use]
pub fn floating_doors(layout: &TacticalLayout) -> Vec<(EdgeAxis, u32, u32)> {
    // Every vertex each segment touches, with the segment's index.
    let ends = |axis: EdgeAxis, x: u32, y: u32| match axis {
        EdgeAxis::Horizontal => [(x, y), (x + 1, y)],
        EdgeAxis::Vertical => [(x, y), (x, y + 1)],
    };
    let mut out = Vec::new();
    for (i, d) in layout.walls.iter().enumerate() {
        if !matches!(d.kind, WallRole::Door | WallRole::Gate) {
            continue;
        }
        let (x, y) = (i64::from(d.x), i64::from(d.y));
        let (a, b) = match d.axis {
            EdgeAxis::Horizontal => ((x, y - 1), (x, y)),
            EdgeAxis::Vertical => ((x - 1, y), (x, y)),
        };
        let between_floors = is_floor(layout, a.0, a.1) && is_floor(layout, b.0, b.1);
        let hung = ends(d.axis, d.x, d.y).iter().all(|v| {
            layout
                .walls
                .iter()
                .enumerate()
                .any(|(j, o)| j != i && ends(o.axis, o.x, o.y).contains(v))
        });
        if !between_floors || !hung {
            out.push((d.axis, d.x, d.y));
        }
    }
    out
}
