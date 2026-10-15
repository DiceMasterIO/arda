//! Cutting a sub-window out of a composed window (review 3.5-8, logic/09
//! §window-crop).
//!
//! Composition is done once over a window grown by a margin (and, in the
//! server, once over the image's apron window) and every smaller window is
//! cut from it, so the image and the scene of a cell come from one
//! composition and a cell's border edges are decided with both of their
//! squares in view. A cut keeps:
//!
//! - the squares, owners and rule cells of the sub-window;
//! - every wall and edge rule on an edge of the sub-window, its border
//!   edges included (a horizontal edge on rows `y0..=y0 + h`, a vertical
//!   one on columns `x0..=x0 + w`);
//! - every placement and light anchored in the sub-window (half-open, so
//!   each belongs to exactly one of two neighbouring cuts). Positions are
//!   on the 1/64-square lattice ([`snap`]), so the shift is exact.
//!
//! What a placement anchored outside the cut implies for the squares
//! inside it (a canopy overhanging a border, a boulder's footprint) is
//! already folded into the rule cells before the cut.

use arda_scene::RulesSidecar;
use arda_tactical::layout::{EdgeAxis, TacticalLayout};

/// A sub-window `[x, x + w) × [y, y + h)` in a layout's own squares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cut {
    /// First column.
    pub x: u32,
    /// First row.
    pub y: u32,
    /// Width in squares.
    pub w: u32,
    /// Height in squares.
    pub h: u32,
}

impl Cut {
    /// Whether `(x, y, axis)` is an edge of the cut, and its coordinates
    /// in the cut.
    #[must_use]
    pub fn edge(&self, x: u32, y: u32, axis: EdgeAxis) -> Option<(u32, u32)> {
        let (lx, ly) = (x.checked_sub(self.x)?, y.checked_sub(self.y)?);
        let ok = match axis {
            EdgeAxis::Horizontal => lx < self.w && ly <= self.h,
            EdgeAxis::Vertical => lx <= self.w && ly < self.h,
        };
        ok.then_some((lx, ly))
    }

    /// Whether a point in squares lies in the cut (half-open).
    #[must_use]
    pub fn holds(&self, px: f32, py: f32) -> bool {
        let (x0, y0) = (f64::from(self.x), f64::from(self.y));
        let (px, py) = (f64::from(px), f64::from(py));
        px >= x0 && py >= y0 && px < x0 + f64::from(self.w) && py < y0 + f64::from(self.h)
    }

    /// Row-major indices of the cut's squares in a `width`-wide window.
    pub fn indices(&self, width: u32) -> impl Iterator<Item = usize> + '_ {
        let width = width as usize;
        (self.y..self.y + self.h).flat_map(move |y| {
            (self.x..self.x + self.w).map(move |x| y as usize * width + x as usize)
        })
    }
}

/// Snaps a coordinate in squares to the 1/64-square lattice that `f32`
/// and JSON hold exactly, measured from the world origin so that every
/// window agrees: `origin + v` is snapped, then `origin` taken off.
#[must_use]
pub fn snap(origin: i64, v: f32) -> f32 {
    #[allow(clippy::cast_precision_loss)] // world squares are far below 2^52
    let world = origin as f64 + f64::from(v);
    #[allow(clippy::cast_precision_loss)]
    let local = (world * 64.0).round() / 64.0 - origin as f64;
    #[allow(clippy::cast_possible_truncation)] // a window is a few hundred squares
    let local = local as f32;
    local
}

/// Snaps every placement and light of `layout` (whose `origin` is its
/// world square, or zero) to the 1/64-square lattice.
pub fn snap_positions(layout: &mut TacticalLayout) {
    let [ox, oy] = layout.origin.unwrap_or([0, 0]);
    for p in &mut layout.placements {
        (p.x, p.y) = (snap(ox, p.x), snap(oy, p.y));
    }
    for l in &mut layout.lights {
        (l.x, l.y) = (snap(ox, l.x), snap(oy, l.y));
    }
}

