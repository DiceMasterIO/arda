//! Worked land on relief tiles agrees with the tactical layers it is drawn
//! from (logic/17 §land), on synthetic inputs (no world needed): field
//! parcels and hedgerows against `arda-fields`, roads against `arda-ways`,
//! at one pixel per square.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    missing_docs
)]

use arda_fields::geom::{Sq, SQUARE_M};
use arda_fields::partition::PartitionInputs;
use arda_fields::synthetic;
use arda_midzoom::land::fields::Parcels;
use arda_midzoom::land::roads::{PixelGrid, RoadRaster};
use arda_people::terrain::Surroundings;
use arda_tactical::layout::{EdgeAxis, TacticalLayout};
use arda_ways::{Feature, Road, RoadClass};
use std::collections::{BTreeMap, BTreeSet};

const SEED: u64 = 7;

#[test]
fn parcels_and_hedgerows_are_the_tactical_fields() {
    let s = synthetic::hedge_country();
    let inputs = s.inputs();
    let win = arda_fields::generate(&inputs, s.origin_m, s.w, s.h, SEED).unwrap();
    let o = win.layout.origin.unwrap();
    let (w, h) = (i64::from(s.w), i64::from(s.h));
    // The parcels of a larger rectangle: the partition is global.
    let parcels = Parcels::build(
        &PartitionInputs::from(&inputs),
        SEED,
        (o[0] - 300, o[1] - 200, o[0] + w + 100, o[1] + h + 250),
    );
    let site = |x: i64, y: i64| {
        let sq = Sq::new(x, y);
        parcels
            .at(sq.centre())
            .and_then(|hit| parcels.site_key(hit.site))
    };
    let field = |x: i64, y: i64| {
        let i = usize::try_from((y - o[1]) * w + (x - o[0])).ok()?;
        win.sidecar.squares.get(i)?.field.clone()
    };
    // Every tactical field is one relief parcel.
    let mut sites_of: BTreeMap<String, BTreeSet<(i64, i64, u32)>> = BTreeMap::new();
    for y in o[1]..o[1] + h {
        for x in o[0]..o[0] + w {
            if let (Some(f), Some(k)) = (field(x, y), site(x, y)) {
                sites_of.entry(f).or_default().insert(k);
            }
        }
    }
    assert!(sites_of.len() > 20, "{} fields", sites_of.len());
    for (f, sites) in &sites_of {
        assert_eq!(sites.len(), 1, "field {f} spans relief parcels {sites:?}");
    }
    // Every wall between two fields is a relief parcel edge.
    let (mut walls, mut between, mut split) = (0, 0, 0);
    for wall in &win.layout.walls {
        if !matches!(wall.kit.as_str(), "hedge" | "drystone" | "wattle") {
            continue;
        }
        let (x, y) = (o[0] + i64::from(wall.x), o[1] + i64::from(wall.y));
        let (a, b) = match wall.axis {
            EdgeAxis::Vertical => ((x - 1, y), (x, y)),
            EdgeAxis::Horizontal => ((x, y - 1), (x, y)),
        };
        walls += 1;
        let (fa, fb) = (field(a.0, a.1), field(b.0, b.1));
        if fa.is_none() || fb.is_none() || fa == fb {
            continue;
        }
        between += 1;
        split += usize::from(site(a.0, a.1) != site(b.0, b.1));
    }
    println!("{walls} field walls, {between} between two fields, {split} on relief parcel edges");
    assert!(between > 500, "the window is hedged ({between})");
    assert_eq!(
        split, between,
        "walls between fields off the relief parcel edges"
    );
}

