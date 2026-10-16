//! Memory admission counts what the server holds (review 3.6-8): composed
//! town blocks with their `ext` maps, refined blocks, the refine source's
//! decoded areas, and the measured world and library, against the real
//! MICRO seed-42 society world.

use crate::support::{config, society_dir, society_state};
use arda_refine::{CellKey, Source, WorldSource};
use arda_server::admission::{pipeline_bytes, Resident};
use arda_server::refine_source::{BoundedSource, REFINE_AREAS};
use arda_server::tactical::block::BlockRequest;
use arda_server::tactical::cells::APRON;
use arda_server::tactical::refine_blocks::{block_bytes, REFINED_BLOCK_BYTES};
use arda_server::{ServerError, MEMORY_CEILING_BYTES};
use std::sync::Arc;

/// The fixture's largest city (Ykroy, population 12,827; recipe 9).
const TOWN: (u32, u32) = (547, 656);
/// The dense forest river cell of `cells.rs`.
const FOREST: (u32, u32) = (518, 822);
const WORLD: (i64, i64) = (1024 * 64, 2048 * 64);

#[test]
fn a_town_blocks_charge_includes_its_ext_maps() {
    let s = society_state();
    let req = BlockRequest::cell(TOWN.0, TOWN.1).with_apron(APRON, WORLD);
    let block = s.tactical.world_block(&req).unwrap();
    let rules = block.rules.as_ref().unwrap();
    let with_ext = rules.squares.iter().filter(|c| c.ext.is_some()).count();
    assert!(with_ext > 1_000, "{with_ext} squares carry ext in town");
    let mut bare = block.clone();
    for c in &mut bare.rules.as_mut().unwrap().squares {
        c.ext = None;
    }
    let (charged, without) = (block_bytes(&block), block_bytes(&bare));
    // Each map is at least one 11-entry B-tree node of strings and JSON
    // values (632 bytes) beyond its serialised text.
    assert!(
        charged - without >= with_ext * 632,
        "{charged} vs {without} bytes for {with_ext} ext maps"
    );
    // The old charge (88 bytes a square, 64 an item) was about 640 KB:
    // under a third of what a town block holds.
    let old = block.layout.squares.len() * 88
        + (block.layout.placements.len() + block.layout.walls.len()) * 64
        + 1024;
    eprintln!("town block: charged {charged} bytes, old charge {old}");
    assert!(charged > 3 * old);
}

#[test]
fn refined_blocks_fit_their_admitted_bound() {
    let src = BoundedSource::open(society_dir()).unwrap();
    let pipeline = arda_blocks::Pipeline::new(Box::new(src), 4);
    for (gx, gy) in [TOWN, FOREST] {
        let b = pipeline
            .refined(CellKey::new(i64::from(gx), i64::from(gy)))
            .unwrap();
        let bytes = std::mem::size_of_val(b.ground.as_slice())
            + std::mem::size_of_val(b.elevation_ft.as_slice())
            + std::mem::size_of_val(b.depth_ft.as_slice())
            + std::mem::size_of_val(b.dryness.as_slice())
            + std::mem::size_of_val(b.fixed.as_slice())
            + std::mem::size_of_val(b.corners.as_slice())
            + std::mem::size_of_val(b.items.as_slice())
            + std::mem::size_of_val(b.rules.as_slice())
            + b.halo.w * b.halo.h * std::mem::size_of::<(i16, u8)>();
        eprintln!("refined {gx},{gy}: {bytes} bytes, {} items", b.items.len());
        assert!(
            (bytes as u64) * 2 < REFINED_BLOCK_BYTES,
            "{gx},{gy}: {bytes}"
        );
    }
}

