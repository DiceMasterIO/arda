//! Unit tests of the overlay seam (`overlays.rs`).
use super::*;

fn layer(ground: &str, owned: Vec<bool>, soft: Vec<bool>) -> OverlayLayer {
    let mut layout = TacticalLayout::new("t", 3, 1, ground);
    layout
        .squares
        .iter_mut()
        .for_each(|s| ground.clone_into(&mut s.ground));
    OverlayLayer {
        layout,
        rules: RulesSidecar::empty(3, 1),
        owned,
        soft,
        elevation: Vec::new(),
        review: Vec::new(),
    }
}

#[test]
fn soft_claims_yield_to_the_layers_they_rank_below() {
    let mut base = TacticalLayout::new("t", 3, 1, "forest_floor");
    let mut rules = RulesSidecar::empty(3, 1);
    let mut owners = vec![Owner::Natural; 3];
    // A road on square 0 and its verge on square 1.
    let ways = layer("dirt", vec![true, true, false], vec![false, true, false]);
    compose(&mut base, &mut rules, &mut owners, &ways, Owner::Ways).unwrap();
    assert_eq!(owners, [Owner::Ways, Owner::Verge, Owner::Natural]);
    // Fields take the verge but not the road.
    let fields = layer("pasture", vec![true; 3], Vec::new());
    compose(&mut base, &mut rules, &mut owners, &fields, Owner::Fields).unwrap();
    assert_eq!(owners, [Owner::Ways, Owner::Fields, Owner::Fields]);
    // Crofts take fields but give way to the road; a building takes all.
    let town = layer("grass", vec![true; 3], vec![true, true, false]);
    compose(&mut base, &mut rules, &mut owners, &town, Owner::Town).unwrap();
    assert_eq!(owners, [Owner::Ways, Owner::Croft, Owner::Town]);
    let grounds: Vec<&str> = base.squares.iter().map(|s| s.ground.as_str()).collect();
    assert_eq!(grounds, ["dirt", "grass", "grass"]);
}

/// Review 3.5-5: a verge keeps the natural ground and its trees, so it
/// keeps the natural rules too (canopy, rough ground) and adds its own;
/// a road square replaces them.
#[test]
fn a_verge_merges_with_the_natural_rules_it_keeps() {
    let mut base = TacticalLayout::new("t", 3, 1, "forest_floor");
    let mut rules = RulesSidecar::empty(3, 1);
    for c in &mut rules.squares {
        c.difficult = Some(true);
        c.blocks_sight = Some(true);
        c.lightly_obscured = Some(true);
        c.water_depth_ft = Some(0);
    }
    let mut owners = vec![Owner::Natural; 3];
    let mut ways = layer("dirt", vec![true, true, false], vec![false, true, false]);
    for c in &mut ways.rules.squares[..2] {
        c.water_depth_ft = Some(0);
        c.deck = Some(false);
        c.set_ext("feature", serde_json::json!("verge"));
    }
    compose(&mut base, &mut rules, &mut owners, &ways, Owner::Ways).unwrap();
    assert_eq!(owners, [Owner::Ways, Owner::Verge, Owner::Natural]);
    let verge = &rules.squares[1];
    assert_eq!(verge.difficult, Some(true));
    assert_eq!(verge.blocks_sight, Some(true));
    assert_eq!(verge.lightly_obscured, Some(true));
    assert!(verge
        .ext
        .as_ref()
        .is_some_and(|x| x.contains_key("feature")));
    // The road surface does not keep the forest's rules.
    assert_eq!(rules.squares[0].blocks_sight, None);
    assert_eq!(rules.squares[0].difficult, None);
}

#[test]
fn a_town_builds_over_a_shallow_fringe_but_not_deep_water() {
    let mut base = TacticalLayout::new("t", 3, 1, "water_shallow");
    base.squares[0].water_depth_ft = 1;
    base.squares[1].water_depth_ft = 4;
    base.squares[2].water_depth_ft = 1;
    let mut rules = RulesSidecar::empty(3, 1);
    let mut owners = vec![Owner::Water; 3];
    // Fields never take water; a town takes the 1-ft fringe, not 4 ft.
    let fields = layer("pasture", vec![true; 3], Vec::new());
    compose(&mut base, &mut rules, &mut owners, &fields, Owner::Fields).unwrap();
    assert_eq!(owners, [Owner::Water; 3]);
    let town = layer("cobbles", vec![true; 3], vec![true, true, false]);
    compose(&mut base, &mut rules, &mut owners, &town, Owner::Town).unwrap();
    let grounds: Vec<&str> = base.squares.iter().map(|s| s.ground.as_str()).collect();
    assert_eq!(grounds, ["cobbles", "water_shallow", "cobbles"]);
    let depths: Vec<u8> = base.squares.iter().map(|s| s.water_depth_ft).collect();
    assert_eq!(depths, [0, 4, 0]);
}

fn fence(x: u32) -> arda_tactical::layout::WallSegment {
    arda_tactical::layout::WallSegment {
        x,
        y: 0,
        axis: EdgeAxis::Vertical,
        kind: arda_tactical::WallRole::Run,
        kit: "wattle".into(),
        tags: Vec::new(),
    }
}

