//! Wall edges from the layout, merged into collinear polylines.

use crate::error::SceneError;
use crate::sidecar::{EdgeRule, RulesSidecar};
use crate::types::{CoverLevel, Scene, Wall, WallKind};
use arda_tactical::catalog::WallRole;
use arda_tactical::layout::{EdgeAxis, TacticalLayout, WallSegment};
use arda_tactical::Library;
use std::collections::BTreeMap;

/// Free tag on a kit's door piece that makes its doors secret doors (the
/// layout schema has no secret role of its own).
pub const SECRET_TAG: &str = "secret";

/// Everything but position that must match for two edges to merge.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Attrs {
    kind: WallKind,
    open: Option<bool>,
    blocks_sight: bool,
    blocks_movement: bool,
    blocks_light: bool,
    cover: CoverLevel,
    kit: String,
}

fn attrs(lib: &Library, seg: &WallSegment) -> Attrs {
    let piece = lib.wall_pieces(&seg.kit, seg.kind).into_iter().next();
    let kind = match seg.kind {
        WallRole::Door if piece.is_some_and(|p| p.tags.free.iter().any(|t| t == SECRET_TAG)) => {
            WallKind::Secret
        }
        WallRole::Door => WallKind::Door,
        WallRole::Window => WallKind::Window,
        WallRole::Gate => WallKind::Gate,
        _ => WallKind::Wall,
    };
    if kind.opens() {
        // Layouts carry no door state: everything starts closed, and a closed
        // door, gate or secret door is solid.
        return closed(kind, seg.kit.clone());
    }
    let (sight, movement, cover) = match piece {
        Some(p) => (p.blocks_sight, p.blocks_movement, CoverLevel::from(p.cover)),
        None if kind == WallKind::Window => (false, true, CoverLevel::ThreeQuarters),
        None => (true, true, CoverLevel::Total),
    };
    Attrs {
        kind,
        open: None,
        blocks_sight: sight,
        blocks_movement: movement,
        blocks_light: sight,
        cover,
        kit: seg.kit.clone(),
    }
}

fn closed(kind: WallKind, kit: String) -> Attrs {
    Attrs {
        kind,
        open: Some(false),
        blocks_sight: true,
        blocks_movement: true,
        blocks_light: true,
        cover: CoverLevel::Total,
        kit,
    }
}

/// Builds the scene's walls. Later segments on an edge replace earlier ones
/// (a door punched into a run), as in `compose::walls::assemble`. Plain walls
/// and windows merge into maximal collinear runs of identical attributes;
/// doors, gates and secret doors stay one edge each so each can be opened.
#[must_use]
pub fn build(layout: &TacticalLayout, lib: &Library, rules: Option<&RulesSidecar>) -> Vec<Wall> {
    let mut edges: BTreeMap<(EdgeAxis, u32, u32), &WallSegment> = BTreeMap::new();
    for w in &layout.walls {
        edges.insert((w.axis, w.x, w.y), w);
    }
    // logic/12 §scene-sidecar: sidecar edge rules override a non-opening
    // edge's blocking and cover (a parapet blocks movement, not sight).
    let overrides: BTreeMap<(EdgeAxis, u32, u32), &EdgeRule> = rules
        .map(|r| r.edges.iter().map(|e| ((e.axis, e.x, e.y), e)).collect())
        .unwrap_or_default();
    // (axis, line, attrs, pos): sorting groups runs and orders them along the line.
    let mut keyed: Vec<(EdgeAxis, u32, Attrs, u32)> = edges
        .values()
        .map(|w| {
            let (line, pos) = match w.axis {
                EdgeAxis::Horizontal => (w.y, w.x),
                EdgeAxis::Vertical => (w.x, w.y),
            };
            let mut a = attrs(lib, w);
            if let Some(e) = overrides.get(&(w.axis, w.x, w.y)) {
                if !a.kind.opens() {
                    a.blocks_sight = e.blocks_sight;
                    a.blocks_light = e.blocks_sight;
                    a.blocks_movement = e.blocks_movement;
                    a.cover = e.cover;
                }
            }
            (w.axis, line, a, pos)
        })
        .collect();
    keyed.sort();
    let mut runs: Vec<(EdgeAxis, u32, u32, u32, Attrs)> = Vec::new();
    for (axis, line, a, pos) in keyed {
        if let Some(last) = runs.last_mut() {
            if last.0 == axis && last.1 == line && last.3 == pos && last.4 == a && !a.kind.opens() {
                last.3 = pos + 1;
                continue;
            }
        }
        runs.push((axis, line, pos, pos + 1, a));
    }
    // Output ordered by position, not attributes, so indices read naturally.
    runs.sort_by_key(|r| (r.0, r.1, r.2));
    runs.into_iter()
        .map(|(axis, line, from, to, a)| {
            let points = match axis {
                EdgeAxis::Horizontal => vec![[from, line], [to, line]],
                EdgeAxis::Vertical => vec![[line, from], [line, to]],
            };
            Wall {
                kind: a.kind,
                open: a.open,
                points,
                blocks_sight: a.blocks_sight,
                blocks_movement: a.blocks_movement,
                blocks_light: a.blocks_light,
                cover: a.cover,
                kit: a.kit,
            }
        })
        .collect()
}

