use super::*;
use crate::layout::TAG_NATURAL;

fn seg(x: u32, y: u32, axis: EdgeAxis, natural: bool) -> WallSegment {
    WallSegment {
        x,
        y,
        axis,
        kind: WallRole::Run,
        kit: "cave".into(),
        tags: if natural {
            vec![TAG_NATURAL.into()]
        } else {
            Vec::new()
        },
    }
}

/// A staircase pocket: the boundary of the squares on or below the
/// diagonal of a 4 × 4 block.
fn stair_layout(natural: bool) -> TacticalLayout {
    let mut l = TacticalLayout::new("stairs", 6, 6, "cave_floor");
    let inside = |x: i64, y: i64| (1..5).contains(&x) && (1..5).contains(&y) && x <= y;
    for y in 0..=6_i64 {
        for x in 0..=6_i64 {
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            let (ux, uy) = (x as u32, y as u32);
            if x < 6 && inside(x, y) != inside(x, y - 1) {
                l.walls.push(seg(ux, uy, EdgeAxis::Horizontal, natural));
            }
            if y < 6 && inside(x, y) != inside(x - 1, y) {
                l.walls.push(seg(ux, uy, EdgeAxis::Vertical, natural));
            }
        }
    }
    l
}

/// Largest turn between consecutive segments of a closed outline, as the
/// cosine of the angle (1 is straight on).
fn sharpest(pts: &[(f32, f32)]) -> f32 {
    let n = pts.len();
    (0..n)
        .map(|i| {
            let (a, b, c) = (pts[i], pts[(i + 1) % n], pts[(i + 2) % n]);
            let (u, v) = ((b.0 - a.0, b.1 - a.1), (c.0 - b.0, c.1 - b.1));
            let (lu, lv) = (u.0.hypot(u.1), v.0.hypot(v.1));
            if lu == 0.0 || lv == 0.0 {
                1.0
            } else {
                (u.0 * v.0 + u.1 * v.1) / (lu * lv)
            }
        })
        .fold(1.0, f32::min)
}

#[test]
fn a_staircase_traces_to_one_closed_chain() {
    let l = stair_layout(true);
    let edges = super::super::walls::natural_edges(&l);
    let cs = chains(&edges);
    assert_eq!(cs.len(), 1);
    assert!(cs[0].closed);
    assert_eq!(cs[0].vertices.len(), edges.len());
    assert_eq!(cs[0].kit, "cave");
}

#[test]
fn the_outline_has_no_stair_steps_and_stays_near_the_edges() {
    let l = stair_layout(true);
    let c = &chains(&super::super::walls::natural_edges(&l))[0];
    let pts = outline(c, 7, (0, 0));
    // No right-angle steps: every turn is gentler than 45 degrees.
    assert!(sharpest(&pts) > 0.7, "cos {}", sharpest(&pts));
    // Within 0.6 squares of a lattice vertex of the chain.
    for p in &pts {
        #[allow(clippy::cast_precision_loss)]
        let d = c
            .vertices
            .iter()
            .map(|&(x, y)| (p.0 - x as f32).hypot(p.1 - y as f32))
            .fold(f32::INFINITY, f32::min);
        assert!(d < 0.75, "{p:?} is {d} from the chain");
    }
    // Deterministic.
    assert_eq!(pts, outline(c, 7, (0, 0)));
}

#[test]
fn natural_edges_take_no_kit_pieces_and_plain_ones_still_do() {
    let natural = stair_layout(true);
    let (edges, joints) = super::super::walls::assemble(&natural);
    assert!(edges.is_empty() && joints.is_empty());
    let plain = stair_layout(false);
    let (edges, joints) = super::super::walls::assemble(&plain);
    assert_eq!(edges.len(), plain.walls.len());
    assert!(!joints.is_empty());
    assert!(super::super::walls::natural_edges(&plain).is_empty());
}

#[test]
fn an_open_chain_keeps_its_ends() {
    // A wall from the map's top edge down and across to its right edge.
    let mut l = TacticalLayout::new("open", 4, 4, "cave_floor");
    for y in 0..2 {
        l.walls.push(seg(2, y, EdgeAxis::Vertical, true));
    }
    for x in 2..4 {
        l.walls.push(seg(x, 2, EdgeAxis::Horizontal, true));
    }
    let cs = chains(&super::super::walls::natural_edges(&l));
    assert_eq!(cs.len(), 1);
    assert!(!cs[0].closed);
    let pts = outline(&cs[0], 3, (10, 10));
    let ends = [cs[0].vertices[0], *cs[0].vertices.last().unwrap()];
    #[allow(clippy::cast_precision_loss)]
    let want = ends.map(|(x, y)| (x as f32, y as f32));
    assert_eq!([pts[0], *pts.last().unwrap()], want);
}

#[test]
fn the_even_odd_fill_lies_between_a_line_and_its_twin() {
    // A unit square against a diamond through its edge midpoints: the
    // fill is the four corner triangles.
    let sq = vec![(0.0, 0.0), (8.0, 0.0), (8.0, 8.0), (0.0, 8.0), (0.0, 0.0)];
    let di = vec![(4.0, 0.0), (8.0, 4.0), (4.0, 8.0), (0.0, 4.0), (4.0, 0.0)];
    let px = even_odd(&[sq, di], [0, 0, 8, 8]);
    assert!(px.contains(&(0, 0)) && px.contains(&(7, 7)));
    assert!(!px.contains(&(4, 4)));
    assert_eq!(px.len(), 32);
}
