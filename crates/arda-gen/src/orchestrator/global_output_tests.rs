//! Continue the same actual private solve through global tables and manifest publication.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::hydrology::{
    area_output::{self, AreaLimits},
    final_index::FinalIndex,
};
use crate::orchestrator::{
    global_output::{self, GlobalOutputError},
    prepared_files::PreparedReader,
    publication::{PublicationError, WorldOutput},
    shared_solve::SharedArtifacts,
};
use arda_core::formats::hydrology::{BasinNodeRow, FixedRecord, TableReader};
use arda_core::hydrology::{
    AnnualCatchment, BasinId, GlobalLake, GlobalReach, HydrologyMetadata, SharedCrossing,
};
use arda_core::{
    read_manifest, AreaCoord, GenerateConfig, Manifest, ValidationStats, FORMAT_VERSION,
    MANIFEST_NAME,
};
use std::{fs, io::Cursor, path::Path};
fn rows<T: FixedRecord>(root: &Path, name: &str) -> Vec<T> {
    let bytes = fs::read(root.join("hydrology").join(name)).unwrap();
    let mut reader = TableReader::<_, T>::open(
        Cursor::new(bytes.clone()),
        u64::try_from(bytes.len()).unwrap(),
        1_000_000,
    )
    .unwrap();
    let count = reader.count();
    let mut result = Vec::new();
    while let Some(row) = reader.next_record().unwrap() {
        result.push(row);
    }
    assert_eq!(u64::try_from(result.len()).unwrap(), count);
    result
}
pub(super) fn check(
    output: WorldOutput,
    root: &Path,
    mut shared: SharedArtifacts,
    mut prepared: PreparedReader,
    mut index: FinalIndex,
    config: GenerateConfig,
) {
    let original_width = index.domain.width_cells;
    index.domain.width_cells += 1;
    assert!(matches!(
        global_output::write(&output, &mut shared, &index),
        Err(GlobalOutputError::Invalid("index/physical domain mismatch"))
    ));
    assert!(!root.join("hydrology").exists());
    index.domain.width_cells = original_width;
    let metadata = global_output::write(&output, &mut shared, &index).unwrap();
    assert_eq!(metadata.budget, shared.balance);
    assert_eq!(metadata.domain, index.domain);
    assert_eq!(metadata.basin_count, 1);
    assert_eq!(metadata.lake_count, 1);
    assert!(!root.join(MANIFEST_NAME).exists());
    assert!(read_manifest(root).is_err());
    let expected_nodes: Vec<_> = (0..metadata.basin_count)
        .map(|i| shared.hierarchy.output_node(i).unwrap())
        .collect();
    assert_eq!(rows::<BasinNodeRow>(root, "basins.bin"), expected_nodes);
    assert!(rows::<BasinId>(root, "children.bin").is_empty());
    assert_eq!(rows::<GlobalLake>(root, "lakes.bin"), shared.lakes);
    assert_eq!(rows::<GlobalReach>(root, "reaches.bin"), index.reaches);
    assert_eq!(
        rows::<SharedCrossing>(root, "crossings.bin"),
        index.crossings
    );
    assert_eq!(
        rows::<AnnualCatchment>(root, "catchments.bin"),
        index.catchments
    );
    assert_eq!(
        rows::<HydrologyMetadata>(root, "metadata.bin"),
        vec![metadata.clone()]
    );
    // Stream the same authority twice into distinct fresh outputs; table bytes must match.
    let mirror_path = output.scratch().join("canonical-repeat");
    let mirror = WorldOutput::begin(&mirror_path).unwrap();
    assert_eq!(
        global_output::write(&mirror, &mut shared, &index).unwrap(),
        metadata
    );
    for name in [
        "basins.bin",
        "children.bin",
        "lakes.bin",
        "reaches.bin",
        "crossings.bin",
        "catchments.bin",
        "metadata.bin",
    ] {
        assert_eq!(
            fs::read(root.join("hydrology").join(name)).unwrap(),
            fs::read(mirror_path.join("hydrology").join(name)).unwrap()
        );
    }
    assert!(matches!(
        global_output::write(&output, &mut shared, &index),
        Err(GlobalOutputError::Publication(PublicationError::Io { .. }))
    ));
    for invalid in [
        "world.json",
        "../escape",
        ".arda-hydrology-scratch/overwrite",
        "",
    ] {
        assert!(matches!(
            output.write_layer(Path::new(invalid), b"bad"),
            Err(PublicationError::InvalidLayer)
        ));
    }
    let tile = prepared.tile(AreaCoord::new(0, 0)).unwrap();
    let (cells, objects) = area_output::compose(
        tile,
        &mut shared.routing,
        &mut shared.flow,
        &index,
        &shared.lakes,
        AreaLimits {
            ram_bytes: 1 << 28,
            operations: 100_000_000,
            context_records: 10000,
            channels: 10000,
        },
    )
    .unwrap();
    let cell_bytes = arda_core::encode_cells(&cells);
    let object_bytes = arda_core::encode_objects(&objects).unwrap();
    output
        .write_layer(Path::new("areas/0_0/cells.bin"), &cell_bytes)
        .unwrap();
    output
        .write_layer(Path::new("areas/0_0/objects.bin"), &object_bytes)
        .unwrap();
    assert!(!root.join(MANIFEST_NAME).exists());
    let manifest = Manifest {
        format_version: FORMAT_VERSION,
        arda_version: "planning-fixture".to_owned(),
        seed: 42,
        config,
        areas_wide: 1,
        areas_high: 1,
        stats: ValidationStats {
            land_fraction_permille: 0,
            area_count: 1,
            settlement_count: 0,
            named_river_count: 0,
            river_count: u32::try_from(objects.rivers.len()).unwrap(),
        },
        fine_terrain: None,
    };
    drop(mirror);
    drop(prepared);
    drop(shared);
    drop(index);
    output.commit(&manifest).unwrap();
    assert_eq!(read_manifest(root).unwrap(), manifest);
    assert_eq!(
        fs::read(root.join(MANIFEST_NAME)).unwrap(),
        serde_json::to_vec_pretty(&manifest).unwrap()
    );
    assert!(!WorldOutput::scratch_path(root).exists());
    assert!(!root.join(".arda-generating").exists());
    assert!(!root.join(".arda-manifest-pending").exists());
    assert_eq!(
        rows::<HydrologyMetadata>(root, "metadata.bin"),
        vec![metadata]
    );
    assert_eq!(
        arda_core::decode_cells(
            "saved cells",
            &fs::read(root.join("areas/0_0/cells.bin")).unwrap()
        )
        .unwrap(),
        cells
    );
    assert_eq!(
        arda_core::decode_objects(
            "saved objects",
            &fs::read(root.join("areas/0_0/objects.bin")).unwrap()
        )
        .unwrap(),
        objects
    );
    assert!(matches!(
        WorldOutput::begin(root),
        Err(PublicationError::Occupied(_))
    ));
}
