//! The picture agrees with the rules: a square the compositor lights is a
//! square the scene's line of sight reaches from the light.
#![allow(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

mod common;

use arda_scene::Sq;
use arda_tactical::catalog::WallRole;
use arda_tactical::{render, RenderOptions};
use common::{lib, L};

const P: u32 = 32;

/// Two rooms and a corridor: a door and a window in the middle wall, a
/// partition with a gap, lights on square centres and one on a corner.
fn fixture() -> L {
    let (w, h) = (20u32, 12u32);
    let mut l = L::new(w, h);
    for x in 0..w {
        l = l.h(x, 0, WallRole::Run).h(x, h, WallRole::Run);
    }
    for y in 0..h {
        l = l.v(0, y, WallRole::Run).v(w, y, WallRole::Run);
    }
    for y in 0..h {
        let kind = match y {
            3 => WallRole::Door,
            8 => WallRole::Window,
            _ => WallRole::Run,
        };
        l = l.v(10, y, kind);
    }
    for x in 11..w {
        let kind = if x == 15 {
            WallRole::Gate
        } else {
            WallRole::Run
        };
        l = l.h(x, 6, kind);
    }
    l
}

#[test]
fn lit_squares_are_squares_the_scene_sees() {
    let base = fixture();
    let dark = render(
        &base.0,
        lib(),
        5,
        &RenderOptions {
            ppsq: P,
            grid: false,
            lighting: true,
        },
    )
    .unwrap();
    let scene = base.scene();
    let idx = scene.queries();
    let mut checked = 0;
    for (lx, ly, ft) in [
        (4.5, 3.5, 30u16),
        (13.5, 3.5, 30),
        (13.5, 9.5, 40),
        (8.5, 8.5, 30),
        (6.5, 6.5, 20),
    ] {
        let mut one = base.0.clone();
        one.lights.clear();
        let with = {
            let mut l = L(one);
            l = l.light(lx, ly, ft);
            render(
                &l.0,
                lib(),
                5,
                &RenderOptions {
                    ppsq: P,
                    grid: false,
                    lighting: true,
                },
            )
            .unwrap()
        };
        let from = Sq(lx as u32, ly as u32);
        // The scene measures grid distance (5-10-5 diagonals), the picture a
        // euclidean disc, which never exceeds the grid distance by more
        // than a quarter.
        let seen = idx.visible_squares(from, u32::from(ft) * 5 / 4);
        for y in 0..12u32 {
            for x in 0..20u32 {
                let (px, py) = (x * P + P / 2, y * P + P / 2);
                let (a, b) = (with.get(px, py), dark.get(px, py));
                let delta: i32 = (0..3)
                    .map(|c| (i32::from(a[c]) - i32::from(b[c])).abs())
                    .sum();
                if delta >= 6 {
                    checked += 1;
                    assert!(
                        seen.contains(&Sq(x, y)),
                        "light {lx},{ly}: square {x},{y} is lit but not visible"
                    );
                }
            }
        }
    }
    assert!(checked > 100, "only {checked} lit squares compared");
}

#[test]
fn walls_share_the_blocking_rule_with_the_picture() {
    use arda_tactical::layout::{EdgeAxis, WallSegment};
    let s = fixture().scene();
    for w in &s.walls {
        let seg = WallSegment {
            x: 0,
            y: 0,
            axis: EdgeAxis::Horizontal,
            kind: match w.kind {
                arda_scene::WallKind::Window => WallRole::Window,
                arda_scene::WallKind::Door => WallRole::Door,
                arda_scene::WallKind::Gate => WallRole::Gate,
                _ => WallRole::Run,
            },
            kit: "stone".into(),
            tags: Vec::new(),
        };
        assert_eq!(w.blocks_light, seg.blocks_light(lib()), "{:?}", w.kind);
    }
}
