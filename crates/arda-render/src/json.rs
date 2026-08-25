//! The versioned JSON export schema (`logic/04` §Q14).
//!
//! snake_case SI keys, additive-only. Field order is fixed by the struct
//! definitions, so repeated exports are byte-identical.

use arda_core::{
    AreaCells, AreaObjects, Block, CellCoord, Cover, Manifest, SquareCoord, TerrainKind,
    AREA_CELLS, BLOCK_SQUARES, SKELETON_TILES,
};
use serde::Serialize;

/// Export schema version, additive-only (`logic/04` §Q14).
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Serialize)]
struct CellOut {
    height_mm: i32,
    terrain: &'static str,
    cover: &'static str,
    slope_milli_deg: u16,
    aspect_deg: u16,
    drainage_area_cells: u32,
    discharge_milli_cumecs: u32,
    watercourse_order: u8,
    watercourse_width_dm: u16,
    height_above_river_dm: u16,
    wetness: u8,
}

#[derive(Serialize)]
struct RiverOut {
    id: u16,
    order: u8,
    width_dm: u16,
    discharge_milli_cumecs: u32,
    feeds: Option<u16>,
    ends: &'static str,
    course: Vec<[u16; 2]>,
}

#[derive(Serialize)]
struct LakeOut {
    id: u16,
    surface_mm: i32,
    depth_mm: u32,
    outlet: Option<[u16; 2]>,
    cells: Vec<[u16; 2]>,
}

const fn terminus_name(t: arda_core::Terminus) -> &'static str {
    match t {
        arda_core::Terminus::Junction => "junction",
        arda_core::Terminus::Sea => "sea",
        arda_core::Terminus::Lake => "lake",
        arda_core::Terminus::OffTile => "off_tile",
    }
}

#[derive(Serialize)]
struct AreaOut {
    schema_version: u32,
    format_version: u32,
    seed: u64,
    area_x: i32,
    area_y: i32,
    cells_per_side: u16,
    cells: Vec<CellOut>,
    rivers: Vec<RiverOut>,
    lakes: Vec<LakeOut>,
}

#[derive(Serialize)]
struct LegendEntry {
    id: u16,
    name: &'static str,
}

#[derive(Serialize)]
struct BlockOut {
    schema_version: u32,
    format_version: u32,
    seed: u64,
    squares_per_side: u8,
    relaxed: bool,
    legend: Vec<LegendEntry>,
    squares: Vec<u16>,
}

const fn terrain_name(t: TerrainKind) -> &'static str {
    match t {
        TerrainKind::Sea => "sea",
        TerrainKind::Land => "land",
        TerrainKind::Lake => "lake",
    }
}

const fn cover_name(c: Cover) -> &'static str {
    match c {
        Cover::Bare => "bare",
        Cover::Grass => "grass",
        Cover::Scrub => "scrub",
        Cover::Forest => "forest",
        Cover::Marsh => "marsh",
        Cover::Rock => "rock",
        Cover::Ice => "ice",
    }
}

/// Serialises one area tile.
#[must_use]
pub fn area_json(
    manifest: &Manifest,
    ax: i32,
    ay: i32,
    cells: &AreaCells,
    objects: &AreaObjects,
) -> String {
    let mut out_cells = Vec::with_capacity(usize::from(AREA_CELLS) * usize::from(AREA_CELLS));
    for y in 0..AREA_CELLS {
        for x in 0..AREA_CELLS {
            let Some(at) = CellCoord::new(x, y) else {
                continue;
            };
            let c = cells.get(at);
            out_cells.push(CellOut {
                height_mm: c.height.raw(),
                terrain: terrain_name(c.terrain),
                cover: cover_name(c.cover),
                slope_milli_deg: c.slope_milli_deg,
                aspect_deg: c.aspect_deg,
                drainage_area_cells: c.drainage_area_cells,
                discharge_milli_cumecs: c.discharge.raw(),
                watercourse_order: c.watercourse_order,
                watercourse_width_dm: c.watercourse_width_dm,
                height_above_river_dm: c.height_above_river_dm,
                wetness: c.wetness,
            });
        }
    }

    let rivers = objects
        .rivers
        .iter()
        .map(|r| RiverOut {
            id: r.id,
            order: r.order,
            width_dm: r.width_dm,
            discharge_milli_cumecs: r.discharge.raw(),
            feeds: r.feeds,
            ends: terminus_name(r.ends),
            course: r.course.iter().map(|c| [c.x(), c.y()]).collect(),
        })
        .collect();

    let lakes: Vec<LakeOut> = objects
        .lakes
        .iter()
        .map(|l| LakeOut {
            id: l.id,
            surface_mm: l.surface.raw(),
            depth_mm: l.depth_mm,
            outlet: l.outlet.map(|c| [c.x(), c.y()]),
            cells: l.cells.iter().map(|c| [c.x(), c.y()]).collect(),
        })
        .collect();

    let doc = AreaOut {
        schema_version: SCHEMA_VERSION,
        format_version: manifest.format_version,
        seed: manifest.seed,
        area_x: ax,
        area_y: ay,
        cells_per_side: AREA_CELLS,
        cells: out_cells,
        rivers,
        lakes,
    };
    serde_json::to_string_pretty(&doc).unwrap_or_default()
}

/// Serialises one block, legend included.
///
/// Build-order step 8 grows the legend with per-tile static attributes
/// (`build-interview.md` §Q5).
#[must_use]
pub fn block_json(manifest: &Manifest, block: &Block) -> String {
    let mut squares = Vec::with_capacity(usize::from(BLOCK_SQUARES) * usize::from(BLOCK_SQUARES));
    for y in 0..BLOCK_SQUARES {
        for x in 0..BLOCK_SQUARES {
            let Some(at) = SquareCoord::new(x, y) else {
                continue;
            };
            squares.push(block.square(at).raw());
        }
    }

    let doc = BlockOut {
        schema_version: SCHEMA_VERSION,
        format_version: manifest.format_version,
        seed: manifest.seed,
        squares_per_side: BLOCK_SQUARES,
        relaxed: block.is_relaxed(),
        legend: SKELETON_TILES
            .iter()
            .map(|t| LegendEntry {
                id: t.id.raw(),
                name: t.name,
            })
            .collect(),
        squares,
    };
    serde_json::to_string_pretty(&doc).unwrap_or_default()
}
