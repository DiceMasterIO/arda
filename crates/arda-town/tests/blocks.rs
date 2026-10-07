//! Tactical block invariants: determinism, seams between neighbouring
//! windows, plan ids kept in blocks, doors on street fronts, curtain-wall
//! continuity and resolution against the placeholder library.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_tactical::catalog::WallRole;
use arda_tactical::layout::{AssetRef, EdgeAxis};
use arda_tactical::{Library, RenderOptions};
use arda_town::block::{self, fallback, TownBlock, Window};
use arda_town::plan::TownPlan;
use common::{library_dir, plan, plans};
use std::collections::{BTreeMap, BTreeSet};

fn market_window(p: &TownPlan, w: i64, h: i64) -> Window {
    let (x, y) = arda_town::plan::square::square_of(p.focal.market);
    Window {
        x: x - w / 2,
        y: y - h / 2,
        w,
        h,
    }
}

type Edge = (i64, i64, EdgeAxis, WallRole, String);

fn global_walls(b: &TownBlock) -> BTreeSet<Edge> {
    b.layout
        .walls
        .iter()
        .map(|s| {
            (
                b.origin[0] + i64::from(s.x),
                b.origin[1] + i64::from(s.y),
                s.axis,
                s.kind,
                s.kit.clone(),
            )
        })
        .collect()
}

fn global_props(b: &TownBlock) -> BTreeSet<(String, i64, i64, u16)> {
    b.layout
        .placements
        .iter()
        .map(|p| {
            let key = match &p.asset {
                AssetRef::Id(id) => id.clone(),
                AssetRef::Query { tags, .. } => tags.join(","),
            };
            // Quarter-square fixed point: anchors sit on halves.
            let fx = arda_town::num::round_i(f64::from(p.x) * 4.0) + b.origin[0] * 4;
            let fy = arda_town::num::round_i(f64::from(p.y) * 4.0) + b.origin[1] * 4;
            (key, fx, fy, p.rotation)
        })
        .collect()
}

#[test]
fn blocks_are_deterministic() {
    let p = plan("aldermere");
    let w = market_window(p, 64, 64);
    let a = block::generate(p, w).unwrap();
    let b = block::generate(p, w).unwrap();
    assert_eq!(a.to_json().unwrap(), b.to_json().unwrap());
}

#[test]
fn neighbouring_blocks_join_without_seams() {
    for p in plans() {
        let big = market_window(p, 128, 64);
        let left = Window { w: 64, ..big };
        let right = Window {
            x: big.x + 64,
            w: 64,
            ..big
        };
        let (b, l, r) = (
            block::generate(p, big).unwrap(),
            block::generate(p, left).unwrap(),
            block::generate(p, right).unwrap(),
        );
        for y in 0..64usize {
            for x in 0..128usize {
                let s = &b.layout.squares[y * 128 + x];
                let part = if x < 64 {
                    &l.layout.squares[y * 64 + x]
                } else {
                    &r.layout.squares[y * 64 + x - 64]
                };
                assert_eq!(s, part, "{} square ({x}, {y})", p.name);
            }
        }
        let walls: BTreeSet<Edge> = global_walls(&l).union(&global_walls(&r)).cloned().collect();
        assert_eq!(
            walls,
            global_walls(&b),
            "{}: walls differ across the seam",
            p.name
        );
        // The shared border carries the same segments in both blocks.
        let seam = |blk: &TownBlock| -> BTreeSet<Edge> {
            global_walls(blk)
                .into_iter()
                .filter(|e| e.2 == EdgeAxis::Vertical && e.0 == big.x + 64)
                .collect()
        };
        assert_eq!(seam(&l), seam(&r), "{}: seam edges", p.name);
        let props: BTreeSet<_> = global_props(&l).union(&global_props(&r)).cloned().collect();
        assert_eq!(
            props,
            global_props(&b),
            "{}: props differ across the seam",
            p.name
        );
    }
}

#[test]
fn block_buildings_keep_their_plan_ids() {
    for p in plans() {
        let w = market_window(p, 96, 96);
        let blk = block::generate(p, w).unwrap();
        assert!(!blk.buildings.is_empty(), "{}", p.name);
        let view = arda_town::plan::grid::SquareRect {
            x0: w.x,
            y0: w.y,
            x1: w.x + w.w,
            y1: w.y + w.h,
        };
        let expect: BTreeSet<u64> = p
            .buildings
            .iter()
            .filter(|b| b.rect.overlaps(&view))
            .map(|b| b.id.0)
            .collect();
        let got: BTreeSet<u64> = blk.buildings.iter().map(|b| b.id.0).collect();
        assert_eq!(got, expect, "{}", p.name);
        for bb in &blk.buildings {
            let pb = p.building(bb.id).unwrap();
            assert_eq!(
                (pb.function, pb.rect),
                (bb.function, bb.rect),
                "{} {:?}",
                p.name,
                bb.id
            );
            assert!(
                !bb.rooms.is_empty()
                    || !pb.function.walled()
                    || bb.function == arda_town::BuildingFunction::Stall
            );
        }
    }
}