impl Scene {
    /// Opens or closes a door, gate or secret door. Open, it blocks nothing;
    /// closed, it blocks sight, movement and light (total cover).
    ///
    /// # Errors
    /// [`SceneError::Wall`] for a missing index or a wall that cannot open.
    pub fn set_open(&mut self, wall: usize, open: bool) -> Result<(), SceneError> {
        let w = self
            .walls
            .get_mut(wall)
            .ok_or_else(|| SceneError::Wall(wall, "no such wall".into()))?;
        if !w.kind.opens() {
            return Err(SceneError::Wall(
                wall,
                format!("a {:?} cannot open", w.kind),
            ));
        }
        let a = if open {
            Attrs {
                kind: w.kind,
                open: Some(true),
                blocks_sight: false,
                blocks_movement: false,
                blocks_light: false,
                cover: CoverLevel::None,
                kit: String::new(),
            }
        } else {
            closed(w.kind, String::new())
        };
        w.open = a.open;
        w.blocks_sight = a.blocks_sight;
        w.blocks_movement = a.blocks_movement;
        w.blocks_light = a.blocks_light;
        w.cover = a.cover;
        Ok(())
    }
}

/// Checks a wall's points: at least two, on the `0..=width × 0..=height`
/// vertex grid, and consecutive points on one grid line (walls are
/// axis-aligned runs of unit edges).
///
/// # Errors
/// A description of the first bad point or run.
pub fn check_points(w: &Wall, width: u32, height: u32) -> Result<(), String> {
    if w.points.len() < 2 {
        return Err("a wall needs at least two points".into());
    }
    if let Some(p) = w.points.iter().find(|[x, y]| *x > width || *y > height) {
        return Err(format!("point {p:?} is off the {width}x{height} grid"));
    }
    if let Some(pair) = w
        .points
        .windows(2)
        .find(|p| p[0][0] != p[1][0] && p[0][1] != p[1][1])
    {
        return Err(format!("run {:?} to {:?} is diagonal", pair[0], pair[1]));
    }
    Ok(())
}

/// Unit edges of a wall polyline as `(axis, x, y)` in layout edge terms.
#[must_use]
pub fn unit_edges(w: &Wall) -> Vec<(EdgeAxis, u32, u32)> {
    let mut out = Vec::new();
    for pair in w.points.windows(2) {
        let ([ax, ay], [bx, by]) = (pair[0], pair[1]);
        if ay == by {
            for x in ax.min(bx)..ax.max(bx) {
                out.push((EdgeAxis::Horizontal, x, ay));
            }
        } else if ax == bx {
            for y in ay.min(by)..ay.max(by) {
                out.push((EdgeAxis::Vertical, ax, y));
            }
        }
    }
    out
}
