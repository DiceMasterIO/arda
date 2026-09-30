//! Render timing (goal 62): a 64 × 64-square map at 128 px per square must
//! render in under 2 s in release mode. Ignored by default because it is a
//! timing test; run it with
//! `cargo test --release -p arda-tactical --test timing -- --ignored --nocapture`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda_tactical::{layouts, render, Library, RenderOptions};
use std::path::Path;
use std::time::Instant;

#[test]
#[ignore = "timing test; run in release with --ignored"]
fn a_64_square_map_at_128_ppsq_renders_in_under_two_seconds() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
    let lib = Library::load(&dir).unwrap();
    let layout = layouts::benchmark(64);
    let opts = RenderOptions::default();
    // Warm up the thread pool and the page cache.
    let small = RenderOptions { ppsq: 16, ..opts };
    render(&layout, &lib, 1, &small).unwrap();
    let mut best = f64::MAX;
    for _ in 0..3 {
        let t = Instant::now();
        let img = render(&layout, &lib, 1, &opts).unwrap();
        best = best.min(t.elapsed().as_secs_f64());
        assert_eq!((img.width, img.height), (64 * 128, 64 * 128));
    }
    eprintln!(
        "64 x 64 squares at 128 ppsq: best of 3 {best:.3} s on {} threads",
        rayon::current_num_threads()
    );
    if !cfg!(debug_assertions) {
        assert!(best < 2.0, "render took {best:.3} s");
    }
}
