//! The fixed global block lattice and the gentle warp of the partition.
//!
//! Land blocks are the first level of the field hierarchy: a lattice of
//! tiles [`TILE`] squares across, with the tiles of each row paired into
//! blocks two tiles wide and alternate rows offset by one tile, like
//! bricks, so every block corner is a T-junction (a vertical edge meets
//! each row line from one side only). The vertices are displaced by a
//! smooth 2 km noise field (which turns, shears and stretches the
//! brickwork from place to place, so the blocks' lanes and parish edges
//! run every way) and jittered by a hash of their own coordinates. A
//! block is a function of its two indices and the seed alone, which is
//! what makes every window agree.
//!
//! The whole partition is drawn in warped coordinates `w = p + δ(p)`: a
//! smooth displacement of a few squares (rotated value noise at 150 and 60
//! squares, logic/17 lesson: never axis-aligned noise), so hedges wander
//! a little like real ones. Old (ancient) enclosure wanders more than
//! planned enclosure; [`ancient`] says which one a place belongs to.

use super::poly::{contains, P};
use crate::geom::{h2, s11};

/// Tile side, squares (500 m); a block is two tiles wide.
pub const TILE: f64 = 320.0;
/// Vertex jitter as a share of a tile.
const JITTER: f64 = 0.17;
/// Amplitude of the smooth vertex displacement, squares.
const DRIFT: f64 = 0.6 * TILE;
/// Wavelength of the smooth vertex displacement, squares (2 km).
const DRIFT_WAVE: f64 = 4.0 * TILE;
/// Tiles searched around a point for its block.
const SEARCH: i64 = 3;
/// Largest warp displacement, squares.
pub const WARP_MAX: f64 = 5.7;
/// Largest distance between two points of one block, squares (2 km;
/// checked by sampling in the tests).
pub const BLOCK_SPAN: f64 = 1_280.0;

/// A block's global key: pair index along its row, and the row.
pub type BlockId = (i64, i64);

/// A smooth value-noise field in `[-1, 1]` at `wavelength` squares,
/// sampled in a frame turned by the (3, 4, 5) rotation.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // floors of map positions
pub fn vnoise(seed: u64, salt: u64, p: P, wavelength: f64) -> f64 {
    let x = (0.8 * p[0] - 0.6 * p[1]) / wavelength;
    let y = (0.6 * p[0] + 0.8 * p[1]) / wavelength;
    let (ix, iy) = (x.floor(), y.floor());
    let (fx, fy) = (x - ix, y - iy);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let (ix, iy) = (ix as i64, iy as i64);
    let v = |dx: i64, dy: i64| s11(h2(seed, salt, ix + dx, iy + dy));
    let top = v(0, 0) + (v(1, 0) - v(0, 0)) * sx;
    let bot = v(0, 1) + (v(1, 1) - v(0, 1)) * sx;
    top + (bot - top) * sy
}

