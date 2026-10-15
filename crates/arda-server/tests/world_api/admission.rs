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

/// The fixture's largest city (Ildeflel, population 8,650; recipe 9).
const TOWN: (u32, u32) = (619, 625);
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
fn the_refine_source_starts_a_new_generation_past_its_area_bound() {
    let bounded = BoundedSource::open(society_dir()).unwrap();
    let world = arda::World::load(society_dir()).unwrap();
    let full = WorldSource::new(&world).unwrap();
    // One cell in each of the MICRO world's 2 x 4 areas, twice.
    let cells: Vec<_> = (0..4)
        .flat_map(|ay| (0..2).map(move |ax| CellKey::new(ax * 512 + 100, ay * 512 + 200)))
        .collect();
    assert!(cells.len() > REFINE_AREAS);
    for round in 0..2 {
        for &c in &cells {
            assert_eq!(bounded.cell(c).unwrap(), full.cell(c).unwrap(), "{c:?}");
            assert_eq!(bounded.lake(c).unwrap(), full.lake(c).unwrap());
            assert_eq!(
                bounded.edges_touching(c).unwrap(),
                full.edges_touching(c).unwrap()
            );
        }
        eprintln!("round {round}: {} generations", bounded.generations());
    }
    // 8 areas a round, 6 per generation: 16 area visits need 3 openings.
    assert_eq!(bounded.generations(), 3);
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
