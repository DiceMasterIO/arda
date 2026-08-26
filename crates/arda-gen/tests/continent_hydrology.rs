//! Feature 02 spec R7 invariants over a real micro continent, plus the
//! persistence round-trip through a generated world.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda_core::{decode_continent_objects, decode_overview, GenerateConfig, LatitudeBand};
use arda_gen::continent::climate::climate;
use arda_gen::continent::generate_continent;
use arda_gen::continent::hydrology::{extract_rivers, hydrology};
use arda_gen::orchestrator::generate_world;
use std::path::PathBuf;

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
    // Spec R7 invariant (a) on the real seed-42 micro continent.
    let g = generate_continent(42, GenerateConfig::MICRO);
    let c = climate(&g, LatitudeBand::new(35, 55));
    let hy = hydrology(&g, &c);
    let (w, h) = (g.width(), g.height());
    for i in 0..(w * h) as usize {
        if g.get(i as i32 % w, i as i32 / w).raw() <= 0 {
            continue;
        }
        let mut at = i;
        let mut steps = 0;
        while g.get(at as i32 % w, at as i32 / w).raw() > 0 {
            let d = hy.downstream[at].unwrap_or_else(|| panic!("dead end at {at}"));
            at = d as usize;
            steps += 1;
            assert!(steps <= (w * h) as usize, "cycle from {i}");
        }
    }
}

#[test]
fn micro_rivers_satisfy_the_course_invariants() {
    // Spec R7 invariants (b) on courses and (c): connected, descending
    // on the routing surface, feeds acyclic.
    let g = generate_continent(42, GenerateConfig::MICRO);
    let c = climate(&g, LatitudeBand::new(35, 55));
    let hy = hydrology(&g, &c);
    let rivers = extract_rivers(&g, &hy);
    assert!(!rivers.is_empty());
    let w = g.width() as usize;
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
            let idx = |c: &arda_core::KmCoord| c.y as usize * w + c.x as usize;
            assert!(
                hy.filled[idx(a)] >= hy.filled[idx(b)],
                "course climbs in river {}",
                r.id
            );
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
