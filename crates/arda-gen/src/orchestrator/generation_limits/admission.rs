//! What a successful admission hands the caller (the domain and every stage
//! reservation) and why an admission is refused.

use super::*;

/// Complete concrete stage reservations, derived without touching the filesystem.
#[derive(Debug, Clone, Copy)]
pub struct GenerationAdmission {
    /// Request-derived modeled and exported rectangles.
    pub domain: PreparedDomain,
    /// Fine spatial ordinal domain.
    pub extent: Extent,
    /// One shared prepared writer/reader lifetime, including later area reads.
    pub prepared: prepared_files::Limits,
    /// All private shared solver stages and retained scratch handles.
    pub shared: SharedLimits,
    /// Completed feature and area-reference index.
    pub index: IndexLimits,
    /// Sequential composition of one area at a time.
    pub area: AreaLimits,
    /// Conservative simultaneous payload and complete declared work/I/O totals.
    pub reservations: GenerationReservations,
}
/// Explicit sums; no temporal-reuse credit is used for stage-owned reservations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationReservations {
    /// Sum of stage reservations plus continent/preparation and final encoding buffers.
    pub ram_bytes: u64,
    /// Prepared files and all simultaneously retained solve scratch.
    pub scratch_bytes: u64,
    /// Routing/MST/hierarchy/flow scratch, excluding separately staged prepared files.
    pub spatial_scratch_bytes: u64,
    /// Complete declared prepared, solve and final-file payload I/O ceilings.
    pub io_bytes: u128,
    /// Complete declared filesystem-operation ceilings.
    pub io_operations: u64,
    /// Sum of declared stage logical-work counters, including extraction/composition.
    pub logical_operations: u64,
    /// Shared initial terrain sampling and all bounded physical evolution steps.
    pub terrain_operations: u64,
    /// Continent, complete shared terrain evolution and one prepared-area payload.
    pub continent_preparation_bytes: u64,
    /// One128MiB objects encoding plus cells/compression and fixed writer buffers.
    pub encoding_bytes: u64,
    /// Final file payload allowance, checked by bounded output encoders/callers.
    pub final_io_bytes: u128,
    /// Final file operation allowance, including all configured areas and fixed global tables.
    pub final_io_operations: u64,
}
/// Invalid request, arithmetic/helper failure or explicit whole-generation resource refusal.
#[derive(Debug, thiserror::Error)]
pub enum AdmissionError {
    /// The requested modeled domain is unsupported.
    #[error(transparent)]
    Domain(#[from] DomainError),
    /// Physical evolution rejected its rectangle or allocation arithmetic.
    #[error(transparent)]
    Terrain(#[from] crate::hydrology::HydrologyError),
    /// A known count or reservation cannot be represented.
    #[error("generation admission overflow: {0}")]
    Overflow(&'static str),
    /// A complete reservation exceeds the user's explicit capacity.
    #[error("generation {resource} requires {required}, limit {limit}")]
    Limit {
        /// Resource being admitted.
        resource: &'static str,
        /// Complete required reservation.
        required: u128,
        /// Public caller allowance.
        limit: u128,
    },
    /// Prepared helper rejected its cache/path/count.
    #[error(transparent)]
    Prepared(#[from] prepared_files::PreparedError),
    /// Exact MST private storage helper failed.
    #[error(transparent)]
    Mst(#[from] mst::StageError),
    /// Exact accepted sorting/replay helper failed.
    #[error(transparent)]
    Sort(#[from] mst::SortError),
    /// Exact child-link sorting/index helper failed.
    #[error(transparent)]
    Child(#[from] crate::orchestrator::child_links::ChildError),
    /// Exact hierarchy cache/row helper failed.
    #[error(transparent)]
    Hierarchy(#[from] crate::hydrology::hierarchy_disk::DiskError),
    /// Exact fine-flow cache/row helper failed.
    #[error(transparent)]
    Flow(#[from] flow_disk::FlowDiskError),
    /// Annual model payload arithmetic failed.
    #[error(transparent)]
    Annual(#[from] annual::AnnualError),
    /// Summed concrete solver reservations failed.
    #[error(transparent)]
    Shared(#[from] shared_solve::SharedError),
}
