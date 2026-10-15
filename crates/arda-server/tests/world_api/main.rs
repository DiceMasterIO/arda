//! In-process tests of the `/v1` service and the cell contract against a real
//! seed-42 MICRO fine world (no mocks of arda).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

mod admission;
mod bake;
mod cells;
mod contract;
mod dungeon;
mod endpoints;
#[path = "../../../../tests/support/fixture_dir.rs"]
mod fixture_dir;
mod gates;
mod gazetteer;
mod inventory;
mod library_asset;
mod looks;
mod mapping;
mod npc;
mod npc_generate;
mod one_composition;
mod overview_overlay;
mod people;
mod prefetch;
mod realms;
mod relief;
mod schemas;
mod seams;
mod serving;
mod society_cells;
mod support;
mod tactical;
mod tactical_images;