#[test]
fn parcels_join_exactly_across_windows() {
    let s = synthetic::hedge_country();
    let inputs = s.inputs();
    let o = [1300_i64, 1300];
    let a = Parcels::build(
        &PartitionInputs::from(&inputs),
        SEED,
        (o[0], o[1], o[0] + 200, o[1] + 200),
    );
    let b = Parcels::build(
        &PartitionInputs::from(&inputs),
        SEED,
        (o[0] + 150, o[1] + 120, o[0] + 400, o[1] + 300),
    );
    for y in o[1] + 120..o[1] + 200 {
        for x in o[0] + 150..o[0] + 200 {
            let q = [x as f64 + 0.3, y as f64 + 0.7];
            let key = |p: &Parcels| {
                p.at(q).map(|hit| {
                    let other = hit.other.and_then(|o| p.site_key(o));
                    (p.site_key(hit.site), other, hit.edge_m)
                })
            };
            assert_eq!(key(&a), key(&b), "square {x},{y}");
        }
    }
}

fn flat_surroundings(cells: i64) -> Surroundings {
    let n = usize::try_from(cells * cells).unwrap();
    Surroundings {
        origin: (0, 0),
        size: (cells, cells),
        height_m: vec![20.0; n],
        open_water: vec![false; n],
        edges: Vec::new(),
        pieces: Vec::new(),
        bins: vec![Vec::new(); n],
    }
}

#[test]
fn roads_cover_the_tactical_road_squares() {
    let roads = vec![
        Road {
            id: 1,
            class: RoadClass::Highway,
            segments: vec![vec![[200, 300], [700, 500], [1000, 450], [1400, 900]]],
            wealth: 128,
        },
        Road {
            id: 2,
            class: RoadClass::Track,
            segments: vec![vec![[700, 500], [800, 900], [600, 1300]]],
            wealth: 128,
        },
        Road {
            id: 3,
            class: RoadClass::Footpath,
            segments: vec![vec![[300, 1200], [900, 1100], [1300, 1250]]],
            wealth: 128,
        },
    ];
    let heights = flat_surroundings(20);
    let (gx0, gy0, side) = (192_i64, 192_i64, 640_u32);
    let mut layout = TacticalLayout::new("t", side, side, "grass");
    let terrain = arda_ways::FnTerrain {
        height: |_: f64, _: f64| 20.0,
        channels: Vec::new(),
    };
    let origin = [gx0 as f64 * SQUARE_M, gy0 as f64 * SQUARE_M];
    let out = arda_ways::apply_ways(&mut layout, origin, &roads, &[], &terrain, SEED).unwrap();
    let grid = PixelGrid {
        origin: (gx0, gy0),
        size: (side as usize, side as usize),
        pixel_m: SQUARE_M,
    };
    let raster = RoadRaster::build(grid, &roads, &heights, SEED, &|_, _| false);
    let (mut inter, mut union) = (0, 0);
    for (i, sq) in out.sidecar.squares.iter().enumerate() {
        let tactical = matches!(sq.feature, Feature::Road | Feature::Ruts);
        let relief = raster
            .class_at(i % side as usize, i / side as usize)
            .is_some();
        inter += usize::from(tactical && relief);
        union += usize::from(tactical || relief);
    }
    let iou = inter as f64 / union as f64;
    println!("road squares: {inter} shared of {union} (IoU {iou:.4})");
    assert!(union > 3_000);
    assert!(iou >= 0.97, "road IoU {iou:.3}");
}

#[test]
fn coarse_pixels_still_show_highways_but_not_footpaths() {
    let roads = vec![
        Road {
            id: 1,
            class: RoadClass::Highway,
            segments: vec![vec![[0, 500], [2000, 520]]],
            wealth: 128,
        },
        Road {
            id: 2,
            class: RoadClass::Footpath,
            segments: vec![vec![[0, 1500], [2000, 1520]]],
            wealth: 128,
        },
    ];
    let heights = flat_surroundings(24);
    let grid = PixelGrid {
        origin: (0, 0),
        size: (64, 64),
        pixel_m: 30.0,
    };
    let raster = RoadRaster::build(grid, &roads, &heights, SEED, &|_, _| false);
    let base = [100_u8, 120, 60];
    let mut highway = 0;
    let mut footpath = 0;
    for x in 0..64 {
        highway += usize::from(raster.paint(base, x, 16) != base);
        footpath += usize::from(raster.paint(base, x, 50) != base);
    }
    assert!(highway > 50, "{highway}");
    assert_eq!(footpath, 0);
}
