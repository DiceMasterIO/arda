//! Town plans survive composition on the real seed-42 MICRO world (review
//! 3.5-2, 3.5-3, 3.5-4, 3.5-6): every dock is drawn as a deck over water,
//! no planned building loses a square to the refined water, a building's
//! floor is level and a wall-walk stands 20 ft over the ground in the
//! composed block, and a road a town's streets replace becomes street or
//! stays road.
//!
//! The default test composes every cell a sampled plan's buildings touch
//! (every eighth plan with docks, the smallest walled plan and the six
//! steepest sites);
//! `cargo test -p arda-blocks --test tactical_drops -- --ignored` measures
//! every plan of the world and prints the totals.
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

use arda_blocks::society::town::{plans_near, Streets};
use arda_blocks::society::SocietyOverlays;
use arda_blocks::{Composed, OverlayCtx, Overlays, Owner, Pipeline};
use arda_tactical::Library;
use arda_town::plan::grid::Kind;
use arda_town::{BuildingFunction, TownPlan};
use rayon::prelude::*;
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

const N: i64 = 64;

/// One composed cell and what the test needs beside it.
struct Cell {
    origin: (i64, i64),
    composed: Composed,
    base: arda_tactical::TacticalLayout,
    /// Row-major: a road square (`road` or `ruts`) inside a town's reach.
    replaced: Vec<bool>,
    /// Row-major: a road square that gives way to a town's street.
    replaced_now: Vec<bool>,
    /// Row-major: a square some nearby plan makes public street ground.
    street: Vec<bool>,
    /// Row-major: final owner and the first nearby plan's kind there.
    what: Vec<String>,
}

impl Cell {
    fn at(&self, gx: i64, gy: i64) -> Option<usize> {
        let (x, y) = (gx - self.origin.0, gy - self.origin.1);
        ((0..N).contains(&x) && (0..N).contains(&y)).then(|| (y * N + x) as usize)
    }
}

fn compose(
    pipeline: &Pipeline,
    society: &SocietyOverlays,
    library: &Library,
    (cx, cy): (i64, i64),
) -> Cell {
    let win = (cx * N, cy * N, N as u32, N as u32);
    let composed = pipeline.window(win, library, Some(society)).unwrap();
    let base = pipeline.window(win, library, None).unwrap().layout;
    let ctx = OverlayCtx {
        gsx0: win.0,
        gsy0: win.1,
        width: N as u32,
        height: N as u32,
        seed: pipeline.seed(),
        base: &base,
        library,
    };
    let n = (N * N) as usize;
    let plans = plans_near(&ctx, society.world(), 0.0).unwrap();
    let plans: Vec<&TownPlan> = plans
        .iter()
        .filter_map(|(_, p)| p.as_ref().as_ref())
        .collect();
    let street = (0..n)
        .map(|i| {
            let (gx, gy) = (win.0 + i as i64 % N, win.1 + i as i64 / N);
            plans
                .iter()
                .any(|p| arda_town::block::ground::kind(p, gx, gy).public())
        })
        .collect();
    let what = (0..n)
        .map(|i| {
            let (gx, gy) = (win.0 + i as i64 % N, win.1 + i as i64 / N);
            let kind = plans
                .iter()
                .map(|p| arda_town::block::ground::kind(p, gx, gy))
                .find(|&k| k != Kind::Open)
                .unwrap_or(Kind::Open);
            format!("{}/{kind:?}", composed.owners[i].name())
        })
        .collect();
    let mut replaced = vec![false; n];
    let mut replaced_now = vec![false; n];
    if let Some(ways) = society.ways(&ctx).unwrap() {
        let streets = Streets::of(
            &plans_near(&ctx, society.world(), arda_blocks::society::WAYS_MARGIN_M).unwrap(),
        );
        for (i, r) in replaced.iter_mut().enumerate() {
            let feature = ways.rules.squares[i]
                .ext
                .as_ref()
                .and_then(|x| x.get("feature"))
                .and_then(|v| v.as_str().map(str::to_owned));
            let road = matches!(feature.as_deref(), Some("road" | "ruts"));
            let (gx, gy) = (win.0 + i as i64 % N, win.1 + i as i64 / N);
            *r = road && streets.within(gx, gy);
            replaced_now[i] = road && streets.replace(gx, gy);
        }
    }
    Cell {
        origin: (win.0, win.1),
        composed,
        base,
        replaced,
        replaced_now,
        street,
        what,
    }
}

