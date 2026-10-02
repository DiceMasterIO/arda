//! The field partition (logic/17 §land-fields): deterministic and exact
//! across windows, and shaped like historical field systems rather than
//! Voronoi cells — mostly four-sided fields, T-junctions, a spread of
//! elongations, and fields growing larger and more varied away from the
//! settlement.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    missing_docs
)]

#[path = "support/shape.rs"]
mod shape;

use arda_fields::fields::FieldKind;
use arda_fields::geom::{Sq, SQUARE_M};
use arda_fields::partition::{Partition, PartitionInputs};
use arda_fields::synthetic::{self, Scenario};

const SEED: u64 = 7;

fn partition(s: &Scenario, rect: (i64, i64, i64, i64)) -> Partition {
    let inputs = s.inputs();
    Partition::new(&PartitionInputs::from(&inputs), SEED, rect)
}

/// A `side`-square raster of field keys centred on world point `c_m`.
fn raster(s: &Scenario, c_m: [f64; 2], side: i64) -> (shape::Ids, Partition) {
    let x0 = (c_m[0] / SQUARE_M) as i64 - side / 2;
    let y0 = (c_m[1] / SQUARE_M) as i64 - side / 2;
    let p = partition(s, (x0, y0, x0 + side, y0 + side));
    // Parcel ids are site indices: one partition, one numbering.
    let ids = shape::Ids::new(side, side, |x, y| {
        p.locate(Sq::new(x0 + x, y0 + y).centre()).map(|k| k as u64)
    });
    (ids, p)
}

#[test]
fn the_partition_is_deterministic_and_joins_exactly() {
    let s = synthetic::hedge_country();
    let a = partition(&s, (1100, 1100, 1500, 1400));
    let b = partition(&s, (1350, 1250, 1900, 1800));
    let again = partition(&s, (1100, 1100, 1500, 1400));
    assert_eq!(a.sites, again.sites, "same inputs, same fields");
    let mut checked = 0;
    for y in (1250..1400).step_by(3) {
        for x in (1350..1500).step_by(3) {
            let q = [f64::from(x) + 0.37, f64::from(y) + 0.71];
            let key = |p: &Partition| {
                p.locate_edge(q).map(|h| {
                    let site = &p.sites[h.site];
                    let other = h.other.map(|o| p.sites[o].key);
                    (site.key, site.kind, site.crop, other, h.edge.to_bits())
                })
            };
            assert_eq!(key(&a), key(&b), "point {q:?}");
            assert!(key(&a).is_some());
            checked += 1;
        }
    }
    assert!(checked > 2_000);
}

#[test]
fn locate_and_locate_edge_agree() {
    let s = synthetic::village_strips();
    let p = partition(&s, (3200, 2900, 3500, 3200));
    for y in (2900..3200).step_by(7) {
        for x in (3200..3500).step_by(5) {
            let q = [f64::from(x) + 0.5, f64::from(y) + 0.5];
            let h = p.locate_edge(q).unwrap();
            assert_eq!(Some(h.site), p.locate(q));
            assert!(h.edge >= 0.0);
            // Just across the nearest edge lies the other field.
            if let Some(o) = h.other {
                assert_ne!(o, h.site);
            }
        }
    }
}

#[test]
fn fields_look_like_enclosure_not_voronoi() {
    let s = synthetic::hedge_country();
    let (ids, _) = raster(&s, [1500.0, 2600.0], 1024);
    let (sum, _) = shape::summary(&ids, 320);
    println!("hedge country: {sum:?}, T share {:.2}", sum.t_share());
    assert!(sum.n >= 120, "{} fields", sum.n);
    // Voronoi cells have five to seven sides and Y-junctions (the old
    // partition: 5 % four-sided, T share 0.50 here and 0.22 by a village).
    assert!(sum.four_sided >= 0.4, "four-sided {:.2}", sum.four_sided);
    assert!(sum.t_share() >= 0.75, "T share {:.2}", sum.t_share());
    // Elongation spreads from squarish closes to long fields.
    assert!(
        sum.elong_median >= 1.4,
        "median elongation {:.2}",
        sum.elong_median
    );
    assert!(sum.elong_p90 >= 2.2, "p90 elongation {:.2}", sum.elong_p90);
}

