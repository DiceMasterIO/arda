//! Biome rules from the cell fields (`arda_refine::biome`): a saturated
//! cell is marsh, sea-level land on a coast meets the sea across a beach,
//! a snowbound peak carries no flowers or shrubs, dry steppe grows dry
//! grass, and a craggy mountainside breaks into cliff bands and outcrops.
#![allow(clippy::unwrap_used, clippy::cast_precision_loss, missing_docs)]

use arda::{Cell, Cover, TerrainKind};
use arda_core::{HeightMm, RainfallMm, TempCentiC};
use arda_refine::{refine, Block, CellKey, GridSource};

const SIDE: u16 = 7;
const MID: CellKey = CellKey::new(3, 3);

fn temperate() -> Cell {
    Cell {
        height: HeightMm::new(80_000),
        terrain: TerrainKind::Land,
        cover: Cover::Grass,
        slope_milli_deg: 1_000,
        temperature: TempCentiC::new(1000),
        rainfall: RainfallMm::new(900),
        moisture: 120,
        wetness: 30,
        height_above_river_dm: 60,
        ..Cell::default()
    }
}

fn world(f: impl Fn(i64, i64, &mut Cell)) -> GridSource {
    let mut g = GridSource::new(42, SIDE, SIDE, temperate());
    for y in 0..i64::from(SIDE) {
        for x in 0..i64::from(SIDE) {
            let mut c = temperate();
            f(x, y, &mut c);
            g.set(CellKey::new(x, y), c);
        }
    }
    g
}

fn share(b: &Block, keys: &[&str]) -> f64 {
    let n = b.ground.iter().filter(|g| keys.contains(g)).count();
    n as f64 / b.ground.len() as f64
}

fn assets(b: &Block) -> Vec<&'static str> {
    b.items.iter().map(|i| i.asset).collect()
}

#[test]
fn a_saturated_cell_is_marsh_with_reeds() {
    let src = world(|_, _, c| c.wetness = 255);
    let b = refine(&src, MID).unwrap();
    let marsh = share(&b, &["marsh", "reed_bed", "mud", "water_shallow"]);
    assert!(marsh > 0.7, "marsh ground share {marsh:.2}");
    assert!(share(&b, &["grass", "meadow"]) < 0.2);
    let a = assets(&b);
    let reeds = a
        .iter()
        .filter(|id| matches!(**id, "veg.reeds" | "veg.cattail"))
        .count();
    assert!(reeds > 50, "{reeds} reeds and cattails");
    assert!(a
        .iter()
        .any(|id| matches!(*id, "veg.sedge" | "veg.marsh_flowers")));
    // A damp but unsaturated meadow stays grassland.
    let damp = world(|_, _, c| c.wetness = 150);
    let d = refine(&damp, MID).unwrap();
    assert!(share(&d, &["marsh", "reed_bed"]) < 0.1);
}

#[test]
fn sea_level_land_on_a_coast_meets_the_sea_across_a_beach() {
    // Land at sea level west of x = 4, open sea east of it.
    let src = world(|x, _, c| {
        if x >= 4 {
            c.terrain = TerrainKind::Sea;
            c.cover = Cover::Bare;
            c.height = HeightMm::new(-6_000);
        } else {
            // A low plain falling to the strand at x = 3.
            c.height = HeightMm::new([4_000, 2_500, 1_200, 1][usize::try_from(x).unwrap()]);
        }
    });
    let b = refine(&src, MID).unwrap();
    let water = share(&b, &["water_shallow", "water_deep"]);
    let sand = share(&b, &["sand"]);
    assert!(
        water > 0.05,
        "the sea reaches into the coast cell: {water:.2}"
    );
    assert!(sand > 0.1, "a beach: {sand:.2}");
    let a = assets(&b);
    assert!(
        a.iter()
            .any(|id| matches!(*id, "veg.dune_grass" | "veg.driftwood" | "veg.sea_rock")),
        "coastal scatter: {a:?}"
    );
    assert!(!a.contains(&"veg.flower_patch") || a.contains(&"veg.dune_grass"));
}

#[test]
fn a_snowbound_peak_grows_no_flowers_or_shrubs() {
    let src = world(|_, _, c| {
        c.temperature = TempCentiC::new(-686);
        c.height = HeightMm::new(3_150_000);
    });
    let b = refine(&src, MID).unwrap();
    assert!(share(&b, &["snow", "ice"]) > 0.8);
    for it in &b.items {
        assert!(
            !it.tag.starts_with("low:") && !it.tag.starts_with("undergrowth:"),
            "{} on snow",
            it.asset
        );
    }
    assert!(b.items.iter().all(|it| it.tag.starts_with("rock:")
        || it.tag.starts_with("tree:dead")
        || it.tag == "tree:conifer:stunted"));
}

#[test]
fn above_the_tree_line_low_plants_are_alpine() {
    // Cold but below the snowline: sparse alpine flowers and cushions.
    let src = world(|_, _, c| c.temperature = TempCentiC::new(0));
    let b = refine(&src, MID).unwrap();
    let low: Vec<_> = b
        .items
        .iter()
        .filter(|i| i.tag.starts_with("low:") && i.tag != "low:mushrooms")
        .collect();
    assert!(!low.is_empty());
    let alpine = low
        .iter()
        .filter(|i| matches!(i.asset, "veg.alpine_flowers" | "veg.tussock"))
        .count();
    assert!(alpine * 2 > low.len(), "{alpine} of {} alpine", low.len());
    assert!(b.items.iter().all(|i| i.asset != "veg.bush_flowering"));
}

