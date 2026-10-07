//! Variant families: `.alt<N>` takes picked per placement, pinned takes,
//! hashed turns for `rot_free` assets, seam stability and the unchanged
//! placeholder output.
#![allow(clippy::cast_precision_loss)]

use super::variants::fits;
use super::*;
use crate::catalog::{AssetClass, WallRole};
use crate::layout::{AssetRef, EdgeAxis, Placement, WallSegment};
use crate::layouts;
use crate::library::family_base;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(super) fn placeholder() -> Library {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
    Library::load(&dir).unwrap()
}

/// The placeholder plus `takes` extra takes of each id in `ids`, each
/// tinted differently, with `extra_tags` added to the whole family.
pub(super) fn with_takes(ids: &[&str], takes: u32, extra_tags: &[&str]) -> Library {
    let base = placeholder();
    let mut cat = base.catalog.clone();
    let mut images: BTreeMap<String, Rgba> = cat
        .assets
        .iter()
        .map(|a| (a.id.clone(), base.image(&a.id).unwrap().clone()))
        .collect();
    for id in ids {
        let a = cat.assets.iter_mut().find(|a| a.id == *id).unwrap();
        a.tags
            .free
            .extend(extra_tags.iter().map(|t| (*t).to_string()));
        let a = a.clone();
        for k in 1..=takes {
            let mut alt = a.clone();
            alt.id = format!("{id}.alt{k}");
            alt.image = format!("test/{}.png", alt.id);
            let mut img = base.image(id).unwrap().clone();
            for px in img.data.as_chunks_mut::<4>().0 {
                if px[3] > 0 {
                    px[k as usize % 3] = px[k as usize % 3].saturating_add(60);
                }
            }
            images.insert(alt.id.clone(), img);
            cat.assets.push(alt);
        }
    }
    Library::from_parts(cat, images).unwrap()
}

pub(super) fn put(l: &mut TacticalLayout, asset: AssetRef, x: f32, y: f32) {
    l.placements.push(Placement {
        asset,
        x,
        y,
        rotation: 0,
        mirror: false,
    });
}

/// A `w × h` grass layout with one barrel per square.
fn barrels(w: u32, h: u32, asset: &AssetRef) -> TacticalLayout {
    let mut l = TacticalLayout::new("barrels", w, h, "grass");
    for y in 0..h {
        for x in 0..w {
            put(&mut l, asset.clone(), x as f32 + 0.5, y as f32 + 0.5);
        }
    }
    l
}

pub(super) fn picks(lib: &Library, l: &TacticalLayout, seed: u64) -> Vec<(String, u8, bool)> {
    (0..l.placements.len())
        .map(|i| {
            let r = resolve(lib, l, i, seed).unwrap();
            (r.asset.id.clone(), r.turns, r.mirror)
        })
        .collect()
}

pub(super) fn plain(ppsq: u32) -> RenderOptions {
    RenderOptions {
        ppsq,
        grid: false,
        lighting: false,
    }
}

#[test]
fn family_base_strips_only_a_numbered_take_suffix() {
    assert_eq!(family_base("prop.barrel.alt3"), "prop.barrel");
    assert_eq!(family_base("prop.barrel.alt12"), "prop.barrel");
    assert_eq!(family_base("wall.stone.run.alt1"), "wall.stone.run");
    for id in [
        "prop.barrel",
        "prop.altar",
        "prop.barrel.alt",
        "prop.x.altx",
        ".alt1",
    ] {
        assert_eq!(family_base(id), id);
    }
}

#[test]
fn ids_resolve_to_their_family_and_takes_pin() {
    let lib = with_takes(&["prop.barrel"], 3, &[]);
    let fam: Vec<_> = candidates(&lib, &AssetRef::Id("prop.barrel".into()))
        .iter()
        .map(|a| a.id.clone())
        .collect();
    assert_eq!(
        fam,
        [
            "prop.barrel",
            "prop.barrel.alt1",
            "prop.barrel.alt2",
            "prop.barrel.alt3"
        ]
    );
    let pinned = AssetRef::Id("prop.barrel.alt2".into());
    let l = barrels(16, 16, &pinned);
    l.check(&lib).unwrap();
    for (id, turns, mirror) in picks(&lib, &l, 5) {
        assert_eq!((id.as_str(), turns, mirror), ("prop.barrel.alt2", 0, false));
    }
}

#[test]
fn variant_picks_are_deterministic() {
    let lib = with_takes(&["prop.barrel"], 3, &["rot_free"]);
    let l = barrels(12, 12, &AssetRef::Id("prop.barrel".into()));
    assert_eq!(picks(&lib, &l, 9), picks(&lib, &l, 9));
    assert_ne!(picks(&lib, &l, 9), picks(&lib, &l, 10), "seed sensitive");
    let a = render(&l, &lib, 9, &plain(16)).unwrap();
    let b = render(&l, &lib, 9, &plain(16)).unwrap();
    assert_eq!(blake3::hash(&a.data), blake3::hash(&b.data));
}