#[test]
fn the_refine_source_keeps_its_area_bound_and_answers_like_the_world() {
    let world = arda::World::load(society_dir()).unwrap();
    let full = WorldSource::new(&world).unwrap();
    // One cell in each of the MICRO world's 2 x 4 areas, twice.
    let cells: Vec<_> = (0..4)
        .flat_map(|ay| (0..2).map(move |ax| CellKey::new(ax * 512 + 100, ay * 512 + 200)))
        .collect();
    assert_eq!(cells.len(), REFINE_AREAS);
    for keep in [REFINE_AREAS, 6] {
        let bounded = BoundedSource::keeping(society_dir(), keep).unwrap();
        for _ in 0..2 {
            for &c in &cells {
                assert_eq!(bounded.cell(c).unwrap(), full.cell(c).unwrap(), "{c:?}");
                assert_eq!(bounded.lake(c).unwrap(), full.lake(c).unwrap());
                assert_eq!(
                    bounded.edges_touching(c).unwrap(),
                    full.edges_touching(c).unwrap()
                );
                assert_eq!(bounded.thalweg(c).unwrap(), full.thalweg(c).unwrap());
                assert_eq!(bounded.shore(c).unwrap(), full.shore(c).unwrap());
                assert_eq!(
                    bounded.fine_mm(c.x * 2, c.y * 2).unwrap(),
                    full.fine_mm(c.x * 2, c.y * 2).unwrap()
                );
            }
        }
        // All 8 kept: 8 openings; 6 kept, visited in a cycle: each of the
        // 16 visits opens its area again.
        let opens = bounded.area_opens();
        eprintln!("keep {keep}: {opens} area openings");
        assert_eq!(opens, if keep == REFINE_AREAS { 8 } else { 16 });
        assert_eq!(bounded.verifications(), 1);
    }
}

#[test]
fn the_default_config_is_admitted_with_the_measured_world() {
    let s = society_state();
    let c = config();
    let need = c.admitted_bytes() + s.resident.total();
    eprintln!(
        "admitted {} MiB (pipeline {} MiB) + resident {:?} = {} MiB of {} MiB",
        c.admitted_bytes() >> 20,
        pipeline_bytes() >> 20,
        s.resident,
        need >> 20,
        MEMORY_CEILING_BYTES >> 20
    );
    assert!(s.resident.library > 0 && s.resident.society > 0 && s.resident.shore > 0);
    assert!(need <= MEMORY_CEILING_BYTES);
    // The defaults leave room for a larger world's society and shore.
    let default = arda_server::ServerConfig::new("unused".into());
    assert!(default.admitted_bytes() + (256 << 20) < MEMORY_CEILING_BYTES);
}

#[test]
fn startup_refuses_a_world_over_the_ceiling() {
    let c = config();
    let fits = Resident::default();
    assert!(c.admit_resident(&fits).is_ok());
    let huge = Resident {
        society: MEMORY_CEILING_BYTES - c.admitted_bytes() + 1,
        ..Resident::default()
    };
    let err = c.admit_resident(&huge).unwrap_err();
    assert!(matches!(err, ServerError::ResourceLimit(ref m) if m.contains("society")));
    // Through AppState::open: budgets that fit alone, but not with the
    // world, refuse to start.
    let mut tight = config();
    tight.world = society_dir().to_path_buf();
    let spare = MEMORY_CEILING_BYTES - tight.admitted_bytes();
    tight.query.area_bytes += usize::try_from(spare).unwrap() - (16 << 20);
    assert!(tight.admit().is_ok());
    let opened = arda_server::AppState::open(&tight).map(Arc::new);
    assert!(
        matches!(opened, Err(ServerError::ResourceLimit(ref m)) if m.contains("library")),
        "{:?}",
        opened.err()
    );
}

/// Four readers, each cycling through its own pair of the MICRO world's
/// 2 x 4 areas, `rounds` times in step (so every round touches all 8
/// areas).
fn read_in_parallel(src: &BoundedSource, rounds: u16) {
    let step = std::sync::Barrier::new(4);
    std::thread::scope(|scope| {
        for reader in 0..4_i64 {
            let step = &step;
            scope.spawn(move || {
                for round in 0..rounds {
                    step.wait();
                    for ax in 0..2 {
                        let at =
                            CellKey::new(ax * 512 + 100 + i64::from(round), reader * 512 + 200);
                        src.cell(at).unwrap();
                        src.lake(at).unwrap();
                    }
                }
            });
        }
    });
}

#[test]
fn concurrent_readers_beyond_the_area_bound_never_reverify_the_world() {
    // The default-size startup hang: readers over more areas than the
    // source kept reopened the whole world every few reads, each opening
    // re-verifying the 1.25 GB fine-terrain file under the source's lock.
    // Now an area opening shares the verified layers and only re-decodes
    // the area, and readers whose areas fit open each once.
    let rounds: u16 = 20;
    let fits = BoundedSource::open(society_dir()).unwrap();
    read_in_parallel(&fits, rounds);
    assert_eq!(fits.area_opens(), 8, "8 areas fit the bound");
    assert_eq!(fits.verifications(), 1);
    assert!(fits.areas_share_layers());
    // Six kept for eight areas read in step: areas are evicted and opened
    // again, but every reader finishes and nothing is verified again.
    let tight = BoundedSource::keeping(society_dir(), 6).unwrap();
    read_in_parallel(&tight, rounds);
    let opens = tight.area_opens();
    eprintln!("4 readers, 8 areas, 6 kept, {rounds} rounds: {opens} area openings");
    assert!(opens > 8 && opens <= 8 * u64::from(rounds), "{opens}");
    assert_eq!(tight.verifications(), 1);
    assert!(tight.areas_share_layers());
}

