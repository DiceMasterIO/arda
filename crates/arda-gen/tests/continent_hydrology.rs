//! Feature 02 spec R7 invariants over a real micro continent, plus the
//! persistence round-trip through a generated world.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda_core::{decode_continent_objects, decode_overview, GenerateConfig, LatitudeBand};
use arda_gen::continent::climate::climate;
use arda_gen::continent::hydrology::{extract_rivers, hydrology};
use arda_gen::continent::{generate_continent, generate_continent_attempt};
use arda_gen::orchestrator::generate_world;
use std::path::PathBuf;

// The seed-42 MICRO attempt accepted by the continent land and river gates.
const ACCEPTED_MICRO_ATTEMPT: u8 = 2;

struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("arda-ch-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn micro_world_persists_a_real_continent_layer() {
    let dir = TempDir::new();
    generate_world(42, GenerateConfig::MICRO, &dir.0).unwrap();

    let bytes = std::fs::read(dir.0.join("continent/overview.bin")).unwrap();
    let overview = decode_overview("continent/overview.bin", &bytes).unwrap();
    assert_eq!(overview.width, 102);
    assert_eq!(overview.height, 204);
    assert!(overview.cells.iter().any(|c| c.rainfall.raw() > 0));
    assert!(overview.cells.iter().any(|c| c.catchment_km2 > 0));

    let bytes = std::fs::read(dir.0.join("continent/objects.bin")).unwrap();
    let objects = decode_continent_objects("continent/objects.bin", &bytes).unwrap();
    assert!(
        !objects.rivers.is_empty(),
        "micro world has no rivers (§Q7 floor)"
    );
}

#[test]
fn every_micro_land_cell_reaches_the_ocean() {
    // Spec R7 invariant (a) on the accepted seed-42 micro continent.
    let g = generate_continent_attempt(42, GenerateConfig::MICRO, ACCEPTED_MICRO_ATTEMPT);
    let c = climate(&g, LatitudeBand::new(35, 55));
    let hy = hydrology(&g, &c);
    let (w, h) = (g.width(), g.height());
    for i in 0..usize::try_from(w * h).unwrap_or(0) {
        if g.get(
            i32::try_from(i).unwrap_or(0) % w,
            i32::try_from(i).unwrap_or(0) / w,
        )
        .raw()
            <= 0
        {
            continue;
        }
        let mut at = i;
        let mut steps = 0;
        while g
            .get(
                i32::try_from(at).unwrap_or(0) % w,
                i32::try_from(at).unwrap_or(0) / w,
            )
            .raw()
            > 0
        {
            let d = hy.downstream[at].unwrap_or_else(|| panic!("dead end at {at}"));
            at = usize::try_from(d).unwrap_or(0);
            steps += 1;
            assert!(
                steps <= usize::try_from(w * h).unwrap_or(0),
                "cycle from {i}"
            );
        }
    }
}

#[test]
fn micro_rivers_satisfy_the_course_invariants() {
    // Spec R7 invariant (c) on courses: connected, descending on the
    // routing surface, feeds acyclic. Invariants (b) (catchment/discharge
    // monotonicity) and (d) (zero catchment/discharge on sea cells) are
    // covered separately below, against this same accepted seed-42 terrain.
    let g = generate_continent_attempt(42, GenerateConfig::MICRO, ACCEPTED_MICRO_ATTEMPT);
    let c = climate(&g, LatitudeBand::new(35, 55));
    let hy = hydrology(&g, &c);
    let rivers = extract_rivers(&g, &hy);
    assert_eq!(rivers.len(), 2, "accepted MICRO terrain lost its rivers");
    assert_eq!(
        rivers.iter().filter(|r| r.feeds.is_none()).count(),
        2,
        "accepted MICRO terrain lost its sea-reaching rivers"
    );
    let w = usize::try_from(g.width()).unwrap_or(0);
    for r in &rivers {
        if let Some(f) = r.feeds {
            assert!(f < r.id);
        }
        for pair in r.course.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            let (dx, dy) = (
                i32::from(b.x) - i32::from(a.x),
                i32::from(b.y) - i32::from(a.y),
            );
            assert!(
                dx.abs() <= 1 && dy.abs() <= 1,
                "course break in river {}",
                r.id
            );
            let idx = |c: &arda_core::KmCoord| usize::from(c.y) * w + usize::from(c.x);
            assert!(
                hy.filled[idx(a)] >= hy.filled[idx(b)],
                "course climbs in river {}",
                r.id
            );
        }
    }
}

#[test]
fn micro_hydrology_satisfies_catchment_and_sea_invariants() {
    // Spec R7 invariants (b) and (d), on the accepted seed-42 micro continent
    // (hydrology.rs's unit tests only cover these against the synthetic
    // "dome" fixture, never against generated terrain).
    let g = generate_continent_attempt(42, GenerateConfig::MICRO, ACCEPTED_MICRO_ATTEMPT);
    let c = climate(&g, LatitudeBand::new(35, 55));
    let hy = hydrology(&g, &c);
    let w = g.width();
    let land = |j: usize| {
        g.get(
            i32::try_from(j).unwrap_or(0) % w,
            i32::try_from(j).unwrap_or(0) / w,
        )
        .raw()
            > 0
    };

    // (b) catchment and discharge never shrink downstream, land to land.
    for (i, d) in hy.downstream.iter().enumerate() {
        let Some(d) = *d else { continue };
        let d = usize::try_from(d).unwrap_or(0);
        if land(i) && land(d) {
            assert!(
                hy.catchment_km2[d] >= hy.catchment_km2[i],
                "catchment shrank downstream from {i} to {d}"
            );
            assert!(
                hy.discharge_l_s[d] >= hy.discharge_l_s[i],
                "discharge shrank downstream from {i} to {d}"
            );
        }
    }

    // (d) sea cells store zero catchment and discharge.
    for i in 0..hy.catchment_km2.len() {
        if !land(i) {
            assert_eq!(hy.catchment_km2[i], 0, "sea cell {i} has nonzero catchment");
            assert_eq!(hy.discharge_l_s[i], 0, "sea cell {i} has nonzero discharge");
        }
    }
}

#[test]
fn the_continent_stage_is_deterministic_end_to_end() {
    // Spec R7 invariant (e) at stage level; the golden gate covers the
    // byte level after Task 10.
    let run = || {
        let g = generate_continent(7, GenerateConfig::MICRO);
        let c = climate(&g, LatitudeBand::new(35, 55));
        let hy = hydrology(&g, &c);
        let r = extract_rivers(&g, &hy);
        (c, hy, r)
    };
    assert_eq!(run(), run());
}
