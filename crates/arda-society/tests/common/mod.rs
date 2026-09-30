//! Shared fixtures for the integration tests.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use arda_society::input::WorldSettlements;
use arda_society::{simulate_society, synthetic, Society};

/// Seeds every property test runs on.
pub const SEEDS: [u64; 4] = [1, 42, 777, 90210];

/// The synthetic world and its society for `seed`.
pub fn build(seed: u64) -> (WorldSettlements, Society) {
    let world = synthetic::world(seed).expect("synthetic world");
    let society = simulate_society(seed, &world).expect("society");
    (world, society)
}
