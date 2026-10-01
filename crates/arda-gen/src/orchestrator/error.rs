//! The batch failure type and its mapping from publication failures.

use super::*;

/// A batch failure (`mockup/01` States).
#[derive(Debug, Error)]
pub enum GenError {
    /// The opt-in canonical source could not be generated.
    #[error(transparent)]
    FineSource(#[from] fine_source::FineSourceError),
    /// An opt-in canonical fine terrain failed validation or sampling.
    #[error(transparent)]
    FineInput(#[from] FineInputError),
    /// Canonical source-stage valley carving failed.
    #[error(transparent)]
    FineValleys(#[from] fine_valleys::ValleyError),
    /// Recipe-5 stream-power formation failed or was refused.
    #[error(transparent)]
    Formation(#[from] crate::formation::FormationError),
    /// The output directory already holds something.
    #[error("output directory {dir} is not empty; refusing to overwrite a world")]
    OutputNotEmpty {
        /// The directory that was targeted.
        dir: String,
    },
    /// A layer could not be written.
    #[error("failed writing {path}: {source}")]
    Write {
        /// The file being written.
        path: String,
        /// Underlying cause.
        #[source]
        source: std::io::Error,
    },
    /// A continent validation gate failed (`logic/01` step 9).
    #[error("continent validation failed: {check}")]
    Validation {
        /// The gate that rejected the continent.
        check: String,
    },
    /// A layer could not be encoded.
    #[error("failed encoding {path}: {source}")]
    Encode {
        /// The layer being encoded.
        path: String,
        /// Underlying cause.
        #[source]
        source: arda_core::FormatError,
    },
    /// Public resource admission failed before creating output files.
    #[error(transparent)]
    Admission(#[from] generation_limits::AdmissionError),
    /// A completed solver changed the domain admitted before output creation.
    #[error("completed hydrology domain differs from the admitted domain")]
    AdmittedDomain,
    /// A final output would exceed its separately admitted write/read envelope.
    #[error("final output {resource} requires {required}, admitted {limit}")]
    ResourceEnvelope {
        /// Requested bytes or counted file operations.
        resource: &'static str,
        /// Total attempted requirement, including this operation.
        required: u128,
        /// Admitted total for the complete final output.
        limit: u128,
    },
    /// Canonical terrain preparation failed.
    #[error(transparent)]
    Prepare(#[from] types::HydrologyError),
    /// Prepared private storage failed.
    #[error(transparent)]
    Prepared(#[from] prepared_files::PreparedError),
    /// The shared physical annual solve failed.
    #[error(transparent)]
    Shared(#[from] shared_solve::SharedError),
    /// Final saved feature indexing failed.
    #[error("final hydrology indexing failed: {0}")]
    Index(#[source] final_index::IndexError<routing_disk::DiskError, flow_disk::FlowDiskError>),
    /// Final immutable area composition failed.
    #[error("shared area composition failed: {0}")]
    Area(#[source] area_output::AreaError<routing_disk::DiskError, flow_disk::FlowDiskError>),
    /// A complete area object's canonical encoder rejected its input.
    #[error("area object encoding failed: {0}")]
    Objects(#[from] arda_core::formats::area_objects_v4::ObjectsFormatError),
    /// A global identity/annual layer failed before publication.
    #[error(transparent)]
    Global(#[from] global_output::GlobalOutputError),
    /// Internal generated output names violated the publication contract.
    #[error("invalid generated world output path")]
    InvalidOutputPath,
    /// The manifest could not be stamped.
    #[error("failed stamping the manifest: {0}")]
    Manifest(#[from] arda_core::LoadError),
}

pub(super) fn publication_error(error: PublicationError) -> GenError {
    match error {
        PublicationError::Occupied(path) => GenError::OutputNotEmpty {
            dir: path.display().to_string(),
        },
        PublicationError::Io { path, source } => GenError::Write {
            path: path.display().to_string(),
            source,
        },
        PublicationError::Manifest(source) => GenError::Manifest(source),
        PublicationError::InvalidLayer => GenError::InvalidOutputPath,
    }
}
