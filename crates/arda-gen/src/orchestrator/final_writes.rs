//! Final layer writes under one admitted receipt: global hydrology tables,
//! per-area files and skeleton blocks.

use super::*;

// This receipt only covers final layer writes and the repeated private child read.
// Mutable backend I/O remains charged by the already-admitted backend owners.
pub(super) struct FinalWrites {
    pub(super) bytes: u128,
    pub(super) operations: u128,
    pub(super) byte_limit: u128,
    pub(super) operation_limit: u128,
}
impl FinalWrites {
    pub(super) fn charge(&mut self, bytes: u128, operations: u128) -> Result<(), GenError> {
        let next_bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or(GenError::ResourceEnvelope {
                resource: "requested bytes",
                required: u128::MAX,
                limit: self.byte_limit,
            })?;
        let next_operations =
            self.operations
                .checked_add(operations)
                .ok_or(GenError::ResourceEnvelope {
                    resource: "file operations",
                    required: u128::MAX,
                    limit: self.operation_limit,
                })?;
        for (resource, required, limit) in [
            ("requested bytes", next_bytes, self.byte_limit),
            ("file operations", next_operations, self.operation_limit),
        ] {
            if required > limit {
                return Err(GenError::ResourceEnvelope {
                    resource,
                    required,
                    limit,
                });
            }
        }
        self.bytes = next_bytes;
        self.operations = next_operations;
        Ok(())
    }
    pub(super) fn global(
        &mut self,
        shared: &shared_solve::SharedArtifacts,
        index: &final_index::FinalIndex,
    ) -> Result<(), GenError> {
        let children = shared.nodes.iter().try_fold(0_u128, |sum, node| {
            sum.checked_add(node.children.len() as u128)
                .ok_or(GenError::ResourceEnvelope {
                    resource: "requested bytes",
                    required: u128::MAX,
                    limit: self.byte_limit,
                })
        })?;
        let tables = [
            (shared.nodes.len() as u128, BasinNodeRow::WIDTH),
            (children, BasinId::WIDTH),
            (shared.lakes.len() as u128, GlobalLake::WIDTH),
            (index.reaches.len() as u128, GlobalReach::WIDTH),
            (index.crossings.len() as u128, SharedCrossing::WIDTH),
            (index.catchments.len() as u128, AnnualCatchment::WIDTH),
            (1, HydrologyMetadata::WIDTH),
        ];
        // One requested payload operation per record, one extra read per child,
        // and64 fixed header/create/flush/open/metadata operations. Buffered calls
        // may coalesce these; no cache-hit or syscall-count assumption is made.
        let mut bytes =
            children
                .checked_mul(BasinId::WIDTH as u128)
                .ok_or(GenError::ResourceEnvelope {
                    resource: "requested bytes",
                    required: u128::MAX,
                    limit: self.byte_limit,
                })?;
        let mut operations = children.checked_add(64).ok_or(GenError::ResourceEnvelope {
            resource: "file operations",
            required: u128::MAX,
            limit: self.operation_limit,
        })?;
        for (count, width) in tables {
            bytes = count
                .checked_mul(width as u128)
                .and_then(|v| v.checked_add(u128::from(TABLE_HEADER_BYTES)))
                .and_then(|v| v.checked_add(bytes))
                .ok_or(GenError::ResourceEnvelope {
                    resource: "requested bytes",
                    required: u128::MAX,
                    limit: self.byte_limit,
                })?;
            operations = operations
                .checked_add(count)
                .ok_or(GenError::ResourceEnvelope {
                    resource: "file operations",
                    required: u128::MAX,
                    limit: self.operation_limit,
                })?;
        }
        self.charge(bytes, operations)
    }
}
pub(super) fn write_file(
    output: &WorldOutput,
    relative: &Path,
    bytes: &[u8],
    writes: &mut FinalWrites,
) -> Result<(), GenError> {
    // create_dir_all, create_new open, write_all and flush: attempted API calls.
    writes.charge(bytes.len() as u128, 4)?;
    output
        .write_layer(relative, bytes)
        .map_err(publication_error)
}
pub(super) fn create_private(path: &Path) -> Result<(), GenError> {
    std::fs::create_dir(path).map_err(|source| GenError::Write {
        path: path.display().to_string(),
        source,
    })
}

/// Writes already composed cells/objects and the unchanged sampled tactical skeleton.
pub(super) fn write_area(
    seed: u64,
    area: AreaCoord,
    cells: &AreaCells,
    objects: &AreaObjects,
    output: &WorldOutput,
    writes: &mut FinalWrites,
) -> Result<(), GenError> {
    let dir: PathBuf = Path::new("areas").join(area.dir_name());
    write_file(output, &dir.join("cells.bin"), &encode_cells(cells), writes)?;
    write_file(
        output,
        &dir.join("objects.bin"),
        &encode_objects(objects)?,
        writes,
    )?;

    let mut archive = BlockArchive::default();
    let mut y = 0u16;
    while y < AREA_CELLS {
        let mut x = 0u16;
        while x < AREA_CELLS {
            if let Some(at) = CellCoord::new(x, y) {
                if cells.get(at).terrain == TerrainKind::Land {
                    let c = constraints_for(cells, at);
                    archive.insert(at, fill_block(seed, area, at, &c));
                }
            }
            x += SKELETON_BLOCK_STRIDE;
        }
        y += SKELETON_BLOCK_STRIDE;
    }

    let name = format!("{}.tiles.zst", area.dir_name());
    let blocks = encode_blocks(&archive).map_err(|e| GenError::Encode {
        path: format!("blocks/{name}"),
        source: e,
    })?;
    write_file(output, &Path::new("blocks").join(name), &blocks, writes)
}
