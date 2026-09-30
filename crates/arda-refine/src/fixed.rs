//! Squares decided before the WFC: water, cliffs and lake or sea shores
//! (goal 46), and the corner masks they impose. Everything here is a
//! function of the physical fields, which are global, so the masks on a
//! shared block edge are the same from both sides.

use crate::classes::{Class, Mask, LAND_MASK};
use crate::context::Ctx;
use crate::grid::Grid;
use crate::terrain::{bank_classes, Phys, Water};
use arda::Cover;

/// Slope above which dry ground is drawn as cliff, degrees.
pub const CLIFF_DEG: f64 = 48.0;

/// What a square is fixed to before the WFC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fixed {
    /// Open ground: the WFC decides.
    Open,
    /// Water of some kind; the ground key follows depth.
    Water,
    /// A cliff edge or face.
    Cliff,
    /// A lake or sea shore of the given class (beach, shingle or rock).
    Shore(Class),
}

/// A fixed square with its allowed corner classes.
#[derive(Debug, Clone, Copy)]
pub struct FixedSquare {
    /// The decision.
    pub fixed: Fixed,
    /// Classes its corners may take.
    pub mask: Mask,
}

/// Fraction of rocky cover around a point.
#[must_use]
pub fn rocky(ctx: &Ctx, u: f64, v: f64) -> f64 {
    ctx.bilinear(u, v, |c, _| match c.cover {
        Cover::Rock => 1.0,
        Cover::Ice => 0.5,
        _ => 0.0,
    })
}

/// Classifies one square.
#[must_use]
pub fn classify(ctx: &Ctx, x: i64, y: i64, p: &Phys) -> FixedSquare {
    let (u, v) = (x as f64 + 0.5, y as f64 + 0.5);
    let rock = rocky(ctx, u, v);
    if p.water != Water::Dry {
        return FixedSquare {
            fixed: Fixed::Water,
            mask: bank_classes(p, rock),
        };
    }
    if p.slope_deg > CLIFF_DEG {
        return FixedSquare {
            fixed: Fixed::Cliff,
            mask: Class::Cliff.bit() | Class::Rock.bit() | Class::Scree.bit(),
        };
    }
    if matches!(p.stand_kind, Water::Sea | Water::Lake) {
        let steep = p.slope_deg > 22.0 || rock > 0.5;
        let (class, band) = if steep {
            (Class::Rock, 0.1)
        } else if p.slope_deg > 9.0 {
            (Class::Gravel, 0.12)
        } else if p.stand_kind == Water::Sea {
            (Class::Sand, 0.3)
        } else {
            (Class::Sand, 0.14)
        };
        if p.stand_v > -band {
            return FixedSquare {
                fixed: Fixed::Shore(class),
                mask: class.bit() | Class::Water.bit(),
            };
        }
    }
    FixedSquare {
        fixed: Fixed::Open,
        mask: LAND_MASK,
    }
}

/// Corner masks for every vertex whose four squares lie in `squares`:
/// the intersection of the incident squares' masks, relaxing cliffs, then
/// shores, when they cannot agree.
#[must_use]
pub fn vertex_masks(squares: &Grid<FixedSquare>) -> Grid<Mask> {
    let mut out = Grid::new(
        squares.x0 + 1,
        squares.y0 + 1,
        squares.w - 1,
        squares.h - 1,
        LAND_MASK,
    );
    let coords: Vec<(i64, i64)> = out.coords().collect();
    for (i, (x, y)) in coords.into_iter().enumerate() {
        let around =
            [(x - 1, y - 1), (x, y - 1), (x - 1, y), (x, y)].map(|(a, b)| *squares.clamped(a, b));
        let meet = |skip: &dyn Fn(Fixed) -> bool| {
            around
                .iter()
                .filter(|s| !skip(s.fixed))
                .fold(crate::classes::ALL_MASK, |m, s| m & s.mask)
        };
        let mut m = meet(&|_| false);
        if m == 0 {
            m = meet(&|f| f == Fixed::Cliff);
        }
        if m == 0 {
            m = meet(&|f| matches!(f, Fixed::Cliff | Fixed::Shore(_)));
        }
        if m == 0 {
            m = meet(&|f| f != Fixed::Water);
        }
        out.data[i] = if m == 0 { LAND_MASK } else { m };
    }
    out
}