#[test]
fn a_lower_layers_walls_never_stand_inside_ground_a_layer_takes_over() {
    let mut base = TacticalLayout::new("t", 4, 1, "grass");
    let mut rules = RulesSidecar::empty(4, 1);
    let mut owners = vec![Owner::Natural; 4];
    // A farmstead fence on every inner edge.
    let mut fields = layer("dirt", vec![true; 3], Vec::new());
    fields.layout = TacticalLayout::new("t", 4, 1, "dirt");
    fields.owned = vec![true; 4];
    fields.rules = RulesSidecar::empty(4, 1);
    fields.layout.walls = vec![fence(1), fence(2), fence(3)];
    compose(&mut base, &mut rules, &mut owners, &fields, Owner::Fields).unwrap();
    assert_eq!(base.walls.len(), 3);
    // The town takes squares 2 and 3 and walls only its own outline.
    let mut town = layer("planks", vec![true; 3], Vec::new());
    town.layout = TacticalLayout::new("t", 4, 1, "planks");
    town.owned = vec![false, false, true, true];
    town.rules = RulesSidecar::empty(4, 1);
    town.layout.walls = vec![fence(2)];
    compose(&mut base, &mut rules, &mut owners, &town, Owner::Town).unwrap();
    let xs: Vec<u32> = base.walls.iter().map(|w| w.x).collect();
    // Edge 1 lies between fields squares, edge 2 is the town's own,
    // edge 3 lies inside the town and loses the old fence.
    assert_eq!(xs, [1, 2]);
}

#[derive(Debug)]
struct Relaxing;

impl Overlays for Relaxing {
    fn ways(&self, _: &OverlayCtx<'_>) -> Result<Option<OverlayLayer>, BlocksError> {
        Ok(Some(layer("dirt", vec![true, false, false], Vec::new())))
    }

    fn town(&self, _: &OverlayCtx<'_>) -> Result<Option<OverlayLayer>, BlocksError> {
        let mut town = layer("grass", vec![true; 3], vec![true, false, false]);
        town.review = vec![true; 3];
        Ok(Some(town))
    }
}

#[test]
fn relaxed_overlay_squares_are_reviewed_where_the_layer_keeps_them() {
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
    let library = Library::load(&dir).unwrap();
    let base = TacticalLayout::new("t", 3, 1, "forest_floor");
    let ctx = OverlayCtx {
        gsx0: 0,
        gsy0: 0,
        width: 3,
        height: 1,
        seed: 1,
        base: &base,
        library: &library,
    };
    let mut layout = base.clone();
    let mut rules = RulesSidecar::empty(3, 1);
    let mut owners = vec![Owner::Natural; 3];
    let applied = compose_all(&ctx, &Relaxing, &mut layout, &mut rules, &mut owners).unwrap();
    // The road keeps square 0 from the town's soft croft claim.
    assert_eq!(owners, [Owner::Ways, Owner::Town, Owner::Town]);
    assert_eq!(applied.layers, ["ways", "town"]);
    assert_eq!(applied.review, [1, 2]);
}

/// Review 3.5-3: a ford a road lays through a marsh pool raises its wading
/// depth; a town building that outranks the road is judged against the
/// pool beneath, made ground, not the ford's water.
#[test]
fn a_town_builds_over_a_ford_on_made_ground() {
    let mut refined = TacticalLayout::new("t", 3, 1, "marsh");
    for s in &mut refined.squares {
        s.water_depth_ft = 1;
    }
    let mut base = refined.clone();
    let mut rules = RulesSidecar::empty(3, 1);
    let mut owners = vec![Owner::Natural; 3];
    let mut ways = layer("dirt", vec![true; 3], Vec::new());
    for s in &mut ways.layout.squares {
        s.water_depth_ft = 2;
    }
    compose_over(
        &mut base,
        &mut rules,
        &mut owners,
        &ways,
        Owner::Ways,
        Some(&refined),
    )
    .unwrap();
    let town = layer("planks", vec![true; 3], Vec::new());
    let mut alone = (base.clone(), rules.clone(), owners.clone());
    compose(&mut alone.0, &mut alone.1, &mut alone.2, &town, Owner::Town).unwrap();
    assert_eq!(alone.2, [Owner::Ways; 3], "without the refined terrain");
    compose_over(
        &mut base,
        &mut rules,
        &mut owners,
        &town,
        Owner::Town,
        Some(&refined),
    )
    .unwrap();
    assert_eq!(owners, [Owner::Town; 3]);
    assert!(base.squares.iter().all(|s| s.water_depth_ft == 0));
}

/// Review 3.5-4: a layer states the elevation of the squares it levels
/// and only those; its other claims keep the refined ground's.
#[test]
fn only_stated_squares_take_the_layer_elevation() {
    let mut base = TacticalLayout::new("t", 3, 1, "grass");
    for (k, s) in base.squares.iter_mut().enumerate() {
        s.elevation_ft = 5 * i16::try_from(k).unwrap();
    }
    let mut rules = RulesSidecar::empty(3, 1);
    let mut owners = vec![Owner::Natural; 3];
    let mut town = layer("planks", vec![true; 3], Vec::new());
    for s in &mut town.layout.squares {
        s.elevation_ft = 40;
    }
    town.elevation = vec![true, false, true];
    compose(&mut base, &mut rules, &mut owners, &town, Owner::Town).unwrap();
    let e: Vec<i16> = base.squares.iter().map(|s| s.elevation_ft).collect();
    assert_eq!(e, [40, 5, 40]);
}
