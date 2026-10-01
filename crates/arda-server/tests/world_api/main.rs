//! In-process tests of the `/v1` service and the cell contract against a real
//! seed-42 MICRO fine world (no mocks of arda).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

mod cells;
mod contract;
mod endpoints;
#[path = "../../../../tests/support/fixture_dir.rs"]
mod fixture_dir;
mod looks;
mod mapping;
mod npc;
mod people;
mod prefetch;
mod relief;
mod schemas;
mod society_cells;
mod support;
mod tactical;
mod tactical_images;
