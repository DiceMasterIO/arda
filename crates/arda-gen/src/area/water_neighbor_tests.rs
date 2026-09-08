//! Regression controls for the exact legacy D8 outside-neighbor coordinates.

use super::*;
use crate::area::{area_rainfall, erosion, fill::fill, relief::relief};
use crate::continent::{
    build_continent,
    bundles::{abs_cell, boundary_height, bundle_for, coarse_height},
    Continent,
};
use arda_core::{AreaCoord, GenerateConfig};
use std::sync::OnceLock;

fn continent() -> &'static Continent {
    static CONTINENT: OnceLock<Continent> = OnceLock::new();
    CONTINENT.get_or_init(|| build_continent(42, GenerateConfig::MICRO, 2))
}

fn real_routing() -> &'static (Filled, TileBundle, Vec<u16>) {
    static FIXTURE: OnceLock<(Filled, TileBundle, Vec<u16>)> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let ctx = continent();
        let area = AreaCoord::new(1, 2);
        let bundle = bundle_for(42, ctx, area);
        let raw = relief(42, &ctx.grid, &bundle);
        let mut heights: Vec<_> = (0..N * N)
            .map(|i| raw.get(coord(i % N, i / N).unwrap()))
            .collect();
        let uplift: Vec<_> = (0..N * N)
            .map(|i| {
                let (ax, ay) = abs_cell(
                    area,
                    u16::try_from(i % N).unwrap(),
                    u16::try_from(i / N).unwrap(),
                );
                coarse_height(&ctx.grid, ax, ay)
            })
            .collect();
        erosion::erode(&mut heights, &uplift, &bundle);
        let filled = fill(&heights, &bundle);
        let rain = area_rainfall(&bundle);
        (filled, bundle, rain)
    })
}

#[test]
fn outside_coordinates_match_canonical_signed_samples_on_the_complete_ring() {
    let ctx = continent();
    let area = AreaCoord::new(1, 2);
    let b = bundle_for(42, ctx, area);
    let (x0, y0) = abs_cell(area, 0, 0);
    let mut mismatches = Vec::new();
    for (x, y) in (-1..=N)
        .flat_map(|x| [(x, -1), (x, N)])
        .chain((0..N).flat_map(|y| [(-1, y), (N, y)]))
    {
        let expected = boundary_height(42, &ctx.grid, x0 + x, y0 + y);
        let got = off_tile_height(&b, x, y);
        if got != Some(expected) {
            mismatches.push(((x, y), got, expected));
        }
    }
    println!(
        "outside ring mismatches: count={}, examples={:?}",
        mismatches.len(),
        &mismatches[..mismatches.len().min(6)]
    );
    assert!(mismatches.is_empty(), "outside ring must name actual cells");
}

#[test]
fn outside_lookup_refuses_interior_and_unrelated_invalid_coordinates() {
    let b = bundle_for(42, continent(), AreaCoord::new(1, 2));
    let invalid = [
        (0, 0),
        (N - 1, N - 1),
        (-2, 0),
        (0, -2),
        (N + 1, 0),
        (0, N + 1),
        (-2, -1),
        (-1, -2),
        (N + 1, -1),
        (-1, N + 1),
        (i32::MIN, 0),
        (0, i32::MIN),
        (i32::MAX, 0),
        (0, i32::MAX),
    ];
    let accepted: Vec<_> = invalid
        .into_iter()
        .filter_map(|(x, y)| off_tile_height(&b, x, y).map(|height| ((x, y), height)))
        .collect();
    assert!(
        accepted.is_empty(),
        "unrelated coordinates accepted: {accepted:?}"
    );
}

#[test]
fn true_north_and_west_descent_routes_real_boundary_cells_outward() {
    let (filled, bundle, rain) = real_routing();
    let routed = water(filled, bundle, rain);
    let cases = [(128, 0), (0, 381)];
    let mut outcomes = Vec::new();
    for (x, y) in cases {
        let at = coord(x, y).unwrap();
        let (ax, ay) = abs_cell(bundle.area, at.x(), at.y());
        let (ox, oy) = if y == 0 { (ax, ay - 1) } else { (ax - 1, ay) };
        // Pin the physical coordinate and routing relationship, not an
        // elevation snapshot that changes when regional detail is corrected.
        let outside = boundary_height(42, &continent().grid, ox, oy);
        assert!(outside < filled.get(at));
        println!("real outside descent: local={at:?}, global=({ax},{ay}), filled={}, outside={outside}, next={:?}, outlet={}", filled.get(at), routed.downstream_of(at), routed.is_outlet(at));
        outcomes.push((routed.downstream_of(at), routed.is_outlet(at)));
    }
    assert_eq!(outcomes, vec![(None, true), (None, true)]);
}

#[test]
fn all_four_diagonal_outside_corners_can_win_real_routing() {
    // The only downhill outside neighbor is the chosen diagonal. The rim
    // also has a smaller downhill in-tile step, excluding the low-edge
    // fallback as an explanation for any successful outlet assertion.
    let mut bundle = bundle_for(42, continent(), AreaCoord::new(1, 2));
    let wall = 40_000;
    for edge in [
        &mut bundle.north,
        &mut bundle.south,
        &mut bundle.east,
        &mut bundle.west,
        &mut bundle.north_outside,
        &mut bundle.west_outside,
    ] {
        edge.fill(wall);
    }
    bundle.outside_corners.fill(wall);
    bundle.entering.clear();
    let mut outcomes = Vec::new();
    for (corner, (x, y, inside_x, inside_y)) in [
        (0, 0, 1, 0),
        (N - 1, 0, N - 2, 0),
        (0, N - 1, 1, N - 1),
        (N - 1, N - 1, N - 2, N - 1),
    ]
    .into_iter()
    .enumerate()
    {
        let mut heights = vec![wall; usize::try_from(N * N).unwrap()];
        let at = coord(x, y).unwrap();
        let inside = coord(inside_x, inside_y).unwrap();
        heights[at.index()] = 20_000;
        heights[inside.index()] = 19_000;
        let filled = fill(&heights, &bundle);
        let rain = vec![1000; heights.len()];
        let blocked = water(&filled, &bundle, &rain);
        assert_eq!(blocked.downstream_of(at), Some(inside));
        assert!(!blocked.is_outlet(at));
        bundle.outside_corners[corner] = 1_000;
        let open = water(&filled, &bundle, &rain);
        println!(
            "diagonal corner {corner}: local={at:?}, blocked_next={:?}, open_next={:?}, outlet={}",
            blocked.downstream_of(at),
            open.downstream_of(at),
            open.is_outlet(at)
        );
        outcomes.push((open.downstream_of(at), open.is_outlet(at)));
        bundle.outside_corners[corner] = wall;
    }
    assert_eq!(outcomes, vec![(None, true); 4]);
}
