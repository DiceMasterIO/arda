//! Solver behaviour: legality, determinism, anchors, limits, repairs and
//! clean failure.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_wfc::{
    solve, violations, Anchor, Dir, Failure, Limit, Problem, Rules, TileSet, Weight, MAX_ATTEMPTS,
};

/// Tiles 0 = grass, 1 = road, 2 = shore; road never touches shore.
fn rules() -> Rules {
    let mut r = Rules::new(3, 1).unwrap();
    for d in Dir::ALL {
        for (a, b) in [(0, 0), (0, 1), (0, 2), (1, 1), (2, 2)] {
            r.allow(a, d, b, 0);
        }
    }
    r
}

fn problem<'a>(r: &'a Rules, w: Weight<'a>) -> Problem<'a> {
    let n = 12 * 9;
    let mut domains = vec![TileSet::first_n(3); n];
    domains[0] = TileSet::single(1);
    domains[n - 1] = TileSet::single(2);
    Problem {
        rules: r,
        w: 12,
        h: 9,
        domains,
        edges: vec![[0; 4]; n],
        weight: w,
        anchors: vec![Anchor {
            options: (0..n).map(|i| (i, TileSet::single(2))).collect(),
            count: 3,
        }],
        limits: vec![Limit {
            tiles: TileSet::single(1),
            cells: Vec::new(),
            max: 10,
        }],
        seed: 42,
        key: [1, 2, 3],
    }
}

#[test]
fn solutions_are_legal_deterministic_and_honour_anchors_and_limits() {
    let r = rules();
    let w = |_: usize, t: usize| [5, 3, 1][t];
    let p = problem(&r, &w);
    let a = solve(&p, MAX_ATTEMPTS, &|_| true).unwrap();
    let b = solve(&p, MAX_ATTEMPTS, &|_| true).unwrap();
    assert_eq!(a, b);
    assert!(violations(&p, &a.tiles).is_empty());
    assert_eq!(a.tiles[0], 1);
    assert!(a.tiles.iter().filter(|&&t| t == 1).count() <= 10);
    assert!(a.tiles.iter().filter(|&&t| t == 2).count() >= 3);
}

#[test]
fn another_seed_gives_another_grid() {
    let r = rules();
    let w = |_: usize, _: usize| 1;
    let mut p = problem(&r, &w);
    let a = solve(&p, MAX_ATTEMPTS, &|_| true).unwrap();
    p.seed = 43;
    let b = solve(&p, MAX_ATTEMPTS, &|_| true).unwrap();
    assert_ne!(a.tiles, b.tiles);
}

#[test]
fn edge_kinds_restrict_tiles() {
    let mut r = rules();
    // Road may not touch the grid's border (kind 0 everywhere here, so
    // forbid it on every side): the fixed road corner cannot stand.
    for d in Dir::ALL {
        r.forbid_edge(1, d, 0);
    }
    let w = |_: usize, _: usize| 1;
    let p = problem(&r, &w);
    assert!(solve(&p, 2, &|_| true).is_err());
}

#[test]
fn a_failing_check_exhausts_the_attempts() {
    let r = rules();
    let w = |_: usize, _: usize| 1;
    let p = problem(&r, &w);
    assert_eq!(
        solve(&p, 3, &|_| false),
        Err(Failure {
            attempts: 3,
            rejected: 3
        })
    );
}

#[test]
fn an_impossible_problem_fails_cleanly() {
    let r = rules();
    let w = |_: usize, _: usize| 1;
    let mut p = problem(&r, &w);
    p.domains[1] = TileSet::single(2);
    assert!(solve(&p, 2, &|_| true).is_err());
}
