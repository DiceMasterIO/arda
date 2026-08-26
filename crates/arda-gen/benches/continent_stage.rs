//! Continent-stage budget bench (feature 02 spec R9).
//!
//! Steps 1–6 (plates through rivers) must finish within 60 s at the
//! default 500×1000 km size in release mode — well inside
//! `architecture-interview.md` §Q7's 30 min stage ceiling, because the
//! climate passes are the new hot loop and a regression here would
//! otherwise surface only in full-batch timing.
#![allow(missing_docs)]

use arda_core::GenerateConfig;
use arda_gen::continent::climate::climate;
use arda_gen::continent::generate_continent;
use arda_gen::continent::hydrology::{extract_rivers, hydrology};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::time::Duration;

const BUDGET: Duration = Duration::from_secs(60);

fn full_stage(seed: u64, config: GenerateConfig) -> usize {
    let g = generate_continent(seed, config);
    let c = climate(&g, config.latitude_band());
    let hy = hydrology(&g, &c);
    extract_rivers(&g, &hy).len()
}

fn bench_continent(c: &mut Criterion) {
    let mut group = c.benchmark_group("continent");
    group.sample_size(10);
    group.bench_function("micro_stage", |b| {
        b.iter(|| black_box(full_stage(42, GenerateConfig::MICRO)));
    });
    group.finish();

    // Fail loudly rather than only reporting: the budget is a gate.
    let config = GenerateConfig::default();
    let start = std::time::Instant::now();
    let rivers = full_stage(42, config);
    let elapsed = start.elapsed();
    assert!(
        elapsed <= BUDGET,
        "continent stage took {elapsed:?} (budget {BUDGET:?}, {rivers} rivers)"
    );
}

criterion_group!(benches, bench_continent);
criterion_main!(benches);
