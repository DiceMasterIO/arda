//! SRD movement, cover, light and sidecar rules.
#![allow(clippy::unwrap_used, missing_docs)]

mod common;

use arda_scene::{build_scene, CoverLevel, Movement, Obscurement, RulesSidecar, SceneError, Sq};
use arda_tactical::catalog::WallRole;
use common::{lib, L};

#[test]
fn difficult_terrain_costs_double() {
    // veg.bush is difficult terrain in the placeholder library.
    let s = L::new(6, 1)
        .put("veg.bush", 2, 0)
        .put("veg.bush", 3, 0)
        .scene();
    assert_eq!(s.movement_at(Sq(2, 0)), Movement::Difficult);
    assert_eq!(s.path_cost(Sq(0, 0), Sq(5, 0)), Some(5 + 10 + 10 + 5 + 5));
    // Leaving difficult terrain is free; only entering costs extra.
    assert_eq!(s.path_cost(Sq(2, 0), Sq(1, 0)), Some(5));
}

#[test]
fn water_is_waded_or_swum_and_both_cost_double() {
    let s = L::new(4, 1).water(1, 0, 3).water(2, 0, 5).scene();
    assert_eq!(s.movement_at(Sq(1, 0)), Movement::Wade);
    assert_eq!(s.movement_at(Sq(2, 0)), Movement::Swim);
    assert_eq!(s.path_cost(Sq(0, 0), Sq(3, 0)), Some(10 + 10 + 5));
}

#[test]
fn impassable_props_are_routed_around() {
    let s = L::new(3, 3).put("prop.barrel", 1, 1).scene();
    assert_eq!(s.movement_at(Sq(1, 1)), Movement::Impassable);
    // Around the barrel: two diagonals.
    assert_eq!(s.path_cost(Sq(0, 1), Sq(2, 1)), Some(10));
    assert_eq!(s.path_cost(Sq(0, 0), Sq(1, 1)), None);
}

#[test]
fn every_diagonal_costs_five_feet() {
    // SRD default: a diagonal step is one square. No 5/10 alternation.
    let s = L::new(6, 6).scene();
    let q = s.queries();
    assert_eq!(q.path_cost(Sq(0, 0), Sq(3, 3)), Some(15));
    assert_eq!(q.path_cost(Sq(0, 0), Sq(5, 5)), Some(25));
    assert_eq!(q.path_cost(Sq(0, 0), Sq(5, 2)), Some(25));
    assert_eq!(q.distance_ft(Sq(0, 0), Sq(3, 3)), 15);
    assert_eq!(q.distance_ft(Sq(1, 4), Sq(5, 2)), 20);
}

#[test]
fn diagonals_may_not_cut_wall_corners() {
    let s = L::new(3, 3).v(1, 0, WallRole::Run).scene();
    // (0,0) -> (1,1) would cut the vertex at (1,1) the wall touches.
    assert_eq!(s.path_cost(Sq(0, 0), Sq(1, 1)), Some(10));
}

#[test]
fn ten_foot_steps_need_climbing() {
    let s = L::new(3, 1).elevation(1, 0, 10).elevation(2, 0, 15).scene();
    assert_eq!(s.climb.0[0], 1 << 2);
    assert_eq!(s.climb.0[1], 1 << 6);
    assert_eq!(s.climb.0[2], 0);
    assert_eq!(s.path_cost(Sq(0, 0), Sq(1, 0)), Some(10));
    assert_eq!(s.path_cost(Sq(1, 0), Sq(2, 0)), Some(5));
}