#[derive(Debug, Default)]
struct Stats {
    plans: usize,
    cells: usize,
    docks: usize,
    docks_drawn: usize,
    /// Docks under a neighbouring plan's earlier claim.
    docks_shadowed: usize,
    walled: usize,
    losing: usize,
    homes_losing: usize,
    steep: usize,
    stepped: usize,
    steep_stepped: usize,
    wall_squares: usize,
    wall_walks: usize,
    replaced: usize,
    gaps: usize,
    /// Road squares given up to a street that is not there.
    given_up: usize,
    /// Gaps by final owner (and plan kind under a town).
    gap_owners: BTreeMap<String, usize>,
    failures: Vec<String>,
}

const fn home(f: BuildingFunction) -> bool {
    matches!(
        f,
        BuildingFunction::House
            | BuildingFunction::Cottage
            | BuildingFunction::Farmhouse
            | BuildingFunction::Manor
    )
}

/// Composes every cell the buildings of `plans` touch and measures them.
fn measure(ids: &[u64]) -> Stats {
    let dir = world_dir();
    let world = Arc::new(arda_people::World::open(&dir).unwrap());
    let society = SocietyOverlays::open(Arc::clone(&world)).unwrap();
    let pipeline = Pipeline::new(Box::new(world.src.clone()), 256);
    let library = lib();
    let plans: Vec<(u64, Arc<Option<TownPlan>>)> = ids
        .iter()
        .map(|&id| (id, world.plan(id).unwrap()))
        .filter(|(_, p)| p.is_some())
        .collect();
    let mut cells: BTreeSet<(i64, i64)> = BTreeSet::new();
    for (_, plan) in &plans {
        let plan = plan.as_ref().as_ref().unwrap();
        for b in &plan.buildings {
            let r = b.rect;
            for cy in r.y0.div_euclid(N)..=(r.y1 - 1).div_euclid(N) {
                for cx in r.x0.div_euclid(N)..=(r.x1 - 1).div_euclid(N) {
                    cells.insert((cx, cy));
                }
            }
        }
        // Wall squares: the curtain's cells.
        let g = &plan.grid;
        for k in 0..g.kind.len() {
            if g.kind[k] == Kind::Wall {
                let (i, j) = g.ij(k);
                cells.insert(((g.gx0 + i).div_euclid(N), (g.gy0 + j).div_euclid(N)));
            }
        }
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(6)
        .build()
        .unwrap();
    let list: Vec<(i64, i64)> = cells.into_iter().collect();
    let composed: BTreeMap<(i64, i64), Cell> = pool.install(|| {
        list.par_iter()
            .map(|&c| (c, compose(&pipeline, &society, &library, c)))
            .collect()
    });
    let find = |gx: i64, gy: i64| -> Option<(&Cell, usize)> {
        let c = composed.get(&(gx.div_euclid(N), gy.div_euclid(N)))?;
        Some((c, c.at(gx, gy)?))
    };
    let mut s = Stats {
        plans: plans.len(),
        cells: composed.len(),
        ..Stats::default()
    };
    for (id, plan) in &plans {
        let plan = plan.as_ref().as_ref().unwrap();
        for b in &plan.buildings {
            let r = b.rect;
            let squares: Vec<(i64, i64)> = (r.y0..r.y1)
                .flat_map(|y| (r.x0..r.x1).map(move |x| (x, y)))
                .collect();
            let tagged = |c: &Cell, i: usize| {
                let ext = c.composed.rules.squares[i].ext.as_ref();
                ext.and_then(|x| x.get("building"))
                    .and_then(|v| v.as_str())
                    .is_some_and(|v| v == b.id.0.to_string())
                    && ext
                        .and_then(|x| x.get("settlement"))
                        .and_then(|v| v.as_str())
                        .is_some_and(|v| v == id.to_string())
            };
            let why = |ok: &dyn Fn(&Cell, usize) -> bool| -> String {
                squares
                    .iter()
                    .filter_map(|&(x, y)| match find(x, y) {
                        None => Some(format!("{x},{y} uncomposed")),
                        Some((c, i)) if !ok(c, i) => Some(format!(
                            "{x},{y} {}/{:?} base {} ft {} deck {:?}",
                            c.composed.owners[i].name(),
                            arda_town::block::ground::kind(plan, x, y),
                            c.base.squares[i].water_depth_ft,
                            c.base.squares[i].ground,
                            c.composed.rules.squares[i].deck
                        )),
                        Some(_) => None,
                    })
                    .take(4)
                    .collect::<Vec<_>>()
                    .join("; ")
            };
            if b.function == BuildingFunction::Dock {
                s.docks += 1;
                let deck = |c: &Cell, i: usize| {
                    c.composed.owners[i] == Owner::Town
                        && c.composed.rules.squares[i].deck == Some(true)
                        && tagged(c, i)
                };
                let drawn = squares
                    .iter()
                    .all(|&(x, y)| find(x, y).is_some_and(|(c, i)| deck(c, i)));
                // Two plans may reach the same water: the first (by id)
                // keeps a square, so a dock under a neighbour's bridge is
                // that neighbour's, not dropped (logic/09 §reservations).
                let neighbour = |c: &Cell, i: usize| {
                    c.composed.owners[i] == Owner::Town
                        && c.composed.rules.squares[i]
                            .ext
                            .as_ref()
                            .and_then(|x| x.get("settlement"))
                            .and_then(|v| v.as_str())
                            .is_none_or(|v| v != id.to_string())
                };
                let shadowed = squares
                    .iter()
                    .any(|&(x, y)| find(x, y).is_some_and(|(c, i)| neighbour(c, i)));
                if drawn {
                    s.docks_drawn += 1;
                } else if shadowed {
                    s.docks_shadowed += 1;
                } else if s.failures.len() < 12 {
                    s.failures.push(format!(
                        "{}: dock {} not drawn: {}",
                        plan.name,
                        b.id.0,
                        why(&deck)
                    ));
                }
                continue;
            }
            if !b.function.walled() {
                continue;
            }
            s.walled += 1;
            let mut lost = 0;
            let mut floors = BTreeSet::new();
            let mut ground = BTreeSet::new();
            for &(x, y) in &squares {
                match find(x, y) {
                    Some((c, i)) if c.composed.owners[i] == Owner::Town && tagged(c, i) => {
                        floors.insert(c.composed.layout.squares[i].elevation_ft);
                        ground.insert(c.base.squares[i].elevation_ft);
                    }
                    _ => lost += 1,
                }
            }
            if lost > 0 {
                s.losing += 1;
                if home(b.function) {
                    s.homes_losing += 1;
                }
                if s.failures.len() < 12 {
                    s.failures.push(format!(
                        "{}: building {} ({:?}) lost {lost} of {} squares: {}",
                        plan.name,
                        b.id.0,
                        b.function,
                        squares.len(),
                        why(&|c: &Cell, i: usize| c.composed.owners[i] == Owner::Town
                            && tagged(c, i))
                    ));
                }
            }
            let rise = ground.last().copied().unwrap_or(0) - ground.first().copied().unwrap_or(0);
            let steep = rise >= 10;
            s.steep += usize::from(steep);
            if floors.len() > 1 {
                s.stepped += 1;
                s.steep_stepped += usize::from(steep);
                if s.failures.len() < 12 {
                    s.failures.push(format!(
                        "{}: building {} floors at {floors:?}",
                        plan.name, b.id.0
                    ));
                }
            }
        }
        let g = &plan.grid;
        for k in 0..g.kind.len() {
            if g.kind[k] != Kind::Wall {
                continue;
            }
            let (i, j) = g.ij(k);
            if let Some((c, q)) = find(g.gx0 + i, g.gy0 + j) {
                if c.composed.owners[q] != Owner::Town {
                    continue;
                }
                s.wall_squares += 1;
                let up = c.composed.layout.squares[q].elevation_ft - c.base.squares[q].elevation_ft;
                s.wall_walks += usize::from(up == 20);
            }
        }
    }
    for c in composed.values() {
        for i in 0..(N * N) as usize {
            if !c.replaced[i] {
                continue;
            }
            s.replaced += 1;
            let o = c.composed.owners[i];
            // A road in a town's reach is road, or the town's: its street,
            // or a building, yard or wall that outranks a road.
            let ok = o == Owner::Ways || o == Owner::Town;
            if c.replaced_now[i] && !(o == Owner::Town && c.street[i]) {
                s.given_up += 1;
            }
            if !ok {
                s.gaps += 1;
                *s.gap_owners.entry(c.what[i].clone()).or_default() += 1;
            }
        }
    }
    s
}

/// Every eighth plan with docks, the smallest walled plan and the six
/// steepest sites.
fn sample() -> Vec<u64> {
    let world = arda_people::World::open(&world_dir()).unwrap();
    let mut ids = BTreeSet::new();
    let mut docks = Vec::new();
    let mut walled: Vec<(u32, u64)> = Vec::new();
    let mut steep: Vec<(i64, u64)> = Vec::new();
    for x in &world.files.settlements.settlements {
        let id = x.id.get();
        let Some(plan) = world.plan(id).unwrap().as_ref().clone() else {
            continue;
        };
        if plan
            .buildings
            .iter()
            .any(|b| b.function == BuildingFunction::Dock)
        {
            docks.push(id);
        }
        if plan.wall.is_some() {
            walled.push((x.population, id));
        }
        let mean = plan.grid.slope.iter().map(|&v| i64::from(v)).sum::<i64>()
            / i64::try_from(plan.grid.slope.len().max(1)).unwrap();
        steep.push((-mean, id));
    }
    ids.extend(docks.iter().step_by(8));
    walled.sort_unstable();
    ids.extend(walled.first().map(|&(_, id)| id));
    steep.sort_unstable();
    ids.extend(steep.iter().take(6).map(|&(_, id)| id));
    ids.into_iter().collect()
}

fn report(s: &Stats) {
    eprintln!(
        "plans {} cells {}: docks drawn {}/{} (under a neighbouring plan {}); walled buildings losing squares {}/{} (homes {}); \
         stepped floors {}/{} (steep sites {}/{}); wall-walks +20 ft {}/{}; road squares in a \
         town's reach that are neither road nor town {}/{} (given up to no street {})",
        s.plans,
        s.cells,
        s.docks_drawn,
        s.docks,
        s.docks_shadowed,
        s.losing,
        s.walled,
        s.homes_losing,
        s.stepped,
        s.walled,
        s.steep_stepped,
        s.steep,
        s.wall_walks,
        s.wall_squares,
        s.gaps,
        s.replaced,
        s.given_up
    );
    eprintln!("  gaps by owner/plan kind: {:?}", s.gap_owners);
    for f in &s.failures {
        eprintln!("  {f}");
    }
}

#[test]
fn planned_buildings_docks_floors_and_streets_survive_composition() {
    let s = measure(&sample());
    report(&s);
    assert!(
        s.docks > 0 && s.walled > 100 && s.steep > 0 && s.wall_squares > 0 && s.replaced > 0,
        "{s:?}"
    );
    assert_eq!(s.docks_drawn + s.docks_shadowed, s.docks, "docks not drawn");
    assert!(
        s.docks_shadowed * 50 <= s.docks,
        "{} docks shadowed",
        s.docks_shadowed
    );
    assert_eq!(s.losing, 0, "buildings lost squares");
    assert_eq!(s.stepped, 0, "stepped floors");
    assert_eq!(s.wall_walks, s.wall_squares, "wall-walks off +20 ft");
    assert_eq!(s.gaps, 0, "road gaps at town entries");
    assert_eq!(s.given_up, 0, "roads given up to no street");
}

#[test]
#[ignore = "measures every plan of the world (minutes)"]
fn measure_every_plan() {
    let world = arda_people::World::open(&world_dir()).unwrap();
    let ids: Vec<u64> = world
        .files
        .settlements
        .settlements
        .iter()
        .filter(|x| {
            std::env::var("ARDA_DROPS_NAMES").map_or(true, |v| v.split(',').any(|n| n == x.name))
        })
        .map(|x| x.id.get())
        .collect();
    report(&measure(&ids));
}
