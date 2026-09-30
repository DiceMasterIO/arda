//! Wall merging, doors, windows and line of sight.
#![allow(clippy::unwrap_used, missing_docs)]

mod common;

use arda_scene::{Sq, WallKind};
use arda_tactical::catalog::WallRole;
use common::L;

#[test]
fn collinear_runs_merge_into_one_polyline() {
    let mut l = L::new(8, 6);
    for x in 1..6 {
        l = l.h(x, 2, WallRole::Run);
    }
    for y in 0..3 {
        l = l.v(7, y, WallRole::Run);
    }
    let s = l.scene();
    assert_eq!(s.walls.len(), 2);
    assert_eq!(s.walls[0].points, vec![[1, 2], [6, 2]]);
    assert_eq!(s.walls[1].points, vec![[7, 0], [7, 3]]);
    assert!(s
        .walls
        .iter()
        .all(|w| w.kind == WallKind::Wall && w.blocks_sight));
}

#[test]
fn a_punched_door_splits_the_run_and_windows_stay_separate() {
    let mut l = L::new(8, 4);
    for x in 0..6 {
        l = l.h(x, 1, WallRole::Run);
    }
    let s = l.h(2, 1, WallRole::Door).h(4, 1, WallRole::Window).scene();
    let got: Vec<_> = s.walls.iter().map(|w| (w.kind, w.points.clone())).collect();
    assert_eq!(
        got,
        vec![
            (WallKind::Wall, vec![[0, 1], [2, 1]]),
            (WallKind::Door, vec![[2, 1], [3, 1]]),
            (WallKind::Wall, vec![[3, 1], [4, 1]]),
            (WallKind::Window, vec![[4, 1], [5, 1]]),
            (WallKind::Wall, vec![[5, 1], [6, 1]]),
        ]
    );
    assert_eq!(s.walls[1].open, Some(false));
    assert_eq!(s.walls[0].open, None);
}

#[test]
fn adjacent_doors_do_not_merge() {
    let s = L::new(5, 3)
        .h(1, 1, WallRole::Door)
        .h(2, 1, WallRole::Door)
        .scene();
    assert_eq!(s.walls.len(), 2);
}

/// A 7 × 5 map split by a wall along x = 3 with a door at (3, 2).
fn door_map() -> arda_scene::Scene {
    let mut l = L::new(7, 5);
    for y in 0..5 {
        l = l.v(3, y, WallRole::Run);
    }
    l.v(3, 2, WallRole::Door).scene()
}

#[test]
fn a_closed_door_blocks_sight_and_movement() {
    let s = door_map();
    let door = s
        .walls
        .iter()
        .position(|w| w.kind == WallKind::Door)
        .unwrap();
    let w = &s.walls[door];
    assert!(w.blocks_sight && w.blocks_movement && w.blocks_light);
    assert!(!s.line_of_sight(Sq(1, 2), Sq(5, 2)));
    assert_eq!(s.path_cost(Sq(1, 2), Sq(5, 2)), None);
}

#[test]
fn an_open_door_blocks_nothing() {
    let mut s = door_map();
    let door = s
        .walls
        .iter()
        .position(|w| w.kind == WallKind::Door)
        .unwrap();
    s.set_open(door, true).unwrap();
    let w = &s.walls[door];
    assert!(!w.blocks_sight && !w.blocks_movement && !w.blocks_light);
    assert!(s.line_of_sight(Sq(1, 2), Sq(5, 2)));
    assert_eq!(s.path_cost(Sq(1, 2), Sq(5, 2)), Some(20));
    // Closing it again restores the block; plain walls refuse to open.
    s.set_open(door, false).unwrap();
    assert!(!s.line_of_sight(Sq(1, 2), Sq(5, 2)));
    assert!(s.set_open(0, true).is_err());
    assert!(s.set_open(99, true).is_err());
}

#[test]
fn a_closed_window_blocks_movement_but_not_sight() {
    let mut l = L::new(7, 3);
    for y in 0..3 {
        l = l.v(3, y, WallRole::Run);
    }
    let s = l.v(3, 1, WallRole::Window).scene();
    let win = s.walls.iter().find(|w| w.kind == WallKind::Window).unwrap();
    assert!(!win.blocks_sight && win.blocks_movement && !win.blocks_light);
    assert!(s.line_of_sight(Sq(1, 1), Sq(5, 1)));
    assert_eq!(s.path_cost(Sq(1, 1), Sq(5, 1)), None);
    assert_eq!(
        s.cover_between(Sq(1, 1), Sq(4, 1)),
        arda_scene::CoverLevel::ThreeQuarters
    );
}

#[test]
fn sight_may_graze_a_wall_end() {
    // One wall edge ending at vertex (2, 2); the diagonal passes its tip.
    let s = L::new(5, 5).v(2, 1, WallRole::Run).scene();
    assert!(s.line_of_sight(Sq(1, 1), Sq(2, 2)));
    assert!(s.line_of_sight(Sq(0, 0), Sq(4, 4)));
}

#[test]
fn sight_cannot_slip_through_a_closed_corner() {
    // Arms north and west of vertex (2, 2) enclose square (1, 1).
    let s = L::new(5, 5)
        .v(2, 1, WallRole::Run)
        .h(1, 2, WallRole::Run)
        .scene();
    assert!(!s.line_of_sight(Sq(1, 1), Sq(2, 2)));
    assert!(!s.line_of_sight(Sq(0, 0), Sq(3, 3)));
}

#[test]
fn sight_passes_the_outside_of_a_corner() {
    // Arms north and east of vertex (2, 2) enclose square (2, 1).
    let s = L::new(5, 5)
        .v(2, 1, WallRole::Run)
        .h(2, 2, WallRole::Run)
        .scene();
    assert!(s.line_of_sight(Sq(1, 1), Sq(2, 2)));
    assert!(!s.line_of_sight(Sq(1, 1), Sq(2, 1)));
}

#[test]
fn walls_are_intersected_exactly() {
    // A wall edge on x = 3 spanning y 1..2. The line (0.5,0.5)->(6.5,2.5)
    // crosses x = 3 at y = 1.333, inside it; the line to (6.5, 3.5) crosses
    // at y = 1.75, still inside; to (6.5, 4.5) crosses at y = 2.1667, past it.
    let s = L::new(7, 5).v(3, 1, WallRole::Run).scene();
    assert!(!s.line_of_sight(Sq(0, 0), Sq(6, 2)));
    assert!(!s.line_of_sight(Sq(0, 0), Sq(6, 3)));
    assert!(s.line_of_sight(Sq(0, 0), Sq(6, 4)));
}

#[test]
fn visible_squares_stop_at_walls_and_radius() {
    let mut l = L::new(9, 9);
    for y in 0..9 {
        l = l.v(5, y, WallRole::Run);
    }
    let s = l.scene();
    let seen = s.visible_squares(Sq(2, 4), 100);
    assert!(seen.iter().all(|q| q.0 < 5));
    assert_eq!(seen.len(), 5 * 9);
    let near = s.visible_squares(Sq(2, 4), 5);
    assert_eq!(near.len(), 9);
    assert!(near.contains(&Sq(2, 4)));
}
