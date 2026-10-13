//! Labels on relief levels: tiles join where a name crosses their edge,
//! and a level below a label's zoom is left as it was.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    missing_docs
)]

use arda_midzoom::labels::{draw_tile, draw_window, labeler};
use arda_midzoom::{Pyramid, Rgba};
use arda_settle::gazetteer::{GazetteerFeature, GazetteerFile, PlaceKind};

fn blank(w: u32, h: u32) -> Rgba {
    Rgba {
        width: w,
        height: h,
        pixels: [70_u8, 110, 60, 255].repeat((w * h) as usize),
    }
}

fn changed(r: &Rgba) -> usize {
    r.pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[..3] != [70, 110, 60])
        .count()
}

fn world() -> (Pyramid, GazetteerFile) {
    let pyramid = Pyramid {
        max_zoom: 4,
        areas_wide: 2,
        areas_high: 2,
    };
    // 102.4 km square; a lake's name centred on the middle.
    let file = GazetteerFile {
        format_version: 1,
        features: vec![GazetteerFeature {
            id: 1,
            kind: PlaceKind::Lake,
            name: "Mirrormere".to_string(),
            name_gloss: "Mirrormere".to_string(),
            culture: "coastal".to_string(),
            anchor_m: [51_200, 51_200],
            path_m: None,
            extent_m: [40_000, 45_000, 62_400, 57_000],
            area_ha: 0,
            length_m: 0,
            rank: 3,
            min_zoom: 7,
        }],
    };
    (pyramid, file)
}

#[test]
fn relief_tiles_join_across_a_label_and_hide_below_its_zoom() {
    let (pyramid, file) = world();
    let l = labeler(&pyramid, &file, &[]).unwrap();
    // z = 9: 102.4 km over 131072 px; the middle is pixel 65536, a tile edge.
    let z = 9;
    let (tx, ty) = (255_u32, 255_u32);
    let mut whole = blank(512, 512);
    draw_window(
        &l,
        z,
        (i64::from(tx) * 256, i64::from(ty) * 256),
        &mut whole,
    );
    assert!(changed(&whole) > 100, "the name is drawn");
    for (dx, dy) in [(0_u32, 0_u32), (1, 0), (0, 1), (1, 1)] {
        let mut t = blank(256, 256);
        draw_tile(&l, z, tx + dx, ty + dy, &mut t);
        for row in 0..256_usize {
            let a = &t.pixels[row * 256 * 4..(row + 1) * 256 * 4];
            let w = ((dy as usize * 256 + row) * 512 + dx as usize * 256) * 4;
            assert_eq!(
                a,
                &whole.pixels[w..w + 256 * 4],
                "tile +{dx},+{dy} row {row}"
            );
        }
    }
    let mut low = blank(256, 256);
    draw_tile(&l, 6, 31, 31, &mut low);
    assert_eq!(changed(&low), 0, "below min_zoom");
}
