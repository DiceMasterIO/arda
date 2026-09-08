//! Actual public core types and format-4 foundation integration.
use crate::*;

#[test]
fn global_coordinates_have_one_public_authority_and_keep_coordinate_order() {
    let at = GlobalCell { x: 1024, y: 513 };
    let same: coords::GlobalCell = at;
    assert_eq!(same, at);
    assert_eq!(hydrology::JunctionId::at(at).cell(), at);
    assert!(GlobalCell { x: 0, y: 10 } < GlobalCell { x: 1, y: 0 });
    let _: BasinId = hydrology::BasinId(1);
    let _: Litres = hydrology::Litres(10_000);
}
#[test]
fn fine_and_coarse_cells_share_the_wide_type_and_explicit_little_endian_layout() {
    let q = DischargeMilli::new(33_249_619_483);
    let width = hydrology::channel_width_dm(q).unwrap();
    let fine = Cell {
        discharge: q,
        watercourse_width_dm: width,
        ..Cell::default()
    };
    let coarse = ContinentCell {
        discharge: fine.discharge,
        ..ContinentCell::default()
    };
    let cells = AreaCells::flat(fine);
    let bytes = encode_cells(&cells);
    assert_eq!(CELL_BYTES, 39);
    assert_eq!(bytes.len(), 512 * 512 * 39);
    assert_eq!(&bytes[20..28], &q.raw().to_le_bytes());
    assert_eq!(&bytes[29..33], &width.to_le_bytes());
    assert_eq!(decode_cells("cells.bin", &bytes).unwrap(), cells);
    assert!(decode_cells("old-33-byte-cells.bin", &vec![0; 512 * 512 * 33]).is_err());
    let grid = ContinentOverview {
        width: 1,
        height: 1,
        cells: vec![coarse],
    };
    let bytes = encode_overview(&grid);
    assert_eq!(OVERVIEW_CELL_BYTES, 22);
    assert_eq!(bytes.len(), 38);
    assert_eq!(&bytes[30..38], &q.raw().to_le_bytes());
    assert_eq!(decode_overview("overview.bin", &bytes).unwrap(), grid);
    assert_eq!(DischargeMilli::new(u64::MAX).raw(), u64::MAX);
}
#[test]
fn retained_continent_objects_and_new_area_wrappers_are_both_public() {
    let coarse = ContinentObjects {
        rivers: vec![ContinentRiver {
            id: 1,
            catchment_km2: 30,
            discharge: DischargeMilli::new(33_249_619_483),
            feeds: None,
            course: vec![KmCoord::new(0, 0)],
        }],
    };
    let bytes = encode_continent_objects(&coarse);
    assert_eq!(bytes.len(), 44);
    assert_eq!(
        decode_continent_objects("continent.bin", &bytes).unwrap(),
        coarse
    );
    let local = AreaObjects::empty();
    let bytes = encode_objects(&local).unwrap();
    assert_eq!(bytes.len(), 78);
    assert_eq!(decode_objects("objects.bin", &bytes).unwrap(), local);
    let bad = decode_objects("objects.bin", &bytes[..20]).unwrap_err();
    assert!(matches!(bad,FormatError::Objects{path,..} if path=="objects.bin"));
    let _public: formats::area_objects_v4::ObjectsFormatError =
        formats::area_objects_v4::ObjectsFormatError::BadMagic;
    let _legacy_path: formats::objects::ObjectsFormatError =
        formats::area_objects_v4::ObjectsFormatError::BadMagic;
}
#[test]
fn major_four_and_typed_hydrology_error_preserve_the_public_contract() {
    assert_eq!(FORMAT_VERSION, 4);
    assert_eq!(formats::FORMAT_VERSION, 4);
    let err = FormatError::Hydrology {
        path: "hydrology/lakes.bin".to_owned(),
        source: formats::hydrology::HydrologyFormatError::Truncated,
    };
    assert!(err.to_string().contains("hydrology/lakes.bin"));
    let load = LoadError::from(err);
    assert!(matches!(load, LoadError::Corrupt { .. }));
}
