//! `--memory-mib` (logic/16 §api-memory) on the MICRO seed-42 society
//! world: the server opens under a 4.5 GiB ceiling with every derived
//! budget fitted, admits a worst case under it, serves a cell at 64 px per
//! square and refuses a ceiling below its floors, naming the least.

use crate::support::{config, get_with, society_dir};
use arda_server::ceiling::{self, ONE_CELL_64_PX};
use arda_server::{AppState, ServerConfig, ServerError};
use axum::http::StatusCode;
use std::sync::Arc;

const MIB: u64 = 1 << 20;

/// DiceMaster's profile: prefetch on, baking off, under `mib`.
fn dicemaster(mib: u64) -> ServerConfig {
    let mut c = config();
    c.world = society_dir().to_path_buf();
    c.bake_workers = 0;
    c.memory_ceiling = Some(mib * MIB);
    c
}

#[tokio::test]
async fn a_ceiling_admits_its_worst_case_and_serves_a_64_px_cell() {
    let state = AppState::open(&dicemaster(4_608)).unwrap();
    let need = state.admitted + state.resident.total();
    eprintln!(
        "admitted {} MiB + resident {} MiB = {} MiB of {} MiB",
        state.admitted / MIB,
        state.resident.total() / MIB,
        need / MIB,
        state.ceiling / MIB
    );
    assert_eq!(state.ceiling, 4_608 * MIB);
    assert!(need <= state.ceiling, "{need} > {}", state.ceiling);
    assert!(state.tactical.limits().max_render_px >= ONE_CELL_64_PX);
    assert!(state.serve_limits.max_connections >= 8);
    let state = Arc::new(state);
    let cell = get_with(
        Arc::clone(&state),
        "/v1/tactical/cell/479/1245?ppsq=64",
        None,
    )
    .await;
    assert_eq!(cell.status, StatusCode::OK);
    let png = get_with(state, "/v1/tactical/cell/479/1245.png?ppsq=128", None).await;
    assert_eq!(
        png.status,
        StatusCode::PAYLOAD_TOO_LARGE,
        "a 128-px cell needs more than the fitted render size"
    );
}

#[test]
fn a_ceiling_below_the_floors_refuses_to_start_naming_the_least() {
    let err = AppState::open(&dicemaster(2_048)).unwrap_err();
    let ServerError::ResourceLimit(text) = &err else {
        panic!("{err}");
    };
    assert!(text.contains("--memory-mib 2048 is below the"), "{text}");
    let least: u64 = text
        .split("below the ")
        .nth(1)
        .and_then(|t| t.split(' ').next())
        .and_then(|n| n.parse().ok())
        .unwrap();
    assert!(AppState::open(&dicemaster(least)).is_ok(), "{least} MiB");
    let probe = dicemaster(least);
    let state = AppState::open(&probe).unwrap();
    assert_eq!(
        ceiling::smallest(&probe, &state.resident).div_ceil(MIB),
        least
    );
}