#[test]
fn a_field_of_barrels_uses_every_take() {
    let lib = with_takes(&["prop.barrel"], 3, &[]);
    let mut l = barrels(64, 64, &AssetRef::Id("prop.barrel".into()));
    for origin in [None, Some([4096, -300])] {
        l.origin = origin;
        let mut count: BTreeMap<String, usize> = BTreeMap::new();
        for (id, turns, mirror) in picks(&lib, &l, 1) {
            assert_eq!((turns, mirror), (0, false), "untagged barrels never turn");
            *count.entry(id).or_default() += 1;
        }
        assert_eq!(count.len(), 4, "{count:?}");
        // 4096 barrels over four takes: each near 1024.
        assert!(
            count.values().all(|&n| (850..1200).contains(&n)),
            "{count:?}"
        );
    }
    // Rendered, the field is not one barrel repeated.
    let small = barrels(8, 8, &AssetRef::Id("prop.barrel".into()));
    let one = with_takes(&["prop.barrel"], 0, &[]);
    let varied = render(&small, &lib, 1, &plain(16)).unwrap();
    let plain_render = render(&small, &one, 1, &plain(16)).unwrap();
    assert_ne!(varied.data, plain_render.data);
}

#[test]
fn queries_include_takes() {
    let lib = with_takes(&["prop.barrel"], 3, &[]);
    let q = AssetRef::Query {
        class: Some(AssetClass::Prop),
        tags: vec!["function:inn".into()],
    };
    let l = barrels(32, 32, &q);
    let used: BTreeSet<String> = picks(&lib, &l, 2).into_iter().map(|p| p.0).collect();
    for k in 1..=3 {
        assert!(used.contains(&format!("prop.barrel.alt{k}")), "{used:?}");
    }
    assert!(used.contains("prop.table"), "{used:?}");
}

#[test]
fn rot_free_assets_turn_and_mirror_by_hash() {
    let lib = with_takes(&["prop.barrel", "prop.cart"], 1, &["rot_free"]);
    let mut l = barrels(16, 16, &AssetRef::Id("prop.barrel".into()));
    let turns: BTreeSet<(u8, bool)> = picks(&lib, &l, 4).into_iter().map(|p| (p.1, p.2)).collect();
    let barrel = lib.asset("prop.barrel").unwrap();
    let want = barrel.rotations.len() * if barrel.mirror { 2 } else { 1 };
    assert_eq!(turns.len(), want, "{turns:?}");
    // An explicit rotation or mirror is kept.
    for p in &mut l.placements {
        p.rotation = 90;
    }
    if barrel.rotations.contains(&90) {
        assert!(picks(&lib, &l, 4).iter().all(|p| (p.1, p.2) == (1, false)));
    }
    // Untagged assets never turn.
    let crates = barrels(16, 16, &AssetRef::Id("prop.crate".into()));
    assert!(picks(&lib, &crates, 4)
        .iter()
        .all(|p| (p.1, p.2) == (0, false)));
    // A non-square footprint only half-turns, keeping its shape.
    let cart = lib.asset("prop.cart").unwrap();
    assert_ne!(cart.footprint.w, cart.footprint.h);
    let carts = barrels(16, 16, &AssetRef::Id("prop.cart".into()));
    assert!(picks(&lib, &carts, 4).iter().all(|p| p.1 % 2 == 0));
}

#[test]
fn wall_takes_are_picked_per_edge() {
    let lib = with_takes(&["wall.stone.run"], 2, &[]);
    assert_eq!(lib.wall_pieces("stone", WallRole::Run).len(), 3);
    let mut l = TacticalLayout::new("w", 24, 3, "grass");
    for x in 0..24 {
        l.walls.push(WallSegment {
            x,
            y: 1,
            axis: EdgeAxis::Horizontal,
            kind: WallRole::Run,
            kit: "stone".into(),
            tags: Vec::new(),
        });
    }
    let varied = render(&l, &lib, 6, &plain(16)).unwrap();
    let flat = render(&l, &placeholder(), 6, &plain(16)).unwrap();
    assert_ne!(varied.data, flat.data, "takes are drawn on some edges");
}

/// A world full of barrels, trees and a wall to cut windows from.
fn world(w: u32, h: u32) -> TacticalLayout {
    let mut l = TacticalLayout::new("world", w, h, "grass");
    for y in 0..h {
        for x in 0..w {
            let id = if (x + y) % 5 == 0 {
                "veg.bush"
            } else {
                "prop.barrel"
            };
            put(
                &mut l,
                AssetRef::Id(id.into()),
                x as f32 + 0.5,
                y as f32 + 0.5,
            );
        }
    }
    for x in 0..w {
        l.walls.push(WallSegment {
            x,
            y: 4,
            axis: EdgeAxis::Horizontal,
            kind: WallRole::Run,
            kit: "stone".into(),
            tags: Vec::new(),
        });
    }
    l
}

