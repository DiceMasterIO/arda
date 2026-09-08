//! Shared physical-terrain preparation contracts and generation resource limits.
use super::budget::BudgetError;
use arda_core::{AreaCoord, BasinId, HeightMm, RainfallMm};
use thiserror::Error;

/// Actual domain sides which can seed marine connectivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MarineBoundary {
    /// The tile touches the modeled northern boundary.
    pub north: bool,
    /// The tile touches the modeled eastern boundary.
    pub east: bool,
    /// The tile touches the modeled southern boundary.
    pub south: bool,
    /// The tile touches the modeled western boundary.
    pub west: bool,
}
/// Valid rows and columns of one prepared tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedExtent {
    /// Number of valid tile columns, from one through 512.
    pub width: u16,
    /// Number of valid tile rows, from one through 512.
    pub height: u16,
    /// Which valid tile sides are actual domain boundaries.
    pub boundary: MarineBoundary,
}
/// The pure preparation function retains full 512² canonical arrays; the private
/// writer crops valid rows only, preserving existing terrain on partial fringes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTerrain {
    /// Canonical tile coordinate within the modeled domain.
    pub area: AreaCoord,
    /// Cropped physical extent and actual domain boundaries.
    pub valid: PreparedExtent,
    /// Full 512 by 512 physical height array, without routing epsilon.
    pub heights: Vec<HeightMm>,
    /// Full 512 by 512 annual rainfall sampled before water classification.
    pub annual_rain: Vec<RainfallMm>,
    /// Unclamped annual reference with the coarse altitude lapse removed.
    /// Finalize using physical height and solved fine marine membership;
    /// this intermediate is never persisted as a final cell temperature.
    pub temperature_base_centi: Vec<i32>,
}

/// A checked shared-stage failure which prevents final publication.
#[derive(Debug, Error)]
pub enum HydrologyError {
    /// Pure physical terrain preparation rejected its inputs.
    #[error("terrain preparation failed: {0}")]
    TerrainPreparation(&'static str),
    /// A checked hydrology calculation overflowed.
    #[error("hydrology arithmetic overflow: {0}")]
    Overflow(&'static str),
    /// The prepared geometry or flow witness is inconsistent.
    #[error("inconsistent shared topology: {0}")]
    InconsistentTopology(&'static str),
    /// The checked water-budget kernel rejected an operation.
    #[error(transparent)]
    Budget(#[from] BudgetError),
    /// The annual solver exhausted its counted event allowance.
    #[error("basin {basin:?} exceeded event allowance {allowed}")]
    EventBudgetExceeded {
        /// Canonical basin associated with the exhausted event work.
        basin: BasinId,
        /// Maximum allowed counted events.
        allowed: u64,
    },
    /// A checked memory, scratch or I/O reservation exceeds its allowance.
    #[error("{stage} requires {requested} bytes or operations, limit {limit}")]
    ResourceLimit {
        /// Shared stage making the reservation.
        stage: &'static str,
        /// Required bytes or operations.
        requested: u128,
        /// Maximum admitted bytes or operations.
        limit: u128,
    },
}

/// Every reservation is checked before allocation/writing. Timing is a release
/// measurement only; it never changes generated water or selects a supported state.
#[derive(Debug, Clone, Copy)]
pub struct HydrologyLimits {
    /// Maximum actual closed physical terminal count; larger inputs fail without truncation.
    pub max_closed_leaves: u64,
    /// Maximum grouped annual support bands, bounded by physical cells.
    pub max_bands: u64,
    /// Maximum saved reach/crossing/catchment records in the final index.
    pub max_feature_records: u64,
    /// Maximum saved area-to-reach references including required endpoint closure and halo.
    pub max_area_references: u64,
    /// Maximum actual outward lake-boundary edges used by metrics, including small flows.
    pub max_lake_boundary_edges: u64,
    /// Maximum counted filesystem operations across the complete declared transaction.
    pub global_io_operations: u64,
    /// Maximum admitted owned payload bytes for the complete sequential generation.
    pub ram_bytes: u64,
    /// Maximum simultaneous spatial scratch bytes.
    pub spatial_scratch_bytes: u64,
    /// Maximum simultaneous bytes across all live scratch stages.
    pub combined_scratch_bytes: u64,
    /// Maximum summed declared logical work across admitted generation stages.
    pub global_event_operations: u64,
    /// Maximum summed declared prepared, solve and final-file payload I/O bytes.
    pub global_io_bytes: u128,
}

impl Default for HydrologyLimits {
    fn default() -> Self {
        Self {
            max_closed_leaves: 100_000,
            max_bands: 6_000_000,
            max_feature_records: 4_000_000,
            max_area_references: 16_000_000,
            max_lake_boundary_edges: 4_000_000,
            ram_bytes: 16 * 1024 * 1024 * 1024,
            spatial_scratch_bytes: 256 * 1024 * 1024 * 1024,
            combined_scratch_bytes: 256 * 1024 * 1024 * 1024,
            global_event_operations: 1 << 48,
            global_io_bytes: 1 << 64,
            global_io_operations: 1 << 54,
        }
    }
}
