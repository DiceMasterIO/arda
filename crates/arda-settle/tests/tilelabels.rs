//! Tile labels: placed once per level and cut into tiles, so a label that
//! crosses a tile edge is the same pixels either way.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_settle::gazetteer::{GazetteerFeature, GazetteerFile, PlaceKind};
use arda_settle::render::tilelabels::Labeler;

const WORLD: f64 = 100_000.0;

fn feature(id: u64, kind: PlaceKind, name: &str, anchor: [i64; 2], rank: u8, zoom: u8) -> GazetteerFeature {
    GazetteerFeature {
        id,
        kind,
        name: name.to_string(),
        name_gloss: name.to_string(),
        culture: "coastal".to_string(),
        anchor_m: anchor,
        path_m: None,
        extent_m: [20_000, 30_000, 80_000, 70_000],
        area_ha: 0,
        length_m: 0,
        rank,
        min_zoom: zoom,
    }
}

fn labeler(features: Vec<GazetteerFeature>) -> Labeler {
    let file = GazetteerFile {
        format_version: 1,
        features,
    };
    Labeler::new(&file, &[], [WORLD, WORLD], WORLD).unwrap()
}

fn blank(w: usize, h: usize) -> Vec<u8> {
    [90_u8, 120, 90, 255].repeat(w * h)
}

fn changed(buf: &[u8]) -> usize {
    buf.chunks_exact(4).filter(|p| p[..3] != [90, 120, 90]).count()
}

#[test]
fn a_label_across_a_tile_edge_joins_exactly() {
    let l = labeler(vec![feature(1, PlaceKind::Sea, "Dawnsea", [50_000, 50_000], 1, 0)]);
    let z = 4;
    let level = l.level(z);
    // The anchor is at pixel (2048, 2048): the corner of four tiles.
    let (x0, y0) = (1792_i64, 1792_i64);
    let mut whole = blank(512, 512);
    l.draw_rgba(&level, (x0, y0), (512, 512), &mut whole);
    assert!(changed(&whole) > 150, "the name is drawn");
    for (tx, ty) in [(0_usize, 0_usize), (1, 0), (0, 1), (1, 1)] {
        let mut tile = blank(256, 256);
        let origin = (x0 + 256 * tx as i64, y0 + 256 * ty as i64);
        l.draw_rgba(&level, origin, (256, 256), &mut tile);
        assert!(changed(&tile) > 0, "tile {tx},{ty} holds part of the name");
        for row in 0..256 {
            let a = &tile[row * 256 * 4..(row + 1) * 256 * 4];
            let wr = ((ty * 256 + row) * 512 + tx * 256) * 4;
            assert_eq!(a, &whole[wr..wr + 256 * 4], "tile {tx},{ty} row {row}");
        }
    }
}

#[test]
fn labels_hide_below_their_zoom_and_never_overlap() {
    let l = labeler(vec![
        feature(1, PlaceKind::Sea, "Dawnsea", [50_000, 50_000], 1, 3),
        feature(2, PlaceKind::Lake, "Dawnsea", [51_000, 50_500], 4, 3),
    ]);
    let tile = |z: u8| {
        let mut b = blank(256, 256);
        let level = l.level(z);
        let n = 1_i64 << z;
        let c = n / 2 * 256 - 128;
        l.draw_rgba(&level, (c, c), (256, 256), &mut b);
        changed(&b)
    };
    assert_eq!(tile(2), 0, "hidden below min_zoom");
    assert!(tile(4) > 100, "shown from min_zoom");
    // The lake's name would sit on the sea's: only one of them is placed.
    let both = tile(4);
    let one = labeler(vec![feature(1, PlaceKind::Sea, "Dawnsea", [50_000, 50_000], 1, 3)]);
    let mut b = blank(256, 256);
    let n = 1_i64 << 4;
    let c = n / 2 * 256 - 128;
    one.draw_rgba(&one.level(4), (c, c), (256, 256), &mut b);
    assert!(both >= changed(&b));
}

#[test]
fn layout_is_deterministic() {
    let fs = vec![
        feature(1, PlaceKind::Sea, "Dawnsea", [50_000, 50_000], 1, 0),
        feature(2, PlaceKind::Forest, "Greenwood", [30_000, 40_000], 4, 2),
    ];
    let (a, b) = (labeler(fs.clone()), labeler(fs));
    let (mut x, mut y) = (blank(512, 512), blank(512, 512));
    a.draw_rgba(&a.level(4), (1792, 1792), (512, 512), &mut x);
    b.draw_rgba(&b.level(4), (1792, 1792), (512, 512), &mut y);
    assert_eq!(x, y);
}
