//! The refined rivers around a settlement, as its town plan reads them:
//! the centreline pieces arda-refine draws each block's channels from
//! ([`arda_refine::water::cell_channels`]) and the exact square test the
//! blocks rasterise river water with ([`RiverWater`]). A plan built on
//! them keeps its plots off the water the tactical map shows and bridges
//! it where it is, not on settle's straight centre-to-centre lines.

use crate::terrain::CELL_M;
use crate::PeopleError;
use arda_refine::rivers::Piece;
use arda_refine::water::{cell_channels, RiverWater};
use arda_refine::{CellKey, Source};
use arda_town::geom::{v2, Vec2};
use arda_town::plan::grid::SQUARE_M;
use arda_town::site::RiverLine;

/// The refined river pieces of every cell within `radius_m` of `at`
/// (clamped into the world), in cell order.
///
/// # Errors
/// [`PeopleError::World`] when a layer fails to load.
pub fn pieces_around(
    src: &dyn Source,
    (x_m, y_m): (f64, f64),
    radius_m: f64,
) -> Result<Vec<Piece>, PeopleError> {
    let (w, h) = src.cells_wide_high();
    #[allow(clippy::cast_possible_truncation)] // world metres fit i64
    let cell = |m: f64| (m / CELL_M).floor() as i64;
    let (x0, x1) = (
        cell(x_m - radius_m).clamp(0, w - 1),
        cell(x_m + radius_m).clamp(0, w - 1),
    );
    let (y0, y1) = (
        cell(y_m - radius_m).clamp(0, h - 1),
        cell(y_m + radius_m).clamp(0, h - 1),
    );
    let mut out = Vec::new();
    for gy in y0..=y1 {
        for gx in x0..=x1 {
            let pieces = cell_channels(src, CellKey::new(gx, gy))
                .map_err(|e| PeopleError::World(e.to_string()))?;
            out.extend(pieces);
        }
    }
    Ok(out)
}

/// A piece as an `arda-town` river line in world metres, as wide as its
/// widest point (the plan keeps that clear of streets).
#[must_use]
pub fn line(p: &Piece) -> RiverLine {
    let widest = p.half.iter().copied().fold(0.0_f64, f64::max);
    RiverLine {
        points: p
            .pts
            .iter()
            .map(|&(u, v)| v2(u * SQUARE_M, v * SQUARE_M))
            .collect(),
        width_m: 2.0 * widest * SQUARE_M,
    }
}

/// The pieces as river lines, the main river first: widest first, then
/// nearest to `at`, then by position (the planner reads the first line's
/// direction as downstream).
#[must_use]
pub fn lines(pieces: &[Piece], at: Vec2) -> Vec<RiverLine> {
    let mut out: Vec<RiverLine> = pieces
        .iter()
        .filter(|p| p.pts.len() >= 2)
        .map(line)
        .collect();
    let key = |r: &RiverLine| {
        let d = arda_town::geom::dist_to(&r.points, at);
        let first = r.points.first().copied().unwrap_or(at);
        (-r.width_m, d, first.x, first.y)
    };
    out.sort_by(|a, b| {
        let (ka, kb) = (key(a), key(b));
        ka.0.total_cmp(&kb.0)
            .then(ka.1.total_cmp(&kb.1))
            .then(ka.2.total_cmp(&kb.2))
            .then(ka.3.total_cmp(&kb.3))
    });
    out
}

/// Whether world point `p` lies in the refined river water.
#[must_use]
pub fn in_water(water: &RiverWater, p: Vec2) -> bool {
    water.contains((p.x / SQUARE_M, p.y / SQUARE_M))
}
