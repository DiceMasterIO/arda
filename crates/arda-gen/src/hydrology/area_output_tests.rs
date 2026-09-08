//! Continue the artificial real-file shared solve into final area codecs.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::hydrology::{
    area_output::{self, AreaError, AreaLimits},
    final_index::FinalIndex,
    fine_flow::FlowStore,
    prepared_codec::{decode_prepared, encode_prepared},
    routing::CellIndex,
    types::PreparedTerrain,
};
use crate::orchestrator::{prepared_files::PreparedReader, shared_solve::SharedArtifacts};
use arda_core::{AreaCoord, CellCoord, HeightMm, RainfallMm, TerrainKind};

pub(super) fn check(
    prepared: &mut PreparedReader,
    artifacts: &mut SharedArtifacts,
    index: &FinalIndex,
) {
    let area = AreaCoord::new(0, 0);
    let tile = prepared.tile(area).unwrap();
    let limits = AreaLimits {
        ram_bytes: 1 << 28,
        operations: 100_000_000,
        context_records: 10000,
        channels: 10000,
    };
    for bad in [
        AreaLimits {
            ram_bytes: 0,
            ..limits
        },
        AreaLimits {
            operations: 0,
            ..limits
        },
        AreaLimits {
            context_records: 1_048_577,
            ..limits
        },
        AreaLimits {
            channels: 262145,
            ..limits
        },
    ] {
        assert!(area_output::compose(
            tile,
            &mut artifacts.routing,
            &mut artifacts.flow,
            index,
            &artifacts.lakes,
            bad
        )
        .is_err());
    }
    let (cells, objects) = area_output::compose(
        tile,
        &mut artifacts.routing,
        &mut artifacts.flow,
        index,
        &artifacts.lakes,
        limits,
    )
    .unwrap();
    assert_eq!(objects.lakes.len(), 1);
    let local = &objects.lakes[0];
    let global = &artifacts.lakes[0];
    assert_eq!(local.global_id, global.basin);
    assert_eq!(local.surface, global.surface);
    assert_eq!(local.depth_mm, 9);
    assert_eq!(
        local.cells,
        vec![
            CellCoord::new(12, 12).unwrap(),
            CellCoord::new(13, 12).unwrap()
        ]
    );
    assert!(local.outlet.is_some());
    assert_eq!(objects.global.lakes, artifacts.lakes);
    assert_eq!(objects.global.model_revision, 2);
    let center = cells.get(CellCoord::new(12, 12).unwrap());
    assert_eq!(center.terrain, TerrainKind::Lake);
    assert_eq!(center.height, HeightMm::new(1));
    assert_eq!(
        cells.get(CellCoord::new(0, 0).unwrap()).terrain,
        TerrainKind::Sea
    );
    assert_eq!(
        cells.get(CellCoord::new(14, 12).unwrap()).terrain,
        TerrainKind::Land
    );
    assert!(!objects.rivers.is_empty());
    assert!(!objects.channel_edges.is_empty());
    let mut owned = std::collections::BTreeSet::new();
    for r in &objects.rivers {
        for &at in &r.course {
            assert!(owned.insert(at));
            assert_eq!(cells.get(at).terrain, TerrainKind::Land);
        }
    }
    for row in &objects.global.reaches {
        assert_eq!(
            index
                .reaches
                .binary_search_by_key(&row.id, |r| r.id)
                .map(|i| &index.reaches[i])
                .unwrap(),
            row
        );
    }
    let cell_bytes = arda_core::encode_cells(&cells);
    let object_bytes = arda_core::encode_objects(&objects).unwrap();
    assert_eq!(cell_bytes.len(), 262144 * 39);
    assert_eq!(
        arda_core::decode_cells("cells.bin", &cell_bytes).unwrap(),
        cells
    );
    assert_eq!(
        arda_core::decode_objects("objects.bin", &object_bytes).unwrap(),
        objects
    );
    let again = area_output::compose(
        tile,
        &mut artifacts.routing,
        &mut artifacts.flow,
        index,
        &artifacts.lakes,
        limits,
    )
    .unwrap();
    assert_eq!(arda_core::encode_cells(&again.0), cell_bytes);
    assert_eq!(arda_core::encode_objects(&again.1).unwrap(), object_bytes);
    // Final flow and copied global levels must agree, even when both separately look valid.
    let extent =
        crate::hydrology::routing::Extent::new(index.domain.width_cells, index.domain.height_cells)
            .unwrap();
    let at = CellIndex::new(12 * 640 + 12, extent).unwrap();
    let original = artifacts.flow.read(at).unwrap();
    artifacts
        .flow
        .write(
            at,
            crate::hydrology::fine_flow::FlowRecord {
                surface_mm: 11,
                ..original
            },
        )
        .unwrap();
    assert!(matches!(
        area_output::compose(
            tile,
            &mut artifacts.routing,
            &mut artifacts.flow,
            index,
            &artifacts.lakes,
            limits
        ),
        Err(AreaError::Invalid("flow/global lake surface disagreement"))
    ));
    artifacts.flow.write(at, original).unwrap();
    // Malformed exported coordinates must be refused before index-area ordinal arithmetic.
    let mut remote_valid = tile.valid();
    remote_valid.boundary.north = false;
    remote_valid.boundary.west = false;
    let mut remote = PreparedTerrain {
        area: AreaCoord::new(i32::MAX, i32::MAX),
        valid: remote_valid,
        heights: vec![HeightMm::new(0); 262144],
        annual_rain: vec![RainfallMm::new(0); 262144],
        temperature_base_centi: vec![0; 262144],
    };
    assert!(encode_prepared(&remote).is_err());
    remote.area = AreaCoord::new(1, 2);
    let remote_tile = decode_prepared(
        remote.area,
        remote.valid,
        &encode_prepared(&remote).unwrap(),
    )
    .unwrap();
    let invalid = FinalIndex {
        domain: arda_core::hydrology::HydrologyDomain {
            exported_areas_wide: u32::MAX,
            exported_areas_high: u32::MAX,
            ..index.domain
        },
        reaches: Vec::new(),
        crossings: Vec::new(),
        catchments: Vec::new(),
        area_reaches: Vec::new(),
        operations: 0,
        payload_bytes: 0,
    };
    assert!(matches!(
        area_output::compose(
            &remote_tile,
            &mut artifacts.routing,
            &mut artifacts.flow,
            &invalid,
            &[],
            limits
        ),
        Err(AreaError::Invalid(_))
    ));
    // Cropped private fringe is not an exported512² area.
    let fringe = prepared.tile(AreaCoord::new(1, 0)).unwrap();
    assert!(matches!(
        area_output::compose(
            fringe,
            &mut artifacts.routing,
            &mut artifacts.flow,
            index,
            &artifacts.lakes,
            limits
        ),
        Err(AreaError::Invalid(_))
    ));
}
