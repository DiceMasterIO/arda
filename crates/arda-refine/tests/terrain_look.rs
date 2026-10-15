//! The tactical terrain look on recipe-9 worlds (review gallery
//! 2026-10-15, images 08, 12, 13 and 19): a forested hillside whose squares
//! run steeper than its world slope keeps its forest instead of breaking into
//! bare rock and broad cliff faces, and the sea's sand stays a beach strip
//! at the water instead of covering whole cells inland, with no trees on it.
#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    missing_docs
)]

use arda::{Cell, Cover, ShoreClass, TerrainKind};
use arda_core::biome::Frame;
use arda_core::{HeightMm, RainfallMm, TempCentiC};
use arda_refine::scatter::Kind;
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

/// A world on a recipe-9 (seasonal) frame.
fn world(seasonal: bool, f: impl Fn(i64, i64, &mut Cell)) -> GridSource {
    let mut g = GridSource::new(42, SIDE, SIDE, temperate());
    for y in 0..i64::from(SIDE) {
        for x in 0..i64::from(SIDE) {
            let mut c = temperate();
            f(x, y, &mut c);
            g.set(CellKey::new(x, y), c);
        }
    }
    g.set_frame(Frame::new(35, 55, u32::from(SIDE)).with_seasons(seasonal));
    g
}

fn share(b: &Block, keys: &[&str]) -> f64 {
    let n = b.ground.iter().filter(|g| keys.contains(g)).count();
    n as f64 / b.ground.len() as f64
}

fn trees(b: &Block) -> usize {
    b.items
        .iter()
        .filter(|i| matches!(i.kind, Kind::TreeLarge | Kind::TreeSmall))
        .count()
}

/// The ground key of the square an item stands on.
fn ground_under(b: &Block, x: f64, y: f64) -> &'static str {
    let (i, j) = (x.floor() as i64 - b.origin.0, y.floor() as i64 - b.origin.1);
    b.ground[usize::try_from(j * 64 + i).unwrap()]
}

/// A forest cell (density 0.78) whose world slope says 27° while its
/// heights fall 120 m per cell: the squares run near 50°, as fine terrain
/// does under cell 716,1423 of MICRO 42.
fn steep_forest(seasonal: bool) -> GridSource {
    world(seasonal, |x, _, c| {
        c.cover = Cover::Forest;
        c.forest_density = 200;
        c.slope_milli_deg = 27_000;
        c.height = HeightMm::new(1_400_000 - 120_000 * i32::try_from(x).unwrap());
        c.temperature = TempCentiC::new(600);
        c.rainfall = RainfallMm::new(1_000);
    })
}

#[test]
fn a_forest_steeper_than_its_world_slope_keeps_its_forest() {
    let b = refine(&steep_forest(true), MID).unwrap();
    let cliff = share(&b, &["cliff"]);
    let rock = share(&b, &["rock", "scree", "cliff"]);
    let n = trees(&b);
    assert!(cliff < 0.02, "broad cliff faces under forest: {cliff:.3}");
    assert!(rock < 0.3, "bare rock under forest: {rock:.2}");
    assert!(n > 400, "{n} trees");
}

#[test]
fn older_recipes_keep_their_steep_ground() {
    // Recipes 5–8 refine as before: the same hillside is rock and cliff.
    let b = refine(&steep_forest(false), MID).unwrap();
    assert!(share(&b, &["rock", "scree", "cliff"]) > 0.5);
}

#[test]
fn an_open_crag_still_breaks_into_cliffs() {
    // Open ground (no forest) keeps the steep rules on recipe 9 too.
    let src = world(true, |x, _, c| {
        c.slope_milli_deg = 27_000;
        c.height = HeightMm::new(1_400_000 - 120_000 * i32::try_from(x).unwrap());
        c.temperature = TempCentiC::new(600);
    });
    let b = refine(&src, MID).unwrap();
    assert!(share(&b, &["cliff"]) > 0.05);
    assert!(share(&b, &["rock", "scree", "cliff"]) > 0.5);
}

/// Land at sea level west of x = 4 behind a sand beach, open sea east of it.
fn sand_coast(seasonal: bool) -> GridSource {
    let mut src = world(seasonal, |x, _, c| {
        if x >= 4 {
            c.terrain = TerrainKind::Sea;
            c.cover = Cover::Bare;
            c.height = HeightMm::new(-6_000);
        } else {
            c.cover = Cover::Forest;
            c.forest_density = 160;
            c.height = HeightMm::new([9_000, 7_000, 5_000, 2_000][usize::try_from(x).unwrap()]);
        }
    });
    for y in 0..i64::from(SIDE) {
        src.set_shore(CellKey::new(3, y), ShoreClass::SandBeach);
    }
    src
}

