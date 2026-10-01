//! The town WFC (goal 44, logic/10 §town-interiors): legal adjacency,
//! every square reachable from the door, function-specific tiles,
//! determinism, seams and the relaxed-fill rate on the synthetic plans.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    missing_docs
)]

mod common;

use arda_tactical::catalog::WallRole;
use arda_tactical::layout::{AssetRef, EdgeAxis};
use arda_town::block::frame::Want;
use arda_town::block::wfc::access;
use arda_town::block::wfc::furnish::Frame as Grid;
use arda_town::block::wfc::{indoor, outdoor, report, rooms, Report, TownFill};
use arda_town::block::{self, Window};
use arda_town::plan::{Building, TownPlan};
use arda_town::BuildingFunction as F;
use common::plans;
use std::collections::{BTreeMap, BTreeSet};

/// Every walled building of every sample, solved by WFC (not relaxed).
fn solved() -> Vec<(&'static TownPlan, &'static Building)> {
    plans()
        .iter()
        .flat_map(|p| p.buildings.iter().map(move |b| (p, b)))
        .filter(|(p, b)| matches!(indoor::solve(p, b), Ok(Some(_))))
        .collect()
}

#[test]
fn room_and_furniture_tiles_are_legal() {
    let all = solved();
    assert!(all.len() > 500, "{} solved interiors", all.len());
    for (p, b) in all {
        let s = indoor::solve(p, b).unwrap().unwrap();
        let (w, d) = if b.front.turns() % 2 == 0 {
            (b.rect.w(), b.rect.h())
        } else {
            (b.rect.h(), b.rect.w())
        };
        assert_eq!(rooms::violations(&s.layout, w, d), 0, "{} rooms", b.id.0);
        let grid = Grid {
            w,
            d,
            layout: &s.layout,
            doors: &s.doors,
        };
        let (v, f) = &s.furnished;
        assert_eq!(
            access::violations(v, &grid, &f.tiles),
            0,
            "{} furniture",
            b.id.0
        );
        assert!(access::reachable(v, &grid, &f.tiles), "{}", b.id.0);
    }
}

#[test]
fn outdoor_tiles_are_legal_within_and_across_chunks() {
    for p in plans() {
        let site = outdoor::Site::of(p);
        let (gx, gy) = p.origin();
        let c = outdoor::CHUNK;
        let (cx0, cy0) = (gx.div_euclid(c) + 1, gy.div_euclid(c) + 1);
        let mut chunks = BTreeMap::new();
        for cy in cy0..cy0 + 4 {
            for cx in cx0..cx0 + 4 {
                let ch = outdoor::chunk(p, &site, cx, cy);
                assert_eq!(outdoor::violations(&ch), 0, "{} chunk {cx},{cy}", p.name);
                assert!(!ch.relaxed || !ch.any, "{} chunk {cx},{cy} relaxed", p.name);
                chunks.insert((cx, cy), ch);
            }
        }
        for (&(cx, cy), ch) in &chunks {
            if let Some(e) = chunks.get(&(cx + 1, cy)) {
                assert_eq!(
                    outdoor::seam_violations(ch, e, true),
                    0,
                    "{} {cx},{cy} east",
                    p.name
                );
            }
            if let Some(s) = chunks.get(&(cx, cy + 1)) {
                assert_eq!(
                    outdoor::seam_violations(ch, s, false),
                    0,
                    "{} {cx},{cy} south",
                    p.name
                );
            }
        }
    }
}

/// Props that people can step over or past (seats, rugs, hangings).
fn passable(want: &Want) -> bool {
    matches!(
        want,
        Want::Id("prop.chair" | "prop.stool" | "prop.bench" | "prop.rug_small" | "prop.banner")
    )
}

