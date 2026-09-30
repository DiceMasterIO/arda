//! A synthetic 16 × 16 cell world with every feature the refiner handles:
//! forest and grassland with a density gradient, a river of saved channel
//! edges down a valley, a lake, a sea coast, a marsh and a steep rocky hill,
//! over an analytic fine terrain lattice. Used by this crate's tests and by
//! downstream crates (arda-blocks, arda-server) that need a small world
//! without generating one.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]

use crate::source::{Edge, LakeInfo};
use crate::{CellKey, GridSource};
use arda::{Cell, Cover, TerrainKind};
use arda_core::{DischargeMilli, HeightMm, TempCentiC};

/// World side in cells.
pub const W: u16 = 16;
/// The row the main river runs along, west to east.
pub const RIVER_ROW: i64 = 7;
/// Lake cells.
pub const LAKE: [(i64, i64); 4] = [(11, 2), (12, 2), (11, 3), (12, 3)];
/// Lake surface, metres above sea level.
pub const LAKE_SURFACE_M: f64 = 20.0;
/// First sea row; everything south of it is sea.
pub const SEA_ROW: i64 = 13;
/// Rocky hill cells.
pub const ROCK: [(i64, i64); 4] = [(2, 10), (3, 10), (2, 11), (3, 11)];
/// Marsh cells.
pub const MARSH: [(i64, i64); 2] = [(8, 10), (9, 10)];

/// Terrain height in metres at world position `(x, y)` in metres.
#[must_use]
pub fn height(x: f64, y: f64) -> f64 {
    let cx = x / 100.0;
    let cy = y / 100.0;
    // Tilted plain falling south-east, a valley along the river row, a
    // rocky hill and a basin under the lake; the sea floor to the south.
    let mut h = 40.0 - 1.2 * cx - 1.0 * cy;
    let dv = cy - (RIVER_ROW as f64 + 0.5);
    h -= 6.0 * (-dv * dv / 1.5).exp();
    let (hx, hy) = (cx - 3.0, cy - 11.0);
    h += 55.0 * (-(hx * hx + hy * hy) / 1.1).exp();
    let (lx, ly) = (cx - 12.0, cy - 3.0);
    h -= 18.0 * (-(lx * lx + ly * ly) / 1.4).exp();
    let s = cy - SEA_ROW as f64;
    if s > -0.5 {
        h -= 30.0 * (s + 0.5);
    }
    h
}

fn cell_at(x: i64, y: i64) -> Cell {
    let (mx, my) = (x as f64 * 100.0 + 50.0, y as f64 * 100.0 + 50.0);
    let h = height(mx, my);
    let e = 1.0;
    let gx = (height(mx + e, my) - height(mx - e, my)) / (2.0 * e);
    let gy = (height(mx, my + e) - height(mx, my - e)) / (2.0 * e);
    let slope = (gx * gx + gy * gy).sqrt().atan().to_degrees();
    let mut c = Cell {
        height: HeightMm::new((h * 1000.0).round() as i32),
        terrain: TerrainKind::Land,
        cover: Cover::Grass,
        slope_milli_deg: (slope * 1000.0).round() as u16,
        aspect_deg: 0,
        temperature: TempCentiC::new(1000),
        moisture: 120,
        forest_density: 20,
        wetness: 60,
        height_above_river_dm: 60,
        ..Cell::default()
    };
    // Forest north-west, thinning eastward across columns 5..8.
    if y < RIVER_ROW - 1 && x < 9 {
        c.cover = Cover::Forest;
        c.forest_density = if x < 5 {
            230
        } else {
            (230 - (x - 4) * 45).max(20) as u8
        };
    }
    if ROCK.contains(&(x, y)) {
        c.cover = Cover::Rock;
        c.forest_density = 0;
    }
    if MARSH.contains(&(x, y)) {
        c.cover = Cover::Marsh;
        c.wetness = 240;
        c.height_above_river_dm = 5;
    }
    if y == RIVER_ROW {
        c.watercourse_order = 3;
        c.watercourse_width_dm = 60 + 10 * x as u32;
        c.discharge = DischargeMilli::new(3_000 + 500 * x as u64);
        c.height_above_river_dm = 0;
        c.wetness = 200;
    }
    if y >= SEA_ROW {
        c.terrain = TerrainKind::Sea;
        c.cover = Cover::Bare;
    }
    c
}

/// The synthetic world for `seed`.
#[must_use]
pub fn world(seed: u64) -> GridSource {
    let mut g = GridSource::new(seed, W, W, Cell::default());
    for y in 0..i64::from(W) {
        for x in 0..i64::from(W) {
            g.set(CellKey::new(x, y), cell_at(x, y));
        }
    }
    for &(x, y) in &LAKE {
        g.set_lake(
            CellKey::new(x, y),
            LakeInfo {
                surface_mm: (LAKE_SURFACE_M * 1000.0) as i32,
                depth_mm: 6_000,
            },
        );
    }
    for x in 0..i64::from(W) - 1 {
        let w = 60 + 10 * x as u32;
        g.add_edge(Edge {
            from: CellKey::new(x, RIVER_ROW),
            to: CellKey::new(x + 1, RIVER_ROW),
            from_width_dm: w,
            to_width_dm: w + 10,
            discharge_milli: 3_000 + 500 * x as u64,
        });
    }
    // A tributary joining from the north at column 10.
    for y in 3..RIVER_ROW {
        g.add_edge(Edge {
            from: CellKey::new(10, y),
            to: CellKey::new(10, y + 1),
            from_width_dm: 20,
            to_width_dm: 25,
            discharge_milli: 400,
        });
        if let Some(c) = g.cell_mut(CellKey::new(10, y)) {
            c.watercourse_order = 1;
            c.watercourse_width_dm = 20;
            c.height_above_river_dm = 0;
        }
    }
    // Fine lattice: sample k sits at world metre 39.0625 k + 50 (the fine
    // layer's point registration, see `context.rs`).
    let n = 1 + (f64::from(W) * 100.0 / 39.0625) as u32 + 2;
    let mut s = Vec::new();
    for ky in 0..n {
        for kx in 0..n {
            let (x, y) = (
                39.0625 * f64::from(kx) + 50.0,
                39.0625 * f64::from(ky) + 50.0,
            );
            s.push((height(x, y) * 1000.0).round() as i32);
        }
    }
    g.set_fine(n, n, s);
    g
}

/// A cell on the main river.
pub const RIVER_CELL: CellKey = CellKey { x: 6, y: RIVER_ROW };
/// A dense forest cell.
pub const FOREST_CELL: CellKey = CellKey { x: 2, y: 2 };
/// A grassland cell.
pub const GRASS_CELL: CellKey = CellKey { x: 6, y: 10 };
