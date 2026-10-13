//! Society layer on area exports (task 15): a settlement straddling an
//! area border paints the same pixels from either side, and the plain
//! export is the unchanged default.
//!
//! World: `$ARDA_MIDZOOM_TEST_WORLD` or `<workspace>/out/micro42` when it is
//! settled, else a MICRO world generated and settled into `target/`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    missing_docs
)]

#[path = "../../../tests/support/fixture_dir.rs"]
mod fixture_dir;

use arda::{ImageQuality, MapStyle, World};
use arda_midzoom::export::{export_area, SocietyMode};
use arda_midzoom::land::export::AreaLandOverlay;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn settled(dir: &Path) -> bool {
    arda::World::load(dir).is_ok_and(|w| w.seed() == 42)
        && dir.join("society/settlements.json").is_file()
        && dir.join("society/society.json").is_file()
}

fn world_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        if let Some(dir) = std::env::var_os("ARDA_MIDZOOM_TEST_WORLD") {
            return PathBuf::from(dir);
        }
        let out = workspace().join("out/micro42");
        if settled(&out) {
            return out;
        }
        let dir = workspace().join("target/arda-midzoom-fixture/micro42-society");
        fixture_dir::ensure(&dir, settled, |tmp| {
            arda::generate_from_fine_source(
                42,
                arda::GenerateConfig::MICRO,
                tmp,
                arda::FineDeliveryLimits::default(),
            )
            .expect("generating the MICRO fixture world");
            arda_settle::generate(tmp, arda_settle::grid::MEMORY_BUDGET).expect("settle");
            arda_people::build(tmp).expect("society build");
        });
        dir
    })
}

/// A settlement within 150 m of a vertical area border: `(area west, y_m, x_m)`.
fn border_settlement() -> (i32, i64, i64) {
    let file: arda_settle::output::SettlementsFile =
        arda_settle::output::read_json(&world_dir().join("society/settlements.json")).unwrap();
    let mut best: Option<(i64, i32, i64, i64)> = None;
    for s in &file.settlements {
        let off = s.x_m.rem_euclid(51_200);
        let d = off.min(51_200 - off);
        let west = if off < 25_600 {
            (s.x_m / 51_200 - 1) as i32
        } else {
            (s.x_m / 51_200) as i32
        };
        // Both neighbours exist in a 2 × 4 world only for west = 0.
        if west == 0 && s.y_m / 51_200 < 4 && best.is_none_or(|b| d < b.0) {
            best = Some((d, west, s.y_m, s.x_m));
        }
    }
    let (_, west, y, x) = best.expect("a settlement near the area border");
    (west, y, x)
}

#[test]
fn a_settlement_on_an_area_border_paints_the_same_pixels_from_both_sides() {
    let side = 4096_u32; // 12.5 m per pixel: settlements and roads drawn
    let (west, y_m, x_m) = border_settlement();
    let land = |ax: i32, ay: i32| {
        AreaLandOverlay::open(world_dir(), (ax, ay), side)
            .unwrap()
            .expect("the world has society")
            .into_pair(ax, ay)
    };
    let ay = (y_m / 51_200) as i32;
    let (a, b) = (land(west, ay), land(west + 1, ay));
    let s = i64::from(side);
    let py = (y_m as f64 / 12.5) as i64 - 120;
    let seam = i64::from(west + 1) * s;
    let (hw, h) = (160_usize, 240_usize);
    let fill = |w: usize| vec![118_u8; w * h * 3];
    // One window across the border, gathered once.
    let mut whole = fill(2 * hw);
    a.0.paint_window((seam - hw as i64, py), (2 * hw, h), &mut whole)
        .unwrap();
    // The same pixels as each area's own window.
    let mut left = fill(hw);
    a.0.paint_window((seam - hw as i64, py), (hw, h), &mut left)
        .unwrap();
    let mut right = fill(hw);
    b.0.paint_window((seam, py), (hw, h), &mut right).unwrap();
    for r in 0..h {
        assert_eq!(
            &whole[r * 2 * hw * 3..(r * 2 * hw + hw) * 3],
            &left[r * hw * 3..(r + 1) * hw * 3],
            "west half, row {r}"
        );
        assert_eq!(
            &whole[(r * 2 * hw + hw) * 3..(r + 1) * 2 * hw * 3],
            &right[r * hw * 3..(r + 1) * hw * 3],
            "east half, row {r}"
        );
    }
    let painted = whole.iter().filter(|&&v| v != 118).count();
    assert!(painted > 1000, "{x_m},{y_m}: only {painted} bytes painted");
}

trait IntoPair: Sized {
    fn into_pair(self, ax: i32, ay: i32) -> (Self, (i32, i32)) {
        (self, (ax, ay))
    }
}
impl IntoPair for AreaLandOverlay {}

#[test]
fn off_is_the_plain_export_and_on_is_deterministic_and_different() {
    let world = World::load(world_dir()).unwrap();
    let q = ImageQuality::new(1024).unwrap();
    let tmp = |n: &str| {
        let d =
            std::env::temp_dir().join(format!("arda-midzoom-export-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    };
    let (plain, off, on1, on2) = (tmp("plain"), tmp("off"), tmp("on1"), tmp("on2"));
    arda::export_area_with_quality_and_style(&world, 0, 1, &plain, q, MapStyle::Atlas).unwrap();
    export_area(&world, (0, 1), &off, q, MapStyle::Atlas, SocietyMode::Off).unwrap();
    export_area(&world, (0, 1), &on1, q, MapStyle::Atlas, SocietyMode::On).unwrap();
    export_area(&world, (0, 1), &on2, q, MapStyle::Atlas, SocietyMode::Auto).unwrap();
    let read = |d: &Path| std::fs::read(d.join("area_00_01.png")).unwrap();
    assert_eq!(read(&plain), read(&off));
    assert_eq!(read(&on1), read(&on2));
    assert_ne!(read(&plain), read(&on1));
    for d in [plain, off, on1, on2] {
        std::fs::remove_dir_all(d).unwrap();
    }
}
