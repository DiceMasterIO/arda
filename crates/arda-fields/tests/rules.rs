//! Enclosure, gates, farm lanes, field-size statistics and SRD rules.

#![allow(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use arda_fields::compounds::CompoundKind;
use arda_fields::degrade::adapt;
use arda_fields::fields::{FieldKind, SQUARE_M2};
use arda_fields::geom::{Sq, SQUARE_M};
use arda_fields::plan::{Cover, Plan};
use arda_fields::strips;
use arda_fields::supplement::{placeholder_library, supplemented_library};
use arda_fields::synthetic::{self, Scenario};
use arda_fields::window::window_rect;
use arda_fields::{generate, FieldsWindow};
use arda_tactical::layout::{AssetRef, EdgeAxis};
use arda_tactical::WallRole;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const SEED: u64 = 11;

fn big(s: &Scenario, side: u32) -> FieldsWindow {
    generate(&s.inputs(), s.origin_m, side, side, SEED).unwrap()
}

#[test]
fn every_enclosed_field_is_walled_all_round_with_a_gate() {
    let mut total = 0;
    for s in [
        synthetic::hedge_country(),
        synthetic::quarry(),
        synthetic::watermill(),
        synthetic::orchard_farmstead(),
    ] {
        let win = big(&s, 256);
        let (l, side) = (&win.layout, &win.sidecar);
        let w = l.width as usize;
        let walls: BTreeMap<(EdgeAxis, u32, u32), WallRole> = l
            .walls
            .iter()
            .map(|x| ((x.axis, x.x, x.y), x.kind))
            .collect();
        let complete: BTreeSet<String> = side
            .fields
            .iter()
            .filter(|f| f.enclosed && f.complete)
            .map(|f| f.id.clone())
            .collect();
        assert!(!complete.is_empty(), "{}: no complete field", s.name);
        total += complete.len();
        let mut gates: BTreeMap<String, usize> = BTreeMap::new();
        for (i, q) in side.squares.iter().enumerate() {
            let Some(id) = q.field.as_ref().filter(|id| complete.contains(*id)) else {
                continue;
            };
            let (x, y) = ((i % w) as u32, (i / w) as u32);
            let nbrs = [
                (x + 1, y, (EdgeAxis::Vertical, x + 1, y)),
                (x.wrapping_sub(1), y, (EdgeAxis::Vertical, x, y)),
                (x, y + 1, (EdgeAxis::Horizontal, x, y + 1)),
                (x, y.wrapping_sub(1), (EdgeAxis::Horizontal, x, y)),
            ];
            for (nx, ny, edge) in nbrs {
                let n = &side.squares[ny as usize * w + nx as usize];
                if n.field.as_ref() == Some(id) || n.water_depth_ft > 0 {
                    continue;
                }
                let role = walls.get(&edge);
                assert!(role.is_some(), "{}: field {id} is open at {edge:?}", s.name);
                if role == Some(&WallRole::Gate) {
                    *gates.entry(id.clone()).or_insert(0) += 1;
                }
            }
        }
        for id in &complete {
            assert!(
                gates.get(id).copied().unwrap_or(0) >= 1,
                "{}: field {id} has no gate",
                s.name
            );
        }
    }
    assert!(total >= 12, "only {total} complete enclosed fields");
}

#[test]
fn farmsteads_and_mills_are_joined_to_a_road_by_lane() {
    for s in [
        synthetic::orchard_farmstead(),
        synthetic::hedge_country(),
        synthetic::watermill(),
    ] {
        let win = window_rect(s.origin_m, s.w, s.h).unwrap();
        let plan = Plan::build(&s.inputs(), win, SEED);
        let mut checked = 0;
        for (c, lane) in plan.compounds.iter().zip(&plan.lanes) {
            let Some(start) = c.lane_start else { continue };
            if !plan.in_window(start)
                || !matches!(c.kind, CompoundKind::Farmstead | CompoundKind::Mill)
            {
                continue;
            }
            assert!(lane.is_some(), "{}: {:?} has no lane", s.name, c.kind);
            // Breadth-first along lane squares from the gate to any road.
            let mut seen = BTreeSet::from([start]);
            let mut queue = VecDeque::from([start]);
            let mut reached = false;
            while let Some(q) = queue.pop_front() {
                if matches!(plan.at(q), Cover::Road(_)) {
                    reached = true;
                    break;
                }
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let n = q.offset(dx, dy);
                    if matches!(plan.at(n), Cover::Lane | Cover::Road(_)) && seen.insert(n) {
                        queue.push_back(n);
                    }
                }
            }
            assert!(reached, "{}: {:?} lane never meets a road", s.name, c.kind);
            checked += 1;
        }
        assert!(checked >= 1, "{}: no compound checked", s.name);
    }
}

