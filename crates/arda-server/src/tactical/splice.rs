//! Pasting a client-edited cell into its apron block (1.0 task 04;
//! logic/16 §api-tactical, logic/11 §seam-art).
//!
//! `POST /v1/tactical/bake` renders an edited cell so its edges join the
//! unedited neighbours: the server takes the cell's apron block from the
//! block source, exactly as the cell route does, replaces its centre with
//! the posted layout and renders the apron window cropped to the cell.
//!
//! [`splice`]'s rules, in squares local to the apron, are those of the cut
//! the cell route sends (`arda_blocks::crop`, review 3.6-9), so whatever a
//! client edits in the body it got lands, and an unedited post reproduces
//! the apron exactly:
//!
//! - **Squares** inside the cell come from the post, the rest from the apron.
//! - **Walls** go by their edge: every edge of the cell, its four border
//!   edges included, comes from the post (a wall a client removes from
//!   the cell's border is gone, one it adds stands); other edges from the
//!   apron.
//! - **Placements and lights** go by their position, half-open as the cut:
//!   one on the cell's west or north border line is the cell's, one on its
//!   east or south line the neighbour's. Items anchored in the cell come
//!   from the post, the rest from the apron, so an item straddling the
//!   edge is drawn exactly once.
//! - Posted items the cell does not hold (a wall off its edges, a
//!   placement or light anchored outside it) cannot be baked into this
//!   cell: they are counted in the job's `ignored`, never dropped silently
//!   ([`ignored`]).
//! - **Order** is kept: each apron item in the cell is replaced, in turn,
//!   by the next posted item; spare posted items follow the apron's, and
//!   spare apron slots are dropped. An unedited post therefore reproduces
//!   the apron layout exactly, item order included.
//! - **Name, size and origin** are the apron's.

use super::bake_dto::BakeIgnored;
use crate::error::{ServerError, ServerResult};
use arda_blocks::Cut;
use arda_tactical::TacticalLayout;

/// Keeps `apron`'s items anchored outside the cell and replaces the inside
/// ones with `posted` (already in apron squares), in the documented order.
fn merge<T: Clone>(apron: &[T], posted: Vec<T>, inside: impl Fn(&T) -> bool) -> Vec<T> {
    let mut posted = posted.into_iter();
    let mut out = Vec::with_capacity(apron.len());
    for item in apron {
        if !inside(item) {
            out.push(item.clone());
        } else if let Some(p) = posted.next() {
            out.push(p);
        }
    }
    out.extend(posted);
    out
}

/// The posted items a bake of `cell` cannot place in it: walls off its
/// edges, placements and lights anchored outside it (review 3.6-9).
#[must_use]
pub fn ignored(cell: &TacticalLayout) -> BakeIgnored {
    let local = Cut {
        x: 0,
        y: 0,
        w: cell.width,
        h: cell.height,
    };
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    BakeIgnored {
        walls: count(
            cell.walls
                .iter()
                .filter(|w| local.edge(w.x, w.y, w.axis).is_none())
                .count(),
        ),
        placements: count(
            cell.placements
                .iter()
                .filter(|p| !local.holds(p.x, p.y))
                .count(),
        ),
        lights: count(
            cell.lights
                .iter()
                .filter(|l| !local.holds(l.x, l.y))
                .count(),
        ),
    }
}