/// Squares `[x0, x0 + w)` of `l` with a world origin; placements and walls
/// inside are listed in reverse, so their indices differ between windows.
pub(super) fn window(l: &TacticalLayout, x0: u32, w: u32) -> TacticalLayout {
    let mut out = TacticalLayout::new("window", w, l.height, "grass");
    out.origin = Some([1000 + i64::from(x0), -40]);
    let inside = |x: f32| (x0 as f32..(x0 + w) as f32).contains(&x);
    for p in l.placements.iter().rev().filter(|p| inside(p.x)) {
        let mut p = p.clone();
        p.x -= x0 as f32;
        out.placements.push(p);
    }
    for s in l.walls.iter().rev().filter(|s| inside(s.x as f32)) {
        let mut s = s.clone();
        s.x -= x0;
        out.walls.push(s);
    }
    out
}

#[test]
fn takes_and_turns_are_seam_stable_across_windows() {
    let lib = with_takes(
        &["prop.barrel", "veg.bush", "wall.stone.run"],
        3,
        &["rot_free"],
    );
    let big = world(36, 10);
    let ppsq = 16;
    // Windows overlapping by twelve squares, their placements listed in
    // a different order. World column 18 is column 18 of `a` and 6 of `b`,
    // six squares from both windows' inner edges (see
    // `adjacent_windows_join_seamlessly_with_an_origin`).
    let a = render(&window(&big, 0, 24), &lib, 3, &plain(ppsq)).unwrap();
    let b = render(&window(&big, 12, 24), &lib, 3, &plain(ppsq)).unwrap();
    let (mut same, mut total) = (0, 0);
    for y in 0..10 * ppsq {
        for x in 0..ppsq {
            total += 1;
            same += usize::from(a.get(18 * ppsq + x, y) == b.get(6 * ppsq + x, y));
        }
    }
    assert_eq!(same, total, "{same} of {total} shared pixels match");
}

#[test]
fn placeholder_libraries_resolve_exactly_as_before() {
    let lib = placeholder();
    for a in &lib.catalog.assets {
        assert_eq!(family_base(&a.id), a.id, "placeholder has no takes");
    }
    for mut l in layouts::all() {
        for origin in [None, Some([77, -5])] {
            l.origin = origin;
            for (i, p) in l.placements.iter().enumerate() {
                let r = resolve(&lib, &l, i, 11).unwrap();
                // The pre-variant rule: an id is one asset; a query picks
                // among its matches by seed and index; the layout's turn.
                let legacy = match &p.asset {
                    AssetRef::Id(id) => lib.asset(id).unwrap(),
                    AssetRef::Query { class, tags } => {
                        let all = lib.query(*class, tags);
                        let n = all.len() as u64;
                        let k = crate::noise::hash2(11 ^ 0x9E50, i64::try_from(i).unwrap(), 0) % n;
                        all[usize::try_from(k).unwrap()]
                    }
                };
                assert_eq!(r.asset.id, legacy.id, "{} placement {i}", l.name);
                assert_eq!((u16::from(r.turns) * 90, r.mirror), (p.rotation, p.mirror));
                assert_eq!((r.scale_pct, r.transpose), (100, false), "no hashed pose");
                assert!(fits(r.asset, p.rotation, p.mirror));
            }
        }
    }
}

#[test]
fn an_upper_layer_family_replaces_the_lower_one() {
    let base = placeholder();
    let rich = with_takes(&["prop.barrel"], 2, &[]);
    // A top layer holding only takes of the barrel (no plain `prop.barrel`).
    let mut cat = rich.catalog.clone();
    cat.library = "imported".into();
    cat.assets.retain(|a| a.id.starts_with("prop.barrel.alt"));
    let images = cat
        .assets
        .iter()
        .map(|a| (a.id.clone(), rich.image(&a.id).unwrap().clone()))
        .collect();
    let top = Library::from_parts(cat, images).unwrap();
    let merged = Library::layered(&[top, base]).unwrap();
    let fam: Vec<_> = merged
        .family("prop.barrel")
        .iter()
        .map(|a| a.id.clone())
        .collect();
    assert_eq!(fam, ["prop.barrel.alt1", "prop.barrel.alt2"]);
    let l = barrels(8, 8, &AssetRef::Id("prop.barrel".into()));
    l.check(&merged).unwrap();
    render(&l, &merged, 1, &plain(16)).unwrap();
}
