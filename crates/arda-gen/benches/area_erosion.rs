//! Erosion budget bench.
//!
//! Both erosion passes must finish within 30 s per 512x512 tile in release
//! mode. Across 190 areas that is ~1.6 h, which leaves headroom under the
//! 12 h whole-batch ceiling for the block stage — the real cost centre once
//! blocks are materialised per land cell.
#![allow(missing_docs)]

use arda_core::{AreaCoord, GenerateConfig, AREA_CELLS};
use arda_gen::area::{erosion, relief};
use arda_gen::continent::bundles::bundle_for;
use arda_gen::continent::generate_continent;
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::time::Duration;

const N: i32 = AREA_CELLS as i32;

/// The budget one tile's erosion must fit inside.
const BUDGET: Duration = Duration::from_secs(30);

fn bench_erosion(c: &mut Criterion) {
    let continent = generate_continent(42, GenerateConfig::MICRO);
    let bundle = bundle_for(42, &continent, AreaCoord::new(0, 1));
    let r = relief(42, &continent, &bundle);
    let base: Vec<i32> = (0..(N * N))
        .filter_map(|i| {
            let at =
                arda_core::CellCoord::new(u16::try_from(i % N).ok()?, u16::try_from(i / N).ok()?)?;
            Some(r.get(at))
        })
        .collect();

    let mut group = c.benchmark_group("area");
    group.sample_size(10);
    group.bench_function("erode_one_tile", |b| {
        b.iter(|| {
            let mut h = base.clone();
            erosion::erode(&mut h, &base, &bundle);
            black_box(h[0])
        });
    });
    group.finish();

    // Fail loudly rather than only reporting: the budget is a gate.
    let start = std::time::Instant::now();
    let mut h = base.clone();
    erosion::erode(&mut h, &base, &bundle);
    let elapsed = start.elapsed();
    assert!(
        elapsed < BUDGET,
        "erosion took {elapsed:?}, over the {BUDGET:?} per-tile budget"
    );
}

criterion_group!(benches, bench_erosion);
criterion_main!(benches);