#[test]
fn front_doors_are_door_pieces_facing_the_street() {
    let p = plan("aldermere");
    let w = market_window(p, 96, 96);
    let blk = block::generate(p, w).unwrap();
    let walls: BTreeMap<(i64, i64, EdgeAxis), WallRole> = global_walls(&blk)
        .into_iter()
        .map(|e| ((e.0, e.1, e.2), e.3))
        .collect();
    let mut checked = 0;
    for bb in blk.buildings.iter().filter(|b| b.function.walled()) {
        let d = bb.doors[0];
        let key = match d.side {
            arda_town::plan::grid::Side::North => (d.x, d.y, EdgeAxis::Horizontal),
            arda_town::plan::grid::Side::South => (d.x, d.y + 1, EdgeAxis::Horizontal),
            arda_town::plan::grid::Side::West => (d.x, d.y, EdgeAxis::Vertical),
            arda_town::plan::grid::Side::East => (d.x + 1, d.y, EdgeAxis::Vertical),
        };
        if let Some(&role) = walls.get(&key) {
            assert!(
                matches!(role, WallRole::Door | WallRole::Gate),
                "{:?}: {role:?}",
                bb.id
            );
            checked += 1;
        }
    }
    assert!(checked > 10, "{checked} doors checked");
}

#[test]
fn curtain_walls_have_no_loose_ends() {
    for p in plans().iter().filter(|p| p.wall.is_some()) {
        let wall = p.wall.as_ref().unwrap();
        for g in wall.gates.iter().filter(|g| g.street.is_some()) {
            let (x, y) = arda_town::plan::square::square_of(g.point);
            let w = Window {
                x: x - 32,
                y: y - 32,
                w: 64,
                h: 64,
            };
            let blk = block::generate(p, w).unwrap();
            let mut degree: BTreeMap<(i64, i64), u32> = BTreeMap::new();
            let mut gates = 0;
            for s in blk.layout.walls.iter().filter(|s| s.kit == "city_wall") {
                let (x0, y0) = (i64::from(s.x), i64::from(s.y));
                let (x1, y1) = if s.axis == EdgeAxis::Horizontal {
                    (x0 + 1, y0)
                } else {
                    (x0, y0 + 1)
                };
                *degree.entry((x0, y0)).or_default() += 1;
                *degree.entry((x1, y1)).or_default() += 1;
                gates += u32::from(s.kind == WallRole::Gate);
            }
            assert!(
                gates >= 2,
                "{}: gate at {:?} has {gates} gate pieces",
                p.name,
                g.point
            );
            for (&(vx, vy), &n) in &degree {
                let border = vx == 0 || vy == 0 || vx == w.w || vy == w.h;
                assert!(
                    border || n >= 2,
                    "{}: loose wall end at local ({vx}, {vy})",
                    p.name
                );
            }
        }
    }
}

#[test]
fn blocks_resolve_and_render_with_the_placeholder_library() {
    let lib = Library::load(&library_dir()).unwrap();
    for p in plans() {
        let blk = block::generate(p, market_window(p, 32, 32)).unwrap();
        let res = fallback::resolve(&blk, &lib, p.seed);
        res.layout.check(&lib).unwrap();
        // The full placeholder library covers most of the vocabulary, so
        // fallbacks may be empty; any that remain substitute something.
        for f in &res.fallbacks {
            assert!(f.count > 0 && f.wanted != f.used);
        }
        let img = arda_tactical::render(
            &res.layout,
            &lib,
            p.seed,
            &RenderOptions {
                ppsq: 8,
                grid: false,
                lighting: true,
            },
        )
        .unwrap();
        assert_eq!((img.width, img.height), (32 * 8, 32 * 8));
    }
}

