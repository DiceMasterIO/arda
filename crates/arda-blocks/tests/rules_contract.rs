//! The rules contract on the real seed-42 MICRO world (review 3.4-1, 3.4-4,
//! 3.5-1, 3.5-5, 3.5-7): every placement whose catalogue entry blocks
//! movement or sight is impassable or opaque in the built scene, on refined
//! terrain alone and in composed town and road windows; a verge keeps the
//! natural rules it stands on; a ford never fills in swimming water; and
//! every step touching cliff ground is a climb.
//!
//! World resolution as in `settlements.rs`: `$ARDA_TEST_WORLD`, then
//! `<workspace>/out/micro42` when it holds `society/`, else a MICRO world
//! generated and settled once into `target/arda-people-fixture/micro42`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    missing_docs
)]

#[path = "../../../tests/support/fixture_dir.rs"]
mod fixture_dir;

use arda_blocks::society::SocietyOverlays;
use arda_blocks::{Composed, Owner, Pipeline};
use arda_refine::{CellKey, Source};
use arda_scene::{build_scene, Movement, Obscurement, Scene};
use arda_tactical::catalog::Layer;
use arda_tactical::Library;
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

/// Squares whose centre lies inside `rect`, clamped to a `w × h` map (the
/// scene's footprint rule).
fn covered(rect: [f32; 4], w: u32, h: u32) -> Vec<usize> {
    let lo = |v: f32, max: u32| (v - 0.5).ceil().clamp(0.0, max as f32) as u32;
    let (x0, x1) = (lo(rect[0], w), lo(rect[2], w));
    let (y0, y1) = (lo(rect[1], h), lo(rect[3], h));
    (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| y as usize * w as usize + x as usize))
        .collect()
}

/// Blocking placements and their squares that the scene leaves open.
#[derive(Debug, Default)]
struct Tally {
    /// Movement-blocking placements checked.
    blocking: usize,
    /// ... of which some square is not impassable in the scene.
    walkable: usize,
    /// ... of which some square the folded sidecar does not block.
    unfolded: usize,
    /// Sight-blocking placements checked.
    opaque: usize,
    /// ... of which some square is not heavily obscured in the scene.
    see_through: usize,
    /// Boulder and large-rock placements, and those left walkable.
    rocks: usize,
    rocks_walkable: usize,
    /// Blocking props on town-owned squares, and those left walkable.
    town: usize,
    town_walkable: usize,
}

fn tally(c: &Composed, lib: &Library, seed: u64, scene: &Scene, t: &mut Tally) {
    let (w, h) = (c.layout.width, c.layout.height);
    let placed = arda_scene::squares::resolve_all(&c.layout, lib, seed).unwrap();
    for p in &placed {
        let a = p.asset;
        if matches!(a.layer, Layer::Canopy | Layer::Floor) {
            continue;
        }
        let squares = covered(p.rect, w, h);
        if squares.is_empty() {
            continue;
        }
        if a.blocks_movement {
            t.blocking += 1;
            let open = squares
                .iter()
                .any(|&i| scene.movement.0[i] != Movement::Impassable);
            t.walkable += usize::from(open);
            t.unfolded += usize::from(
                squares
                    .iter()
                    .any(|&i| c.rules.squares[i].blocks_movement != Some(true)),
            );
            if matches!(a.id.as_str(), "veg.boulder" | "veg.rock_large") {
                t.rocks += 1;
                t.rocks_walkable += usize::from(open);
            }
            let at = {
                let (x, y) = (p.at.0.floor() as i64, p.at.1.floor() as i64);
                (x >= 0 && y >= 0 && x < i64::from(w) && y < i64::from(h))
                    .then(|| y as usize * w as usize + x as usize)
            };
            if at.is_some_and(|i| c.owners[i] == Owner::Town) {
                t.town += 1;
                t.town_walkable += usize::from(open);
            }
        }
        if a.blocks_sight {
            t.opaque += 1;
            t.see_through += usize::from(
                squares
                    .iter()
                    .any(|&i| scene.obscured.0[i] != Obscurement::Heavy),
            );
        }
    }
}

/// Dry 8-neighbour edges touching cliff ground: all, those with a step,
/// those that are 10-ft steps, and those the scene makes a climb.
fn cliff_edges(c: &Composed, scene: &Scene) -> [usize; 4] {
    let (w, h) = (i64::from(c.layout.width), i64::from(c.layout.height));
    let mut n = [0; 4];
    let sq = |x: i64, y: i64| &c.layout.squares[(y * w + x) as usize];
    for y in 0..h {
        for x in 0..w {
            let a = sq(x, y);
            if a.water_depth_ft > 0 {
                continue;
            }
            for (bit, (dx, dy)) in arda_scene::squares::DIRS.iter().enumerate() {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let b = sq(nx, ny);
                if b.water_depth_ft > 0 || (a.ground != "cliff" && b.ground != "cliff") {
                    continue;
                }
                let dz = (i32::from(a.elevation_ft) - i32::from(b.elevation_ft)).abs();
                n[0] += 1;
                n[1] += usize::from(dz > 0);
                n[2] += usize::from(dz >= 10);
                let climb = scene.climb.0[(y * w + x) as usize] & (1 << bit) != 0;
                n[3] += usize::from(climb && dz > 0);
            }
        }
    }
    n
}

