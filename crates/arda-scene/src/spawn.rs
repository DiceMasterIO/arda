//! Token placement hints and light sources.

use crate::index::SceneIndex;
use crate::squares::Placed;
use crate::types::{
    CoverLevel, Edge, EdgeExit, Entrance, Movement, Obscurement, SceneLight, SpawnHints, Sq,
};
use crate::walls::unit_edges;
use arda_tactical::layout::{EdgeAxis, TacticalLayout};

/// Spawn hints for an indexed scene.
#[must_use]
pub fn hints(ix: &SceneIndex<'_>) -> SpawnHints {
    SpawnHints {
        open: open_squares(ix),
        entrances: entrances(ix),
        exits: exits(ix),
    }
}

fn open_squares(ix: &SceneIndex<'_>) -> Vec<Sq> {
    let (w, h) = (ix.scene.width, ix.scene.height);
    let clear = |x: i64, y: i64| {
        ix.movement(x, y) == Movement::Normal
            && ix.cover(x, y) == CoverLevel::None
            && ix
                .scene
                .square_index(Sq(
                    u32::try_from(x).unwrap_or(u32::MAX),
                    u32::try_from(y).unwrap_or(u32::MAX),
                ))
                .and_then(|i| ix.scene.obscured.get(i).copied())
                == Some(Obscurement::Clear)
    };
    let mut out = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let (xi, yi) = (i64::from(x), i64::from(y));
            if !clear(xi, yi) {
                continue;
            }
            let roomy = crate::squares::DIRS
                .iter()
                .enumerate()
                .all(|(d, (dx, dy))| clear(xi + dx, yi + dy) && ix.can_step(xi, yi, d));
            if roomy {
                out.push(Sq(x, y));
            }
        }
    }
    out
}

fn entrances(ix: &SceneIndex<'_>) -> Vec<Entrance> {
    let enterable = |x: i64, y: i64| ix.inside(x, y) && ix.movement(x, y) != Movement::Impassable;
    ix.scene
        .walls
        .iter()
        .enumerate()
        // A secret door looks like wall until found: it is no spawn
        // entrance (review round 2 #39).
        .filter(|(_, w)| w.kind.opens() && w.kind != crate::types::WallKind::Secret)
        .map(|(i, w)| {
            let mut squares = Vec::new();
            for (axis, x, y) in unit_edges(w) {
                let (xi, yi) = (i64::from(x), i64::from(y));
                let sides = match axis {
                    EdgeAxis::Horizontal => [(xi, yi - 1), (xi, yi)],
                    EdgeAxis::Vertical => [(xi - 1, yi), (xi, yi)],
                };
                for (sx, sy) in sides {
                    if enterable(sx, sy) {
                        if let (Ok(ux), Ok(uy)) = (u32::try_from(sx), u32::try_from(sy)) {
                            squares.push(Sq(ux, uy));
                        }
                    }
                }
            }
            Entrance { wall: i, squares }
        })
        .collect()
}

fn exits(ix: &SceneIndex<'_>) -> Vec<EdgeExit> {
    let (w, h) = (ix.scene.width, ix.scene.height);
    let (wi, hi) = (i64::from(w), i64::from(h));
    let mut out = Vec::new();
    let sides: [(Edge, u32); 4] = [(Edge::N, w), (Edge::E, h), (Edge::S, w), (Edge::W, h)];
    for (edge, len) in sides {
        let square = |i: u32| match edge {
            Edge::N => Sq(i, 0),
            Edge::S => Sq(i, h - 1),
            Edge::W => Sq(0, i),
            Edge::E => Sq(w - 1, i),
        };
        let open = |i: u32| {
            let s = square(i);
            let (x, y) = (i64::from(s.0), i64::from(s.1));
            let border = match edge {
                Edge::N => ix.edge(EdgeAxis::Horizontal, x, 0),
                Edge::S => ix.edge(EdgeAxis::Horizontal, x, hi),
                Edge::W => ix.edge(EdgeAxis::Vertical, 0, y),
                Edge::E => ix.edge(EdgeAxis::Vertical, wi, y),
            };
            ix.movement(x, y) != Movement::Impassable && !border.movement
        };
        let mut run: Option<u32> = None;
        for i in 0..=len {
            match (i < len && open(i), run) {
                (true, None) => run = Some(i),
                (false, Some(start)) => {
                    out.push(EdgeExit {
                        edge,
                        from: square(start),
                        to: square(i - 1),
                    });
                    run = None;
                }
                _ => {}
            }
        }
    }
    out
}

/// Free layout lights first, then emissive assets in placement order. A
/// light has one radius, its bright light; dim light reaches as far again,
/// as for the SRD candle, torch and lanterns. The SRD lamp (15 ft bright,
/// 30 ft more dim) does not fit that rule; no lamp asset exists, and one
/// needs a two-radius light schema (review round 2 #39).
#[must_use]
pub fn lights(layout: &TacticalLayout, placed: &[Placed<'_>]) -> Vec<SceneLight> {
    let free = layout.lights.iter().map(|l| SceneLight {
        x: l.x,
        y: l.y,
        bright_ft: l.radius_ft,
        dim_ft: l.radius_ft.saturating_mul(2),
        colour: l.colour,
        asset: None,
    });
    let emissive = placed.iter().filter_map(|p| {
        p.asset.light.map(|l| SceneLight {
            x: p.at.0,
            y: p.at.1,
            bright_ft: l.radius_ft,
            dim_ft: l.radius_ft.saturating_mul(2),
            colour: l.colour,
            asset: Some(p.asset.id.clone()),
        })
    });
    free.chain(emissive).collect()
}
