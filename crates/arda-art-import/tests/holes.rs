//! Enclosed backdrop pockets: leaf gaps in a canopy on white are cleared,
//! a white sheet's large interior is kept, real alpha is left alone, and the
//! result is deterministic.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]

use arda_art_import::holes::HolePolicy;
use arda_art_import::image_io::{GenMeta, RawImage};
use arda_art_import::manifest::{HoleMode, ShadowMode};
use arda_art_import::matte::cut_out;
use arda_art_import::meta::class_default;
use arda_art_import::naming::Target;
use arda_art_import::pipeline::{process, Processed};
use arda_art_import::{import, ImportOptions};
use arda_tactical::noise::hash2;
use arda_tactical::Rgba;
use std::path::{Path, PathBuf};

const SIZE: u32 = 240;
/// Leaf-gap centres and radii inside the canopy.
const GAPS: [(f32, f32, f32); 5] = [
    (100.0, 95.0, 5.0),
    (140.0, 110.0, 4.0),
    (115.0, 140.0, 6.0),
    (150.0, 145.0, 3.0),
    (85.0, 130.0, 4.5),
];

fn grain(seed: u64, x: u32, y: u32, amp: u32) -> i32 {
    (hash2(seed, i64::from(x), i64::from(y)) % u64::from(2 * amp + 1)) as i32 - amp as i32
}

fn clamp(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

fn backdrop_px(x: u32, y: u32) -> [u8; 4] {
    let n = grain(1, x, y, 2);
    [clamp(246 + n), clamp(245 + n), clamp(241 + n), 255]
}

fn backdrop() -> Rgba {
    let mut img = Rgba::new(SIZE, SIZE);
    for y in 0..SIZE {
        for x in 0..SIZE {
            img.set(x, y, backdrop_px(x, y));
        }
    }
    img
}

fn mix(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    std::array::from_fn(|c| (f32::from(a[c]) * (1.0 - t) + f32::from(b[c]) * t + 0.5) as u8)
}

fn d(x: u32, y: u32, cx: f32, cy: f32) -> f32 {
    ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt()
}

/// A textured oak canopy on white with sky-white gaps (and a thin crack)
/// between the leaf clusters, each with a one-pixel anti-aliased rim.
fn canopy_on_white() -> Rgba {
    let mut img = backdrop();
    for y in 0..SIZE {
        for x in 0..SIZE {
            if d(x, y, 120.0, 120.0) > 80.0 {
                continue;
            }
            let n = grain(7, x, y, 22);
            let leaf = [clamp(70 + n), clamp(112 + n), clamp(38 + n / 2), 255];
            let crack = (126..=158).contains(&x) && (y == 80 || y == 81);
            let gap = GAPS
                .iter()
                .map(|&(cx, cy, r)| d(x, y, cx, cy) - r)
                .fold(f32::MAX, f32::min);
            let px = if crack || gap <= 0.0 {
                backdrop_px(x, y)
            } else if gap <= 1.0 {
                mix(leaf, backdrop_px(x, y), 0.5)
            } else {
                leaf
            };
            img.set(x, y, px);
        }
    }
    img
}

/// A pale linen sheet with a brown hem: its large interior is exactly the
/// backdrop colour, but it is part of the object.
fn sheet_on_white() -> Rgba {
    let mut img = backdrop();
    for y in 60..180 {
        for x in 40..200 {
            let hem = !(44..196).contains(&x) || !(64..176).contains(&y);
            if hem {
                let n = grain(3, x, y, 8);
                img.set(x, y, [clamp(120 + n), clamp(90 + n), clamp(60 + n), 255]);
            }
        }
    }
    img
}

fn holes_fix(fixes: &[String]) -> Option<&String> {
    fixes
        .iter()
        .find(|f| f.contains("enclosed background pocket"))
}

#[test]
fn leaf_gaps_in_a_canopy_on_white_become_transparent() {
    let raw = canopy_on_white();
    let before = cut_out(&raw, ShadowMode::Strip, HolePolicy::Off);
    for &(cx, cy, _) in &GAPS {
        assert_eq!(before.img.get(cx as u32, cy as u32)[3], 255, "the defect");
    }
    let cut = cut_out(&raw, ShadowMode::Strip, HolePolicy::Foliage);
    for &(cx, cy, _) in &GAPS {
        assert_eq!(cut.img.get(cx as u32, cy as u32)[3], 0, "gap at {cx},{cy}");
    }
    assert_eq!(cut.img.get(140, 80)[3], 0, "thin crack");
    assert_eq!(cut.img.get(170, 120)[3], 255, "leaves");
    assert_eq!(cut.img.get(5, 5)[3], 0, "outer backdrop");
    let fix = holes_fix(&cut.fixes).expect("a pockets fix");
    assert!(
        fix.starts_with("cleared 6 enclosed background pockets ("),
        "{fix}"
    );
    // Soft matte and defringe around the new holes: no pale opaque rim.
    let pale = cut
        .img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[3] >= 200 && p[0] > 200 && p[1] > 200 && p[2] > 200)
        .count();
    assert_eq!(pale, 0, "{pale} pale opaque px left");
    assert!(cut.flags.is_empty(), "{:?}", cut.flags);
}