#[test]
fn sand_lies_only_at_the_water() {
    let src = sand_coast(true);
    // Two cells in from the sea: no beach, woods instead.
    let inland = refine(&src, CellKey::new(2, 3)).unwrap();
    let sand = share(&inland, &["sand"]);
    assert!(sand < 0.02, "sand two cells from the sea: {sand:.3}");
    assert!(trees(&inland) > 200, "{} trees inland", trees(&inland));
    // The strand cell still meets the sea across a beach.
    let strand = refine(&src, CellKey::new(3, 3)).unwrap();
    assert!(share(&strand, &["sand"]) > 0.05);
}

#[test]
fn no_tree_stands_on_beach_sand() {
    let src = sand_coast(true);
    for x in 1..=3 {
        let b = refine(&src, CellKey::new(x, 3)).unwrap();
        let on_sand: Vec<_> = b
            .items
            .iter()
            .filter(|i| matches!(i.kind, Kind::TreeLarge | Kind::TreeSmall))
            .filter(|i| ground_under(&b, i.x, i.y) == "sand")
            .map(|i| (i.x, i.y))
            .collect();
        let first: Vec<_> = on_sand.iter().take(5).collect();
        assert!(
            on_sand.is_empty(),
            "cell {x}: {} trees on sand, first at {first:?}",
            on_sand.len()
        );
    }
}

/// A level forest cell of density 0.59.
fn open_forest(seasonal: bool) -> GridSource {
    world(seasonal, |_, _, c| {
        c.cover = Cover::Forest;
        c.forest_density = 150;
    })
}

#[test]
fn forest_glades_open_to_grass() {
    // The grove field leaves about two fifths of the cell without trees;
    // on recipe 9 its ground is turf, not bare litter.
    let new = refine(&open_forest(true), MID).unwrap();
    let old = refine(&open_forest(false), MID).unwrap();
    let turf = |b: &Block| share(b, &["grass", "meadow"]);
    println!(
        "turf {:.3} -> {:.3}, litter {:.3} -> {:.3}",
        turf(&old),
        turf(&new),
        share(&old, &["forest_floor", "leaf_litter"]),
        share(&new, &["forest_floor", "leaf_litter"])
    );
    println!(
        "scrub {:.3} -> {:.3}",
        share(&old, &["scrub"]),
        share(&new, &["scrub"])
    );
    assert!(turf(&new) > turf(&old) + 0.1);
    assert!(share(&new, &["forest_floor", "leaf_litter"]) > 0.3);
    // Trees stand on litter, not in the glades' turf.
    let on_turf = new
        .items
        .iter()
        .filter(|i| i.kind == Kind::TreeLarge)
        .filter(|i| matches!(ground_under(&new, i.x, i.y), "grass" | "meadow"))
        .count();
    assert!(
        on_turf * 4 < trees(&new),
        "{on_turf} of {} trees on turf",
        trees(&new)
    );
}

/// A cold, open 26° hillside rising westward: craggy ground under the
/// tree line, as on the cold check world's cell 943,1609.
fn cold_crag(seasonal: bool) -> GridSource {
    world(seasonal, |x, _, c| {
        c.slope_milli_deg = 26_000;
        c.height = HeightMm::new(2_200_000 - 49_000 * i32::try_from(x).unwrap());
        c.temperature = TempCentiC::new(-900);
        c.rainfall = RainfallMm::new(700);
    })
}

/// Of the rows of a block, the share whose cliff squares come in more than
/// `runs` separate runs across it: ledges ringing the whole hillside.
fn banded_rows(b: &Block, runs: usize) -> f64 {
    let mut n = 0;
    for y in 0..64 {
        let mut count = 0;
        let mut inside = false;
        for x in 0..64 {
            let cliff = b.ground[y * 64 + x] == "cliff";
            if cliff && !inside {
                count += 1;
            }
            inside = cliff;
        }
        n += usize::from(count > runs);
    }
    n as f64 / 64.0
}

#[test]
fn crag_ledges_break_instead_of_ringing_the_hill() {
    let old = refine(&cold_crag(false), MID).unwrap();
    let new = refine(&cold_crag(true), MID).unwrap();
    let (c_old, c_new) = (share(&old, &["cliff"]), share(&new, &["cliff"]));
    let (r_old, r_new) = (banded_rows(&old, 4), banded_rows(&new, 4));
    println!("cliff {c_old:.3} -> {c_new:.3}; rows with 5+ ledges {r_old:.2} -> {r_new:.2}");
    assert!(c_new > 0.02, "some ledges stay: {c_new:.3}");
    assert!(c_new < 0.6 * c_old, "cliff {c_old:.3} -> {c_new:.3}");
    assert!(
        r_new < 0.5 * r_old.max(0.02),
        "ringing rows {r_old:.2} -> {r_new:.2}"
    );
}