#[test]
fn interiors_are_furnished_by_function() {
    let p = plan("aldermere");
    let inn = p
        .buildings
        .iter()
        .find(|b| b.function == arda_town::BuildingFunction::Inn)
        .unwrap();
    let r = inn.rect;
    let blk = block::generate(
        p,
        Window {
            x: r.x0 - 2,
            y: r.y0 - 2,
            w: r.w() + 4,
            h: r.h() + 4,
        },
    )
    .unwrap();
    let bb = blk.buildings.iter().find(|b| b.id == inn.id).unwrap();
    let rooms: BTreeSet<&str> = bb.rooms.iter().map(|r| r.0.as_str()).collect();
    for want in ["common_room", "kitchen", "stair_hall"] {
        assert!(rooms.contains(want), "inn rooms {rooms:?}");
    }
    let ids: BTreeSet<String> = blk
        .layout
        .placements
        .iter()
        .filter_map(|pl| match &pl.asset {
            AssetRef::Id(id) => Some(id.clone()),
            AssetRef::Query { .. } => None,
        })
        .collect();
    for want in [
        "prop.bar_counter",
        "prop.hearth",
        "prop.oven",
        "prop.stairs",
        "prop.table",
    ] {
        assert!(ids.contains(want), "inn lacks {want}");
    }
    assert!(!blk.layout.lights.is_empty());
    let smithy = p
        .buildings
        .iter()
        .find(|b| b.function == arda_town::BuildingFunction::Smithy)
        .unwrap();
    let r = smithy.rect;
    let blk = block::generate(
        p,
        Window {
            x: r.x0,
            y: r.y0,
            w: r.w(),
            h: r.h(),
        },
    )
    .unwrap();
    assert!(blk
        .layout
        .placements
        .iter()
        .any(|pl| pl.asset == AssetRef::Id("prop.forge".into())));
    let temple = p
        .buildings
        .iter()
        .find(|b| b.function == arda_town::BuildingFunction::Temple)
        .unwrap();
    let r = temple.rect;
    let blk = block::generate(
        p,
        Window {
            x: r.x0,
            y: r.y0,
            w: r.w(),
            h: r.h(),
        },
    )
    .unwrap();
    let rooms: BTreeSet<String> = blk
        .buildings
        .iter()
        .find(|b| b.id == temple.id)
        .unwrap()
        .rooms
        .iter()
        .map(|r| r.0.clone())
        .collect();
    assert!(rooms.contains("nave"), "{rooms:?}");
    assert!(blk
        .layout
        .placements
        .iter()
        .any(|pl| pl.asset == AssetRef::Id("prop.pew".into())));
}

/// Adapter A8 (logic/12 §scene-sidecar): a town block's rules are
/// `arda-scene`'s format 2, stated only on squares the town reserves, with
/// building ids in `ext`; the layout carries its world origin (I10) and
/// yields a scene.
#[test]
fn town_rules_are_scene_sidecar_format_2() {
    let lib = Library::load(&library_dir()).unwrap();
    for p in plans() {
        let win = market_window(p, 32, 32);
        let blk = block::generate(p, win).unwrap();
        assert_eq!(blk.layout.origin, Some([win.x, win.y]));
        let r = blk.to_rules();
        assert_eq!((r.format_version, r.width, r.height), (2, 32, 32));
        assert!(blk.owned.iter().any(|o| *o), "{}", p.name);
        for (cell, own) in r.squares.iter().zip(&blk.owned) {
            assert_eq!(cell.deck.is_some(), *own);
        }
        assert!(r
            .squares
            .iter()
            .any(|c| c.ext.as_ref().is_some_and(|e| e.contains_key("building"))));
        let res = fallback::resolve(&blk, &lib, p.seed);
        arda_scene::build_scene(&res.layout, &lib, p.seed, Some(&r)).unwrap();
    }
}

/// Unpaved street squares around the market, and how many are mud.
fn street_mud(p: &TownPlan, poor: bool) -> (usize, usize) {
    use arda_town::block::ground;
    use arda_town::plan::grid::Kind;
    let win = market_window(p, 160, 160);
    let (mut streets, mut mud) = (0, 0);
    for y in win.y..win.y + win.h {
        for x in win.x..win.x + win.w {
            if ground::kind(p, x, y) != Kind::Street {
                continue;
            }
            let key = ground::at(p, poor, x, y).key;
            if key == "cobbles" {
                continue;
            }
            streets += 1;
            mud += usize::from(key == "mud");
        }
    }
    (streets, mud)
}

