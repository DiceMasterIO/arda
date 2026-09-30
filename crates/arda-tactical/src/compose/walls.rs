//! Wall-kit assembly (goal 62): edge pieces on square edges, joint pieces on
//! grid vertices chosen from each vertex's four-arm connectivity mask.

use crate::catalog::WallRole;
use crate::layout::{EdgeAxis, TacticalLayout, WallSegment};
use std::collections::BTreeMap;

/// Arm directions in mask order.
pub const NORTH: usize = 0;
/// East arm.
pub const EAST: usize = 1;
/// South arm.
pub const SOUTH: usize = 2;
/// West arm.
pub const WEST: usize = 3;

/// Canonical arms of each joint role, `[n, e, s, w]`, matching
/// [`WallRole`]'s documentation and the art convention in the README.
#[must_use]
pub fn canonical_arms(role: WallRole) -> [bool; 4] {
    match role {
        WallRole::Corner => [false, true, true, false],
        WallRole::Tee => [false, true, true, true],
        WallRole::Cross => [true; 4],
        WallRole::End => [false, true, false, false],
        WallRole::Post => [false, true, false, true],
        _ => [false; 4],
    }
}

/// Rotates an arm mask clockwise by one quarter turn: north becomes east.
#[must_use]
pub fn rotate_cw(m: [bool; 4]) -> [bool; 4] {
    [m[WEST], m[NORTH], m[EAST], m[SOUTH]]
}

/// Chooses the joint piece and its clockwise quarter turns for an arm mask.
/// Two opposite arms give a `Post`; no arms give `None`.
#[must_use]
pub fn select_joint(arms: [bool; 4]) -> Option<(WallRole, u8)> {
    let role = match arms.iter().filter(|a| **a).count() {
        0 => return None,
        1 => WallRole::End,
        2 if arms[NORTH] == arms[SOUTH] => WallRole::Post,
        2 => WallRole::Corner,
        3 => WallRole::Tee,
        _ => WallRole::Cross,
    };
    let mut m = canonical_arms(role);
    for turns in 0..4 {
        if m == arms {
            return Some((role, turns));
        }
        m = rotate_cw(m);
    }
    None
}

/// An edge piece to draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeDraw<'a> {
    /// The segment.
    pub segment: &'a WallSegment,
    /// Centre in squares ×2 (half-square units keep it integral).
    pub centre2: (u32, u32),
    /// Quarter turns: 0 for horizontal edges, 1 for vertical.
    pub turns: u8,
}

/// A joint piece to draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JointDraw<'a> {
    /// Vertex in squares.
    pub vertex: (u32, u32),
    /// Kit of the first arm found in N, E, S, W order.
    pub kit: &'a str,
    /// The chosen role.
    pub role: WallRole,
    /// Clockwise quarter turns.
    pub turns: u8,
}

/// Every edge and joint piece of a layout, in draw order.
#[must_use]
pub fn assemble(layout: &TacticalLayout) -> (Vec<EdgeDraw<'_>>, Vec<JointDraw<'_>>) {
    let mut edges: BTreeMap<(EdgeAxis, u32, u32), &WallSegment> = BTreeMap::new();
    for w in &layout.walls {
        // Later segments on the same edge replace earlier ones (a door
        // punched into a run).
        edges.insert((w.axis, w.x, w.y), w);
    }
    let edge_draws = edges
        .values()
        .map(|w| match w.axis {
            EdgeAxis::Horizontal => EdgeDraw {
                segment: w,
                centre2: (2 * w.x + 1, 2 * w.y),
                turns: 0,
            },
            EdgeAxis::Vertical => EdgeDraw {
                segment: w,
                centre2: (2 * w.x, 2 * w.y + 1),
                turns: 1,
            },
        })
        .collect();
    let mut joints = Vec::new();
    for vy in 0..=layout.height {
        for vx in 0..=layout.width {
            let arm = |axis, x: Option<u32>, y: Option<u32>| match (x, y) {
                (Some(x), Some(y)) => edges.get(&(axis, x, y)).copied(),
                _ => None,
            };
            let found = [
                arm(EdgeAxis::Vertical, Some(vx), vy.checked_sub(1)),
                arm(EdgeAxis::Horizontal, Some(vx), Some(vy)),
                arm(EdgeAxis::Vertical, Some(vx), Some(vy)),
                arm(EdgeAxis::Horizontal, vx.checked_sub(1), Some(vy)),
            ];
            let mask = found.map(|f| f.is_some());
            let Some((role, turns)) = select_joint(mask) else {
                continue;
            };
            if let Some(seg) = found.iter().flatten().next() {
                joints.push(JointDraw {
                    vertex: (vx, vy),
                    kit: &seg.kit,
                    role,
                    turns,
                });
            }
        }
    }
    (edge_draws, joints)
}

