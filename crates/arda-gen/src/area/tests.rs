use super::*;
use crate::continent::build_continent;
use crate::continent::bundles::bundle_for;
use arda_core::{AreaCoord, GenerateConfig, Terminus};

fn world(area: AreaCoord) -> (AreaCells, AreaObjects) {
    let c = build_continent(42, GenerateConfig::MICRO, 0);
    let b = bundle_for(42, &c, area);
    generate_area(42, &c, &b).unwrap()
}

#[test]
fn physical_width_supports_the_maximum_world_and_rejects_unrepresentable_inputs() {
    let width = channel_width_dm(DischargeMilli::new(33_249_619_483)).unwrap();
    assert!(width > u32::from(u16::MAX));
    assert_eq!(width, 230_649);
    assert!(channel_width_dm(DischargeMilli::new(u64::MAX)).is_err());
}

#[test]
fn width_follows_the_artifact_relation() {
    // "one cubic metre a second is about four metres wide, a river
    // carrying twenty-five is twenty"
    let four_m = channel_width_dm(DischargeMilli::new(1_000)).unwrap();
    let twenty_m = channel_width_dm(DischargeMilli::new(25_000)).unwrap();
    assert!((38..=42).contains(&four_m), "1 m3/s gave {four_m} dm");
    assert!(
        (190..=210).contains(&twenty_m),
        "25 m3/s gave {twenty_m} dm"
    );
}

#[test]
fn area_generation_is_deterministic() {
    assert_eq!(world(AreaCoord::new(1, 1)), world(AreaCoord::new(1, 1)));
}

#[test]
fn segments_know_how_they_end() {
    let (_, o) = world(AreaCoord::new(0, 1));
    assert!(!o.rivers.is_empty(), "no segments emitted");
    for s in &o.rivers {
        if s.ends == Terminus::Junction {
            assert!(
                s.feeds.is_some(),
                "segment {} ends at a junction but feeds nothing",
                s.id
            );
        } else {
            assert_eq!(
                s.feeds, None,
                "segment {} ends at {:?} yet feeds",
                s.id, s.ends
            );
        }
    }
}

#[test]
fn segments_partition_the_channel_network() {
    let (c, o) = world(AreaCoord::new(0, 1));
    let mut seen = std::collections::HashSet::new();
    for s in &o.rivers {
        for cell in &s.course {
            assert!(seen.insert(*cell), "cell {cell:?} is in two segments");
        }
    }
    for y in 0..N {
        for x in 0..N {
            let Some(at) = coord(x, y) else { continue };
            if c.get(at).watercourse_order > 0 {
                assert!(seen.contains(&at), "channel cell {x},{y} is in no segment");
            }
        }
    }
}

#[test]
fn non_land_cells_carry_no_flow() {
    let (c, _) = world(AreaCoord::new(0, 0));
    for y in 0..N {
        for x in 0..N {
            let Some(at) = coord(x, y) else { continue };
            let cell = c.get(at);
            if cell.terrain != TerrainKind::Land {
                assert_eq!(cell.drainage_area_cells, 0);
                assert_eq!(cell.discharge.raw(), 0);
                assert_eq!(cell.watercourse_order, 0);
            }
        }
    }
}

#[test]
fn slope_and_aspect_are_populated() {
    let (c, _) = world(AreaCoord::new(0, 1));
    let sloped = (0..N)
        .flat_map(|y| (0..N).map(move |x| (x, y)))
        .filter_map(|(x, y)| coord(x, y))
        .filter(|&at| c.get(at).slope_milli_deg > 0)
        .count();
    assert!(sloped > 1_000, "only {sloped} cells have a slope");
}

#[test]
fn rainfall_is_sampled_onto_every_land_cell() {
    let (c, _) = world(AreaCoord::new(0, 1));
    let mut wet = 0;
    for y in 0..N {
        for x in 0..N {
            let Some(at) = coord(x, y) else { continue };
            let cell = c.get(at);
            match cell.terrain {
                TerrainKind::Land => wet += u32::from(cell.rainfall.raw() > 0),
                _ => assert_eq!(cell.rainfall.raw(), 0),
            }
        }
    }
    assert!(wet > 10_000, "only {wet} land cells got rain");
}