#[test]
fn furlongs_and_fields_by_the_village_keep_the_shape() {
    let s = synthetic::village_strips();
    let c = [5050.0, 5050.0];
    let (ids, p) = raster(&s, c, 1400);
    let (sum, _) = shape::summary(&ids, 320);
    println!("village: {sum:?}, T share {:.2}", sum.t_share());
    assert!(sum.four_sided >= 0.35, "four-sided {:.2}", sum.four_sided);
    assert!(sum.t_share() >= 0.7, "T share {:.2}", sum.t_share());
    let strips = p
        .sites
        .iter()
        .filter(|x| x.kind == FieldKind::Strips)
        .count();
    assert!(strips >= 8, "{strips} furlongs");
}

#[test]
fn closes_by_the_hamlet_give_way_to_larger_more_varied_fields() {
    let s = synthetic::hedge_country();
    let (ids, p) = raster(&s, [1500.0, 2600.0], 1024);
    // Enclosed ploughland and pasture (meadows, orchards and woods keep
    // sizes of their own).
    let ps: Vec<shape::Parcel> = shape::parcels(&ids, 320)
        .into_iter()
        .filter(|f| {
            matches!(
                p.sites[f.id as usize].kind,
                FieldKind::Arable | FieldKind::Pasture | FieldKind::Fallow
            )
        })
        .collect();
    // Squares from the hamlet (the raster's centre).
    let bands = [(0.0, 320.0), (320.0, 720.0)];
    let by = shape::size_by_distance(&ps, [512.0, 512.0], &bands);
    for (b, (m, sd, n)) in bands.iter().zip(&by) {
        println!(
            "  {b:?}: {n} fields, {:.2} ± {:.2} ha",
            m / 4096.0,
            sd / 4096.0
        );
    }
    assert!(by[0].2 >= 10 && by[1].2 >= 10);
    assert!(by[1].0 > 1.3 * by[0].0, "mean size grows outward");
    assert!(by[1].1 > 1.3 * by[0].1, "size spread grows outward");
}

/// Squares from `q` (world squares) to the nearest road or river channel
/// of the scenario.
fn to_road_or_river(s: &Scenario, q: [f64; 2]) -> f64 {
    let seg = |a: [f64; 2], b: [f64; 2]| {
        let (a, b) = (
            [a[0] / SQUARE_M, a[1] / SQUARE_M],
            [b[0] / SQUARE_M, b[1] / SQUARE_M],
        );
        let ab = [b[0] - a[0], b[1] - a[1]];
        let l2 = (ab[0] * ab[0] + ab[1] * ab[1]).max(1e-12);
        let t = (((q[0] - a[0]) * ab[0] + (q[1] - a[1]) * ab[1]) / l2).clamp(0.0, 1.0);
        (q[0] - a[0] - ab[0] * t).hypot(q[1] - a[1] - ab[1] * t)
    };
    let roads = s
        .roads
        .iter()
        .flat_map(|r| r.points.windows(2).map(|w| seg(w[0], w[1])));
    let rivers = s.rivers.iter().map(|r| seg(r.a, r.b));
    roads.chain(rivers).fold(f64::INFINITY, f64::min)
}

/// No field comes out as a sharp wedge: every corner is 35° or more,
/// except where a road or river boundary forces a sharper one.
#[test]
fn no_field_corner_is_a_wedge() {
    for (s, c, side) in [
        (synthetic::hedge_country(), [1500.0, 2600.0], 1024_i64),
        (synthetic::village_strips(), [5050.0, 5050.0], 1400),
        (synthetic::orchard_farmstead(), [1500.0, 2180.0], 1024),
        (synthetic::watermill(), [1600.0, 1800.0], 1024),
        (synthetic::quarry(), [1500.0, 2300.0], 1024),
    ] {
        let (ids, _) = raster(&s, c, side);
        let x0 = (c[0] / SQUARE_M) as i64 - side / 2;
        let y0 = (c[1] / SQUARE_M) as i64 - side / 2;
        let ps = shape::parcels(&ids, 320);
        let mut sharp = Vec::new();
        let mut forced = 0;
        for p in &ps {
            for &(at, deg) in &p.angles {
                if deg >= 35.0 {
                    continue;
                }
                let q = [at[0] + x0 as f64, at[1] + y0 as f64];
                if to_road_or_river(&s, q) <= 10.0 {
                    forced += 1;
                } else {
                    sharp.push((p.id, q, deg));
                }
            }
        }
        let least = ps
            .iter()
            .map(shape::Parcel::min_corner_deg)
            .fold(180.0, f64::min);
        println!(
            "{}: {} fields, least corner {least:.1}°, {forced} road- or river-forced, {} wedges",
            s.name,
            ps.len(),
            sharp.len()
        );
        assert!(ps.len() >= 40, "{} fields", ps.len());
        assert!(sharp.is_empty(), "{}: wedge corners {sharp:?}", s.name);
    }
}
