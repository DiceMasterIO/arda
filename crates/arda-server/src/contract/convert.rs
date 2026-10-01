//! Stored fixed-point cell fields to contract units (see the field docs in [`super`]).

use super::{
    snow, CellSample, CoastSample, CoverDto, FineHeights, LakeMembership, LandUseDto,
    RiverMembership, RoadDto, TerrainKindDto, CELL_M, CONTRACT_VERSION,
};
use crate::society_cells::SocietyCell;
use arda_core::{Cell, Cover, HeightMm, Lake, RiverSegment, RoadClass, TerrainKind};

/// One drainage cell is 100 m × 100 m = 0.01 km².
pub const KM2_PER_CELL: f64 = 0.01;

/// Millimetres to metres.
#[must_use]
pub fn mm_to_m(mm: i64) -> f64 {
    // Heights and depths stay well inside f64's exact-integer range.
    #[allow(clippy::cast_precision_loss)]
    let mm = mm as f64;
    mm / 1000.0
}

/// Height newtype to metres.
#[must_use]
pub fn height_m(h: HeightMm) -> f64 {
    mm_to_m(i64::from(h.raw()))
}

/// Terrain kind to its contract name.
#[must_use]
pub const fn terrain(kind: TerrainKind) -> TerrainKindDto {
    match kind {
        TerrainKind::Sea => TerrainKindDto::Sea,
        TerrainKind::Land => TerrainKindDto::Land,
        TerrainKind::Lake => TerrainKindDto::Lake,
    }
}

/// Cover to its contract name.
#[must_use]
pub const fn cover(cover: Cover) -> CoverDto {
    match cover {
        Cover::Bare => CoverDto::Bare,
        Cover::Grass => CoverDto::Grass,
        Cover::Scrub => CoverDto::Scrub,
        Cover::Forest => CoverDto::Forest,
        Cover::Marsh => CoverDto::Marsh,
        Cover::Rock => CoverDto::Rock,
        Cover::Ice => CoverDto::Ice,
    }
}

/// Road class to its contract name.
#[must_use]
pub const fn road(road: RoadClass) -> RoadDto {
    match road {
        RoadClass::None => RoadDto::None,
        RoadClass::Track => RoadDto::Track,
        RoadClass::Road => RoadDto::Road,
        RoadClass::Highway => RoadDto::Highway,
    }
}

/// A 0–255 index as a 0–1 fraction.
#[must_use]
pub fn unit(v: u8) -> f64 {
    f64::from(v) / 255.0
}

/// Discharge in L/s (thousandths of m³/s) to m³/s.
#[must_use]
pub fn discharge_m3s(raw: u64) -> f64 {
    // Annual discharge never approaches 2^53 L/s.
    #[allow(clippy::cast_precision_loss)]
    let raw = raw as f64;
    raw / 1000.0
}

/// Where a cell sits, in every coordinate frame the contract reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellPlace {
    /// Global column.
    pub gx: u32,
    /// Global row.
    pub gy: u32,
    /// Area column.
    pub ax: i32,
    /// Area row.
    pub ay: i32,
    /// Local column.
    pub cx: u16,
    /// Local row.
    pub cy: u16,
}

/// Saved river segment to the cell-level membership record.
#[must_use]
pub fn river(segment: &RiverSegment) -> RiverMembership {
    RiverMembership {
        segment_id: segment.id,
        global_reach_id: segment.global_id.0.to_string(),
        order: segment.order,
        width_m: f64::from(segment.width_dm) / 10.0,
        discharge_m3s: discharge_m3s(segment.discharge.raw()),
    }
}

/// Saved lake to the cell-level membership record at a cell of height `ground`.
#[must_use]
pub fn lake(lake: &Lake, ground: HeightMm) -> LakeMembership {
    let depth = (i64::from(lake.surface.raw()) - i64::from(ground.raw())).max(0);
    LakeMembership {
        lake_id: lake.id,
        global_basin_id: lake.global_id.0.to_string(),
        surface_m: height_m(lake.surface),
        depth_m: mm_to_m(depth),
        max_depth_m: mm_to_m(i64::from(lake.depth_mm)),
    }
}

/// Derived inputs that need neighbours, objects or the fine layer.
#[derive(Debug, Clone, Default)]
pub struct Derived {
    /// Coast facts.
    pub coast: Option<CoastSample>,
    /// River membership.
    pub river: Option<RiverMembership>,
    /// Lake membership.
    pub lake: Option<LakeMembership>,
    /// Fine heights.
    pub fine: Option<FineHeights>,
    /// The cell's `society/` raster values, when the world has them.
    pub society: Option<SocietyCell>,
}

