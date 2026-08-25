//! Isostatic base elevation and the coastline cut (`logic/01` step 3).

use super::plates::{CrustType, Plate};

/// Base elevation of continental crust before tectonics, millimetres.
pub const CONTINENTAL_BASE_MM: i32 = 320_000;
/// Base elevation of oceanic crust before tectonics, millimetres.
pub const OCEANIC_BASE_MM: i32 = -2_100_000;

/// Base elevation for a cell owned by `plate`.
#[must_use]
pub const fn base_elevation_mm(plate: &Plate) -> i32 {
    match plate.crust {
        CrustType::Continental => CONTINENTAL_BASE_MM,
        CrustType::Oceanic => OCEANIC_BASE_MM,
    }
}

/// Forces a cell to ocean when it lies within `margin` cells of the visible
/// map edge (`logic/01` invariant: every map-edge cell is ocean).
#[must_use]
pub const fn rim_forced_ocean(x: i32, y: i32, width: i32, height: i32, margin: i32) -> bool {
    x < margin || y < margin || x >= width - margin || y >= height - margin
}
