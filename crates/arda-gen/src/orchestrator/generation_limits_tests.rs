//! Pure capacity arithmetic; no generation, natural fixtures or directory creation.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::hydrology::types::HydrologyLimits;
use crate::orchestrator::generation_limits::{admit, AdmissionError};
use arda_core::{GenerateConfig, LatitudeBand, SizeKm};

#[test]
fn regional_detail_sampling_is_admitted_before_output_creation() {
    let path = std::env::temp_dir().join(format!("arda-detail-admission-{}", std::process::id()));
    assert!(!path.exists());
    // Retain the actual C05 caller budget before neighborhood-based refinement.
    let prior = HydrologyLimits {
        global_event_operations: 84_297_105_428,
        ..HydrologyLimits::default()
    };
    assert!(matches!(
        admit(GenerateConfig::MICRO, &path, prior),
        Err(AdmissionError::Limit {
            resource: "logical operations",
            ..
        })
    ));
    assert!(!path.exists());
}

#[test]
fn the_previous_water_only_work_allowance_refuses_shared_erosion() {
    let path = std::env::temp_dir().join(format!("arda-work-admission-{}", std::process::id()));
    assert!(!path.exists());
    // Measured candidate04 allowance omitted the new 40-step physical pass.
    // Retain it as a caller's fixed budget, not as the new reservation formula.
    let water_only = HydrologyLimits {
        global_event_operations: 48_192_536_596,
        ..HydrologyLimits::default()
    };
    assert!(matches!(
        admit(GenerateConfig::MICRO, &path, water_only),
        Err(AdmissionError::Limit {
            resource: "logical operations",
            ..
        })
    ));
    assert!(!path.exists());
    let current = admit(GenerateConfig::MICRO, &path, HydrologyLimits::default()).unwrap();
    let exact = HydrologyLimits {
        global_event_operations: current.reservations.logical_operations,
        ..HydrologyLimits::default()
    };
    assert!(admit(GenerateConfig::MICRO, &path, exact).is_ok());
    assert_eq!(
        current.reservations.logical_operations,
        water_only.global_event_operations + current.reservations.terrain_operations
    );
}

#[test]
fn preparation_reserves_the_whole_fine_domain_before_allocation() {
    let config = GenerateConfig::default();
    let admission = admit(
        config,
        std::path::Path::new("/tmp/arda-terrain-admission"),
        HydrologyLimits::default(),
    )
    .unwrap();
    let cells = u64::from(admission.extent.cells());
    let size = config.size_km();
    let coarse = u64::from(size.width) * u64::from(size.height) * 256;
    // The shared erosion scratch needs45bytes/cell in addition to the physical
    // and regional i32 inputs. A fixed per-area allowance misses this lifetime.
    assert!(admission.reservations.continent_preparation_bytes >= coarse + cells * 53);
}

#[test]
fn default_generation_admission_is_complete_and_lowered_resources_refuse() {
    let path =
        std::env::temp_dir().join(format!("arda-admission-no-create-{}", std::process::id()));
    assert!(!path.exists());
    let maximum =
        GenerateConfig::new(SizeKm::new(4000, 4000), LatitudeBand::new(35, 55), 15).unwrap();
    for (name, config) in [
        ("micro", GenerateConfig::MICRO),
        ("default", GenerateConfig::default()),
    ] {
        let limits = HydrologyLimits::default();
        let admission = admit(config, &path, limits).unwrap();
        let r = admission.reservations;
        println!(
            "{name}: cells={}, RAM={}, scratch={}, I/O={}, I/O_calls={}, work={}",
            admission.extent.cells(),
            r.ram_bytes,
            r.scratch_bytes,
            r.io_bytes,
            r.io_operations,
            r.logical_operations
        );
        assert!(r.ram_bytes <= limits.ram_bytes);
        assert!(r.scratch_bytes <= limits.combined_scratch_bytes);
        assert_eq!(admission.prepared.cache_tiles, admission.domain.columns());
        assert_eq!(
            admission.shared.topology.nodes,
            2 * admission.shared.topology.leaves - 1
        );
        assert!(admission.shared.annual.bands <= u64::from(admission.extent.cells()));
        assert!(
            r.ram_bytes
                >= admission.prepared.ram_bytes
                    + admission.shared.reservations().unwrap().ram_bytes
                    + admission.index.ram_bytes
                    + admission.area.ram_bytes
                    + r.continent_preparation_bytes
                    + r.encoding_bytes
        );
        for fault in 0..6 {
            let mut short = limits;
            match fault {
                0 => short.ram_bytes = r.ram_bytes - 1,
                1 => short.spatial_scratch_bytes = r.spatial_scratch_bytes - 1,
                2 => short.combined_scratch_bytes = r.scratch_bytes - 1,
                3 => short.global_event_operations = r.logical_operations - 1,
                4 => short.global_io_bytes = r.io_bytes - 1,
                _ => short.global_io_operations = r.io_operations - 1,
            }
            assert!(matches!(
                admit(config, &path, short),
                Err(AdmissionError::Limit { .. })
            ));
        }
    }
    // The dense shared terrain kernel is admitted honestly: a maximum-size
    // request needs more RAM than the default rather than reverting to tiled
    // erosion and recreating the visible seams.
    assert!(matches!(
        admit(maximum, &path, HydrologyLimits::default()),
        Err(AdmissionError::Limit {
            resource: "RAM bytes",
            ..
        })
    ));
    let sufficient = HydrologyLimits {
        ram_bytes: 128 * 1024 * 1024 * 1024,
        ..HydrologyLimits::default()
    };
    assert!(admit(maximum, &path, sufficient).is_ok());
    // No capacity is silently reduced to squeeze a larger requested model into RAM.
    let larger = HydrologyLimits {
        max_bands: 8_000_000,
        ..HydrologyLimits::default()
    };
    assert!(matches!(
        admit(maximum, &path, larger),
        Err(AdmissionError::Limit {
            resource: "RAM bytes",
            ..
        })
    ));
    // Zero feature capacities are explicit support for empty authority, not guessed terrain.
    let empty = HydrologyLimits {
        max_closed_leaves: 0,
        max_bands: 0,
        max_feature_records: 0,
        max_area_references: 0,
        max_lake_boundary_edges: 0,
        ..HydrologyLimits::default()
    };
    let empty = admit(GenerateConfig::MICRO, &path, empty).unwrap();
    assert_eq!(empty.shared.topology.nodes, 0);
    assert_eq!(empty.shared.topology.leaves, 0);
    assert_eq!(empty.index.records, 0);
    assert!(!path.exists());
}