fn smoothstep(lo: f64, hi: f64, v: f64) -> f64 {
    let t = ((v - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How much a place belongs to old, piecemeal enclosure (`1`) rather than
/// planned enclosure (`0`): a 4 km noise field shifted by the culture's
/// bias (`bias > 0` favours old enclosure).
#[must_use]
pub fn ancient(seed: u64, p: P, bias: f64) -> f64 {
    smoothstep(-0.45, -0.05, vnoise(seed, 0xA9C1, p, 2_600.0) + bias)
}

/// The warp displacement at a real point (squares).
#[must_use]
pub fn displacement(seed: u64, p: P, bias: f64) -> P {
    let amp = 4.0 * (0.35 + 0.65 * ancient(seed, p, bias));
    let n =
        |salt: u64| 0.75 * vnoise(seed, salt, p, 150.0) + 0.25 * vnoise(seed, salt ^ 0x55, p, 60.0);
    [amp * n(0x3A21), amp * n(0x3A22)]
}

/// The warped position of a real point.
#[must_use]
pub fn warp(seed: u64, p: P, bias: f64) -> P {
    let d = displacement(seed, p, bias);
    [p[0] + d[0], p[1] + d[1]]
}

/// A jittered lattice vertex.
#[must_use]
#[allow(clippy::cast_precision_loss)] // lattice indices are small
pub fn vertex(seed: u64, i: i64, j: i64) -> P {
    let base = [i as f64 * TILE, j as f64 * TILE];
    [
        base[0]
            + DRIFT * vnoise(seed, 0xB1A0, base, DRIFT_WAVE)
            + JITTER * TILE * s11(h2(seed, 0xB10C, i, j)),
        base[1]
            + DRIFT * vnoise(seed, 0xB1A1, base, DRIFT_WAVE)
            + JITTER * TILE * s11(h2(seed, 0xB10D, i, j)),
    ]
}

/// The block holding tile `(i, j)`.
#[must_use]
pub fn block_of_tile(i: i64, j: i64) -> BlockId {
    ((i - j.rem_euclid(2)).div_euclid(2), j)
}

/// A block's outline: six vertices, clockwise on the map (y south).
#[must_use]
pub fn outline(seed: u64, (b, j): BlockId) -> Vec<P> {
    let i = 2 * b + j.rem_euclid(2);
    vec![
        vertex(seed, i, j),
        vertex(seed, i + 1, j),
        vertex(seed, i + 2, j),
        vertex(seed, i + 2, j + 1),
        vertex(seed, i + 1, j + 1),
        vertex(seed, i, j + 1),
    ]
}

/// Candidate blocks around a warped point, in a fixed order.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // floors of map positions
pub fn candidates(w: P) -> Vec<BlockId> {
    let (ti, tj) = ((w[0] / TILE).floor() as i64, (w[1] / TILE).floor() as i64);
    let mut out = Vec::with_capacity(32);
    for j in tj - SEARCH..=tj + SEARCH {
        for i in ti - SEARCH..=ti + SEARCH {
            let b = block_of_tile(i, j);
            if !out.contains(&b) {
                out.push(b);
            }
        }
    }
    out
}

/// The block holding a warped point.
#[must_use]
pub fn block_at(seed: u64, w: P) -> Option<BlockId> {
    candidates(w)
        .into_iter()
        .find(|&b| contains(&outline(seed, b), w))
}

/// Every block whose outline meets the warped rectangle `[x0, y0, x1, y1]`
/// (by bounding box), in row-major order.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // floors of map positions
pub fn blocks_meeting(seed: u64, r: [f64; 4]) -> Vec<BlockId> {
    let (i0, i1) = (
        (r[0] / TILE).floor() as i64 - SEARCH - 1,
        (r[2] / TILE).floor() as i64 + SEARCH + 1,
    );
    let (j0, j1) = (
        (r[1] / TILE).floor() as i64 - SEARCH,
        (r[3] / TILE).floor() as i64 + SEARCH,
    );
    let mut out = Vec::new();
    for j in j0..=j1 {
        for b in block_of_tile(i0, j).0..=block_of_tile(i1, j).0 {
            let o = outline(seed, (b, j));
            let meets = o.iter().any(|p| p[0] >= r[0]) && o.iter().any(|p| p[0] <= r[2]);
            let meets = meets && o.iter().any(|p| p[1] >= r[1]) && o.iter().any(|p| p[1] <= r[3]);
            if meets {
                out.push((b, j));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_point_lies_in_exactly_one_block() {
        for k in 0..4000_i64 {
            #[allow(clippy::cast_precision_loss)]
            let w = [
                (k * 37 % 3001) as f64 - 1500.3,
                (k * 53 % 2003) as f64 - 1000.7,
            ];
            let n = candidates(w)
                .into_iter()
                .filter(|&b| contains(&outline(9, b), w))
                .count();
            assert_eq!(n, 1, "{w:?}");
        }
    }

    #[test]
    fn blocks_stay_within_their_span() {
        for k in 0..400_i64 {
            let id = (k % 23 - 11, k / 23 - 8);
            let o = outline(5, id);
            for a in &o {
                for b in &o {
                    assert!((a[0] - b[0]).hypot(a[1] - b[1]) <= BLOCK_SPAN, "{id:?}");
                }
            }
        }
    }

    #[test]
    fn blocks_tile_like_bricks() {
        // The middle vertex of a block's top edge is a corner of the row
        // above: a T-junction.
        let above = outline(3, (0, -1));
        let here = outline(3, (0, 0));
        assert!(above[3..].contains(&here[1]) || above[3..].contains(&here[0]));
        let d = displacement(3, [100.0, 200.0], 0.0);
        assert!(d[0].abs() <= WARP_MAX && d[1].abs() <= WARP_MAX);
    }
}
