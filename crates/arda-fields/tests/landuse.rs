//! Review round 2 #38: a land-use raster's size is checked before it is
//! allocated.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

#[test]
fn a_huge_land_use_raster_is_refused_not_allocated() {
    use arda_fields::input::LandUseGrid;
    let err = LandUseGrid::filled((0, 0), u32::MAX, u32::MAX, None).unwrap_err();
    assert!(
        matches!(err, arda_fields::FieldsError::LandUseSize { .. }),
        "{err}"
    );
    let ok = LandUseGrid::filled((0, 0), 4, 3, None).unwrap();
    assert_eq!(ok.cells.len(), 12);
}