#[test]
fn field_sizes_look_like_real_countryside() {
    let s = synthetic::hedge_country();
    let win = window_rect([s.origin_m[0] - 300.0, s.origin_m[1] - 300.0], 640, 640).unwrap();
    let plan = Plan::build(&s.inputs(), win, SEED);
    let inside = |f: &arda_fields::fields::Field| {
        let c = f.centroid();
        plan.in_window(Sq::containing(c))
    };
    let mut ha: Vec<f64> = plan
        .fields
        .iter()
        .filter(|f| f.kind.enclosed() && inside(f))
        .map(arda_fields::fields::Field::hectares)
        .collect();
    ha.sort_by(f64::total_cmp);
    assert!(ha.len() >= 20, "only {} enclosed fields", ha.len());
    let mean = ha.iter().sum::<f64>() / ha.len() as f64;
    let median = ha[ha.len() / 2];
    println!(
        "enclosed fields: n={} mean={mean:.2} ha median={median:.2} ha min={:.2} max={:.2}",
        ha.len(),
        ha[0],
        ha[ha.len() - 1]
    );
    // Ancient enclosed countryside: fields of roughly half a hectare to a
    // few hectares, a median near one.
    assert!((0.6..=2.5).contains(&mean), "mean {mean:.2} ha");
    assert!((0.4..=2.0).contains(&median), "median {median:.2} ha");
    let min_sq = arda_fields::plan::MIN_FIELD_SQ as f64;
    assert!(
        ha[0] * 10_000.0 >= min_sq * SQUARE_M2 - 1e-9,
        "sliver of {:.3} ha",
        ha[0]
    );
    assert!(
        *ha.last().unwrap() <= 8.0,
        "largest {:.2} ha",
        ha.last().unwrap()
    );

    // Open-field strips near the village: a few rods wide.
    let v = synthetic::village_strips();
    let vw = window_rect(v.origin_m, v.w, v.h).unwrap();
    let vp = Plan::build(&v.inputs(), vw, SEED);
    let strips: Vec<f64> = vp
        .fields
        .iter()
        .filter(|f| f.kind == FieldKind::Strips)
        .map(|f| strips::furlong(SEED, &vp.partition.sites[f.site]).1 * SQUARE_M)
        .collect();
    assert!(strips.len() >= 5, "only {} furlongs", strips.len());
    for w in strips {
        assert!((10.0..=18.0).contains(&w), "strip width {w:.1} m");
    }
}

#[test]
fn sidecar_edges_follow_the_srd_reading_of_each_kit() {
    let win = big(&synthetic::hedge_country(), 192);
    let mut hedge = 0;
    for e in &win.sidecar.edges {
        match (e.kit.as_str(), e.kind) {
            ("hedge", WallRole::Gate) => assert!(!e.blocks_sight && !e.blocks_movement),
            ("hedge", _) => {
                hedge += 1;
                assert!(e.blocks_sight && e.blocks_movement);
            }
            ("drystone" | "wattle", WallRole::Run) => {
                assert!(!e.blocks_sight && e.climb);
                assert_eq!(serde_json::to_string(&e.cover).unwrap(), "\"half\"");
            }
            _ => {}
        }
    }
    assert!(hedge > 100);
    let json = serde_json::to_string(&win.sidecar).unwrap();
    assert!(json.contains("\"water_depth_ft\"") && json.contains("\"deck\""));
    assert!(!json.contains("\"full\""), "cover must say total, not full");
    assert_eq!(win.sidecar.squares.len(), 192 * 192);
    let q = synthetic::quarry();
    let upland = big(&q, 192);
    assert!(upland.sidecar.edges.iter().any(|e| e.kit == "drystone"));
    assert!(upland
        .layout
        .squares
        .iter()
        .all(|s| s.elevation_ft % 5 == 0));
}

#[test]
fn special_sites_carry_their_workings() {
    let m = big(&synthetic::watermill(), 192);
    let ids: BTreeSet<String> = m
        .layout
        .placements
        .iter()
        .filter_map(|p| match &p.asset {
            AssetRef::Id(id) => Some(id.clone()),
            AssetRef::Query { .. } => None,
        })
        .collect();
    for id in ["prop.waterwheel", "prop.millstone", "prop.sacks"] {
        assert!(ids.contains(id), "mill lacks {id}");
    }
    let q = synthetic::quarry();
    let plan = Plan::build(
        &q.inputs(),
        window_rect(q.origin_m, q.w, q.h).unwrap(),
        SEED,
    );
    let kinds: BTreeSet<CompoundKind> = plan.compounds.iter().map(|c| c.kind).collect();
    assert!(kinds.contains(&CompoundKind::Quarry) && kinds.contains(&CompoundKind::Mine));
    let quarry = big(&q, q.w);
    for g in ["scree", "gravel", "cliff"] {
        assert!(
            quarry.layout.squares.iter().any(|s| s.ground == g),
            "no {g}"
        );
    }
    assert!(quarry.earthworks_ft.iter().any(|e| *e < 0), "no pit");
    assert!(quarry.earthworks_ft.iter().any(|e| *e > 0), "no spoil heap");
}