/// A society raster road code (`arda_ids::RoadClass::code`) to its contract
/// name; unknown codes read as no road.
#[must_use]
pub const fn society_road(code: u8) -> RoadDto {
    match arda_ids::RoadClass::from_code(code) {
        Some(arda_ids::RoadClass::Track) => RoadDto::Track,
        Some(arda_ids::RoadClass::Road) => RoadDto::Road,
        Some(arda_ids::RoadClass::Highway) => RoadDto::Highway,
        Some(arda_ids::RoadClass::Footpath) => RoadDto::Footpath,
        Some(arda_ids::RoadClass::None) | None => RoadDto::None,
    }
}

/// A `landuse.bin` code to its contract name; unknown codes read as none.
#[must_use]
pub const fn land_use(code: u8) -> LandUseDto {
    use arda_ids::LandUse as L;
    match L::from_code(code) {
        Some(L::Built) => LandUseDto::Built,
        Some(L::Field) => LandUseDto::Field,
        Some(L::Pasture) => LandUseDto::Pasture,
        Some(L::Orchard) => LandUseDto::Orchard,
        Some(L::Woodland) => LandUseDto::Woodland,
        Some(L::Mill) => LandUseDto::Mill,
        Some(L::Mine) => LandUseDto::Mine,
        Some(L::Meadow) => LandUseDto::Meadow,
        Some(L::Fallow) => LandUseDto::Fallow,
        Some(L::Farmstead) => LandUseDto::Farmstead,
        Some(L::None) | None => LandUseDto::None,
    }
}

/// `road`, `built_by`, `land_use` and `realm_id` of a cell: from its society
/// rasters when present (logic/16 §api-cell-society), else the stored cell.
fn society_fields(
    cell: &Cell,
    society: Option<SocietyCell>,
) -> (RoadDto, Option<String>, Option<LandUseDto>, Option<String>) {
    let id = |v: u32| (v != 0).then(|| v.to_string());
    match society {
        Some(s) => (
            s.road.map_or_else(|| road(cell.road), society_road),
            id(s.owner),
            Some(land_use(s.land_use)),
            id(u32::from(s.realm)),
        ),
        None => (road(cell.road), id(u32::from(cell.built_by)), None, None),
    }
}