/// Steep land cells (cliffs, boulder fields) and coast, by a stride scan.
fn rough_cells(src: &dyn Source) -> Vec<(i64, i64)> {
    let (cw, ch) = src.cells_wide_high();
    let mut out = Vec::new();
    for y in (1..ch - 1).step_by(23) {
        for x in (1..cw - 1).step_by(29) {
            let c = src.cell(CellKey::new(x, y)).unwrap();
            if c.terrain == arda::TerrainKind::Land && c.slope_milli_deg > 22_000 {
                out.push((x, y));
            }
        }
    }
    out.truncate(10);
    out
}

#[test]
fn every_catalogue_blocker_blocks_and_overlays_keep_the_natural_rules() {
    let dir = world_dir();
    let world = Arc::new(arda_people::World::open(&dir).unwrap());
    let society = SocietyOverlays::open(Arc::clone(&world)).unwrap();
    let pipeline = Pipeline::new(Box::new(world.src.clone()), 32);
    let library = lib();
    let seed = pipeline.seed();
    // The settlement cells of every tier (composed), then rough land.
    let mut towns: Vec<(i64, i64)> = vec![(726, 1687), (400, 1449), (997, 1540), (908, 1344)];
    for s in world.files.settlements.settlements.iter().step_by(40) {
        towns.push((i64::from(s.cell_x), i64::from(s.cell_y)));
    }
    // Fords and other crossings, and stretches of road (verges).
    let roads = &world.files.roads;
    let fords = roads
        .crossings
        .iter()
        .filter(|c| format!("{:?}", c.kind) == "Ford")
        .take(30);
    let others = roads
        .crossings
        .iter()
        .filter(|c| format!("{:?}", c.kind) != "Ford")
        .take(6);
    for c in fords.chain(others) {
        towns.push((c.x_m.div_euclid(100), c.y_m.div_euclid(100)));
    }
    for r in roads.roads.iter().take(12) {
        if let Some(p) = r.segments.first().and_then(|s| s.get(s.len() / 2)) {
            towns.push((p[0].div_euclid(100), p[1].div_euclid(100)));
        }
    }
    towns.sort_unstable();
    towns.dedup();
    let rough = rough_cells(&world.src);
    assert!(!rough.is_empty(), "no steep cells sampled");
    let (mut natural, mut composed) = (Tally::default(), Tally::default());
    let mut cliffs = [0usize; 4];
    let (mut verge, mut verge_lost_difficult, mut verge_lost_sight) = (0, 0, 0);
    let (mut fords, mut fords_filled, mut all_fords) = (0, 0, 0);
    for (k, &(gx, gy)) in towns.iter().chain(&rough).enumerate() {
        let win = (gx * 64, gy * 64, 64, 64);
        let base = pipeline.window(win, &library, None).unwrap();
        let s = build_scene(&base.layout, &library, seed, Some(&base.rules)).unwrap();
        tally(&base, &library, seed, &s, &mut natural);
        let e = cliff_edges(&base, &s);
        for (a, b) in cliffs.iter_mut().zip(e) {
            *a += b;
        }
        if k >= towns.len() {
            continue;
        }
        let c = pipeline.window(win, &library, Some(&society)).unwrap();
        let s = build_scene(&c.layout, &library, seed, Some(&c.rules)).unwrap();
        tally(&c, &library, seed, &s, &mut composed);
        // Refine's own rules for the cell (the window is the whole cell).
        let block = pipeline.refined(CellKey::new(gx, gy)).unwrap();
        for i in 0..c.owners.len() {
            let (nat, now) = (&block.rules[i], &c.rules.squares[i]);
            if c.owners[i] == Owner::Verge {
                verge += 1;
                verge_lost_difficult += usize::from(nat.difficult && now.difficult != Some(true));
                verge_lost_sight += usize::from(nat.blocks_sight && now.blocks_sight != Some(true));
            }
            let ford = now
                .ext
                .as_ref()
                .and_then(|x| x.get("feature"))
                .is_some_and(|f| f == "ford");
            all_fords += usize::from(ford);
            if ford && base.layout.squares[i].water_depth_ft >= 5 {
                fords += 1;
                fords_filled += usize::from(c.layout.squares[i].water_depth_ft < 5);
            }
        }
    }
    println!("refined only: {natural:?}");
    println!("composed: {composed:?}");
    println!(
        "cliff edges: {} dry, {} stepped, {} of 10 ft, {} stepped climbs",
        cliffs[0], cliffs[1], cliffs[2], cliffs[3]
    );
    println!(
        "verge squares {verge}: lost difficult {verge_lost_difficult}, lost blocks_sight {verge_lost_sight}"
    );
    println!("ford squares {all_fords}, on >= 5 ft water {fords}, filled in {fords_filled}");
    assert!(natural.blocking > 0 && natural.rocks > 0, "{natural:?}");
    assert!(composed.town > 0, "{composed:?}");
    for t in [&natural, &composed] {
        assert_eq!(t.walkable, 0, "{t:?}");
        assert_eq!(t.see_through, 0, "{t:?}");
        assert_eq!(t.unfolded, 0, "{t:?}");
    }
    assert!(cliffs[1] > 0, "no stepped cliff edges sampled");
    assert_eq!(cliffs[3], cliffs[1], "a cliff step that is not a climb");
    assert!(verge > 0);
    assert_eq!((verge_lost_difficult, verge_lost_sight), (0, 0));
    assert_eq!(fords_filled, 0);
}