#[test]
fn every_room_and_furniture_square_is_reachable_from_the_door() {
    for (p, b) in solved() {
        let int = indoor::build(p, b).interior;
        let r = b.rect;
        let mut blocked = BTreeSet::new();
        for pr in int.props.iter().filter(|pr| !passable(&pr.want)) {
            let (x0, y0) = (pr.extent[0].round() as i64, pr.extent[1].round() as i64);
            let (x1, y1) = (pr.extent[2].round() as i64, pr.extent[3].round() as i64);
            for y in y0..y1 {
                for x in x0..x1 {
                    blocked.insert((x, y));
                }
            }
        }
        let wall: BTreeMap<(i64, i64, EdgeAxis), WallRole> =
            int.walls.iter().map(|&(k, (role, _))| (k, role)).collect();
        let shut = |k: (i64, i64, EdgeAxis)| {
            wall.get(&k)
                .is_some_and(|r| !matches!(r, WallRole::Door | WallRole::Gate))
        };
        let step = |a: (i64, i64), b: (i64, i64)| {
            let k = if a.0 == b.0 {
                (a.0, a.1.max(b.1), EdgeAxis::Horizontal)
            } else {
                (a.0.max(b.0), a.1, EdgeAxis::Vertical)
            };
            r.contains(b.0, b.1) && !shut(k)
        };
        let mut seen = BTreeSet::new();
        let mut stack: Vec<(i64, i64)> = b
            .doors
            .iter()
            .map(|d| (d.x, d.y))
            .filter(|c| !blocked.contains(c))
            .collect();
        assert!(!stack.is_empty(), "{}: every door is blocked", b.id.0);
        while let Some(c) = stack.pop() {
            if !seen.insert(c) {
                continue;
            }
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let n = (c.0 + dx, c.1 + dy);
                if !blocked.contains(&n) && step(c, n) && !seen.contains(&n) {
                    stack.push(n);
                }
            }
        }
        for y in r.y0..r.y1 {
            for x in r.x0..r.x1 {
                if !blocked.contains(&(x, y)) {
                    assert!(
                        seen.contains(&(x, y)),
                        "{} {:?}: square {x},{y} unreachable",
                        b.id.0,
                        b.function
                    );
                }
            }
        }
        for pr in int.props.iter().filter(|pr| !passable(&pr.want)) {
            let (x0, y0) = (pr.extent[0].round() as i64, pr.extent[1].round() as i64);
            let (x1, y1) = (pr.extent[2].round() as i64, pr.extent[3].round() as i64);
            let usable = (y0..y1).any(|y| {
                (x0..x1).any(|x| {
                    [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| {
                        seen.contains(&(x + dx, y + dy)) && step((x, y), (x + dx, y + dy))
                    })
                })
            });
            assert!(
                usable,
                "{} {:?}: {:?} out of reach",
                b.id.0, b.function, pr.want
            );
        }
    }
}

fn ids(b: &Building, p: &TownPlan) -> BTreeSet<String> {
    indoor::build(p, b)
        .interior
        .props
        .iter()
        .filter_map(|pr| match &pr.want {
            Want::Id(id) => Some((*id).to_string()),
            Want::Query(_) => None,
        })
        .collect()
}

#[test]
fn buildings_hold_their_function_tiles() {
    let want: &[(F, &[&str])] = &[
        (
            F::Inn,
            &["prop.hearth", "prop.oven", "prop.stairs", "prop.table"],
        ),
        (F::Tavern, &["prop.hearth", "prop.oven", "prop.table"]),
        (F::Smithy, &["prop.forge", "prop.anvil"]),
        (F::Temple, &["prop.altar"]),
        // Poor households may keep an open fire pit (goal 64).
        (F::House, &["prop.bed", "prop.hearth|prop.brazier"]),
        (F::Cottage, &["prop.bed", "prop.hearth|prop.brazier"]),
        (F::Farmhouse, &["prop.bed", "prop.hearth", "prop.hay_bale"]),
        (F::Mill, &["prop.millstone"]),
        (F::Keep, &["prop.throne", "prop.weapon_rack", "prop.bed"]),
    ];
    let mut seen: BTreeMap<F, BTreeSet<String>> = BTreeMap::new();
    let mut count: BTreeMap<F, usize> = BTreeMap::new();
    for (p, b) in solved() {
        let got = ids(b, p);
        if let Some((_, need)) = want.iter().find(|(f, _)| *f == b.function) {
            for id in *need {
                assert!(
                    id.split('|').any(|one| got.contains(one)),
                    "{} {:?} lacks {id}: {got:?}",
                    b.id.0,
                    b.function
                );
            }
            *count.entry(b.function).or_default() += 1;
        }
        seen.entry(b.function).or_default().extend(got);
    }
    for f in [F::Inn, F::Smithy, F::Temple, F::House] {
        assert!(count.get(&f).copied().unwrap_or(0) > 0, "no solved {f:?}");
    }
    // Across the samples, inns have a bar and temples have pews.
    assert!(
        seen[&F::Inn].contains("prop.bar_counter"),
        "{:?}",
        seen[&F::Inn]
    );
    assert!(
        seen[&F::Temple].contains("prop.pew"),
        "{:?}",
        seen[&F::Temple]
    );
    // Room kinds follow the function.
    let (p, inn) = solved()
        .into_iter()
        .find(|(_, b)| b.function == F::Inn)
        .unwrap();
    let tags: BTreeSet<&str> = indoor::build(p, inn)
        .interior
        .rooms
        .iter()
        .map(|r| r.0)
        .collect();
    for t in ["common_room", "kitchen", "stair_hall"] {
        assert!(tags.contains(t), "inn rooms {tags:?}");
    }
    let (p, temple) = solved()
        .into_iter()
        .find(|(_, b)| b.function == F::Temple)
        .unwrap();
    let tags: BTreeSet<&str> = indoor::build(p, temple)
        .interior
        .rooms
        .iter()
        .map(|r| r.0)
        .collect();
    assert!(
        tags.contains("nave") && tags.contains("chancel"),
        "temple rooms {tags:?}"
    );
}