#[test]
fn a_white_sheet_keeps_its_large_white_interior() {
    let raw = sheet_on_white();
    for policy in [HolePolicy::Strict, HolePolicy::Foliage] {
        let cut = cut_out(&raw, ShadowMode::Strip, policy);
        assert_eq!(cut.img.get(120, 120)[3], 255, "{policy:?}");
        assert_eq!(cut.img.get(60, 80)[3], 255, "{policy:?}");
        assert!(holes_fix(&cut.fixes).is_none(), "{:?}", cut.fixes);
        assert!(
            cut.flags.iter().any(|f| f.contains("holes = \"clear\"")),
            "{:?}",
            cut.flags
        );
    }
    // Explicit `holes = "clear"` opts in to clearing it (a frame, a ring).
    let forced = cut_out(&raw, ShadowMode::Strip, HolePolicy::Forced);
    assert_eq!(forced.img.get(120, 120)[3], 0);
}

fn run(raw: &RawImage, target: Target, holes: Option<HoleMode>) -> Processed {
    process(
        raw,
        class_default(&target),
        target,
        "raw.png".into(),
        (64, ShadowMode::Strip, holes),
    )
}

fn veg() -> Target {
    Target::Vegetation {
        name: "tree_oak".into(),
    }
}

#[test]
fn class_defaults_and_the_manifest_key_choose_the_policy() {
    let raw = RawImage {
        rgba: canopy_on_white(),
        has_alpha: false,
        meta: GenMeta::default(),
    };
    let prop = Target::Prop {
        name: "wreath".into(),
    };
    assert_eq!(HolePolicy::for_asset(&veg(), None), HolePolicy::Foliage);
    assert_eq!(HolePolicy::for_asset(&prop, None), HolePolicy::Strict);
    assert!(holes_fix(&run(&raw, veg(), None).fixes).is_some());
    assert!(holes_fix(&run(&raw, veg(), Some(HoleMode::Keep)).fixes).is_none());
    let sheet = RawImage {
        rgba: sheet_on_white(),
        ..raw
    };
    let p = run(&sheet, prop.clone(), None);
    assert!(holes_fix(&p.fixes).is_none(), "{:?}", p.fixes);
    assert!(holes_fix(&run(&sheet, prop, Some(HoleMode::Clear)).fixes).is_some());
}

#[test]
fn an_image_with_real_alpha_is_left_alone() {
    let mut rgba = canopy_on_white();
    for y in 0..SIZE {
        for x in 0..SIZE {
            if d(x, y, 120.0, 120.0) > 80.0 {
                rgba.set(x, y, [0, 0, 0, 0]);
            }
        }
    }
    let raw = RawImage {
        rgba,
        has_alpha: true,
        meta: GenMeta::default(),
    };
    let keep = run(&raw, veg(), Some(HoleMode::Keep));
    for holes in [None, Some(HoleMode::Clear)] {
        let p = run(&raw, veg(), holes);
        assert_eq!(p.img.data, keep.img.data, "{holes:?}");
        assert!(holes_fix(&p.fixes).is_none(), "{:?}", p.fixes);
    }
    let white = keep
        .img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[3] == 255 && p[0] > 230 && p[1] > 230)
        .count();
    assert!(white > 0, "the opaque white gaps of the source are kept");
}

fn import_once(root: &Path) -> (Vec<u8>, String) {
    let _ = std::fs::remove_dir_all(root);
    let raw = root.join("raw");
    std::fs::create_dir_all(&raw).unwrap();
    canopy_on_white()
        .write_png(&raw.join("veg.tree_oak.png"))
        .unwrap();
    let report = import(&ImportOptions {
        raw_dir: raw,
        out_dir: root.join("lib"),
        ..ImportOptions::default()
    })
    .unwrap();
    let png = std::fs::read(root.join("lib/vegetation/veg.tree_oak.png")).unwrap();
    (png, report.to_markdown())
}

#[test]
fn clearing_pockets_is_deterministic() {
    let raw = canopy_on_white();
    let a = cut_out(&raw, ShadowMode::Strip, HolePolicy::Foliage);
    let b = cut_out(&raw, ShadowMode::Strip, HolePolicy::Foliage);
    assert_eq!(a.img.data, b.img.data);
    assert_eq!(a.fixes, b.fixes);
    let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("art-import-holes");
    let (png1, md1) = import_once(&base.join("a"));
    let (png2, md2) = import_once(&base.join("b"));
    assert_eq!(png1, png2);
    assert_eq!(md1, md2);
    assert!(md1.contains("enclosed background pocket"), "{md1}");
}