#[test]
fn only_poor_towns_have_a_little_mud_on_their_streets() {
    let mut any_poor_mud = false;
    for p in plans() {
        let (streets, mud) = street_mud(p, false);
        assert_eq!(mud, 0, "{}: mud on an ordinary town's streets", p.name);
        let (streets_poor, mud_poor) = street_mud(p, true);
        assert_eq!(streets, streets_poor);
        assert!(
            mud_poor * 8 < streets_poor.max(1),
            "{}: {mud_poor} of {streets_poor} poor street squares are mud",
            p.name
        );
        any_poor_mud |= mud_poor > 0;
    }
    assert!(any_poor_mud, "poor streets keep some mud");
    let mut poor = plan("thornby").clone();
    for b in &mut poor.buildings {
        b.wealth = 40;
    }
    assert!(arda_town::block::ground::poor(&poor));
}

/// Yards, gardens and croft parcels each take one ground to their fences
/// and hedges (no noise contours or one-square stripes that break into
/// ragged blobs once borders blend).
#[test]
fn yards_gardens_and_croft_parcels_have_one_ground_each() {
    use arda_town::block::ground;
    use arda_town::plan::croft::{parcel, rim};
    use arda_town::plan::grid::Kind;
    for p in plans() {
        let g = &p.grid;
        let mut gardens: BTreeMap<u32, BTreeSet<&str>> = BTreeMap::new();
        let mut parcels: BTreeMap<u64, BTreeSet<&str>> = BTreeMap::new();
        for j in 0..g.h {
            for i in 0..g.w {
                let (x, y) = (g.gx0 + i, g.gy0 + j);
                let k = g.gidx(x, y).unwrap();
                let key = ground::at(p, false, x, y).key;
                match g.kind[k] {
                    Kind::Yard => assert_eq!(key, "packed_earth", "{}: yard {x},{y}", p.name),
                    Kind::Garden => {
                        gardens.entry(g.plot[k]).or_default().insert(key);
                    }
                    Kind::Croft
                        if !rim(p.seed, |x, y| ground::kind(p, x, y) == Kind::Open, x, y) =>
                    {
                        parcels.entry(parcel(p.seed, x, y)).or_default().insert(key);
                    }
                    _ => {}
                }
            }
        }
        for (plot, keys) in &gardens {
            assert_eq!(keys.len(), 1, "{}: garden of plot {plot}: {keys:?}", p.name);
        }
        for (id, keys) in &parcels {
            assert_eq!(keys.len(), 1, "{}: croft parcel {id}: {keys:?}", p.name);
        }
    }
}

/// Yards are dressed for their building's trade, sparsely: a few distinct
/// pieces that never touch one another and never stand on a doorway's
/// path, on the building's own yard.
#[test]
fn yards_hold_a_few_distinct_props_clear_of_doors() {
    use arda_town::block::ground::kind;
    use arda_town::block::interior::GProp;
    use arda_town::block::yard::{working, YARD_MOST};
    use arda_town::plan::grid::Kind;
    let mut dressed = 0;
    for p in plans() {
        let clear: BTreeSet<(i64, i64)> = p
            .buildings
            .iter()
            .flat_map(|b| b.doors.iter())
            .flat_map(|d| {
                let (ox, oy) = d.outside();
                let (sx, sy) = d.side.step();
                [(ox, oy), (ox + sx, oy + sy), (ox + 2 * sx, oy + 2 * sy)]
            })
            .collect();
        for b in &p.buildings {
            let mut out: Vec<GProp> = Vec::new();
            working(p, b, &mut out);
            assert!(
                out.len() <= YARD_MOST,
                "{}: {} yard props",
                p.name,
                out.len()
            );
            let ids: BTreeSet<String> = out.iter().map(|g| format!("{:?}", g.want)).collect();
            assert_eq!(ids.len(), out.len(), "{}: a repeated yard prop", p.name);
            #[allow(clippy::cast_possible_truncation)]
            let squares = |g: &GProp| {
                let e = g.extent.map(|v| v.round() as i64);
                (e[1]..e[3]).flat_map(move |y| (e[0]..e[2]).map(move |x| (x, y)))
            };
            for (i, g) in out.iter().enumerate() {
                for (x, y) in squares(g) {
                    assert_eq!(kind(p, x, y), Kind::Yard, "{}: prop off the yard", p.name);
                    assert!(!clear.contains(&(x, y)), "{}: prop in a doorway", p.name);
                }
                for o in &out[i + 1..] {
                    let (a, c) = (g.extent, o.extent);
                    let apart = a[0] > c[2] || c[0] > a[2] || a[1] > c[3] || c[1] > a[3];
                    assert!(apart, "{}: yard props touch", p.name);
                }
            }
            dressed += usize::from(!out.is_empty());
        }
    }
    assert!(dressed > 10, "only {dressed} yards dressed");
}
