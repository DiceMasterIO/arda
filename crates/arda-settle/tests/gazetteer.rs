//! Feature detection on a hand-made grid, and the gazetteer of the
//! synthetic world.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    missing_docs
)]

use arda::TerrainKind;
use arda_names::PlaceKind;
use arda_settle::features::Peak;
use arda_settle::gazetteer::{find, Found, Thresholds};
use arda_settle::grid::{Grid, MEMORY_BUDGET};
use arda_settle::{run, synthetic, PlaceParams};
use std::collections::BTreeSet;

const W: usize = 640;
const H: usize = 480;

fn paint(g: &mut Grid, x0: usize, y0: usize, x1: usize, y1: usize, t: TerrainKind) {
    for y in y0..y1 {
        for x in x0..x1 {
            let i = y * W + x;
            g.terrain[i] = t;
            g.height_mm[i] = if t == TerrainKind::Land {
                100_000
            } else {
                -1000
            };
        }
    }
}

fn disc(g: &mut Grid, cx: i64, cy: i64, r: i64, t: TerrainKind) {
    for y in 0..H as i64 {
        for x in 0..W as i64 {
            if (x - cx).pow(2) + (y - cy).pow(2) <= r * r {
                let i = y as usize * W + x as usize;
                g.terrain[i] = t;
                g.height_mm[i] = if t == TerrainKind::Land {
                    100_000
                } else {
                    -1000
                };
            }
        }
    }
}

/// Sea in the west, a bay cut into the coast, two lakes, one island, a
/// peninsula and a ridge of five summits on the land.
fn world() -> (Grid, Vec<Peak>) {
    let mut g = Grid::sea(W, H, MEMORY_BUDGET).unwrap();
    paint(&mut g, 150, 0, W, H, TerrainKind::Land);
    disc(&mut g, 230, 240, 50, TerrainKind::Sea);
    paint(&mut g, 150, 231, 185, 249, TerrainKind::Sea);
    paint(&mut g, 400, 60, 430, 90, TerrainKind::Lake);
    paint(&mut g, 400, 380, 460, 400, TerrainKind::Lake);
    disc(&mut g, 60, 60, 12, TerrainKind::Land);
    paint(&mut g, 30, 300, 35, 305, TerrainKind::Land);
    paint(&mut g, 100, 400, 150, 412, TerrainKind::Land);
    let mut peaks = Vec::new();
    for k in 0..5_usize {
        let (x, y) = (300 + k * 15, 240);
        for dx in 0..15 {
            g.height_mm[y * W + x + dx] = 1_800_000;
        }
        g.height_mm[y * W + x] = 2_000_000;
        peaks.push(Peak {
            cell: y * W + x,
            height_m: 2000,
        });
    }
    (g, peaks)
}

fn small() -> Thresholds {
    Thresholds {
        sea_ha: 30_000,
        bay_ha: 3_000,
        lake_ha: 500,
        island_ha: 400,
        cape_ha: 300,
        forest_ha: 5_000,
        marsh_ha: 2_000,
        ..Thresholds::default()
    }
}

fn count(f: &[Found], k: PlaceKind) -> usize {
    f.iter().filter(|x| x.kind == k).count()
}

#[test]
fn a_made_world_yields_exactly_its_features() {
    let (g, peaks) = world();
    let found = find(&g, &peaks, &small());
    assert_eq!(count(&found, PlaceKind::Sea), 1, "{found:?}");
    assert_eq!(count(&found, PlaceKind::Bay), 1);
    assert_eq!(count(&found, PlaceKind::Lake), 2);
    assert_eq!(
        count(&found, PlaceKind::Island),
        1,
        "the 25-cell islet is too small"
    );
    // The peninsula, and the headlands either side of the bay's mouth.
    let capes: Vec<&Found> = found.iter().filter(|f| f.kind == PlaceKind::Cape).collect();
    assert!(capes.iter().any(|c| {
        (10_000..15_000).contains(&c.anchor_m[0]) && (40_000..41_300).contains(&c.anchor_m[1])
    }));
    assert_eq!(count(&found, PlaceKind::Mountain), 1);
    assert_eq!(
        count(&found, PlaceKind::Forest) + count(&found, PlaceKind::Marsh),
        0
    );
    assert_eq!(found.len(), 6 + capes.len());
    let again = find(&g, &peaks, &small());
    assert_eq!(format!("{found:?}"), format!("{again:?}"), "deterministic");
}

#[test]
fn anchors_and_paths_lie_inside_their_feature() {
    let (g, peaks) = world();
    let at = |p: [i64; 2]| g.at(p[0] / 100, p[1] / 100).unwrap();
    for f in find(&g, &peaks, &small()) {
        let mut points = vec![f.anchor_m];
        points.extend(f.path_m.iter().flatten().copied());
        for p in points {
            let t = g.terrain[at(p)];
            let ok = match f.kind {
                PlaceKind::Sea | PlaceKind::Bay => t == TerrainKind::Sea,
                PlaceKind::Lake => t == TerrainKind::Lake,
                PlaceKind::Island | PlaceKind::Cape | PlaceKind::Mountain => t == TerrainKind::Land,
                _ => true,
            };
            assert!(ok, "{:?} point {p:?} is {t:?}", f.kind);
            assert!(
                p[0] >= f.extent_m[0]
                    && p[0] <= f.extent_m[2]
                    && p[1] >= f.extent_m[1]
                    && p[1] <= f.extent_m[3],
                "{:?} point {p:?} outside {:?}",
                f.kind,
                f.extent_m
            );
        }
        assert!(f.path_m.as_ref().is_none_or(|p| p.len() <= 24));
    }
}

#[test]
fn the_ridge_is_one_range_and_the_long_lake_has_a_path() {
    let (g, peaks) = world();
    let found = find(&g, &peaks, &small());
    let range = found
        .iter()
        .find(|f| f.kind == PlaceKind::Mountain)
        .unwrap();
    assert_eq!(range.detail, 5);
    assert_eq!(range.path_m.as_ref().unwrap().len(), 5);
    let lakes: Vec<&Found> = found.iter().filter(|f| f.kind == PlaceKind::Lake).collect();
    assert_eq!(lakes.iter().filter(|l| l.path_m.is_some()).count(), 1);
}

#[test]
fn the_synthetic_world_has_a_named_unique_gazetteer() {
    let params = PlaceParams {
        seed: 7,
        density_per_km2: 15,
    };
    let g = synthetic::landscape(params.seed).unwrap();
    let a = run(&g, params).unwrap();
    let b = run(&g, params).unwrap();
    assert_eq!(a.gazetteer, b.gazetteer);
    assert!(!a.gazetteer.is_empty());
    let mut names = BTreeSet::new();
    for (k, f) in a.gazetteer.iter().enumerate() {
        assert_eq!(f.id, u64::try_from(k + 1).unwrap(), "ids are file order");
        assert!(!f.name.is_empty() && !f.name_gloss.is_empty());
        assert!((1..=5).contains(&f.rank));
        assert!(
            names.insert(arda_names::scope::skeleton(&f.name)),
            "{} twice",
            f.name
        );
    }
    for s in &a.settlements {
        assert!(
            names.insert(arda_names::scope::skeleton(&s.name)),
            "{} clashes",
            s.name
        );
    }
}