/// The cut of `layout`, named `name`; its `origin` moves by the cut's
/// offset.
#[must_use]
pub fn layout(layout: &TacticalLayout, cut: Cut, name: &str) -> TacticalLayout {
    let squares = cut
        .indices(layout.width)
        .filter_map(|i| layout.squares.get(i).cloned())
        .collect();
    let walls = layout
        .walls
        .iter()
        .filter_map(|s| {
            let (x, y) = cut.edge(s.x, s.y, s.axis)?;
            let mut s = s.clone();
            (s.x, s.y) = (x, y);
            Some(s)
        })
        .collect();
    #[allow(clippy::cast_precision_loss)] // window offsets are small
    let (dx, dy) = (cut.x as f32, cut.y as f32);
    let placements = layout
        .placements
        .iter()
        .filter(|p| cut.holds(p.x, p.y))
        .map(|p| {
            let mut p = p.clone();
            (p.x, p.y) = (p.x - dx, p.y - dy);
            p
        })
        .collect();
    let lights = layout
        .lights
        .iter()
        .filter(|l| cut.holds(l.x, l.y))
        .map(|l| {
            let mut l = *l;
            (l.x, l.y) = (l.x - dx, l.y - dy);
            l
        })
        .collect();
    TacticalLayout {
        name: name.to_owned(),
        width: cut.w,
        height: cut.h,
        squares,
        walls,
        placements,
        lights,
        origin: layout
            .origin
            .map(|[x, y]| [x + i64::from(cut.x), y + i64::from(cut.y)]),
    }
}

/// The cut of `rules`.
#[must_use]
pub fn rules(rules: &RulesSidecar, cut: Cut) -> RulesSidecar {
    let mut out = RulesSidecar::empty(cut.w, cut.h);
    out.squares = cut
        .indices(rules.width)
        .filter_map(|i| rules.squares.get(i).cloned())
        .collect();
    out.edges = rules
        .edges
        .iter()
        .filter_map(|e| {
            let (x, y) = cut.edge(e.x, e.y, e.axis)?;
            let mut e = e.clone();
            (e.x, e.y) = (x, y);
            Some(e)
        })
        .collect();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_tactical::catalog::WallRole;
    use arda_tactical::layout::{AssetRef, Placement, WallSegment};

    fn wall(x: u32, y: u32, axis: EdgeAxis) -> WallSegment {
        WallSegment {
            x,
            y,
            axis,
            kind: WallRole::Run,
            kit: "stone".into(),
            tags: Vec::new(),
        }
    }

    #[test]
    fn a_cut_keeps_its_border_edges_and_its_own_anchors() {
        let mut l = TacticalLayout::new("t", 8, 8, "grass");
        l.origin = Some([100, 200]);
        l.walls = vec![
            wall(2, 2, EdgeAxis::Vertical),   // west border: kept
            wall(6, 3, EdgeAxis::Vertical),   // east border: kept
            wall(7, 3, EdgeAxis::Vertical),   // outside
            wall(3, 6, EdgeAxis::Horizontal), // south border: kept
            wall(1, 3, EdgeAxis::Horizontal), // outside
        ];
        let at = |x: f32, y: f32| Placement {
            asset: AssetRef::Id("prop.barrel".into()),
            x,
            y,
            rotation: 0,
            mirror: false,
        };
        l.placements = vec![at(2.0, 2.0), at(6.0, 3.5), at(5.5, 5.984375)];
        let cut = Cut {
            x: 2,
            y: 2,
            w: 4,
            h: 4,
        };
        let c = layout(&l, cut, "c");
        assert_eq!(c.origin, Some([102, 202]));
        assert_eq!((c.width, c.height, c.squares.len()), (4, 4, 16));
        let walls: Vec<_> = c.walls.iter().map(|w| (w.x, w.y, w.axis)).collect();
        assert_eq!(
            walls,
            vec![
                (0, 0, EdgeAxis::Vertical),
                (4, 1, EdgeAxis::Vertical),
                (1, 4, EdgeAxis::Horizontal)
            ]
        );
        let at: Vec<_> = c.placements.iter().map(|p| (p.x, p.y)).collect();
        assert_eq!(at, vec![(0.0, 0.0), (3.5, 3.984_375)]);
    }

    #[test]
    fn snapping_is_the_same_from_any_origin() {
        // 79679.99999 world squares: one window holds it at 24 - 1e-5,
        // another at -1e-5; both snap to the lattice point.
        assert_eq!(snap(79_656, 23.999_99), 24.0);
        assert_eq!(snap(79_680, -0.000_01), 0.0);
        assert_eq!(snap(0, 1.0 / 64.0), 1.0 / 64.0);
    }
}
