use super::*;
use arda_core::{AreaCells, Cell};
use arda_core::{
    FineTerrainDescriptor, HeightMm, TerrainKind, ValidationStats,
    FINE_TERRAIN_LATEST_RECIPE_VERSION, FINE_TERRAIN_RECIPE_VERSION, FORMAT_VERSION,
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "arda-lazy-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        let manifest = Manifest {
            format_version: FORMAT_VERSION,
            arda_version: "0.1.0".into(),
            seed: 42,
            config: GenerateConfig::MICRO,
            areas_wide: 2,
            areas_high: 4,
            stats: ValidationStats {
                land_fraction_permille: 500,
                area_count: 8,
                settlement_count: 0,
                named_river_count: 0,
                river_count: 0,
            },
            fine_terrain: None,
        };
        arda_core::write_manifest(&path, &manifest).unwrap();
        Self(path)
    }
    fn write_area(&self, x: i32, y: i32) {
        let path = self.0.join("areas").join(AreaCoord::new(x, y).dir_name());
        std::fs::create_dir_all(&path).unwrap();
        let cells = AreaCells::flat(Cell {
            terrain: TerrainKind::Land,
            height: HeightMm::new(1234),
            ..Cell::default()
        });
        std::fs::write(path.join("cells.bin"), arda_core::encode_cells(&cells)).unwrap();
        std::fs::write(
            path.join("objects.bin"),
            arda_core::encode_objects(&AreaObjects::empty()).unwrap(),
        )
        .unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn manifest_load_does_not_read_any_layer() {
    let fixture = Fixture::new();
    let world = World::load(&fixture.0).unwrap();
    assert_eq!(world.areas(), 8);
    assert_eq!(
        world.area_coords().collect::<Vec<_>>(),
        [
            (0, 0),
            (1, 0),
            (0, 1),
            (1, 1),
            (0, 2),
            (1, 2),
            (0, 3),
            (1, 3)
        ]
    );
    let error = world.area(1, 2).unwrap_err();
    assert!(matches!(
        error,
        LoadError::Corrupt {
            source: arda_core::FormatError::Io { path, .. }
        } if Path::new(&path) == fixture.0.join("areas").join("01_02").join("cells.bin")
    ));
    assert!(matches!(
        world.area(-1, 0),
        Err(LoadError::OutOfRange { what: "area", .. })
    ));
}

#[test]
fn declared_fine_layer_never_falls_back_when_missing_or_corrupt() {
    let fixture = Fixture::new();
    let legacy = World::load(&fixture.0).unwrap();
    assert!(legacy.fine_terrain(1 << 20).unwrap().is_none());

    let mut manifest = arda_core::read_manifest(&fixture.0).unwrap();
    manifest.fine_terrain = Some(FineTerrainDescriptor {
        recipe_version: FINE_TERRAIN_RECIPE_VERSION,
        attempt: 0,
    });
    arda_core::write_manifest(&fixture.0, &manifest).unwrap();
    let world = World::load(&fixture.0).unwrap();
    assert!(matches!(
        world.fine_terrain(1 << 20),
        Err(LoadError::Corrupt {
            source: FormatError::Terrain { .. }
        })
    ));
    let path = fixture.0.join(FINE_TERRAIN_PATH);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"not a fine terrain").unwrap();
    assert!(matches!(
        world.fine_terrain(1 << 20),
        Err(LoadError::Corrupt {
            source: FormatError::Terrain { .. }
        })
    ));

    manifest.fine_terrain.as_mut().unwrap().recipe_version = FINE_TERRAIN_LATEST_RECIPE_VERSION + 1;
    arda_core::write_manifest(&fixture.0, &manifest).unwrap();
    assert!(matches!(
        World::load(&fixture.0),
        Err(LoadError::ManifestUnreadable { .. })
    ));
}

#[test]
fn owned_export_reads_do_not_fill_borrowed_cache() {
    let fixture = Fixture::new();
    fixture.write_area(0, 0);
    let world = World::load(&fixture.0).unwrap();
    let owned = world.read_area(0, 0).unwrap();
    assert_eq!(owned.cell(0, 0).unwrap().height.raw(), 1234);
    std::fs::remove_file(fixture.0.join("areas/00_00/cells.bin")).unwrap();
    assert!(world.area(0, 0).is_err());
}

