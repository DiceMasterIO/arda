//! Settlement cells of the real seed-42 MICRO world, composed with every
//! society overlay: buildings never overlap, no wall doubles up on an edge,
//! and no lower layer's wall (a farmstead's fence, a field hedge) stands
//! inside the ground the town owns, so no fence cuts through a building.
//!
//! World resolution: `$ARDA_TEST_WORLD`, then `<workspace>/out/micro42`
//! when it holds `society/`, else a MICRO world generated and settled once
//! into `target/arda-people-fixture/micro42` (shared with arda-people).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

#[path = "../../../tests/support/fixture_dir.rs"]
mod fixture_dir;

use arda_blocks::society::SocietyOverlays;
use arda_blocks::{OverlayCtx, Overlays, Owner, Pipeline};
use arda_settle::model::Tier;
use arda_tactical::layout::EdgeAxis;
use arda_tactical::Library;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

fn ready(dir: &Path) -> bool {
    dir.join("society/settlements.json").is_file()
        && dir.join("society/landuse.bin").is_file()
        && arda::World::load(dir)
            .is_ok_and(|w| w.manifest().fine_terrain.is_some() && w.seed() == 42)
}

fn world_dir() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        if let Some(dir) = std::env::var_os("ARDA_TEST_WORLD") {
            return PathBuf::from(dir);
        }
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let out = root.join("out/micro42");
        if ready(&out) {
            return out;
        }
        let dir = root.join("target/arda-people-fixture/micro42");
        fixture_dir::ensure(&dir, ready, |tmp| {
            arda::generate_from_fine_source(
                42,
                arda::GenerateConfig::MICRO,
                tmp,
                arda::FineDeliveryLimits::default(),
            )
            .expect("generating the MICRO fixture world");
            arda_settle::generate(tmp, arda_settle::grid::MEMORY_BUDGET).expect("settle");
        });
        dir
    })
    .clone()
}

fn lib() -> Library {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
    Library::load(&dir).unwrap()
}

type Edge = (u32, u32, EdgeAxis);

/// The squares an edge separates, inside the `w × h` window.
fn sides((x, y, axis): Edge, w: u32, h: u32) -> Vec<usize> {
    let (x, y) = (i64::from(x), i64::from(y));
    let pair = match axis {
        EdgeAxis::Horizontal => [(x, y), (x, y - 1)],
        EdgeAxis::Vertical => [(x, y), (x - 1, y)],
    };
    pair.iter()
        .filter(|&&(sx, sy)| sx >= 0 && sy >= 0 && sx < i64::from(w) && sy < i64::from(h))
        .map(|&(sx, sy)| usize::try_from(sy * i64::from(w) + sx).unwrap())
        .collect()
}

/// Every settlement's own cell for the four tiers' first few settlements
/// (the review's city, town, village and hamlet among them).
fn cells(world: &arda_people::World) -> Vec<(i64, i64)> {
    let mut want: Vec<(i64, i64)> = vec![(726, 1687), (400, 1449), (997, 1540), (908, 1344)];
    let mut per: BTreeMap<u8, usize> = BTreeMap::new();
    for s in &world.files.settlements.settlements {
        let t = match s.tier {
            Tier::City => 0,
            Tier::Town => 1,
            Tier::Village => 2,
            Tier::Hamlet => 3,
        };
        let n = per.entry(t).or_default();
        if *n < 2 {
            *n += 1;
            want.push((i64::from(s.cell_x), i64::from(s.cell_y)));
        }
    }
    want.sort_unstable();
    want.dedup();
    want
}

#[test]
fn settlement_cells_have_one_wall_per_edge_and_no_foreign_walls_in_town() {
    let dir = world_dir();
    let world = Arc::new(arda_people::World::open(&dir).unwrap());
    let sampled = cells(&world);
    // Plans of the sampled cells and of every 25th settlement.
    for (k, s) in world.files.settlements.settlements.iter().enumerate() {
        let own = sampled.contains(&(i64::from(s.cell_x), i64::from(s.cell_y)));
        if !own && k % 25 != 0 {
            continue;
        }
        if let Some(plan) = world.plan(s.id.get()).unwrap().as_ref() {
            let o = arda_town::plan::check::overlaps(plan);
            assert!(o.is_empty(), "{}: buildings overlap: {o:?}", plan.name);
        }
    }
    let society = SocietyOverlays::open(Arc::clone(&world)).unwrap();
    let pipeline = Pipeline::new(Box::new(world.src.clone()), 16);
    let library = lib();
    let mut checked = 0;
    for (gx, gy) in sampled {
        let win = (gx * 64, gy * 64, 64, 64);
        let composed = pipeline.window(win, &library, Some(&society)).unwrap();
        let base = pipeline.window(win, &library, None).unwrap().layout;
        let ctx = OverlayCtx {
            gsx0: win.0,
            gsy0: win.1,
            width: 64,
            height: 64,
            seed: pipeline.seed(),
            base: &base,
            library: &library,
        };
        let Some(town) = society.town(&ctx).unwrap() else {
            continue;
        };
        let walls = &composed.layout.walls;
        let edges: BTreeSet<Edge> = walls.iter().map(|w| (w.x, w.y, w.axis)).collect();
        assert_eq!(edges.len(), walls.len(), "cell {gx},{gy}: a doubled wall");
        let town_walls: BTreeMap<Edge, &str> = town
            .layout
            .walls
            .iter()
            .map(|w| ((w.x, w.y, w.axis), w.kit.as_str()))
            .collect();
        let townish = |i: usize| matches!(composed.owners[i], Owner::Town | Owner::Croft);
        let building = |i: usize| {
            composed.rules.squares[i]
                .ext
                .as_ref()
                .is_some_and(|x| x.contains_key("building"))
        };
        for w in walls {
            let e = (w.x, w.y, w.axis);
            let s = sides(e, 64, 64);
            // Inside the town's ground, or against a building, only the
            // town's own walls stand.
            let inside = s.iter().all(|&i| townish(i));
            let at_building = s.iter().any(|&i| building(i));
            if inside || at_building {
                assert_eq!(
                    town_walls.get(&e).copied(),
                    Some(w.kit.as_str()),
                    "cell {gx},{gy}: foreign {} wall at {e:?}",
                    w.kit
                );
            }
        }
        checked += 1;
    }
    assert!(checked >= 4, "only {checked} settlement cells checked");
}
