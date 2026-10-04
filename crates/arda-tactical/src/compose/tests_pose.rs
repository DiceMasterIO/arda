//! Hashed vegetation pose: per-placement scale and diagonal transpose,
//! deterministic, seam-stable, varied, and off for `fixed_pose`.
#![allow(clippy::cast_precision_loss)]

use super::*;
use crate::layout::{AssetRef, Placement};
use std::collections::BTreeMap;
use std::path::Path;

/// The placeholder library with `fixed_pose` stripped from every asset, so
/// its vegetation takes hashed poses.
fn posed() -> Library {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
    let base = Library::load(&dir).unwrap();
    let mut cat = base.catalog.clone();
    for a in &mut cat.assets {
        a.tags.free.retain(|t| t != FIXED_POSE);
    }
    let images = cat
        .assets
        .iter()
        .map(|a| (a.id.clone(), base.image(&a.id).unwrap().clone()))
        .collect();
    Library::from_parts(cat, images).unwrap()
}

fn put(l: &mut TacticalLayout, id: &str, x: f32, y: f32) {
    l.placements.push(Placement {
        asset: AssetRef::Id(id.into()),
        x,
        y,
        rotation: 0,
        mirror: false,
    });
}

/// A `w × h` grass layout with one `id` per square.
fn field(id: &str, w: u32, h: u32) -> TacticalLayout {
    let mut l = TacticalLayout::new("field", w, h, "grass");
    for y in 0..h {
        for x in 0..w {
            put(&mut l, id, x as f32 + 0.5, y as f32 + 0.5);
        }
    }
    l
}

fn poses(lib: &Library, l: &TacticalLayout, seed: u64) -> Vec<(u8, bool)> {
    (0..l.placements.len())
        .map(|i| {
            let r = resolve(lib, l, i, seed).unwrap();
            (r.scale_pct, r.transpose)
        })
        .collect()
}

fn plain(ppsq: u32) -> RenderOptions {
    RenderOptions {
        ppsq,
        grid: false,
        lighting: false,
    }
}

#[test]
fn transpose_mirrors_across_the_main_diagonal() {
    let mut img = Rgba::new(3, 2);
    img.set(2, 0, [1, 2, 3, 255]);
    img.set(0, 1, [4, 5, 6, 255]);
    let t = img.transposed();
    assert_eq!((t.width, t.height), (2, 3));
    assert_eq!(t.get(0, 2), [1, 2, 3, 255]);
    assert_eq!(t.get(1, 0), [4, 5, 6, 255]);
    assert_eq!(t.transposed(), img);
}

#[test]
fn poses_are_deterministic() {
    let lib = posed();
    let mut l = field("veg.bush", 12, 12);
    l.origin = Some([300, 40]);
    assert_eq!(poses(&lib, &l, 7), poses(&lib, &l, 7));
    assert_ne!(poses(&lib, &l, 7), poses(&lib, &l, 8), "seed sensitive");
    let a = render(&l, &lib, 7, &plain(32)).unwrap();
    let b = render(&l, &lib, 7, &plain(32)).unwrap();
    assert_eq!(blake3::hash(&a.data), blake3::hash(&b.data));
}

#[test]
fn a_field_of_trees_varies_in_size_and_transpose() {
    let lib = posed();
    let mut l = field("veg.tree_birch", 48, 48);
    for origin in [None, Some([-2048, 911])] {
        l.origin = origin;
        let all = poses(&lib, &l, 3);
        let mut scales: BTreeMap<u8, usize> = BTreeMap::new();
        for (pct, _) in &all {
            assert!((SCALE_MIN_PCT..=SCALE_MAX_PCT).contains(pct));
            *scales.entry(*pct).or_default() += 1;
        }
        // 2304 trees over 7 scales: every scale used, each near 329.
        let steps = usize::from((SCALE_MAX_PCT - SCALE_MIN_PCT) / SCALE_STEP_PCT) + 1;
        assert_eq!(steps, 7);
        assert_eq!(scales.len(), steps, "{scales:?}");
        assert!(scales.keys().all(|p| p % SCALE_STEP_PCT == 0), "{scales:?}");
        assert!(
            scales.values().all(|&n| (250..420).contains(&n)),
            "{scales:?}"
        );
        let flipped = all.iter().filter(|p| p.1).count();
        assert!(
            (1000..1300).contains(&flipped),
            "{flipped} of {}",
            all.len()
        );
    }
    // Rendered, the field differs from the same field at a fixed pose.
    let small = field("veg.tree_birch", 6, 6);
    let varied = render(&small, &lib, 3, &plain(16)).unwrap();
    let fixed = render(&small, &placeholder(), 3, &plain(16)).unwrap();
    assert_ne!(varied.data, fixed.data);
}

fn placeholder() -> Library {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
    Library::load(&dir).unwrap()
}