#[test]
fn borrowed_area_is_cached_and_safe_for_concurrent_readers() {
    let fixture = Fixture::new();
    fixture.write_area(0, 0);
    let world = World::load(&fixture.0).unwrap();
    std::thread::scope(|scope| {
        let a = scope.spawn(|| world.area(0, 0).unwrap());
        let b = scope.spawn(|| world.area(0, 0).unwrap());
        assert!(std::ptr::eq(a.join().unwrap(), b.join().unwrap()));
    });
    std::fs::remove_file(fixture.0.join("areas/00_00/cells.bin")).unwrap();
    assert_eq!(
        world
            .area(0, 0)
            .unwrap()
            .cell(511, 511)
            .unwrap()
            .height
            .raw(),
        1234
    );
    assert!(world
        .block(0, 0, 0, 0)
        .unwrap_err()
        .to_string()
        .contains("00_00.tiles.zst"));
}

#[test]
fn malformed_manifest_dimensions_fail_before_allocating_caches() {
    let fixture = Fixture::new();
    let mut manifest = arda_core::read_manifest(&fixture.0).unwrap();
    manifest.areas_wide = i32::MAX;
    arda_core::write_manifest(&fixture.0, &manifest).unwrap();
    assert!(matches!(
        World::load(&fixture.0),
        Err(LoadError::ManifestUnreadable { .. })
    ));
}

fn saved_context(objects: &AreaObjects) -> (Fixture, World) {
    let fixture = Fixture::new();
    fixture.write_area(0, 0);
    std::fs::write(
        fixture.0.join("areas/00_00/objects.bin"),
        arda_core::encode_objects(objects).unwrap(),
    )
    .unwrap();
    let world = World::load(&fixture.0).unwrap();
    (fixture, world)
}

fn spill_objects(from: arda_core::GlobalCell, to: Option<arda_core::GlobalCell>) -> AreaObjects {
    use arda_core::hydrology::{BasinId, GlobalLake, Litres, ReceivingAccount, SpillConnection};
    let mut objects = AreaObjects::empty();
    objects.global.lakes.push(GlobalLake {
        basin: BasinId(1),
        surface: HeightMm::new(1),
        deepest_bed: HeightMm::new(0),
        submerged_cells: 1,
        outlet: Some(SpillConnection {
            from,
            to,
            sill: HeightMm::new(1),
            receiving: if to.is_some() {
                ReceivingAccount::Sea
            } else {
                ReceivingAccount::DomainExport
            },
        }),
        annual_outflow: Litres(1),
        mean_outflow: arda_core::DischargeMilli::new(0),
    });
    objects
}

#[test]
fn saved_spills_use_the_actual_requested_exported_union_rim() {
    use arda_core::GlobalCell;
    for objects in [
        spill_objects(GlobalCell { x: 20, y: 20 }, None),
        spill_objects(
            GlobalCell { x: 1023, y: 20 },
            Some(GlobalCell { x: 1024, y: 20 }),
        ),
        spill_objects(GlobalCell { x: 20, y: 2048 }, None),
    ] {
        let (fixture, world) = saved_context(&objects);
        let error = world.read_area(0, 0).unwrap_err();
        assert!(matches!(
            error,
            LoadError::Corrupt {
                source: arda_core::FormatError::Hydrology { path, .. }
            } if Path::new(&path) == fixture.0.join("areas").join("00_00").join("objects.bin")
        ));
    }
    // MICRO's exported overshoot is part of the physical domain.
    let (_fixture, world) = saved_context(&spill_objects(GlobalCell { x: 1023, y: 2047 }, None));
    assert!(world.read_area(0, 0).is_ok());
    // A 64 km request has a private fringe beyond its single 512-cell export.
    let (fixture, _) = saved_context(&spill_objects(GlobalCell { x: 639, y: 300 }, None));
    let mut manifest = arda_core::read_manifest(&fixture.0).unwrap();
    manifest.config = GenerateConfig::new(
        SizeKm {
            width: 64,
            height: 64,
        },
        manifest.config.latitude_band(),
        manifest.config.mean_density_per_km2(),
    )
    .unwrap();
    manifest.areas_wide = 1;
    manifest.areas_high = 1;
    arda_core::write_manifest(&fixture.0, &manifest).unwrap();
    let world = World::load(&fixture.0).unwrap();
    assert!(world.read_area(0, 0).is_ok());
    assert!(!fixture.0.join("hydrology").exists());
}

