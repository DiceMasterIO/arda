//! Stream completed global identities and annual authority before manifest publication.
#![deny(missing_docs)]

use super::child_links::CHILD_FILE;
use super::publication::{PublicationError, WorldOutput};
use super::shared_solve::SharedArtifacts;
use crate::hydrology::final_index::FinalIndex;
use crate::hydrology::hierarchy_disk;
use crate::hydrology::routing::{CellIndex, RoutingStore};
use arda_core::formats::hydrology::{FixedRecord, HydrologyFormatError, TableWriter};
use arda_core::hydrology::{BasinId, HydrologyMetadata};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::path::{Path, PathBuf};

const BUFFER: usize = 65536;

/// An incomplete or invalid global layer cannot acquire a completion manifest.
#[derive(Debug, thiserror::Error)]
pub enum GlobalOutputError {
    /// Fresh final output creation failed.
    #[error(transparent)]
    Publication(#[from] PublicationError),
    /// Fixed-table encoding, count/order or write failed.
    #[error(transparent)]
    Format(#[from] HydrologyFormatError),
    /// A finalized original hierarchy read failed.
    #[error(transparent)]
    Hierarchy(#[from] hierarchy_disk::DiskError),
    /// Original private child-table I/O failure.
    #[error("global child output read failed on {path}: {source}")]
    Io {
        /// Exact private file involved.
        path: PathBuf,
        /// Original operating-system failure.
        #[source]
        source: std::io::Error,
    },
    /// Completed hierarchy and final index authorities disagree.
    #[error("invalid global output: {0}")]
    Invalid(&'static str),
}

fn count(n: usize) -> Result<u64, GlobalOutputError> {
    u64::try_from(n).map_err(|_| GlobalOutputError::Invalid("record count overflow"))
}
fn table<T: FixedRecord>(
    output: &WorldOutput,
    name: &str,
    rows: &[T],
) -> Result<(), GlobalOutputError> {
    let file = output.create_layer(&Path::new("hydrology").join(name))?;
    let mut writer =
        TableWriter::create(BufWriter::with_capacity(BUFFER, file), count(rows.len())?)?;
    for row in rows {
        writer.write_record(row)?;
    }
    writer.finish()?;
    Ok(())
}

/// Persist existing solved authority with bounded file buffers and no full-table encode.
/// All source vectors are borrowed from the separately admitted completed shared stage.
/// The extra private child read is exactly eight bytes per declared child, plus metadata;
/// hierarchy row reads use its still-active explicit backend I/O allowance.
pub fn write(
    output: &WorldOutput,
    shared: &mut SharedArtifacts,
    index: &FinalIndex,
) -> Result<HydrologyMetadata, GlobalOutputError> {
    let extent = shared.routing.extent();
    let last = CellIndex::new(extent.cells() - 1, extent)
        .ok_or(GlobalOutputError::Invalid("empty routing domain"))?;
    let (mx, my) = extent.coordinates(last);
    if index.domain.width_cells != mx + 1 || index.domain.height_cells != my + 1 {
        return Err(GlobalOutputError::Invalid("index/physical domain mismatch"));
    }
    let (basin_count, _) = shared.hierarchy.output_counts()?;
    if count(shared.nodes.len())? != basin_count {
        return Err(GlobalOutputError::Invalid("retained node count"));
    }
    let file = output.create_layer(Path::new("hydrology/basins.bin"))?;
    let mut writer = TableWriter::create(BufWriter::with_capacity(BUFFER, file), basin_count)?;
    let mut child_count = 0_u64;
    for i in 0..basin_count {
        let row = shared.hierarchy.output_node(i)?;
        let at = usize::try_from(i).map_err(|_| GlobalOutputError::Invalid("node ordinal"))?;
        let node = &shared.nodes[at];
        if row.id != node.id
            || row.parent != node.parent
            || row.floor != node.floor
            || row.spill != node.spill
            || row.children.count != count(node.children.len())?
            || (row.children.count > 0 && row.children.offset != child_count)
            || (row.children.count == 0 && row.children.offset != 0)
        {
            return Err(GlobalOutputError::Invalid("retained node or child span"));
        }
        child_count = child_count
            .checked_add(row.children.count)
            .ok_or(GlobalOutputError::Invalid("child count overflow"))?;
        writer.write_record(&row)?;
    }
    writer.finish()?;
    let source = shared
        .hierarchy
        .output_paths()?
        .0
        .parent()
        .ok_or(GlobalOutputError::Invalid("hierarchy directory"))?
        .join(CHILD_FILE);
    let fail = |source_error| GlobalOutputError::Io {
        path: source.clone(),
        source: source_error,
    };
    let input = File::open(&source).map_err(fail)?;
    let expected = child_count
        .checked_mul(8)
        .ok_or(GlobalOutputError::Invalid("child byte count"))?;
    if input.metadata().map_err(fail)?.len() != expected {
        return Err(GlobalOutputError::Invalid("child file length"));
    }
    let mut input = BufReader::with_capacity(BUFFER, input);
    let file = output.create_layer(Path::new("hydrology/children.bin"))?;
    let mut writer = TableWriter::create(BufWriter::with_capacity(BUFFER, file), child_count)?;
    let mut copied = 0_u64;
    for parent in &shared.nodes {
        let mut previous = None;
        for _ in parent.children.clone() {
            let mut bytes = [0; 8];
            input.read_exact(&mut bytes).map_err(fail)?;
            let child = BasinId(u64::from_le_bytes(bytes));
            let at = shared
                .nodes
                .binary_search_by_key(&child, |n| n.id)
                .map_err(|_| GlobalOutputError::Invalid("unknown child"))?;
            if shared.nodes[at].parent != Some(parent.id) || previous.is_some_and(|p| p >= child) {
                return Err(GlobalOutputError::Invalid(
                    "child parent or canonical order",
                ));
            }
            previous = Some(child);
            writer.write_record(&child)?;
            copied = copied
                .checked_add(1)
                .ok_or(GlobalOutputError::Invalid("copied count overflow"))?;
        }
    }
    if copied != child_count {
        return Err(GlobalOutputError::Invalid("copied child count"));
    }
    writer.finish()?;
    table(output, "lakes.bin", &shared.lakes)?;
    table(output, "reaches.bin", &index.reaches)?;
    table(output, "crossings.bin", &index.crossings)?;
    table(output, "catchments.bin", &index.catchments)?;
    let metadata = HydrologyMetadata {
        model_revision: 2,
        domain: index.domain,
        basin_count,
        lake_count: count(shared.lakes.len())?,
        reach_count: count(index.reaches.len())?,
        crossing_count: count(index.crossings.len())?,
        catchment_count: count(index.catchments.len())?,
        budget: shared.balance,
    };
    table(output, "metadata.bin", std::slice::from_ref(&metadata))?;
    Ok(metadata)
}