#[test]
fn missing_assets_degrade_to_the_nearest_and_are_recorded() {
    let win = big(&synthetic::hedge_country(), 96);
    // The full placeholder library covers hedges and farmland, so trim
    // them away again to exercise the fallback path.
    let (mut cat, mut imgs) =
        arda_tactical::placeholders::generate(arda_tactical::placeholders::DEFAULT_SEED);
    let trimmed = |a: &arda_tactical::Asset| {
        a.wall.as_ref().is_some_and(|w| w.kit == "hedge") || a.ground.as_deref() == Some("farmland")
    };
    let gone: Vec<String> = cat
        .assets
        .iter()
        .filter(|a| trimmed(a))
        .map(|a| a.image.clone())
        .collect();
    cat.assets.retain(|a| !trimmed(a));
    imgs.retain(|k, _| !gone.contains(k));
    let bare = arda_tactical::Library::from_parts(cat, imgs).unwrap();
    let _ = placeholder_library().unwrap();
    let (layout, fallbacks) = adapt(&win.layout, &bare);
    layout.check(&bare).unwrap();
    let find = |k: &str| fallbacks.iter().find(|f| f.wanted == k);
    assert_eq!(
        find("hedge").and_then(|f| f.used.as_deref()),
        Some("wattle")
    );
    assert_eq!(
        find("farmland").and_then(|f| f.used.as_deref()),
        Some("dirt")
    );
    let (full, _) = supplemented_library().unwrap();
    let (layout, fewer) = adapt(&win.layout, &full);
    layout.check(&full).unwrap();
    assert!(fewer.len() < fallbacks.len());
    assert!(!fewer.iter().any(|f| f.kind == "kit" || f.kind == "ground"));
}

/// Adapter A8 (logic/12 §scene-sidecar): fields rules are `arda-scene`'s
/// format 2, stated on owned and woodland-fringe squares only, with the
/// field extras in `ext`;
/// hedges and gates become edge rules; the layout carries its origin.
#[test]
fn fields_rules_are_scene_sidecar_format_2() {
    let s = synthetic::hedge_country();
    let win = big(&s, 96);
    let (x0, y0, _, _) = window_rect(s.origin_m, 96, 96).unwrap();
    assert_eq!(win.layout.origin, Some([x0, y0]));
    let r = &win.rules;
    assert_eq!((r.format_version, r.width, r.height), (2, 96, 96));
    assert!(win.owned.iter().any(|o| *o));
    for ((cell, own), fringe) in r.squares.iter().zip(&win.owned).zip(&win.fringe) {
        assert_eq!(cell.difficult.is_some(), *own || fringe.is_some());
        assert!(
            !(*own && fringe.is_some()),
            "a fringe square is never owned"
        );
    }
    assert!(r
        .squares
        .iter()
        .any(|c| c.ext.as_ref().is_some_and(|e| e.contains_key("field"))));
    assert!(r
        .edges
        .iter()
        .any(|e| e.role == arda_scene::EdgeRole::Boundary));
    let json = serde_json::to_string(r).unwrap();
    assert_eq!(arda_scene::RulesSidecar::from_json(&json).unwrap(), *r);
}

#[test]
fn deep_water_is_swum_not_impassable() {
    // I15: five feet or more is swimming. In RulesSidecar format 2,
    // blocks_movement = true means impassable, so deep squares must not set it.
    let mut deep = 0;
    for s in synthetic::all() {
        let win = big(&s, 256);
        for (i, sq) in win.sidecar.squares.iter().enumerate() {
            if sq.water_depth_ft >= 5 {
                deep += 1;
                let placed = win.layout.placements.iter().any(|p| {
                    (p.y.floor() as usize) * win.layout.width as usize + p.x.floor() as usize == i
                });
                assert!(placed || !sq.blocks_movement, "{} square {i}", s.name);
            }
        }
    }
    assert!(deep > 0, "some scenario has deep water");
}

#[test]
fn compounds_never_stand_on_a_settlement_core() {
    let s = synthetic::orchard_farmstead();
    let win = window_rect(s.origin_m, s.w, s.h).unwrap();
    let free = Plan::build(&s.inputs(), win, SEED);
    let first = free.compounds.first().unwrap();
    // A core touching one square of the farmstead removes the whole
    // compound: none of its walls may cross the town's ground.
    let hit = *first.squares.keys().next().unwrap();
    let core = move |x: i64, y: i64| (x, y) == (hit.x, hit.y);
    let mut inputs = s.inputs();
    inputs.cores = Some(&core);
    let cored = Plan::build(&inputs, win, SEED);
    assert_eq!(cored.compounds.len() + 1, free.compounds.len());
    for c in &cored.compounds {
        assert!(c
            .squares
            .keys()
            .all(|q| (q.x - hit.x).abs() > 1 || (q.y - hit.y).abs() > 1));
    }
}
