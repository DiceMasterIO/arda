//! Compositor tests: determinism, blend geometry, anchors and layering.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use super::*;
use crate::layout::{AssetRef, Placement};
use crate::layouts;
use std::path::Path;

fn lib() -> Library {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
    Library::load(&dir).unwrap()
}

fn small(grid: bool) -> RenderOptions {
    RenderOptions {
        ppsq: 32,
        grid,
        lighting: true,
    }
}

fn hash(img: &Rgba) -> blake3::Hash {
    blake3::hash(&img.data)
}

#[test]
fn render_is_byte_identical_twice_and_seed_sensitive() {
    let lib = lib();
    for l in layouts::all() {
        let a = render(&l, &lib, 7, &small(true)).unwrap();
        let b = render(&l, &lib, 7, &small(true)).unwrap();
        assert_eq!(hash(&a), hash(&b), "{} is not deterministic", l.name);
        assert_eq!((a.width, a.height), (l.width * 32, l.height * 32));
    }
    let l = layouts::by_name("timber_house").unwrap();
    let a = render(&l, &lib, 7, &small(false)).unwrap();
    let c = render(&l, &lib, 8, &small(false)).unwrap();
    assert_ne!(hash(&a), hash(&c));
}

#[test]
fn output_is_opaque_and_the_grid_changes_it() {
    let lib = lib();
    let l = layouts::by_name("wall_junctions").unwrap();
    let plain = render(&l, &lib, 1, &small(false)).unwrap();
    assert!(plain.data.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    let grid = render(&l, &lib, 1, &small(true)).unwrap();
    assert!(grid.get(0, 5)[0] < plain.get(0, 5)[0], "grid line darkens");
    assert_eq!(
        grid.get(16, 16),
        plain.get(16, 16),
        "square interiors untouched"
    );
}

#[test]
fn rejects_out_of_range_ppsq_and_unknown_assets() {
    let lib = lib();
    let mut l = layouts::by_name("wall_junctions").unwrap();
    let opts = RenderOptions {
        ppsq: 4,
        ..small(false)
    };
    assert!(matches!(
        render(&l, &lib, 1, &opts),
        Err(TacticalError::Options(_))
    ));
    l.placements.push(Placement {
        asset: AssetRef::Id("prop.dragon".into()),
        x: 1.0,
        y: 1.0,
        rotation: 0,
        mirror: false,
    });
    assert!(matches!(
        render(&l, &lib, 1, &small(false)),
        Err(TacticalError::Layout { .. })
    ));
}

/// Boundary statistics for a left/right split at column `split`: per-row
/// x of the first type change, and the share of all type changes (between
/// horizontal neighbours) that fall within one pixel of a grid line.
fn boundary_stats(map: &[usize], w: usize, h: usize, ppsq: usize) -> (f32, f32) {
    let (mut xs, mut changes, mut on_grid) = (Vec::new(), 0usize, 0usize);
    for y in 0..h {
        let row = &map[y * w..(y + 1) * w];
        if let Some(x) = (1..w).find(|&x| row[x] != row[x - 1]) {
            xs.push(x as f32);
        }
        for x in 1..w {
            if row[x] != row[x - 1] {
                changes += 1;
                let m = x % ppsq;
                on_grid += usize::from(m <= 1 || m >= ppsq - 1);
            }
        }
    }
    let mean = xs.iter().sum::<f32>() / xs.len() as f32;
    let sd = (xs.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / xs.len() as f32).sqrt();
    (sd / ppsq as f32, on_grid as f32 / changes.max(1) as f32)
}

#[test]
fn ground_blends_are_not_grid_aligned() {
    let lib = lib();
    let ppsq = 32u32;
    let textures = ground::TextureSet::new(&lib, ppsq);
    // A straight, grid-aligned split in the input.
    let mut split = TacticalLayout::new("split", 12, 12, "grass");
    for y in 0..12 {
        for x in 6..12 {
            split.square_mut(x, y).unwrap().ground = "dirt".into();
        }
    }
    let (w, h, p) = (12 * 32, 12 * 32, 32);
    // The metric flags a naive per-square map as fully grid-aligned.
    let naive: Vec<usize> = (0..w * h).map(|i| usize::from(i % w >= 6 * p)).collect();
    let (sd0, grid0) = boundary_stats(&naive, w, h, p);
    assert!(sd0 == 0.0 && grid0 == 1.0);
    for seed in [1, 2, 3] {
        let map = ground::dominant_ground(&split, &textures, seed, ppsq).unwrap();
        let (sd, grid_share) = boundary_stats(&map, w, h, p);
        assert!(
            sd > 0.12,
            "seed {seed}: border wanders only {sd:.3} squares"
        );
        // Uniform placement would put 3/32 ≈ 9 % of changes on grid lines.
        assert!(
            grid_share < 0.2,
            "seed {seed}: {grid_share:.2} of border pixels on grid lines"
        );
    }
    // A checkerboard: many borders in both directions.
    let mut checker = TacticalLayout::new("checker", 12, 12, "grass");
    for y in 0..12 {
        for x in 0..12 {
            if (x / 2 + y / 2) % 2 == 1 {
                checker.square_mut(x, y).unwrap().ground = "sand".into();
            }
        }
    }
    let map = ground::dominant_ground(&checker, &textures, 5, ppsq).unwrap();
    let (_, grid_share) = boundary_stats(&map, w, h, p);
    assert!(
        grid_share < 0.2,
        "checkerboard: {grid_share:.2} on grid lines"
    );
}

#[test]
fn floors_stop_at_walls() {
    let lib = lib();
    let l = layouts::by_name("wall_junctions").unwrap();
    let textures = ground::TextureSet::new(&lib, 32);
    let map = ground::dominant_ground(&l, &textures, 3, 32).unwrap();
    let w = 16 * 32;
    // Stone floor (index 2: cobbles, dirt, stone_floor) inside the rooms,
    // never outside them: sample just outside each room wall.
    let stone = 2;
    for (x, y) in [(20, 5 * 32 - 3), (9 * 32 - 3, 6 * 32), (4 * 32, 8 * 32 + 3)] {
        assert_ne!(map[y * w + x], stone, "floor leaked to ({x}, {y})");
    }
    assert_eq!(map[(6 * 32 + 16) * w + 2 * 32 + 16], stone);
}

#[test]
fn anchors_follow_rotation_and_mirroring() {
    let lib = lib();
    let cart = lib.asset("prop.cart").unwrap(); // 1 × 2, anchor at centre
    assert_eq!(anchor_origin(cart, 3.0, 3.0, 0, false, 32), (80, 64));
    // Rotated a quarter turn it is 2 × 1, still centred on (3, 3).
    assert_eq!(anchor_origin(cart, 3.0, 3.0, 1, false, 32), (64, 80));
    let mut odd = cart.clone();
    odd.anchor = Some(crate::catalog::Anchor { x: 0.0, y: 0.0 });
    assert_eq!(anchor_origin(&odd, 3.0, 3.0, 0, false, 32), (96, 96));
    // Mirroring moves the pivot to the top-right, turning puts it top-right
    // of a 2 × 1 image: (h - y, x) = (2, 0)… then mirrored input gives (2, 1).
    assert_eq!(anchor_origin(&odd, 3.0, 3.0, 0, true, 32), (64, 96));
    assert_eq!(anchor_origin(&odd, 3.0, 3.0, 1, false, 32), (32, 96));
}

#[test]
fn walls_draw_over_props_and_canopies_over_walls() {
    let lib = lib();
    let mut l = TacticalLayout::new("order", 4, 4, "dirt");
    l.walls.push(crate::layout::WallSegment {
        x: 1,
        y: 2,
        axis: crate::layout::EdgeAxis::Horizontal,
        kind: crate::catalog::WallRole::Run,
        kit: "stone".into(),
        tags: Vec::new(),
    });
    l.placements.push(Placement {
        asset: AssetRef::Id("prop.crate".into()),
        x: 1.5,
        y: 2.0,
        rotation: 0,
        mirror: false,
    });
    let opts = RenderOptions {
        lighting: false,
        ..small(false)
    };
    let wall_only = {
        let mut w = l.clone();
        w.placements.clear();
        render(&w, &lib, 1, &opts).unwrap()
    };
    let both = render(&l, &lib, 1, &opts).unwrap();
    // On the wall's centre line the wall wins over the crate beneath it.
    assert_eq!(both.get(48, 64), wall_only.get(48, 64));
    l.placements.push(Placement {
        asset: AssetRef::Id("veg.tree_oak".into()),
        x: 1.5,
        y: 2.0,
        rotation: 0,
        mirror: false,
    });
    let tree = render(&l, &lib, 1, &opts).unwrap();
    assert_ne!(tree.get(48, 64), wall_only.get(48, 64));
}

#[test]
fn oversized_canvases_are_refused_before_allocation() {
    // goal-prompt §8: limits refuse the run; nothing is silently allocated.
    let lib = lib();
    let big = TacticalLayout::new("big", 64, 64, "grass");
    let opts = RenderOptions {
        ppsq: 1024,
        grid: false,
        lighting: true,
    };
    assert!(matches!(
        render(&big, &lib, 1, &opts),
        Err(TacticalError::ResourceLimit(_))
    ));
    // Pixel dimensions that overflow u32 are an error, not a panic or a wrap.
    let mut wide = TacticalLayout::new("wide", 1, 1, "grass");
    wide.width = 5_000_000;
    assert!(matches!(
        canvas_size(&wide, 1024),
        Err(TacticalError::ResourceLimit(_))
    ));
    assert_eq!(canvas_size(&big, 32).unwrap(), (2048, 2048));
}

/// Goal 50: a region render equals the crop of the full render, pixel for
/// pixel, for regions on and off the square grid and at the canvas edge.
#[test]
fn region_renders_equal_crops_of_the_full_render() {
    let lib = lib();
    for name in ["riverside", "timber_house", "forest_glade"] {
        let l = layouts::by_name(name).unwrap();
        let opts = small(false);
        let full = render(&l, &lib, 7, &opts).unwrap();
        let (w, h) = (full.width, full.height);
        for [x, y, rw, rh] in [
            [32, 64, 96, 64],
            [5, 7, 41, 33],
            [w - 40, h - 30, 40, 30],
            [0, 0, w, h],
        ] {
            let part = render_region(&l, &lib, 7, &opts, [x, y, rw, rh]).unwrap();
            assert_eq!((part.width, part.height), (rw, rh));
            for r in 0..rh {
                let from = ((y + r) * w + x) as usize * 4;
                let row = rw as usize * 4;
                let at = (r * rw) as usize * 4;
                assert_eq!(
                    part.data[at..at + row],
                    full.data[from..from + row],
                    "{name} region {x},{y} {rw}x{rh} row {r}"
                );
            }
        }
        assert!(render_region(&l, &lib, 7, &opts, [w - 1, 0, 2, 1]).is_err());
    }
}

/// Goal 50: the library's cached texture set equals a freshly built one.
#[test]
fn cached_texture_sets_equal_fresh_ones() {
    let lib = lib();
    let cached = lib.texture_set(32);
    let fresh = ground::TextureSet::new(&lib, 32);
    for key in ["grass", "cobbles", "water_shallow"] {
        let (a, b) = (cached.get(key).unwrap(), fresh.get(key).unwrap());
        assert_eq!(a.variants, b.variants, "{key}");
    }
    assert!(std::sync::Arc::ptr_eq(&cached, &lib.texture_set(32)));
}

/// Rows of the centre column a lone horizontal timber run changes over a
/// plain render, with or without the `partition` tag.
fn wall_rows(partition: bool) -> usize {
    use crate::catalog::WallRole;
    use crate::layout::{EdgeAxis, WallSegment, TAG_PARTITION};
    let opts = RenderOptions {
        ppsq: 64,
        grid: false,
        lighting: false,
    };
    let lib = lib();
    let empty = TacticalLayout::new("wall", 3, 2, "dirt");
    let mut l = empty.clone();
    l.walls.push(WallSegment {
        x: 1,
        y: 1,
        axis: EdgeAxis::Horizontal,
        kind: WallRole::Run,
        kit: "timber".into(),
        tags: if partition {
            vec![TAG_PARTITION.into()]
        } else {
            Vec::new()
        },
    });
    let a = render(&empty, &lib, 7, &opts).unwrap();
    let b = render(&l, &lib, 7, &opts).unwrap();
    (0..a.height)
        .filter(|&y| a.get(96, y) != b.get(96, y))
        .count()
}

#[test]
fn a_partition_is_drawn_about_half_as_thick_as_an_outer_wall() {
    let (outer, inner) = (wall_rows(false), wall_rows(true));
    assert!(inner > 0, "the partition is drawn");
    assert!(
        inner * 10 <= outer * 7 && inner * 10 >= outer * 3,
        "outer {outer} rows, partition {inner} rows"
    );
}

#[test]
fn a_partition_uses_dedicated_partition_art_when_the_library_has_it() {
    use crate::catalog::WallRole;
    use crate::layout::{EdgeAxis, WallSegment, TAG_PARTITION};
    let base = lib();
    let mut cat = base.catalog.clone();
    let mut images: std::collections::BTreeMap<String, Rgba> = cat
        .assets
        .iter()
        .map(|a| (a.id.clone(), base.image(&a.id).unwrap().clone()))
        .collect();
    let extra: Vec<_> = cat
        .assets
        .iter()
        .filter(|a| a.wall.as_ref().is_some_and(|w| w.kit == "timber"))
        .cloned()
        .collect();
    for mut a in extra {
        let src = &images[&a.id];
        a.id = a.id.replace("wall.timber.", "wall.timber_partition.");
        if let Some(w) = a.wall.as_mut() {
            w.kit = "timber_partition".into();
        }
        // The same alpha, painted pure red.
        let mut red = src.clone();
        for y in 0..red.height {
            for x in 0..red.width {
                let alpha = red.get(x, y)[3];
                red.set(x, y, [255, 0, 0, alpha]);
            }
        }
        images.insert(a.id.clone(), red);
        cat.assets.push(a);
    }
    let lib = Library::from_parts(cat, images).unwrap();
    let mut l = TacticalLayout::new("wall", 3, 2, "dirt");
    l.walls.push(WallSegment {
        x: 1,
        y: 1,
        axis: EdgeAxis::Horizontal,
        kind: WallRole::Run,
        kit: "timber".into(),
        tags: vec![TAG_PARTITION.into()],
    });
    let opts = RenderOptions {
        ppsq: 64,
        grid: false,
        lighting: false,
    };
    let img = render(&l, &lib, 7, &opts).unwrap();
    assert_eq!(img.get(96, 64)[..3], [255, 0, 0]);
}