/// Assembles the contract sample for one stored cell.
#[must_use]
pub fn cell_sample(cell: &Cell, at: CellPlace, derived: Derived) -> CellSample {
    let height = height_m(cell.height);
    let temperature_c = f64::from(cell.temperature.raw()) / 100.0;
    let slope_deg = f64::from(cell.slope_milli_deg) / 1000.0;
    let snow = snow::snow(
        cell.cover,
        cell.terrain,
        temperature_c,
        height,
        slope_deg,
        derived.fine.map(|f| f.max_m),
    );
    let (road, built_by, land_use, realm_id) = society_fields(cell, derived.society);
    CellSample {
        contract_version: CONTRACT_VERSION,
        gx: at.gx,
        gy: at.gy,
        ax: at.ax,
        ay: at.ay,
        cx: at.cx,
        cy: at.cy,
        x_m: f64::from(at.gx) * CELL_M,
        y_m: f64::from(at.gy) * CELL_M,
        centre_x_m: (f64::from(at.gx) + 0.5) * CELL_M,
        centre_y_m: (f64::from(at.gy) + 0.5) * CELL_M,
        height_m: height,
        terrain: terrain(cell.terrain),
        cover: cover(cell.cover),
        slope_deg,
        aspect_deg: f64::from(cell.aspect_deg),
        temperature_c,
        rainfall_mm: f64::from(cell.rainfall.raw()),
        moisture: unit(cell.moisture),
        wetness: unit(cell.wetness),
        forest_density: unit(cell.forest_density),
        drainage_area_km2: f64::from(cell.drainage_area_cells) * KM2_PER_CELL,
        discharge_m3s: discharge_m3s(cell.discharge.raw()),
        watercourse_order: cell.watercourse_order,
        watercourse_width_m: f64::from(cell.watercourse_width_dm) / 10.0,
        height_above_river_m: f64::from(cell.height_above_river_dm) / 10.0,
        road,
        built_by,
        land_use,
        realm_id,
        coast: derived.coast.unwrap_or(CoastSample {
            is_coast: false,
            distance_m: None,
        }),
        snow,
        river: derived.river,
        lake: derived.lake,
        fine: derived.fine,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::{DischargeMilli, RainfallMm, TempCentiC};

    fn place() -> CellPlace {
        CellPlace {
            gx: 515,
            gy: 3,
            ax: 1,
            ay: 0,
            cx: 3,
            cy: 3,
        }
    }

    #[test]
    fn every_stored_field_converts_to_its_documented_unit() {
        let cell = Cell {
            height: HeightMm::new(1_234_567),
            terrain: TerrainKind::Land,
            cover: Cover::Forest,
            slope_milli_deg: 12_500,
            aspect_deg: 270,
            temperature: TempCentiC::new(-325),
            rainfall: RainfallMm::new(840),
            moisture: 255,
            forest_density: 51,
            drainage_area_cells: 250,
            discharge: DischargeMilli::new(12_345),
            watercourse_order: 3,
            watercourse_width_dm: 45,
            height_above_river_dm: 123,
            wetness: 0,
            road: RoadClass::Track,
            built_by: 0,
        };
        let s = cell_sample(&cell, place(), Derived::default());
        assert_eq!(s.contract_version, CONTRACT_VERSION);
        assert_eq!((s.x_m, s.y_m), (51_500.0, 300.0));
        assert_eq!((s.centre_x_m, s.centre_y_m), (51_550.0, 350.0));
        assert!((s.height_m - 1234.567).abs() < 1e-9);
        assert_eq!(s.terrain, TerrainKindDto::Land);
        assert_eq!(s.cover, CoverDto::Forest);
        assert!((s.slope_deg - 12.5).abs() < 1e-12);
        assert!((s.aspect_deg - 270.0).abs() < 1e-12);
        assert!((s.temperature_c + 3.25).abs() < 1e-12);
        assert!((s.rainfall_mm - 840.0).abs() < 1e-12);
        assert!((s.moisture - 1.0).abs() < 1e-12);
        assert!((s.forest_density - 0.2).abs() < 1e-12);
        assert!(s.wetness.abs() < 1e-12);
        assert!((s.drainage_area_km2 - 2.5).abs() < 1e-12);
        assert!((s.discharge_m3s - 12.345).abs() < 1e-12);
        assert_eq!(s.watercourse_order, 3);
        assert!((s.watercourse_width_m - 4.5).abs() < 1e-12);
        assert!((s.height_above_river_m - 12.3).abs() < 1e-12);
        assert_eq!(s.road, RoadDto::Track);
        assert_eq!(s.built_by, None);
        assert!(s.fine.is_none() && s.river.is_none() && s.lake.is_none());
    }

    #[test]
    fn negative_heights_and_settlements_survive() {
        let cell = Cell {
            height: HeightMm::new(-2_000_500),
            built_by: 7,
            ..Cell::default()
        };
        let s = cell_sample(&cell, place(), Derived::default());
        assert!((s.height_m + 2000.5).abs() < 1e-9);
        assert_eq!(s.built_by.as_deref(), Some("7"));
        assert_eq!(s.terrain, TerrainKindDto::Sea);
        assert_eq!((s.land_use, s.realm_id), (None, None));
    }

    #[test]
    fn society_rasters_override_the_stored_road_and_owner() {
        let cell = Cell {
            road: RoadClass::Highway,
            built_by: 7,
            ..Cell::default()
        };
        let society = |road| Derived {
            society: Some(SocietyCell {
                land_use: 9,
                owner: 70_000,
                realm: 2,
                road,
            }),
            ..Derived::default()
        };
        let s = cell_sample(&cell, place(), society(Some(4)));
        assert_eq!(s.road, RoadDto::Footpath);
        assert_eq!(s.built_by.as_deref(), Some("70000"));
        assert_eq!(s.land_use, Some(LandUseDto::Fallow));
        assert_eq!(s.realm_id.as_deref(), Some("2"));
        // A society written before roads.bin keeps the stored road.
        assert_eq!(
            cell_sample(&cell, place(), society(None)).road,
            RoadDto::Highway
        );
        // Unowned society cells are null, not the stored owner.
        let wild = Derived {
            society: Some(SocietyCell {
                road: Some(0),
                ..SocietyCell::default()
            }),
            ..Derived::default()
        };
        let w = cell_sample(&cell, place(), wild);
        assert_eq!((w.built_by, w.realm_id), (None, None));
        assert_eq!(w.land_use, Some(LandUseDto::None));
        assert_eq!(w.road, RoadDto::None);
    }

    #[test]
    fn every_society_code_has_its_contract_name() {
        for class in arda_ids::LandUse::ALL {
            let json = serde_json::to_value(land_use(class.code())).unwrap();
            assert_eq!(json, serde_json::to_value(class).unwrap());
        }
        for code in 0..=4 {
            let class = arda_ids::RoadClass::from_code(code).unwrap();
            let json = serde_json::to_value(society_road(code)).unwrap();
            assert_eq!(json, serde_json::to_value(class).unwrap());
        }
        assert_eq!(land_use(200), LandUseDto::None);
        assert_eq!(society_road(200), RoadDto::None);
    }

    #[test]
    fn lake_depth_is_surface_minus_ground_and_never_negative() {
        let lake_record = Lake {
            global_id: arda_core::BasinId(u64::MAX),
            id: 2,
            surface: HeightMm::new(10_000),
            depth_mm: 4_000,
            outlet: None,
            cells: Vec::new(),
        };
        let wet = lake(&lake_record, HeightMm::new(7_500));
        assert!((wet.depth_m - 2.5).abs() < 1e-12);
        assert_eq!(wet.global_basin_id, u64::MAX.to_string());
        assert!(lake(&lake_record, HeightMm::new(12_000)).depth_m.abs() < 1e-12);
    }
}
