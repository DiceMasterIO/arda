//! Landform at 5-ft scale: hollows and knolls from the curvature of the
//! refined land surface, and the foot of steep ground where fallen rock
//! collects (goals 19, 43). Both read only the physical grid, which is a
//! function of global position, so neighbouring blocks agree wherever their
//! grids overlap.

use crate::grid::Grid;
use crate::noise::smoothstep;
use crate::terrain::Phys;

/// Squares the shape reads beyond a point: a physical grid must reach this
/// far past every point whose shape is asked for.
pub const REACH: i64 = 5;

/// The landform at one square.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Shape {
    /// Curvature index: positive in hollows and swales, negative on knolls
    /// and ridges, within `-1..=1`.
    pub hollow: f64,
    /// How strongly the square lies at the foot of steep ground, `0..=1`:
    /// a cliff or crag rises within a few squares while the square itself is
    /// gentler, so rock falling from above comes to rest here.
    pub talus: f64,
}

impl Shape {
    /// The ridge share, `0..=1`.
    #[must_use]
    pub fn ridge(&self) -> f64 {
        (-self.hollow).max(0.0)
    }

    /// The hollow share, `0..=1`.
    #[must_use]
    pub fn bowl(&self) -> f64 {
        self.hollow.max(0.0)
    }
}

/// Mean land height on a ring of radius `r` around `(x, y)` minus the
/// height at the point, metres.
fn ring(phys: &Grid<Phys>, x: i64, y: i64, r: i64) -> f64 {
    let d = (r * 7 + 5) / 10;
    let pts = [
        (r, 0),
        (-r, 0),
        (0, r),
        (0, -r),
        (d, d),
        (-d, d),
        (d, -d),
        (-d, -d),
    ];
    let here = phys.clamped(x, y).land_m;
    pts.iter()
        .map(|&(dx, dy)| phys.clamped(x + dx, y + dy).land_m)
        .sum::<f64>()
        / 8.0
        - here
}

/// The shape at square `(x, y)`; `phys` must cover [`REACH`] squares
/// around it.
#[must_use]
pub fn shape_at(phys: &Grid<Phys>, x: i64, y: i64) -> Shape {
    // Two scales: a swale a few squares across, and the broader knoll or
    // hollow it sits in. Curvature in metres over the ring radius.
    let c = 0.6 * ring(phys, x, y, 2) / 0.1 + 0.4 * ring(phys, x, y, REACH) / 0.25;
    let hollow = c.clamp(-1.0, 1.0);
    let own = phys.clamped(x, y);
    let mut steep: f64 = 0.0;
    for dy in -3..=3_i64 {
        for dx in -3..=3_i64 {
            if dx * dx + dy * dy > 10 || (dx == 0 && dy == 0) {
                continue;
            }
            let o = phys.clamped(x + dx, y + dy);
            if o.land_m > own.land_m + 0.5 {
                steep = steep.max(o.slope_deg);
            }
        }
    }
    let talus = smoothstep(32.0, 46.0, steep) * (1.0 - smoothstep(30.0, 42.0, own.slope_deg));
    Shape { hollow, talus }
}

/// Shapes over the square window `[x0, x0 + side)²`.
#[must_use]
pub fn shapes(phys: &Grid<Phys>, x0: i64, y0: i64, side: usize) -> Grid<Shape> {
    let mut g = Grid::new(x0, y0, side, side, Shape::default());
    for (i, (x, y)) in g.coords().collect::<Vec<_>>().into_iter().enumerate() {
        g.data[i] = shape_at(phys, x, y);
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bowl_grid(sign: f64) -> Grid<Phys> {
        let mut g = Grid::new(-10, -10, 21, 21, Phys::default());
        for (i, (x, y)) in g.coords().collect::<Vec<_>>().into_iter().enumerate() {
            let r2 = (x * x + y * y) as f64;
            g.data[i].land_m = sign * 0.05 * r2;
        }
        g
    }

    #[test]
    fn a_bowl_is_a_hollow_and_a_dome_a_ridge() {
        assert!(shape_at(&bowl_grid(1.0), 0, 0).hollow > 0.5);
        assert!(shape_at(&bowl_grid(-1.0), 0, 0).hollow < -0.5);
        let flat = Grid::new(-10, -10, 21, 21, Phys::default());
        assert!(shape_at(&flat, 0, 0).hollow.abs() < 1e-9);
    }

    #[test]
    fn the_foot_of_a_crag_is_talus() {
        let mut g = Grid::new(-10, -10, 21, 21, Phys::default());
        for (i, (x, _)) in g.coords().collect::<Vec<_>>().into_iter().enumerate() {
            if x > 1 {
                g.data[i].land_m = 10.0;
                g.data[i].slope_deg = 60.0;
            }
        }
        assert!(shape_at(&g, 0, 0).talus > 0.9);
        assert!(shape_at(&g, -8, 0).talus < 1e-9);
    }
}