#[cfg(test)]
mod tests {
    use super::*;

    const N: bool = true;
    const O: bool = false;

    #[test]
    fn every_junction_type_selects_the_right_piece() {
        // Ends: canonical arm east.
        assert_eq!(select_joint([O, N, O, O]), Some((WallRole::End, 0)));
        assert_eq!(select_joint([O, O, N, O]), Some((WallRole::End, 1)));
        assert_eq!(select_joint([O, O, O, N]), Some((WallRole::End, 2)));
        assert_eq!(select_joint([N, O, O, O]), Some((WallRole::End, 3)));
        // Corners: canonical east + south.
        assert_eq!(select_joint([O, N, N, O]), Some((WallRole::Corner, 0)));
        assert_eq!(select_joint([O, O, N, N]), Some((WallRole::Corner, 1)));
        assert_eq!(select_joint([N, O, O, N]), Some((WallRole::Corner, 2)));
        assert_eq!(select_joint([N, N, O, O]), Some((WallRole::Corner, 3)));
        // Tees: canonical open to the north.
        assert_eq!(select_joint([O, N, N, N]), Some((WallRole::Tee, 0)));
        assert_eq!(select_joint([N, O, N, N]), Some((WallRole::Tee, 1)));
        assert_eq!(select_joint([N, N, O, N]), Some((WallRole::Tee, 2)));
        assert_eq!(select_joint([N, N, N, O]), Some((WallRole::Tee, 3)));
        // Straight runs and crosses.
        assert_eq!(select_joint([O, N, O, N]), Some((WallRole::Post, 0)));
        assert_eq!(select_joint([N, O, N, O]), Some((WallRole::Post, 1)));
        assert_eq!(select_joint([N, N, N, N]), Some((WallRole::Cross, 0)));
        assert_eq!(select_joint([O, O, O, O]), None);
    }

    #[test]
    fn all_sixteen_masks_round_trip_through_rotation() {
        for bits in 1u8..16 {
            let mask = [bits & 1 != 0, bits & 2 != 0, bits & 4 != 0, bits & 8 != 0];
            let (role, turns) = select_joint(mask).unwrap();
            let mut m = canonical_arms(role);
            for _ in 0..turns {
                m = rotate_cw(m);
            }
            assert_eq!(m, mask, "{role:?} x{turns}");
        }
    }

    fn seg(x: u32, y: u32, axis: EdgeAxis, kind: WallRole, kit: &str) -> WallSegment {
        WallSegment {
            x,
            y,
            axis,
            kind,
            kit: kit.into(),
        }
    }

    #[test]
    fn a_room_with_a_partition_has_corners_tees_and_a_door() {
        let mut l = TacticalLayout::new("room", 4, 2, "dirt");
        for x in 0..4 {
            l.walls
                .push(seg(x, 0, EdgeAxis::Horizontal, WallRole::Run, "stone"));
            l.walls
                .push(seg(x, 2, EdgeAxis::Horizontal, WallRole::Run, "stone"));
        }
        for y in 0..2 {
            l.walls
                .push(seg(0, y, EdgeAxis::Vertical, WallRole::Run, "stone"));
            l.walls
                .push(seg(4, y, EdgeAxis::Vertical, WallRole::Run, "stone"));
            l.walls
                .push(seg(2, y, EdgeAxis::Vertical, WallRole::Run, "timber"));
        }
        l.walls
            .push(seg(1, 2, EdgeAxis::Horizontal, WallRole::Door, "stone"));
        let (edges, joints) = assemble(&l);
        assert_eq!(edges.len(), 14);
        assert!(edges
            .iter()
            .any(|e| e.segment.kind == WallRole::Door && e.centre2 == (3, 4)));
        let at = |v| {
            joints
                .iter()
                .find(|j| j.vertex == v)
                .map(|j| (j.role, j.turns, j.kit))
        };
        assert_eq!(at((0, 0)), Some((WallRole::Corner, 0, "stone")));
        assert_eq!(at((4, 0)), Some((WallRole::Corner, 1, "stone")));
        assert_eq!(at((4, 2)), Some((WallRole::Corner, 2, "stone")));
        assert_eq!(at((0, 2)), Some((WallRole::Corner, 3, "stone")));
        assert_eq!(at((2, 0)), Some((WallRole::Tee, 0, "stone")));
        assert_eq!(at((2, 2)), Some((WallRole::Tee, 2, "timber")));
        assert_eq!(at((2, 1)), Some((WallRole::Post, 1, "timber")));
        assert_eq!(at((1, 0)), Some((WallRole::Post, 0, "stone")));
    }
}
