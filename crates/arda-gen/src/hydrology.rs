//! Shared physical terrain routing and representative annual water state.
pub mod annual;
pub mod annual_aggregation;
pub mod annual_records;
pub mod annual_topology;
pub mod annual_transfers;
pub mod area_output;
pub mod budget;
pub mod extraction;
pub mod final_index;
pub mod fine_flow;
pub mod flow_adjacency;
pub mod flow_metrics;
pub mod forcing;
pub mod forcing_sample;
pub mod forcing_tables;
pub mod handoff;
pub mod hierarchy;
pub mod hierarchy_disk;
pub mod local_rivers;
pub mod mst;
pub mod ocean;
pub mod prepared_codec;
pub mod prepared_domain;
pub mod private_rows;
pub mod routing;
pub mod saddles;
pub mod types;
pub mod witness_binding;
pub use types::{HydrologyError, PreparedExtent, PreparedTerrain};
#[cfg(test)]
mod annual_integration_tests;
#[cfg(test)]
mod annual_records_tests;
#[cfg(test)]
mod annual_topology_tests;
#[cfg(test)]
mod annual_transfers_tests;
#[cfg(test)]
mod extraction_tests;
#[cfg(test)]
mod final_index_tests;
#[cfg(test)]
mod fine_flow_tests;
#[cfg(test)]
mod flow_metrics_tests;
#[cfg(test)]
mod hierarchy_tests;
#[cfg(test)]
mod memory_flow;
#[cfg(test)]
mod witness_binding_tests;
