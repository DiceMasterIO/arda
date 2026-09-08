//! Public entrypoint refusal checks; neither test runs continent/terrain generation.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::{generate_world, generate_world_with_limits, GenError, HydrologyLimits};
use arda_core::GenerateConfig;
use std::path::PathBuf;
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "arda-entrypoint-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn resource_refusal_precedes_output_creation() {
    let parent = Directory::new();
    let out = parent.0.join("not-created");
    let limits = HydrologyLimits {
        ram_bytes: 0,
        ..HydrologyLimits::default()
    };
    assert!(matches!(
        generate_world_with_limits(42, GenerateConfig::MICRO, &out, limits),
        Err(GenError::Admission(_))
    ));
    assert!(!out.exists());
    assert_eq!(std::fs::read_dir(&parent.0).unwrap().count(), 0);
}
#[test]
fn occupied_output_refuses_without_changing_existing_files() {
    let out = Directory::new();
    let sentinel = out.0.join("existing-data.bin");
    std::fs::write(&sentinel, b"keep exactly").unwrap();
    assert!(matches!(
        generate_world(42, GenerateConfig::default(), &out.0),
        Err(GenError::OutputNotEmpty { .. })
    ));
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"keep exactly");
    assert_eq!(std::fs::read_dir(&out.0).unwrap().count(), 1);
}

#[test]
fn final_write_envelope_refuses_before_layer_creation() {
    use super::{write_file, FinalWrites, WorldOutput};
    let directory = Directory::new();
    let output = WorldOutput::begin(&directory.0).unwrap();
    let name = std::path::Path::new("test/layer.bin");
    for (bytes, operations) in [(3, 4), (4, 3)] {
        let mut writes = FinalWrites {
            bytes: 0,
            operations: 0,
            byte_limit: bytes,
            operation_limit: operations,
        };
        assert!(matches!(
            write_file(&output, name, b"four", &mut writes),
            Err(GenError::ResourceEnvelope { .. })
        ));
        assert_eq!((writes.bytes, writes.operations), (0, 0));
        assert!(!directory.0.join(name).exists());
        assert!(!directory.0.join(arda_core::MANIFEST_NAME).exists());
    }
    let mut writes = FinalWrites {
        bytes: 0,
        operations: 0,
        byte_limit: 4,
        operation_limit: 4,
    };
    write_file(&output, name, b"four", &mut writes).unwrap();
    assert_eq!((writes.bytes, writes.operations), (4, 4));
    assert_eq!(std::fs::read(directory.0.join(name)).unwrap(), b"four");
    assert!(!directory.0.join(arda_core::MANIFEST_NAME).exists());
}
