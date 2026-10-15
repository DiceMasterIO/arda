//! Server release gates (1.0 task 09; logic/16 Invariant 7): wall-clock
//! budgets measured in-process through the real router on the shared
//! settled society fixture. All are `#[ignore]`; run them with
//! `tools/release-gates.sh` on the release machine. Each prints one
//! `GATE <name> <measured_ms> <budget_ms>` line before it asserts, plus the
//! `X-Arda-Cache` state of the measured request. Best of three, fresh
//! servers where the request is cold, so a loaded machine does not flake.
//! Timing never reaches a response body.

use crate::support::{config, get_with, society_dir, Reply};
use arda_server::AppState;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A river cell of the fixture world (a built block, neighbours refined).
const CELL: (u64, u64) = (518, 822);
const REPS: usize = 3;

const TILE_MS: f64 = 20.0;
const COLD_CELL_MS: f64 = 500.0;
const COLD_WINDOW_MS: f64 = 1000.0;
const OPEN_MS: f64 = 2000.0;
const LOAD_MS: f64 = 100.0;
const AREA_JSON_MS: f64 = 500.0;
const AREA_BIN_MS: f64 = 250.0;
const OVERVIEW_MS: f64 = 20.0;

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1e3
}

fn fresh() -> Arc<AppState> {
    let mut c = config();
    c.world = society_dir().to_path_buf();
    Arc::new(AppState::open(&c).unwrap())
}

fn cache(r: &Reply) -> &str {
    r.headers
        .get("x-arda-cache")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
}

async fn timed(state: &Arc<AppState>, uri: &str) -> (f64, Reply) {
    let start = Instant::now();
    let r = get_with(Arc::clone(state), uri, None).await;
    let took = ms(start.elapsed());
    assert_eq!(r.status, 200, "{uri}: {}", String::from_utf8_lossy(&r.body));
    (took, r)
}

/// Prints the uniform line, then fails when over budget.
fn report(name: &str, measured: f64, budget: f64, cache_state: &str) {
    println!("GATE {name} {measured:.3} {budget:.0}");
    println!("  {name}: x-arda-cache {cache_state}, best of {REPS}");
    assert!(measured < budget, "{name} took {measured:.3} ms");
}

/// Resident-set high-water mark in KiB (Linux), for the receipt.
fn peak_rss_kib() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("VmHWM:"))
                .and_then(|v| v.split_whitespace().next()?.parse().ok())
        })
        .unwrap_or(0)
}

#[tokio::test]
#[ignore = "wall-clock timing; release gate"]
async fn a_cached_cell_webp_tile_returns_in_under_20_ms() {
    let state = fresh();
    let (gx, gy) = CELL;
    let uri = format!("/v1/tactical/cell/{gx}/{gy}/tiles/2/1/1.webp?ppsq=64");
    let (_, first) = timed(&state, &uri).await;
    assert_eq!(&first.body[8..12], b"WEBP");
    let mut best = f64::MAX;
    let mut state_seen = String::new();
    for _ in 0..REPS {
        let (t, r) = timed(&state, &uri).await;
        assert_eq!(r.body, first.body);
        best = best.min(t);
        state_seen = cache(&r).to_owned();
    }
    assert_eq!(state_seen, "hit");
    report("server_cached_cell_tile", best, TILE_MS, &state_seen);
}

#[tokio::test]
#[ignore = "wall-clock timing; release gate"]
async fn a_cold_cell_layout_and_scene_return_in_under_500_ms() {
    let (gx, gy) = CELL;
    let mut best = f64::MAX;
    let mut state_seen = String::new();
    for _ in 0..REPS {
        let state = fresh();
        let (a, layout) = timed(&state, &format!("/v1/tactical/cell/{gx}/{gy}")).await;
        let (b, scene) = timed(&state, &format!("/v1/tactical/cell/{gx}/{gy}/scene")).await;
        assert_eq!(layout.json()["layout"]["width"], 64);
        assert!(scene.json().is_object());
        best = best.min(a + b);
        state_seen = format!("{}+{}", cache(&layout), cache(&scene));
    }
    report(
        "server_cold_cell_json_scene",
        best,
        COLD_CELL_MS,
        &state_seen,
    );
}

#[tokio::test]
#[ignore = "wall-clock timing; release gate"]
async fn a_cold_quarter_block_window_at_128_ppsq_returns_in_under_1_s() {
    let (gx, gy) = CELL;
    let (x, y) = (gx * 64 + 48, gy * 64 + 48);
    let uri = format!("/v1/tactical/window.png?gsx={x}&gsy={y}&w=32&h=32&ppsq=128");
    let mut best = f64::MAX;
    let mut state_seen = String::new();
    for _ in 0..REPS {
        let state = fresh();
        let (t, r) = timed(&state, &uri).await;
        assert_eq!(&r.body[..8], b"\x89PNG\r\n\x1a\n");
        best = best.min(t);
        state_seen = cache(&r).to_owned();
    }
    report("server_cold_window_128", best, COLD_WINDOW_MS, &state_seen);
}

#[test]
#[ignore = "wall-clock timing; release gate"]
fn opening_the_society_server_takes_under_budget() {
    let mut c = config();
    c.world = society_dir().to_path_buf();
    let mut best = f64::MAX;
    for _ in 0..REPS {
        let start = Instant::now();
        let state = AppState::open(&c).unwrap();
        best = best.min(ms(start.elapsed()));
        drop(state);
    }
    // Process high-water mark: includes the gates that ran before in this binary.
    println!("MEM server_gates_peak_rss_kib {}", peak_rss_kib());
    report("server_open", best, OPEN_MS, "-");
}

#[test]
#[ignore = "wall-clock timing; release gate"]
fn loading_the_world_takes_under_100_ms() {
    let dir = society_dir();
    let mut best = f64::MAX;
    for _ in 0..REPS {
        let start = Instant::now();
        let world = arda::World::load(dir).unwrap();
        best = best.min(ms(start.elapsed()));
        drop(world);
    }
    report("world_load", best, LOAD_MS, "-");
}

#[tokio::test]
#[ignore = "wall-clock timing; release gate"]
async fn an_area_cells_body_is_served_within_budget_as_json_and_bin() {
    for (name, uri, budget) in [
        ("server_area_cells_json", "/v1/area/1/2/cells", AREA_JSON_MS),
        (
            "server_area_cells_bin",
            "/v1/area/1/2/cells?format=bin",
            AREA_BIN_MS,
        ),
    ] {
        let mut best = f64::MAX;
        let mut state_seen = String::new();
        for _ in 0..REPS {
            let state = fresh();
            let (t, r) = timed(&state, uri).await;
            assert!(r.body.len() > 1 << 20, "{name}: {} bytes", r.body.len());
            best = best.min(t);
            state_seen = cache(&r).to_owned();
        }
        report(name, best, budget, &state_seen);
    }
}

#[tokio::test]
#[ignore = "wall-clock timing; release gate"]
async fn a_warm_overview_tile_returns_in_under_20_ms() {
    let state = fresh();
    let uri = "/v1/tiles/overview/1/0/1.png";
    let (_, first) = timed(&state, uri).await;
    let mut best = f64::MAX;
    let mut state_seen = String::new();
    for _ in 0..REPS {
        let (t, r) = timed(&state, uri).await;
        assert_eq!(r.body, first.body);
        best = best.min(t);
        state_seen = cache(&r).to_owned();
    }
    report("server_warm_overview_tile", best, OVERVIEW_MS, &state_seen);
}
