//! Review round 2 #31: the stage admits a world at `BYTES_PER_CELL` bytes a
//! cell; the measured peak must stay inside that estimate. Alone in its test
//! binary, so no other test's allocations share the high-water mark.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda_settle::grid::BYTES_PER_CELL;

/// Resident bytes that do not scale with the world: allocator arenas, the
/// test harness and the embedded tables. The peak varies by a few MiB
/// from run to run with allocator behaviour (111 to 133 B a cell seen on
/// this 307,200-cell world), so a fixed allowance keeps the per-cell check
/// honest without flaking.
const FIXED_BYTES: u64 = 16 << 20;
use arda_settle::{run, synthetic, PlaceParams};

/// The process's peak resident set, bytes (Linux `VmHWM`).
#[cfg(target_os = "linux")]
fn peak_rss() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let line = status.lines().find(|l| l.starts_with("VmHWM:")).unwrap();
    let kb: u64 = line
        .split_whitespace()
        .nth(1)
        .and_then(|v| v.parse().ok())
        .unwrap();
    kb * 1024
}

#[cfg(target_os = "linux")]
#[test]
fn the_stage_peak_stays_inside_the_admitted_bytes_per_cell() {
    let before = peak_rss();
    let params = PlaceParams {
        seed: 42,
        density_per_km2: 15,
    };
    let grid = synthetic::landscape(params.seed).unwrap();
    let society = run(&grid, params).unwrap();
    let dir = std::env::temp_dir().join(format!("arda-settle-memory-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    arda_settle::write(&dir, &grid, params.seed, &society).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let cells = u64::try_from(grid.width * grid.height).unwrap();
    let grown = peak_rss().saturating_sub(before);
    let per_cell = grown.saturating_sub(FIXED_BYTES) / cells;
    println!(
        "peak growth {grown} B over {cells} cells: {per_cell} B a cell past {FIXED_BYTES} B fixed"
    );
    assert!(
        per_cell <= BYTES_PER_CELL,
        "{per_cell} B a cell, admitted {BYTES_PER_CELL}"
    );
}
