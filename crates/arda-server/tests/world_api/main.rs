//! In-process tests of the `/v1` service and the cell contract against a real
//! seed-42 MICRO fine world (no mocks of arda).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

mod cells;
mod contract;
mod endpoints;
mod npc;
mod relief;
mod support;
mod tactical;
mod tactical_images;