#[test]
fn fixed_pose_explicit_turns_and_other_assets_opt_out() {
    // The placeholder's vegetation carries `fixed_pose`.
    let base = placeholder();
    let bushes = field("veg.bush", 16, 16);
    assert!(poses(&base, &bushes, 4).iter().all(|&p| p == (100, false)));
    let lib = posed();
    assert!(poses(&lib, &bushes, 4).iter().any(|&p| p != (100, false)));
    // An explicit rotation or mirror keeps the catalogued pose.
    for (rotation, mirror) in [(90, false), (0, true)] {
        let mut l = bushes.clone();
        for p in &mut l.placements {
            (p.rotation, p.mirror) = (rotation, mirror);
        }
        assert!(poses(&lib, &l, 4).iter().all(|&p| p == (100, false)));
    }
    // Non-square vegetation and props never take a pose.
    for id in ["veg.fallen_log", "prop.barrel"] {
        let l = field(id, 16, 16);
        assert!(
            poses(&lib, &l, 4).iter().all(|&p| p == (100, false)),
            "{id}"
        );
    }
}

#[test]
fn a_scaled_sprite_stays_centred_on_its_anchor() {
    // A "rock" that is a solid block over the middle half of its 2x2
    // frame, so its drawn extent is plain to measure.
    let base = posed();
    let images = base
        .catalog
        .assets
        .iter()
        .map(|a| {
            let img = base.image(&a.id).unwrap();
            let img = if a.id == "veg.rock_large" {
                let mut block = Rgba::new(img.width, img.height);
                for y in img.height / 4..img.height * 3 / 4 {
                    for x in img.width / 4..img.width * 3 / 4 {
                        block.set(x, y, [255, 0, 255, 255]);
                    }
                }
                block
            } else {
                img.clone()
            };
            (a.id.clone(), img)
        })
        .collect();
    let lib = Library::from_parts(base.catalog.clone(), images).unwrap();
    let ppsq = 32;
    let mut l = TacticalLayout::new("one", 6, 6, "grass");
    put(&mut l, "veg.rock_large", 3.0, 2.0);
    let mut seen = BTreeMap::new();
    for seed in 0..60 {
        // `render` resolves with the seed mixed with the library version.
        let art = seed ^ hash_str(0, &lib.catalog.library_version);
        let r = resolve(&lib, &l, 0, art).unwrap();
        if seen.contains_key(&r.scale_pct) {
            continue;
        }
        seen.insert(r.scale_pct, ());
        let img = render(&l, &lib, seed, &plain(ppsq)).unwrap();
        let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
        for y in 0..img.height {
            for x in 0..img.width {
                if img.get(x, y) == [255, 0, 255, 255] {
                    (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
                }
            }
        }
        let side = ppsq * u32::from(r.scale_pct) / 100;
        let (w, h) = (x1 - x0, y1 - y0);
        assert!(
            w.abs_diff(side) <= 2 && h.abs_diff(side) <= 2,
            "{}%: {w}x{h}, want {side}",
            r.scale_pct
        );
        // Centred on the anchor (3, 2) squares = (96, 64) px, to a pixel.
        assert!((x0 + x1).abs_diff(192) <= 1 && (y0 + y1).abs_diff(128) <= 1);
    }
    assert!(seen.len() >= 5, "{seen:?}");
}

/// Squares `[x0, x0 + w)` of `l` with a world origin; placements listed in
/// reverse, so their indices differ between windows.
fn window(l: &TacticalLayout, x0: u32, w: u32) -> TacticalLayout {
    let mut out = TacticalLayout::new("window", w, l.height, "grass");
    out.origin = Some([1000 + i64::from(x0), -40]);
    let inside = |x: f32| (x0 as f32..(x0 + w) as f32).contains(&x);
    for p in l.placements.iter().rev().filter(|p| inside(p.x)) {
        let mut p = p.clone();
        p.x -= x0 as f32;
        out.placements.push(p);
    }
    out
}

#[test]
fn poses_are_seam_stable_across_windows() {
    let lib = posed();
    let mut big = TacticalLayout::new("world", 36, 10, "grass");
    for y in 0..10 {
        for x in 0..36 {
            if (x + 2 * y) % 3 == 0 {
                put(&mut big, "veg.bush", x as f32 + 0.5, y as f32 + 0.5);
            }
        }
    }
    for y in (1..10).step_by(3) {
        for x in (1..36).step_by(3) {
            put(&mut big, "veg.tree_birch", x as f32 + 0.25, y as f32);
        }
    }
    let ppsq = 16;
    let a = window(&big, 0, 24);
    let b = window(&big, 12, 24);
    // The windows draw scaled and transposed sprites.
    let art = 3 ^ hash_str(0, &lib.catalog.library_version);
    assert!(poses(&lib, &a, art).iter().any(|p| p.0 != 100));
    assert!(poses(&lib, &a, art).iter().any(|p| p.1));
    let a = render(&a, &lib, 3, &plain(ppsq)).unwrap();
    let b = render(&b, &lib, 3, &plain(ppsq)).unwrap();
    // World column 18 is column 18 of `a` and 6 of `b`, six squares from
    // both windows' inner edges.
    let (mut same, mut total) = (0, 0);
    for y in 0..10 * ppsq {
        for x in 0..ppsq {
            total += 1;
            same += usize::from(a.get(18 * ppsq + x, y) == b.get(6 * ppsq + x, y));
        }
    }
    assert_eq!(same, total, "{same} of {total} shared pixels match");
}