#[test]
fn cover_levels_follow_the_obstacle() {
    let open = L::new(8, 3).scene();
    assert_eq!(open.cover_between(Sq(0, 1), Sq(5, 1)), CoverLevel::None);
    // prop.crate grants half cover, veg.boulder three-quarters.
    let crate_ = L::new(8, 3).put("prop.crate", 4, 1).scene();
    assert_eq!(crate_.cover_at(Sq(4, 1)), CoverLevel::Half);
    assert_eq!(crate_.cover_between(Sq(0, 1), Sq(5, 1)), CoverLevel::Half);
    // The full library's boulder also blocks sight; blocking sight grants
    // no cover of its own, so the boulder's three-quarters stands.
    let boulder = L::new(8, 3).put("veg.boulder", 4, 1).scene();
    assert_eq!(boulder.cover_at(Sq(4, 1)), CoverLevel::ThreeQuarters);
    assert_eq!(
        boulder.cover_between(Sq(0, 1), Sq(5, 1)),
        CoverLevel::ThreeQuarters
    );
    let mut l = L::new(8, 3);
    for y in 0..3 {
        l = l.v(5, y, WallRole::Run);
    }
    assert_eq!(
        l.scene().cover_between(Sq(0, 1), Sq(5, 1)),
        CoverLevel::Total
    );
    // A creature's own square gives it no cover from what stands in it.
    assert_eq!(crate_.cover_between(Sq(0, 1), Sq(4, 1)), CoverLevel::None);
}

#[test]
fn a_wall_end_gives_partial_cover() {
    // Wall on x = 3 for y 0..2; the target at (3, 2) peeks past its end.
    // From the best attacker corner one of four lines is blocked: half.
    let s = L::new(6, 5)
        .v(3, 0, WallRole::Run)
        .v(3, 1, WallRole::Run)
        .scene();
    assert_eq!(s.cover_between(Sq(0, 0), Sq(3, 2)), CoverLevel::Half);
    assert_eq!(s.cover_between(Sq(0, 0), Sq(3, 0)), CoverLevel::Total);
}

#[test]
fn only_the_most_protective_cover_applies() {
    let mut l = L::new(8, 3);
    for y in 0..3 {
        l = l.v(5, y, WallRole::Run);
    }
    // A crate (half) in front of a window (three-quarters).
    let s = l.v(5, 1, WallRole::Window).put("prop.crate", 3, 1).scene();
    assert_eq!(
        s.cover_between(Sq(0, 1), Sq(5, 1)),
        CoverLevel::ThreeQuarters
    );
}

#[test]
fn lights_carry_srd_bright_and_dim_radii() {
    let s = L::new(6, 6)
        .light(1.0, 1.0, 15)
        .put("prop.brazier", 4, 4)
        .scene();
    assert_eq!(s.lights.len(), 2);
    let (free, brazier) = (&s.lights[0], &s.lights[1]);
    assert_eq!(
        (free.bright_ft, free.dim_ft, free.asset.as_deref()),
        (15, 30, None)
    );
    assert_eq!(brazier.asset.as_deref(), Some("prop.brazier"));
    assert_eq!((brazier.bright_ft, brazier.dim_ft), (20, 40));
    assert_eq!((brazier.x, brazier.y), (4.5, 4.5));
}

#[test]
fn the_sidecar_overrides_and_merges() {
    let l = L::new(6, 3)
        .put("veg.bush", 0, 0)
        .put("veg.boulder", 1, 0)
        .put("prop.crate", 2, 0);
    let mut r = RulesSidecar::empty(6, 3);
    let mut set = |x, y, f: &dyn Fn(&mut arda_scene::RulesCell)| f(r.cell_mut(x, y).unwrap());
    set(0, 0, &|c| c.difficult = Some(false));
    set(1, 0, &|c| c.cover = Some(CoverLevel::Half));
    set(2, 0, &|c| c.cover = Some(CoverLevel::Total));
    set(3, 0, &|c| c.difficult = Some(true));
    set(4, 0, &|c| c.water_depth_ft = Some(6));
    set(5, 0, &|c| c.blocks_movement = Some(true));
    set(2, 0, &|c| c.blocks_movement = Some(false));
    set(3, 1, &|c| c.blocks_sight = Some(true));
    let s = l.scene_with(&r);
    assert_eq!(s.movement_at(Sq(0, 0)), Movement::Normal);
    assert_eq!(s.cover_at(Sq(1, 0)), CoverLevel::ThreeQuarters);
    assert_eq!(s.cover_at(Sq(2, 0)), CoverLevel::Total);
    assert_eq!(s.movement_at(Sq(3, 0)), Movement::Difficult);
    assert_eq!(s.movement_at(Sq(4, 0)), Movement::Swim);
    assert_eq!(s.water_depth_ft.0[4], 6);
    assert_eq!(s.movement_at(Sq(5, 0)), Movement::Impassable);
    assert_eq!(s.movement_at(Sq(2, 0)), Movement::Normal);
    assert_eq!(s.obscured.0[6 + 3], Obscurement::Heavy);
    assert!(!s.line_of_sight(Sq(2, 1), Sq(4, 1)));
    // Without the sidecar the bush is difficult and the lane is clear.
    let plain = l.scene();
    assert_eq!(plain.movement_at(Sq(0, 0)), Movement::Difficult);
    assert!(plain.line_of_sight(Sq(2, 1), Sq(4, 1)));
}

