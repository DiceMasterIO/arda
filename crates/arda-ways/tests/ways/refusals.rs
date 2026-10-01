//! Inputs refused before any work: off-lattice and far origins, absurd widths.
use super::*;

#[test]
fn an_off_lattice_origin_is_refused() {
    let (roads, crossings, t) = bridge_scene();
    let mut l = TacticalLayout::new("t", 8, 8, "grass");
    let r = arda_ways::apply_ways(&mut l, [1.0, 0.0], &roads, &crossings, &t, 1);
    assert!(matches!(r, Err(WaysError::UnalignedOrigin { .. })));
}

#[test]
fn an_absurd_crossing_width_is_refused_not_scanned() {
    // A u32::MAX-metre crossing made a 1e10-square scan (a hang).
    let roads = vec![road(1, RoadClass::Road, 90, &[[-500, 37], [600, 37]])];
    let c = vec![crossing(
        1,
        CrossingKind::Ferry,
        [50, 37],
        u32::MAX,
        RoadClass::Road,
    )];
    let t = terrain(flat, Vec::new());
    let mut l = TacticalLayout::new("test", 16, 16, "grass");
    let r = arda_ways::apply_ways(&mut l, [0.0, 0.0], &roads, &c, &t, 1);
    assert!(matches!(r, Err(WaysError::Limit(_))), "{r:?}");
}

#[test]
fn a_far_origin_is_refused_not_overflowed() {
    // On the lattice and finite, but i64 square arithmetic saturated.
    let t = terrain(flat, Vec::new());
    let mut l = TacticalLayout::new("test", 8, 8, "grass");
    let far = 1.6e19 * SQUARE_M;
    let r = arda_ways::apply_ways(&mut l, [far, 0.0], &[], &[], &t, 1);
    assert!(r.is_err(), "{r:?}");
}