#[test]
fn copied_coordinates_and_receiving_identities_cannot_leave_the_manifest_domain() {
    use arda_core::hydrology::{
        AnnualCatchment, CatchmentId, ChannelEdge, GlobalReach, Litres, ReachId, ReceivingAccount,
        SharedCrossing,
    };
    use arda_core::{DischargeMilli, GlobalCell};
    let at = |x, y| GlobalCell { x, y };
    let owner = AnnualCatchment {
        catchment: CatchmentId(1),
        terminal: at(0, 0),
        contributing_cells: 1,
        basin: None,
        representative_lake: None,
        potential_spill: None,
        receiving: ReceivingAccount::Sea,
    };
    let reach = GlobalReach {
        id: ReachId::from_step(at(20, 20), at(21, 20)).unwrap(),
        from: at(20, 20),
        to: at(21, 20),
        receiving: ReceivingAccount::Sea,
        catchment: owner.catchment,
        drainage_cells: 1,
        annual_volume: Litres(40 * 31_536_000),
        mean_discharge: DischargeMilli::new(40),
    };
    let mut cases = Vec::new();
    let mut objects = AreaObjects::empty();
    objects.global.catchments.push(AnnualCatchment {
        terminal: at(1024, 0),
        ..owner
    });
    cases.push(objects);
    let mut objects = AreaObjects::empty();
    let bad_spill = spill_objects(at(20, 20), None).global.lakes[0]
        .outlet
        .unwrap();
    objects.global.catchments.push(AnnualCatchment {
        basin: Some(arda_core::hydrology::BasinId(1)),
        potential_spill: Some(bad_spill),
        receiving: ReceivingAccount::DomainExport,
        ..owner.clone()
    });
    cases.push(objects);
    for bad in [
        GlobalReach {
            id: ReachId::from_step(at(1023, 20), at(1024, 20)).unwrap(),
            from: at(1023, 20),
            to: at(1024, 20),
            ..reach.clone()
        },
        GlobalReach {
            receiving: ReceivingAccount::Reach(
                ReachId::from_step(at(1024, 0), at(1025, 0)).unwrap(),
            ),
            ..reach.clone()
        },
        GlobalReach {
            id: ReachId::point(at(20, 20)).unwrap(),
            from: at(20, 20),
            to: at(20, 20),
            receiving: ReceivingAccount::DomainExport,
            ..reach.clone()
        },
    ] {
        let mut objects = AreaObjects::empty();
        objects.global.catchments.push(owner.clone());
        objects.global.reaches.push(bad);
        cases.push(objects);
    }
    let mut objects = AreaObjects::empty();
    objects.global.catchments.push(owner.clone());
    objects.global.reaches.push(reach.clone());
    objects.global.crossings.push(SharedCrossing {
        id: arda_core::hydrology::CrossingId {
            low: at(1023, 20),
            high: at(1024, 20),
        },
        from: at(1023, 20),
        to: at(1024, 20),
        reach: reach.id,
        catchment: owner.catchment,
        drainage_cells: 1,
        annual_volume: reach.annual_volume,
        mean_discharge: reach.mean_discharge,
        receiving: ReceivingAccount::Sea,
    });
    cases.push(objects);
    let mut objects = AreaObjects::empty();
    objects.channel_edges.push(ChannelEdge {
        from: at(1023, 1),
        to: at(1024, 1),
        from_width_dm: 8,
        to_width_dm: 8,
        discharge: DischargeMilli::new(40),
    });
    cases.push(objects);
    for objects in cases {
        let (_fixture, world) = saved_context(&objects);
        assert!(world.read_area(0, 0).is_err());
    }
}

#[test]
fn oversized_requested_layers_are_rejected_before_read_allocation() {
    use std::io::Write;

    for name in ["cells.bin", "objects.bin"] {
        let fixture = Fixture::new();
        fixture.write_area(0, 0);
        let dir = fixture.0.join("areas").join("00_00");
        let layer_bytes =
            |name| usize::try_from(std::fs::metadata(dir.join(name)).unwrap().len()).unwrap();
        let cell_bytes = layer_bytes("cells.bin");
        let object_bytes = layer_bytes("objects.bin");
        let world = World::load(&fixture.0).unwrap();
        assert!(world.read_area(0, 0).is_ok());
        assert!(world
            .read_area_with_byte_limits(0, 0, cell_bytes, object_bytes)
            .is_ok());

        // Small explicit caps exercise the same admission path on filesystems
        // where extending a file allocates every byte rather than sparse space.
        let path = dir.join(name);
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(&[0])
            .unwrap();
        let error = world
            .read_area_with_byte_limits(0, 0, cell_bytes, object_bytes)
            .unwrap_err();
        assert!(matches!(
            error,
            LoadError::Corrupt {
                source: arda_core::FormatError::Hydrology {
                    path: error_path,
                    source: arda_core::formats::hydrology::HydrologyFormatError::Limit(
                        "saved layer byte limit"
                    ),
                }
            } if Path::new(&error_path) == path
        ));
    }
}