/// `apron` with its `cell.width × cell.height` squares at `at` (apron
/// squares) and the edges and items of the cell replaced by `cell`'s.
///
/// # Errors
/// [`ServerError::InvalidLayout`] when `cell` does not fit inside `apron`
/// at `at`, or its square count does not match its size.
pub fn splice(
    apron: &TacticalLayout,
    cell: &TacticalLayout,
    at: (u32, u32),
) -> ServerResult<TacticalLayout> {
    let rect = Cut {
        x: at.0,
        y: at.1,
        w: cell.width,
        h: cell.height,
    };
    let fits = |o: u32, n: u32, max: u32| o.checked_add(n).is_some_and(|e| e <= max);
    if !fits(at.0, cell.width, apron.width) || !fits(at.1, cell.height, apron.height) {
        return Err(ServerError::InvalidLayout(format!(
            "a {}x{} cell at {},{} leaves the {}x{} apron",
            cell.width, cell.height, at.0, at.1, apron.width, apron.height
        )));
    }
    let count = |l: &TacticalLayout| l.width as usize * l.height as usize;
    if cell.squares.len() != count(cell) || apron.squares.len() != count(apron) {
        return Err(ServerError::InvalidLayout(
            "squares do not match the layout size".into(),
        ));
    }
    let mut out = apron.clone();
    let (aw, cw) = (apron.width as usize, cell.width as usize);
    for row in 0..cell.height as usize {
        let dst = (at.1 as usize + row) * aw + at.0 as usize;
        if let (Some(d), Some(s)) = (
            out.squares.get_mut(dst..dst + cw),
            cell.squares.get(row * cw..(row + 1) * cw),
        ) {
            d.clone_from_slice(s);
        }
    }
    let local = Cut { x: 0, y: 0, ..rect };
    #[allow(clippy::cast_precision_loss)] // window offsets are small
    let (dx, dy) = (at.0 as f32, at.1 as f32);
    let walls = cell
        .walls
        .iter()
        .filter(|w| local.edge(w.x, w.y, w.axis).is_some())
        .map(|w| {
            let mut w = w.clone();
            w.x += at.0;
            w.y += at.1;
            w
        })
        .collect();
    out.walls = merge(&apron.walls, walls, |w| {
        rect.edge(w.x, w.y, w.axis).is_some()
    });
    let placements = cell
        .placements
        .iter()
        .filter(|p| local.holds(p.x, p.y))
        .map(|p| {
            let mut p = p.clone();
            p.x += dx;
            p.y += dy;
            p
        })
        .collect();
    out.placements = merge(&apron.placements, placements, |p| rect.holds(p.x, p.y));
    let lights = cell
        .lights
        .iter()
        .filter(|l| local.holds(l.x, l.y))
        .map(|l| {
            let mut l = *l;
            l.x += dx;
            l.y += dy;
            l
        })
        .collect();
    out.lights = merge(&apron.lights, lights, |l| rect.holds(l.x, l.y));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_tactical::layout::{AssetRef, EdgeAxis, LightSource, Placement, WallSegment};
    use arda_tactical::WallRole;

    fn wall(x: u32, y: u32) -> WallSegment {
        WallSegment {
            x,
            y,
            axis: EdgeAxis::Vertical,
            kind: WallRole::Run,
            kit: "stone".into(),
            tags: Vec::new(),
        }
    }

    fn prop(id: &str, x: f32, y: f32) -> Placement {
        Placement {
            asset: AssetRef::Id(id.into()),
            x,
            y,
            rotation: 0,
            mirror: false,
        }
    }

    fn light(x: f32, y: f32) -> LightSource {
        LightSource {
            x,
            y,
            radius_ft: 20,
            colour: [255, 200, 120],
        }
    }

    /// A 10 × 8 apron with items inside and outside a 4 × 4 cell at (3, 2),
    /// interleaved so the inside ones are not contiguous.
    fn apron() -> TacticalLayout {
        let mut l = TacticalLayout::new("apron", 10, 8, "grass");
        for (i, s) in l.squares.iter_mut().enumerate() {
            s.elevation_ft = i16::try_from(i).unwrap();
        }
        l.origin = Some([100, 200]);
        l.walls = vec![wall(1, 1), wall(4, 3), wall(9, 7), wall(6, 5), wall(7, 2)];
        l.placements = vec![
            prop("a", 3.5, 2.5),
            prop("b", 0.5, 0.5),
            prop("c", 6.9, 5.1),
            prop("d", 7.0, 3.0),
            prop("e", 2.99, 4.0),
            prop("f", 3.0, 4.0),
        ];
        l.lights = vec![light(8.0, 1.0), light(4.25, 4.75)];
        l
    }

    /// The `w × h` cut of `apron` at `at`, as the cell route sends it.
    fn cut(apron: &TacticalLayout, at: (u32, u32), (w, h): (u32, u32)) -> TacticalLayout {
        let c = Cut {
            x: at.0,
            y: at.1,
            w,
            h,
        };
        arda_blocks::crop::layout(apron, c, "cell")
    }

    #[test]
    fn splicing_the_aprons_own_centre_reproduces_it_exactly() {
        let a = apron();
        let c = cut(&a, (3, 2), (4, 4));
        // The east border edge (7, 2) and the prop on the west border line
        // (3, 4) are the cell's (the cut's rules).
        assert_eq!(c.walls.len(), 3);
        assert_eq!(c.placements.len(), 3);
        assert_eq!(c.lights.len(), 1);
        assert_eq!(splice(&a, &c, (3, 2)).unwrap(), a);
        // Every position, including the corners of the apron.
        for at in [(0, 0), (6, 4), (2, 1)] {
            let c = cut(&a, at, (4, 4));
            assert_eq!(splice(&a, &c, at).unwrap(), a, "{at:?}");
        }
    }

    #[test]
    fn edited_squares_land_where_expected() {
        let a = apron();
        let mut c = cut(&a, (3, 2), (4, 4));
        c.square_mut(1, 2).unwrap().ground = "water".into();
        let out = splice(&a, &c, (3, 2)).unwrap();
        for y in 0..8 {
            for x in 0..10 {
                let want = if (x, y) == (4, 4) { "water" } else { "grass" };
                assert_eq!(out.square(x, y).ground, want, "{x},{y}");
            }
        }
        assert_eq!(
            out.square(4, 4).elevation_ft,
            44,
            "the posted square is kept"
        );
        assert_eq!((out.name.as_str(), out.origin), ("apron", Some([100, 200])));
    }

    #[test]
    fn straddling_items_come_from_exactly_one_source() {
        let a = apron();
        let mut c = cut(&a, (3, 2), (4, 4));
        // Posted items anchored outside the cell are ignored and counted:
        // the apron's own copy of a prop just past the east edge stays,
        // once.
        c.placements.push(prop("d", 4.0, 1.0));
        c.walls.push(wall(5, 0));
        // A new prop straddling the west edge, anchored inside.
        c.placements.push(prop("tree", 0.5, 1.0));
        // A posted prop on the west border line is the cell's.
        c.placements.push(prop("g", 0.0, 2.0));
        // Remove the inside light; add none.
        c.lights.clear();
        assert_eq!(
            ignored(&c),
            BakeIgnored {
                walls: 1,
                placements: 1,
                lights: 0
            }
        );
        let out = splice(&a, &c, (3, 2)).unwrap();
        let ids: Vec<_> = out
            .placements
            .iter()
            .map(|p| match &p.asset {
                AssetRef::Id(id) => (id.as_str(), p.x, p.y),
                AssetRef::Query { .. } => ("?", p.x, p.y),
            })
            .collect();
        assert_eq!(
            ids,
            vec![
                ("a", 3.5, 2.5),
                ("b", 0.5, 0.5),
                ("c", 6.9, 5.1),
                ("d", 7.0, 3.0),
                ("e", 2.99, 4.0),
                ("f", 3.0, 4.0),
                ("tree", 3.5, 3.0),
                ("g", 3.0, 4.0),
            ]
        );
        assert_eq!(out.walls, a.walls);
        assert_eq!(out.lights, vec![light(8.0, 1.0)]);
    }

    /// Review 3.6-9: an edit on the cell's border is baked, not dropped.
    #[test]
    fn border_edits_are_kept() {
        let a = apron();
        let mut c = cut(&a, (3, 2), (4, 4));
        // Take the wall off the east border edge, add one on the south.
        c.walls.retain(|w| (w.x, w.y) != (4, 0));
        c.walls.push(WallSegment {
            axis: EdgeAxis::Horizontal,
            ..wall(2, 4)
        });
        assert_eq!(ignored(&c), BakeIgnored::default());
        let out = splice(&a, &c, (3, 2)).unwrap();
        let edges: Vec<_> = out.walls.iter().map(|w| (w.x, w.y, w.axis)).collect();
        assert!(!edges.contains(&(7, 2, EdgeAxis::Vertical)), "{edges:?}");
        assert!(edges.contains(&(5, 6, EdgeAxis::Horizontal)), "{edges:?}");
        assert_eq!(out.walls.len(), a.walls.len());
    }

    #[test]
    fn cells_that_leave_the_apron_are_refused() {
        let a = apron();
        let c = TacticalLayout::new("cell", 4, 4, "grass");
        assert!(matches!(
            splice(&a, &c, (7, 0)),
            Err(ServerError::InvalidLayout(_))
        ));
        let mut short = c.clone();
        short.squares.pop();
        assert!(matches!(
            splice(&a, &short, (0, 0)),
            Err(ServerError::InvalidLayout(_))
        ));
    }
}