#[test]
fn dry_steppe_grows_dry_grass_and_bare_patches() {
    let src = world(|_, _, c| {
        c.rainfall = RainfallMm::new(400);
        c.temperature = TempCentiC::new(1280);
    });
    let b = refine(&src, MID).unwrap();
    let a = assets(&b);
    let dry = a
        .iter()
        .filter(|id| matches!(**id, "veg.dry_grass" | "veg.tussock" | "veg.sagebrush"))
        .count();
    let flowers = a.iter().filter(|id| **id == "veg.flower_patch").count();
    assert!(
        dry > 3 * flowers.max(1),
        "{dry} dry plants, {flowers} flowers"
    );
    assert!(share(&b, &["dirt"]) > 0.03, "bare patches on steppe");
    let wet = refine(&world(|_, _, _| {}), MID).unwrap();
    assert!(assets(&wet).iter().all(|id| *id != "veg.dry_grass"));
    // The compositor's dry-grass tint rides on every square.
    assert!(b.dryness.iter().all(|&d| d > 200), "steppe dryness");
    assert!(wet.dryness.iter().all(|&d| d == 0), "temperate dryness");
}

#[test]
fn a_craggy_mountainside_breaks_into_cliffs_and_outcrops() {
    // A 35-degree slope: 70 m of rise per 100 m cell, falling eastward.
    let src = world(|x, _, c| {
        c.slope_milli_deg = 35_000;
        c.height = HeightMm::new(1_500_000 - 70_000 * i32::try_from(x).unwrap());
        c.temperature = TempCentiC::new(400);
    });
    let b = refine(&src, MID).unwrap();
    let cliff = share(&b, &["cliff"]);
    let rock = share(&b, &["rock", "cliff", "scree"]);
    assert!(cliff > 0.05, "cliff bands: {cliff:.2}");
    assert!(rock > 0.6, "rock ground: {rock:.2}");
    assert!(share(&b, &["grass"]) < 0.3);
    let outcrops = b
        .items
        .iter()
        .filter(|i| i.asset == "veg.rock_outcrop")
        .count();
    assert!(outcrops >= 2, "{outcrops} outcrops");
}

/// Blocks refined independently agree on their shared edges' elevation,
/// water and corner classes under the new rules (strata, the coast strip,
/// marsh).
#[test]
fn biome_rules_stay_seamless() {
    fn check(a: &Block, b: &Block) {
        for k in 0..64_i64 {
            let halo = *a.halo.get(a.origin.0 + 64, a.origin.1 + k).unwrap();
            let i = usize::try_from(k).unwrap() * 64;
            assert_eq!(halo.0, b.elevation_ft[i], "elevation at {k}");
            assert_eq!(halo.1, b.depth_ft[i], "water depth at {k}");
        }
        for k in 0..=64_usize {
            assert_eq!(a.corners[k * 65 + 64], b.corners[k * 65], "corner {k}");
        }
    }
    let worlds = [
        world(|x, _, c| {
            c.slope_milli_deg = 35_000;
            c.height = HeightMm::new(1_500_000 - 70_000 * i32::try_from(x).unwrap());
        }),
        world(|x, y, c| {
            if x + y >= 7 {
                c.terrain = TerrainKind::Sea;
                c.cover = Cover::Bare;
                c.height = HeightMm::new(-6_000);
            } else if x + y == 6 {
                c.height = HeightMm::new(1);
            }
        }),
        world(|x, _, c| c.wetness = if x >= 3 { 255 } else { 40 }),
    ];
    for src in &worlds {
        let a = refine(src, CellKey::new(2, 3)).unwrap();
        let b = refine(src, MID).unwrap();
        check(&a, &b);
    }
}

/// Share of the block under tree crowns, from the art's footprints: 3 × 3
/// broadleaves show about 2.3 squares of crown, 2 × 2 trees 1.5, stunted
/// ones 0.8.
fn canopy(b: &Block) -> f64 {
    const N: i64 = 4;
    let side = 64 * N;
    let mut cov = vec![false; usize::try_from(side * side).unwrap()];
    for it in &b.items {
        let r = match it.asset {
            "veg.tree_oak" | "veg.tree_elm" | "veg.tree_willow" => 1.15,
            "veg.tree_stunted" => 0.4,
            a if a.starts_with("veg.tree_") => 0.75,
            _ => continue,
        };
        let (x, y) = (it.x - b.origin.0 as f64, it.y - b.origin.1 as f64);
        for j in 0..side {
            for i in 0..side {
                let (u, v) = ((i as f64 + 0.5) / N as f64, (j as f64 + 0.5) / N as f64);
                if (u - x).powi(2) + (v - y).powi(2) <= r * r {
                    cov[usize::try_from(j * side + i).unwrap()] = true;
                }
            }
        }
    }
    cov.iter().filter(|c| **c).count() as f64 / cov.len() as f64
}

#[test]
fn forest_canopy_closes_to_about_the_forest_density() {
    let forest = |fd: u8| {
        world(move |_, _, c| {
            c.cover = Cover::Forest;
            c.forest_density = fd;
        })
    };
    let dense = canopy(&refine(&forest(215), MID).unwrap());
    let sparse = canopy(&refine(&forest(100), MID).unwrap());
    assert!(dense > 0.65, "dense forest canopy {dense:.2}");
    assert!(
        (0.2..0.6).contains(&sparse),
        "open woodland canopy {sparse:.2}"
    );
    assert!(dense > sparse + 0.2, "{dense:.2} vs {sparse:.2}");
}