#[test]
fn wfc_output_is_deterministic() {
    for p in plans() {
        for b in p.buildings.iter().take(40) {
            let (a, c) = (indoor::build(p, b), indoor::build(p, b));
            assert_eq!(format!("{:?}", a.interior), format!("{:?}", c.interior));
        }
        let site = outdoor::Site::of(p);
        let (gx, gy) = p.origin();
        let (cx, cy) = (
            gx.div_euclid(outdoor::CHUNK) + 2,
            gy.div_euclid(outdoor::CHUNK) + 2,
        );
        let (a, c) = (
            outdoor::chunk(p, &site, cx, cy),
            outdoor::chunk(p, &site, cx, cy),
        );
        assert_eq!(a.tiles, c.tiles);
    }
}

#[test]
fn unaligned_blocks_join_without_seams_in_both_directions() {
    for p in plans() {
        let (ox, oy) = p.origin();
        let (w, h) = p.size();
        // Windows that cut through chunks and buildings alike.
        let big = Window {
            x: ox + w / 2 - 47,
            y: oy + h / 2 - 41,
            w: 80,
            h: 80,
        };
        let full = block::generate(p, big).unwrap();
        for (sx, sy) in [(37, 0), (0, 29)] {
            let a = Window {
                w: if sx > 0 { sx } else { 80 },
                h: if sy > 0 { sy } else { 80 },
                ..big
            };
            let b = Window {
                x: big.x + sx,
                y: big.y + sy,
                w: 80 - sx,
                h: 80 - sy,
            };
            let (ba, bb) = (
                block::generate(p, a).unwrap(),
                block::generate(p, b).unwrap(),
            );
            for (part, win) in [(&ba, a), (&bb, b)] {
                for y in 0..win.h {
                    for x in 0..win.w {
                        let s = &part.layout.squares[(y * win.w + x) as usize];
                        let g = &full.layout.squares
                            [((win.y - big.y + y) * big.w + (win.x - big.x + x)) as usize];
                        assert_eq!(s, g, "{} square {x},{y}", p.name);
                    }
                }
            }
            let props = |blk: &block::TownBlock| -> BTreeSet<(String, i64, i64)> {
                blk.layout
                    .placements
                    .iter()
                    .filter_map(|pl| match &pl.asset {
                        AssetRef::Id(id) => Some((
                            id.clone(),
                            (f64::from(pl.x) * 4.0).round() as i64 + blk.window.x * 4,
                            (f64::from(pl.y) * 4.0).round() as i64 + blk.window.y * 4,
                        )),
                        AssetRef::Query { .. } => None,
                    })
                    .collect()
            };
            let union: BTreeSet<_> = props(&ba).union(&props(&bb)).cloned().collect();
            assert_eq!(
                union,
                props(&full),
                "{}: props differ across the seam",
                p.name
            );
        }
    }
}

#[test]
fn relaxed_fill_rate_on_samples_is_below_one_percent() {
    let mut total = Report::default();
    for p in plans() {
        total.add(&report(p));
    }
    #[allow(clippy::cast_precision_loss)]
    let rate = total.relaxed() as f64 / total.problems().max(1) as f64;
    println!("{total:?} rate {rate}");
    assert!(total.problems() > 1000, "{total:?}");
    assert!(rate < 0.01, "{total:?}");
}

#[test]
fn the_rule_programmes_stay_available_as_the_other_fill() {
    let p = common::plan("aldermere");
    let inn = p.buildings.iter().find(|b| b.function == F::Inn).unwrap();
    let r = inn.rect;
    let win = Window {
        x: r.x0 - 2,
        y: r.y0 - 2,
        w: r.w() + 4,
        h: r.h() + 4,
    };
    let rules = block::generate_with(p, win, TownFill::Rules).unwrap();
    let wfc = block::generate_with(p, win, TownFill::Wfc).unwrap();
    assert_ne!(rules.to_json().unwrap(), wfc.to_json().unwrap());
    let bars = |b: &block::TownBlock| {
        b.layout
            .placements
            .iter()
            .any(|pl| pl.asset == AssetRef::Id("prop.bar_counter".into()))
    };
    assert!(bars(&rules));
    assert!(rules.relaxed.is_empty());
}