#[test]
fn the_plan_cache_is_bounded_in_bytes_and_admits_its_largest_plan() {
    let s = society_state();
    let w = s.people.as_ref().unwrap();
    let largest = w.largest_plan_drawn();
    assert!(largest > 0, "the warm-up drew no plan");
    assert_eq!(s.resident.plans, largest);
    let held = w.cached_plan_bytes().unwrap();
    assert!(
        held <= arda_people::people::PLAN_CACHE_BYTES + largest,
        "{held}"
    );
    // A plan's charge includes its square grid, which JSON omits.
    let id = w
        .files
        .settlements
        .settlements
        .iter()
        .max_by_key(|s| s.population)
        .unwrap()
        .id
        .get();
    let plan = w.plan(id).unwrap();
    let plan = plan.as_ref().as_ref().unwrap();
    let json = serde_json::to_vec(plan).unwrap().len() as u64;
    let bytes = arda_people::people::plan_bytes(plan);
    assert!(
        bytes >= 2 * json + plan.grid.kind.len() as u64,
        "{bytes} vs {json}"
    );
    assert!(pipeline_bytes() >= arda_people::people::PLAN_CACHE_BYTES);
}

#[test]
fn plan_draws_reserve_their_estimate_within_the_plan_budget() {
    use arda_people::people::{plan_bytes, plan_estimate};
    use rayon::prelude::*;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    let w = arda_people::World::open(society_dir()).unwrap();
    // Small enough that the towns' and cities' draws (about 26 to 95 MB
    // reserved each) must wait for one another.
    let budget = 128 << 20;
    w.set_plan_budget(budget);
    let mut all: Vec<_> = w.files.settlements.settlements.iter().collect();
    all.sort_by_key(|s| (std::cmp::Reverse(s.population), s.id.get()));
    let ids: Vec<u64> = all
        .iter()
        .enumerate()
        .filter(|&(i, _)| i < 9 || i % 97 == 0)
        .map(|(_, s)| s.id.get())
        .collect();
    let (done, peak) = (AtomicBool::new(false), AtomicU64::new(0));
    std::thread::scope(|scope| {
        scope.spawn(|| {
            while !done.load(Ordering::SeqCst) {
                let held = w.cached_plan_bytes().unwrap() + w.drawing_plan_bytes().unwrap();
                peak.fetch_max(held, Ordering::SeqCst);
                std::thread::yield_now();
            }
        });
        // Rayon callers (the warm-up, overlays, relief) and plain threads
        // (request handlers) at once: neither deadlocks on a reservation.
        scope.spawn(|| {
            for &id in ids.iter().rev() {
                w.plan(id).unwrap();
            }
        });
        ids.par_iter().for_each(|&id| {
            w.plan(id).unwrap();
        });
        done.store(true, Ordering::SeqCst);
    });
    let largest = w.largest_plan_drawn();
    let peak = peak.load(Ordering::SeqCst);
    eprintln!("budget {budget}, peak held {peak}, largest plan {largest}");
    // Within the budget, but for the newest plan, kept even past it.
    assert!(peak <= budget + largest, "{peak}");
    assert_eq!(w.drawing_plan_bytes().unwrap(), 0);
    // Each estimate covers its plan, and the plans are the ones a world
    // with the default budget draws.
    let reference = arda_people::World::open(society_dir()).unwrap();
    for s in all.iter().filter(|s| ids.contains(&s.id.get())) {
        let plan = w.plan(s.id.get()).unwrap();
        if let Some(p) = plan.as_ref() {
            assert!(plan_bytes(p) <= plan_estimate(s), "{}", s.id.get());
        }
        let again = reference.plan(s.id.get()).unwrap();
        assert_eq!(
            serde_json::to_vec(plan.as_ref()).unwrap(),
            serde_json::to_vec(again.as_ref()).unwrap()
        );
    }
}
