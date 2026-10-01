use super::*;
use arda_core::{LatitudeBand, SizeKm};

#[test]
fn full_fine_world_admits_observed_closed_leaves_with_complete_resource_envelope() {
    let config =
        GenerateConfig::new(SizeKm::new(500, 1000), LatitudeBand::new(35, 55), 15).unwrap();
    let limits = FineDeliveryLimits::default();
    let observed = 194_453;
    assert!(HydrologyLimits::default().max_closed_leaves < observed);
    assert!(limits.world.max_closed_leaves >= observed);
    let source = fine_source::admit(config, limits.source).unwrap();
    let source_bytes = u64::try_from(source.file_bytes).unwrap();
    let staged_peak = source_bytes * 2;
    let mut world_limits = limits.world;
    world_limits.combined_scratch_bytes -= staged_peak;
    let fine = fine_world::admit_bytes(source_bytes, config, world_limits).unwrap();
    let admission = generation_limits::admit(
        config,
        Path::new("/tmp/arda-fine-500x1000-admission/.arda-hydrology-scratch"),
        fine.remaining,
    )
    .unwrap();
    let r = admission.reservations;
    assert_eq!(
        admission.shared.mst.max_leaves,
        limits.world.max_closed_leaves
    );
    assert!(r.ram_bytes <= fine.remaining.ram_bytes);
    assert!(r.spatial_scratch_bytes <= fine.remaining.spatial_scratch_bytes);
    assert!(r.scratch_bytes + staged_peak <= limits.world.combined_scratch_bytes);
    assert!(r.io_bytes <= fine.remaining.global_io_bytes);
    assert!(r.io_operations <= fine.remaining.global_io_operations);
    assert!(r.logical_operations <= fine.remaining.global_event_operations);
    eprintln!(
        "fine500x1000: source_bytes={source_bytes} leaves={} mst_scratch={} ram={} scratch_with_source={} io_bytes={} io_calls={} work={}",
        admission.shared.mst.max_leaves,
        admission.shared.mst.scratch_bytes,
        r.ram_bytes,
        r.scratch_bytes + staged_peak,
        r.io_bytes,
        r.io_operations,
        r.logical_operations,
    );
}

#[test]
fn source_admission_refuses_before_touching_world_or_stage() {
    let parent = std::env::temp_dir().join(format!("arda-fine-admit-{}", std::process::id()));
    fs::create_dir_all(&parent).unwrap();
    let out = parent.join("world");
    let mut limits = FineDeliveryLimits::default();
    limits.source.max_ram_bytes = 0;
    assert!(generate_world_with_fine_source(42, GenerateConfig::MICRO, &out, limits).is_err());
    assert!(!out.exists());
    assert_eq!(fs::read_dir(&parent).unwrap().count(), 0);
    fs::remove_dir(&parent).unwrap();
}

#[test]
fn world_admission_and_occupied_output_refuse_before_staging() {
    let parent = std::env::temp_dir().join(format!("arda-fine-refuse-{}", std::process::id()));
    fs::create_dir_all(&parent).unwrap();
    let out = parent.join("world");
    let mut limits = FineDeliveryLimits::default();
    limits.world.ram_bytes = 0;
    assert!(generate_world_with_fine_source(42, GenerateConfig::MICRO, &out, limits).is_err());
    assert!(!out.exists());
    fs::create_dir(&out).unwrap();
    fs::write(out.join("keep"), b"user data").unwrap();
    assert!(matches!(
        generate_world_with_fine_source(
            42,
            GenerateConfig::MICRO,
            &out,
            FineDeliveryLimits::default(),
        ),
        Err(GenError::OutputNotEmpty { .. })
    ));
    assert_eq!(fs::read(out.join("keep")).unwrap(), b"user data");
    assert_eq!(fs::read_dir(&parent).unwrap().count(), 1);
    fs::remove_dir_all(parent).unwrap();
}

#[test]
fn private_stage_cleanup_does_not_remove_unrelated_sibling() {
    let parent = std::env::temp_dir().join(format!("arda-fine-stage-{}", std::process::id()));
    fs::create_dir_all(&parent).unwrap();
    let keep = parent.join("keep");
    fs::write(&keep, b"user data").unwrap();
    let stage = SourceStage::create(&parent.join("world")).unwrap();
    let staged = stage.source();
    fs::write(&staged, b"candidate").unwrap();
    stage.remove().unwrap();
    assert!(!staged.exists());
    assert_eq!(fs::read(&keep).unwrap(), b"user data");
    fs::remove_file(keep).unwrap();
    fs::remove_dir(parent).unwrap();
}
