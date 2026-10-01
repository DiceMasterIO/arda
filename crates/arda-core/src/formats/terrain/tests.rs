use std::{
    fs::{self, File, OpenOptions},
    io::Write,
};

use super::*;
use crate::{formats::TempDir, TerrainField};

const BUDGET: u64 = 65_536 + 2 * 4 * 4;
const ORIGIN: TerrainPoint = TerrainPoint {
    x_um: -91,
    y_um: 23,
};
const HEIGHTS: [i32; 12] = [-9, 0, 7, 12, 5, -2, 21, -14, 17, 8, -31, 4];

fn create(path: &std::path::Path) {
    let file = OpenOptions::new()
        .write(true)
        .read(true)
        .create_new(true)
        .open(path)
        .unwrap();
    let mut writer = TerrainFileWriter::new(file, ORIGIN, 7, 4, 3).unwrap();
    for row in HEIGHTS.as_chunks::<4>().0 {
        writer
            .write_row(&row.iter().copied().map(HeightMm::new).collect::<Vec<_>>())
            .unwrap();
    }
    writer.finish().unwrap();
}

fn error_for_bytes(bytes: &[u8]) -> TerrainFileError {
    let temp = TempDir::new();
    let path = temp.path().join("field.bin");
    fs::write(&path, bytes).unwrap();
    match TerrainFileReader::open(File::open(path).unwrap(), BUDGET) {
        Ok(_) => panic!("corrupt file was accepted"),
        Err(error) => error,
    }
}

#[test]
fn real_file_matches_owned_field_across_cache_order_and_signed_fractions() {
    let temp = TempDir::new();
    let path = temp.path().join("field.bin");
    create(&path);
    let mut reader = TerrainFileReader::open(File::open(&path).unwrap(), BUDGET).unwrap();
    let owned = TerrainField::new(ORIGIN, 7, 4, 3, HEIGHTS.map(HeightMm::new).to_vec()).unwrap();
    assert_eq!(
        (
            reader.origin(),
            reader.spacing_um(),
            reader.width(),
            reader.height()
        ),
        (ORIGIN, 7, 4, 3)
    );
    for &y in &[23, 37, 26, 30, 24, 37, 22, 38] {
        for x in -92..=-69 {
            let point = TerrainPoint { x_um: x, y_um: y };
            assert_eq!(
                reader.sample(point).unwrap(),
                owned.sample(point),
                "{point:?}"
            );
        }
    }
}

#[test]
fn header_payload_version_and_completion_are_validated() {
    let temp = TempDir::new();
    let path = temp.path().join("field.bin");
    create(&path);
    let bytes = fs::read(&path).unwrap();
    let mut changed = bytes.clone();
    changed[HEADER_LEN + 9] ^= 1;
    assert!(matches!(
        error_for_bytes(&changed),
        TerrainFileError::ChecksumMismatch
    ));
    let mut changed = bytes.clone();
    changed[16] ^= 1;
    assert!(matches!(
        error_for_bytes(&changed),
        TerrainFileError::ChecksumMismatch
    ));
    let mut changed = bytes.clone();
    changed[8] = 2;
    assert!(matches!(
        error_for_bytes(&changed),
        TerrainFileError::UnsupportedVersion { .. }
    ));
    let mut changed = bytes.clone();
    changed[10] = 1;
    assert!(matches!(
        error_for_bytes(&changed),
        TerrainFileError::UnsupportedVersion { .. }
    ));
    let mut changed = bytes.clone();
    changed[44] = 1;
    assert!(matches!(
        error_for_bytes(&changed),
        TerrainFileError::InvalidHeader("reserved")
    ));
    let mut changed = bytes.clone();
    changed[12] = 0;
    assert!(matches!(
        error_for_bytes(&changed),
        TerrainFileError::InvalidHeader("not finalized")
    ));
    let mut changed = bytes.clone();
    changed[48] ^= 1;
    assert!(matches!(
        error_for_bytes(&changed),
        TerrainFileError::LengthMismatch
    ));
    assert!(matches!(
        error_for_bytes(&bytes[..bytes.len() - 1]),
        TerrainFileError::LengthMismatch
    ));
    let mut changed = bytes;
    changed.push(0);
    assert!(matches!(
        error_for_bytes(&changed),
        TerrainFileError::LengthMismatch
    ));
}

#[test]
fn writer_rejects_wrong_rows_nonempty_dest_and_low_reader_budget() {
    let temp = TempDir::new();
    let path = temp.path().join("partial.bin");
    let file = OpenOptions::new()
        .write(true)
        .read(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    let mut writer = TerrainFileWriter::new(file, ORIGIN, 7, 4, 3).unwrap();
    assert!(matches!(
        writer.write_row(&[HeightMm::new(1)]),
        Err(TerrainFileError::RowWidthMismatch)
    ));
    writer.write_row(&[HeightMm::new(1); 4]).unwrap();
    assert!(matches!(
        writer.finish(),
        Err(TerrainFileError::IncompleteRows)
    ));
    assert!(matches!(
        TerrainFileReader::open(File::open(&path).unwrap(), BUDGET),
        Err(TerrainFileError::InvalidHeader("not finalized"))
    ));
    assert!(matches!(
        TerrainFileWriter::new(
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .unwrap(),
            ORIGIN,
            7,
            4,
            3
        ),
        Err(TerrainFileError::DestinationNotEmpty)
    ));

    let full = temp.path().join("full.bin");
    create(&full);
    assert!(matches!(
        TerrainFileReader::open(File::open(full).unwrap(), BUDGET - 1),
        Err(TerrainFileError::ResourceLimit)
    ));

    let extra = temp.path().join("extra.bin");
    let file = OpenOptions::new()
        .write(true)
        .read(true)
        .create_new(true)
        .open(extra)
        .unwrap();
    let mut writer = TerrainFileWriter::new(file, ORIGIN, 7, 4, 3).unwrap();
    for _ in 0..3 {
        writer.write_row(&[HeightMm::new(-1); 4]).unwrap();
    }
    assert!(matches!(
        writer.write_row(&[HeightMm::new(-1); 4]),
        Err(TerrainFileError::TooManyRows)
    ));
    writer.finish().unwrap();
}

#[test]
fn short_row_read_invalidates_cache_and_can_be_retried() {
    let temp = TempDir::new();
    let path = temp.path().join("field.bin");
    create(&path);
    let original = fs::read(&path).unwrap();
    let mut reader = TerrainFileReader::open(File::open(&path).unwrap(), BUDGET).unwrap();
    let p0 = TerrainPoint {
        x_um: -90,
        y_um: 24,
    };
    let p1 = TerrainPoint {
        x_um: -90,
        y_um: 37,
    };
    reader.sample(p0).unwrap();
    OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(HEADER_BYTES + 2 * 16 + 2)
        .unwrap();
    assert!(matches!(reader.sample(p1), Err(TerrainFileError::Io(_))));
    let mut restore = OpenOptions::new().write(true).open(&path).unwrap();
    restore.write_all(&original).unwrap();
    assert_eq!(reader.sample(p1).unwrap(), Some(HeightMm::new(16)));
    assert_eq!(reader.sample(p0).unwrap(), Some(HeightMm::new(-6)));
}
