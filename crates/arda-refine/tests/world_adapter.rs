//! The stored-world adapter: a world written to disk and read back through
//! `arda::World` refines exactly like the same data held in memory.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::unwrap_used,
    missing_docs
)]

use arda::{Cell, Cover, GenerateConfig, Manifest, TerrainKind, ValidationStats};
use arda_core::{AreaCells, AreaObjects, CellCoord, HeightMm, FORMAT_VERSION};
use arda_refine::{refine_block, CellKey, GridSource, WorldSource};
use std::path::PathBuf;

struct Dir(PathBuf);
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn cell(x: u16, y: u16) -> Cell {
    Cell {
        height: HeightMm::new(20_000 + i32::from(x) * 150 - i32::from(y) * 90),
        terrain: if y > 300 {
            TerrainKind::Sea
        } else {
            TerrainKind::Land
        },
        cover: if (x / 3 + y / 5).is_multiple_of(2) {
            Cover::Forest
        } else {
            Cover::Grass
        },
        forest_density: u8::try_from((x * 37 + y * 11) % 256).unwrap(),
        slope_milli_deg: 3_000,
        moisture: 120,
        wetness: 40,
        height_above_river_dm: 50,
        ..Cell::default()
    }
}

#[test]
fn a_stored_world_refines_like_the_same_cells_in_memory() {
    let dir = Dir(std::env::temp_dir().join(format!("arda-refine-adapter-{}", std::process::id())));
    std::fs::create_dir_all(&dir.0).unwrap();
    let manifest = Manifest {
        format_version: FORMAT_VERSION,
        arda_version: "0.1.0".into(),
        seed: 77,
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
    arda_core::write_manifest(&dir.0, &manifest).unwrap();
    let mut cells = AreaCells::flat(Cell::default());
    let mut mem = GridSource::new(77, 1024, 2048, Cell::default());
    for y in 290..310u16 {
        for x in 10..30u16 {
            cells.set(CellCoord::new(x, y).unwrap(), cell(x, y));
        }
    }
    for y in 0..512u16 {
        for x in 0..512u16 {
            mem.set(
                CellKey::new(i64::from(x), i64::from(y)),
                *cells.get(CellCoord::new(x, y).unwrap()),
            );
        }
    }
    let area = dir.0.join("areas").join("00_00");
    std::fs::create_dir_all(&area).unwrap();
    std::fs::write(area.join("cells.bin"), arda_core::encode_cells(&cells)).unwrap();
    std::fs::write(
        area.join("objects.bin"),
        arda_core::encode_objects(&AreaObjects::empty()).unwrap(),
    )
    .unwrap();
    let world = arda::World::load(&dir.0).unwrap();
    let src = WorldSource::new(&world).unwrap();
    for k in [
        CellKey::new(15, 295),
        CellKey::new(20, 300),
        CellKey::new(25, 301),
    ] {
        let a = refine_block(&src, k).unwrap();
        let b = refine_block(&mem, k).unwrap();
        assert_eq!(a.layout_json().unwrap(), b.layout_json().unwrap(), "{k:?}");
    }
    assert!(refine_block(&src, CellKey::new(5000, 0)).is_err());
}
