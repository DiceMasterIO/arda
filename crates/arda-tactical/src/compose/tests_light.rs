//! Light occlusion: pools stop at walls, windows pass light, doors block,
//! open ground is unchanged and tiles join.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use super::light_los::{build, LightWalls};
use super::*;
use crate::catalog::WallRole;
use crate::layout::{EdgeAxis, LightSource, WallSegment};
use std::path::Path;

const P: u32 = 32;

fn lib() -> Library {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
    Library::load(&dir).unwrap()
}

fn opts() -> RenderOptions {
    RenderOptions {
        ppsq: P,
        grid: false,
        lighting: true,
    }
}

fn seg(axis: EdgeAxis, x: u32, y: u32, kind: WallRole) -> WallSegment {
    WallSegment {
        x,
        y,
        axis,
        kind,
        kit: "stone".into(),
        tags: Vec::new(),
    }
}

/// 12 × 8 grass with a light at (4.5, 4.5), radius 30 ft, and a wall along
/// x = 7 whose edge at row 4 is `gap` (or a plain run when `None`).
fn room(gap: Option<WallRole>) -> TacticalLayout {
    let mut l = TacticalLayout::new("room", 12, 8, "grass");
    for y in 0..8 {
        let kind = match (y, gap) {
            (4, Some(g)) => g,
            _ => WallRole::Run,
        };
        l.walls.push(seg(EdgeAxis::Vertical, 7, y, kind));
    }
    l.lights.push(LightSource {
        x: 4.5,
        y: 4.5,
        radius_ft: 30,
        colour: [255, 200, 120],
    });
    l
}

/// Pixels the light changed: the render with the light minus the render
/// without it.
fn lit(l: &TacticalLayout) -> Vec<bool> {
    let lib = lib();
    let with = render(l, &lib, 3, &opts()).unwrap();
    let mut dark = l.clone();
    dark.lights.clear();
    let without = render(&dark, &lib, 3, &opts()).unwrap();
    with.data
        .as_chunks::<4>()
        .0
        .iter()
        .zip(without.data.as_chunks::<4>().0)
        .map(|(a, b)| a != b)
        .collect()
}

fn at(m: &[bool], sx: f32, sy: f32) -> bool {
    let w = (12 * P) as usize;
    m[(sy * P as f32) as usize * w + (sx * P as f32) as usize]
}

#[test]
fn a_wall_stops_the_pool() {
    let m = lit(&room(None));
    assert!(at(&m, 5.5, 4.5) && at(&m, 6.2, 3.0), "near side is lit");
    // Beyond the feathered edge (a third of a square) nothing is lit, even
    // where the plain disc (radius 6 squares) would reach.
    for (x, y) in [
        (7.6, 4.5),
        (8.5, 4.5),
        (9.5, 4.5),
        (8.5, 3.0),
        (8.5, 6.0),
        (7.7, 2.5),
    ] {
        assert!(!at(&m, x, y), "light leaked to ({x}, {y})");
    }
    let mut open = room(None);
    open.walls.clear();
    assert!(at(&lit(&open), 8.5, 4.5), "without the wall it reaches");
}

#[test]
fn a_window_passes_light_and_a_closed_door_blocks() {
    let m = lit(&room(Some(WallRole::Window)));
    assert!(at(&m, 8.5, 4.5) && at(&m, 9.5, 4.5), "through the window");
    assert!(!at(&m, 8.5, 1.5), "beside the window stays dark");
    for blocker in [WallRole::Door, WallRole::Gate] {
        let m = lit(&room(Some(blocker)));
        assert!(!at(&m, 8.5, 4.5), "{blocker:?} blocks light");
    }
}

#[test]
fn light_on_a_wall_edge_belongs_to_the_south_east_square() {
    let mut l = room(None);
    l.lights[0].x = 7.0; // exactly on the wall line
    let m = lit(&l);
    assert!(at(&m, 8.5, 4.5), "east side lit");
    assert!(!at(&m, 5.5, 4.5), "west side dark");
}

#[test]
fn open_ground_keeps_the_plain_disc() {
    let lib = lib();
    let lay = room(None);
    let walls = LightWalls::new(&lay, &lib);
    // A light far from the wall has no mask: the old fast path.
    let far = Pool {
        x: 0,
        y: 0,
        r: 0,
        colour: [0; 3],
        at: (1.5, 1.5),
        radius_ft: 10,
        mask: None,
    };
    assert!(build(&walls, &far, P).is_none());
    // A lit map without walls is untouched; the hash was taken from the pre-occlusion compositor (PNG-identical); re-pinned for library 0.2.0 (the version is part of the render seed).
    let mut open = room(None);
    open.walls.clear();
    let a = render(&open, &lib, 3, &opts()).unwrap();
    assert_eq!(blake3::hash(&a.data).to_hex().as_str(), GOLDEN_OPEN);
}

const GOLDEN_OPEN: &str = "e04284ee89fe1ca55b48b67f0e4b9190134a6e9298fafd4d9232e173fff09a14";

#[test]
fn occluded_renders_are_deterministic() {
    let lib = lib();
    let l = room(Some(WallRole::Window));
    let a = render(&l, &lib, 3, &opts()).unwrap();
    let b = render(&l, &lib, 3, &opts()).unwrap();
    assert_eq!(a.data, b.data);
}

#[test]
fn clipped_tiles_agree_with_the_full_render() {
    let lib = lib();
    let l = room(Some(WallRole::Window));
    let full = render(&l, &lib, 3, &opts()).unwrap();
    // Tiles straddling the wall, the window and the pool's edge, off-grid.
    for [x, y, w, h] in [
        [100, 20, 150, 200],
        [180, 100, 97, 61],
        [0, 0, 384, 256],
        [230, 120, 100, 40],
    ] {
        let tile = render_region(&l, &lib, 3, &opts(), [x, y, w, h]).unwrap();
        for j in 0..h {
            for i in 0..w {
                assert_eq!(
                    tile.get(i, j),
                    full.get(x + i, y + j),
                    "({i}, {j}) of {x},{y}"
                );
            }
        }
    }
}
