//! Determinism and exact seams across windows.

#![allow(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use arda_fields::geom::SQUARE_M;
use arda_fields::synthetic::{self, Scenario};
use arda_fields::{generate, FieldsWindow};
use arda_tactical::layout::{EdgeAxis, TacticalLayout};
use std::collections::BTreeMap;

const SEED: u64 = 7;

fn window(s: &Scenario, dx: i64, dy: i64, w: u32, h: u32) -> (FieldsWindow, (i64, i64)) {
    #[allow(clippy::cast_precision_loss)]
    let o = [
        s.origin_m[0] + dx as f64 * SQUARE_M,
        s.origin_m[1] + dy as f64 * SQUARE_M,
    ];
    let win = generate(&s.inputs(), o, w, h, SEED).unwrap();
    let origin = (win.sidecar.origin_square[0], win.sidecar.origin_square[1]);
    (win, origin)
}

type SquareKey = (String, i16, u8);
type Global = (
    BTreeMap<(i64, i64), SquareKey>,
    BTreeMap<(EdgeAxis, i64, i64), String>,
    BTreeMap<(i64, i64, String), usize>,
);

/// Everything in a layout keyed by global coordinates.
fn global(l: &TacticalLayout, o: (i64, i64)) -> Global {
    let mut squares = BTreeMap::new();
    for (i, sq) in l.squares.iter().enumerate() {
        let (x, y) = (i as i64 % i64::from(l.width), i as i64 / i64::from(l.width));
        squares.insert(
            (o.0 + x, o.1 + y),
            (sq.ground.clone(), sq.elevation_ft, sq.water_depth_ft),
        );
    }
    let walls = l
        .walls
        .iter()
        .map(|w| {
            (
                (w.axis, o.0 + i64::from(w.x), o.1 + i64::from(w.y)),
                format!("{}:{:?}", w.kit, w.kind),
            )
        })
        .collect();
    let mut placements = BTreeMap::new();
    for p in &l.placements {
        // Placements are keyed by their global anchor in 1/64 squares.
        let key = (
            o.0 * 64 + (f64::from(p.x) * 64.0).round() as i64,
            o.1 * 64 + (f64::from(p.y) * 64.0).round() as i64,
            format!("{:?}/{}/{}", p.asset, p.rotation, p.mirror),
        );
        *placements.entry(key).or_insert(0) += 1;
    }
    (squares, walls, placements)
}

/// Asserts two windows agree on everything inside `(x0, y0)..(x1, y1)`.
fn agree(a: &Global, b: &Global, (x0, y0, x1, y1): (i64, i64, i64, i64)) {
    let inside = |x: i64, y: i64| x >= x0 && y >= y0 && x < x1 && y < y1;
    let sq = |g: &Global| {
        g.0.iter()
            .filter(|((x, y), _)| inside(*x, *y))
            .map(|(k, v)| (*k, v.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(sq(a), sq(b), "squares differ");
    let edge_in = |(axis, x, y): &(EdgeAxis, i64, i64)| match axis {
        EdgeAxis::Vertical => *x >= x0 && *x <= x1 && *y >= y0 && *y < y1,
        EdgeAxis::Horizontal => *x >= x0 && *x < x1 && *y >= y0 && *y <= y1,
    };
    let walls = |g: &Global| {
        g.1.iter()
            .filter(|(k, _)| edge_in(k))
            .map(|(k, v)| (*k, v.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(walls(a), walls(b), "walls differ");
    let props = |g: &Global| {
        g.2.iter()
            .filter(|((x, y, _), _)| inside(x.div_euclid(64), y.div_euclid(64)))
            .map(|(k, v)| (k.clone(), *v))
            .collect::<Vec<_>>()
    };
    assert_eq!(props(a), props(b), "placements differ");
}

#[test]
fn the_same_seed_gives_identical_windows() {
    let s = synthetic::watermill();
    let (a, _) = window(&s, 0, 0, 96, 96);
    let (b, _) = window(&s, 0, 0, 96, 96);
    assert_eq!(a.layout.to_json().unwrap(), b.layout.to_json().unwrap());
    assert_eq!(
        serde_json::to_string(&a.sidecar).unwrap(),
        serde_json::to_string(&b.sidecar).unwrap()
    );
    let c = generate(&s.inputs(), s.origin_m, 96, 96, SEED + 1).unwrap();
    assert_ne!(a.layout, c.layout, "the seed must matter");
}

#[test]
fn overlapping_windows_agree_exactly() {
    for s in [
        synthetic::hedge_country(),
        synthetic::watermill(),
        synthetic::quarry(),
    ] {
        let (a, oa) = window(&s, 0, 0, 128, 128);
        let (b, ob) = window(&s, 53, 37, 128, 128);
        let (ga, gb) = (global(&a.layout, oa), global(&b.layout, ob));
        agree(&ga, &gb, (ob.0, ob.1, oa.0 + 128, oa.1 + 128));
    }
}

#[test]
fn four_blocks_tile_their_parent_window() {
    let s = synthetic::orchard_farmstead();
    let (big, o) = window(&s, 0, 0, 128, 128);
    let gbig = global(&big.layout, o);
    for (dx, dy) in [(0, 0), (64, 0), (0, 64), (64, 64)] {
        let (part, op) = window(&s, dx, dy, 64, 64);
        agree(
            &gbig,
            &global(&part.layout, op),
            (op.0, op.1, op.0 + 64, op.1 + 64),
        );
    }
}

/// Per-square rules and field ids keyed by global square.
fn rules(w: &FieldsWindow, o: (i64, i64)) -> BTreeMap<(i64, i64), String> {
    let width = i64::from(w.layout.width);
    w.sidecar
        .squares
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let (x, y) = (i as i64 % width, i as i64 / width);
            let v = format!(
                "{}/{}/{:?}/{}/{}/{:?}/{:?}/{:?}",
                r.difficult,
                r.water_depth_ft,
                r.cover,
                r.blocks_sight,
                r.blocks_movement,
                r.field,
                r.crop,
                r.furrow
            );
            ((o.0 + x, o.1 + y), v)
        })
        .collect()
}

#[test]
fn overlapping_windows_agree_on_the_rules_sidecar() {
    // The exact-seam claim covers the sidecar (field ids, crops, SRD rules),
    // not only the layout.
    for s in [
        synthetic::hedge_country(),
        synthetic::watermill(),
        synthetic::quarry(),
    ] {
        let (a, oa) = window(&s, 0, 0, 128, 128);
        let (b, ob) = window(&s, 53, 37, 128, 128);
        let (ra, rb) = (rules(&a, oa), rules(&b, ob));
        let inside =
            |(x, y): &(i64, i64)| *x >= ob.0 && *y >= ob.1 && *x < oa.0 + 128 && *y < oa.1 + 128;
        let pick = |r: &BTreeMap<(i64, i64), String>| {
            r.iter()
                .filter(|(k, _)| inside(k))
                .map(|(k, v)| (*k, v.clone()))
                .collect::<Vec<_>>()
        };
        let (pa, pb) = (pick(&ra), pick(&rb));
        assert!(!pa.is_empty());
        let diff: Vec<_> = pa.iter().zip(&pb).filter(|(x, y)| x != y).take(3).collect();
        assert!(diff.is_empty(), "{}: {diff:?}", s.name);
    }
}