#[test]
fn diagonal_opaque_squares_block_the_gap_between_them() {
    let mut r = RulesSidecar::empty(4, 4);
    r.cell_mut(2, 1).unwrap().blocks_sight = Some(true);
    r.cell_mut(1, 2).unwrap().blocks_sight = Some(true);
    let s = L::new(4, 4).scene_with(&r);
    assert!(!s.line_of_sight(Sq(1, 1), Sq(2, 2)));
    let mut one = RulesSidecar::empty(4, 4);
    one.cell_mut(2, 1).unwrap().blocks_sight = Some(true);
    assert!(L::new(4, 4)
        .scene_with(&one)
        .line_of_sight(Sq(1, 1), Sq(2, 2)));
}

#[test]
fn a_mismatched_sidecar_is_refused() {
    let l = L::new(4, 4);
    let r = RulesSidecar::empty(3, 4);
    let err = build_scene(&l.0, lib(), 1, Some(&r)).unwrap_err();
    assert!(matches!(err, SceneError::Sidecar(_)));
}

#[test]
fn sidecar_json_round_trips() {
    let mut r = RulesSidecar::empty(2, 2);
    r.cell_mut(1, 1).unwrap().cover = Some(CoverLevel::Half);
    let json = serde_json::to_string(&r).unwrap();
    assert_eq!(RulesSidecar::from_json(&json).unwrap(), r);
    assert!(json.contains(r#"{},{},{},{"cover":"half"}"#));
}

#[test]
fn a_sidecar_deck_is_walked_over_deep_water() {
    let l = L::new(3, 1).water(1, 0, 8);
    let mut r = RulesSidecar::empty(3, 1);
    r.cell_mut(1, 0).unwrap().deck = Some(true);
    let s = l.scene_with(&r);
    assert_eq!(s.movement_at(Sq(1, 0)), Movement::Normal);
    assert_eq!(s.path_cost(Sq(0, 0), Sq(2, 0)), Some(10));
    assert_eq!(l.scene().movement_at(Sq(1, 0)), Movement::Swim);
    // A layout floor asset (dock planks) can be cleared by the sidecar.
    let dock = L::new(3, 1).water(1, 0, 8).put("prop.dock_planks", 1, 0);
    assert_eq!(dock.scene().movement_at(Sq(1, 0)), Movement::Normal);
    let mut off = RulesSidecar::empty(3, 1);
    off.cell_mut(1, 0).unwrap().deck = Some(false);
    assert_eq!(dock.scene_with(&off).movement_at(Sq(1, 0)), Movement::Swim);
}

/// Adapter A8 (logic/12 §scene-sidecar): a format-2 bridge square is walked
/// at deck level, and a parapet edge blocks movement but not sight.
#[test]
fn format_2_bridge_deck_and_parapet_follow_the_sidecar() {
    use arda_scene::{EdgeRole, EdgeRule};
    use arda_tactical::layout::EdgeAxis;
    // A 3 x 3 river crossing: the middle row is the deck, parapets on the
    // north edges of row 1 (between rows 0 and 1).
    let mut l = L::new(3, 3);
    for x in 0..3 {
        l = l.water(x, 0, 8).water(x, 1, 8).water(x, 2, 8);
        l = l.h(x, 1, WallRole::Run);
    }
    let mut r = RulesSidecar::empty(3, 3);
    for x in 0..3 {
        let c = r.cell_mut(x, 1).unwrap();
        c.deck = Some(true);
        c.set_ext("deck_elevation_ft", serde_json::json!(15));
        c.set_ext("feature", serde_json::json!("bridge"));
        r.edges.push(EdgeRule {
            x,
            y: 1,
            axis: EdgeAxis::Horizontal,
            role: EdgeRole::Parapet,
            blocks_movement: true,
            blocks_sight: false,
            cover: CoverLevel::Half,
        });
    }
    let s = l.scene_with(&r);
    assert_eq!(s.movement_at(Sq(1, 1)), Movement::Normal);
    assert_eq!(s.movement_at(Sq(1, 0)), Movement::Swim);
    assert_eq!(s.path_cost(Sq(0, 1), Sq(2, 1)), Some(10));
    assert!(s.line_of_sight(Sq(1, 0), Sq(1, 2)));
    let plain = l.scene();
    assert!(!plain.line_of_sight(Sq(1, 0), Sq(1, 2)));
    let parapet = s.walls.iter().find(|w| w.points[0][1] == 1).unwrap();
    assert!(parapet.blocks_movement && !parapet.blocks_sight);
}

#[test]
fn heavy_obscurement_hides_but_grants_no_cover() {
    // SRD 5.1: a heavily obscured area blinds sight into it; cover comes
    // only from obstacles. A dense-canopy square between attacker and target
    // blocks line of sight but must not make the target untargetable.
    let mut r = RulesSidecar::empty(8, 3);
    r.cell_mut(3, 1).unwrap().blocks_sight = Some(true);
    let s = L::new(8, 3).scene_with(&r);
    assert!(!s.line_of_sight(Sq(0, 1), Sq(5, 1)));
    assert_eq!(s.cover_between(Sq(0, 1), Sq(5, 1)), CoverLevel::None);
    // A solid obstacle still grants cover.
    let s = L::new(8, 3).put("prop.crate", 3, 1).scene();
    assert_eq!(s.cover_between(Sq(0, 1), Sq(5, 1)), CoverLevel::Half);
}

#[test]
fn the_stronger_cover_wins_whichever_comes_first() {
    // The window (three-quarters) is nearer the attacker than the crate
    // (half), so a last-obstacle-wins merge fails here.
    let mut l = L::new(8, 3);
    for y in 0..3 {
        l = l.v(2, y, WallRole::Run);
    }
    let s = l.v(2, 1, WallRole::Window).put("prop.crate", 4, 1).scene();
    assert_eq!(
        s.cover_between(Sq(0, 1), Sq(6, 1)),
        CoverLevel::ThreeQuarters
    );
}

#[test]
fn four_feet_of_water_is_waded_and_five_is_swum() {
    let s = L::new(3, 1).water(0, 0, 4).water(1, 0, 5).scene();
    assert_eq!(s.movement_at(Sq(0, 0)), Movement::Wade);
    assert_eq!(s.movement_at(Sq(1, 0)), Movement::Swim);
}

#[test]
fn line_of_sight_is_symmetric_including_obscured_endpoints() {
    let mut r = RulesSidecar::empty(7, 6);
    for (x, y) in [(2, 2), (4, 1), (0, 5), (6, 0)] {
        r.cell_mut(x, y).unwrap().blocks_sight = Some(true);
    }
    let s = L::new(7, 6)
        .v(3, 0, WallRole::Run)
        .v(3, 1, WallRole::Window)
        .h(5, 4, WallRole::Run)
        .scene_with(&r);
    let q = s.queries();
    let all: Vec<Sq> = (0..6).flat_map(|y| (0..7).map(move |x| Sq(x, y))).collect();
    for &a in &all {
        for &b in &all {
            assert_eq!(q.line_of_sight(a, b), q.line_of_sight(b, a), "{a:?} {b:?}");
        }
    }
}
