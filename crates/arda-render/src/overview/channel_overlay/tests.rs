use super::shapes::display_width;
use super::*;
use crate::channel_geometry::Q;
use crate::overview::ChannelStyle;
use crate::{AtlasHalo, AtlasNeighbor, AtlasTerrain, OverviewRaster};
use arda_core::{Cell, DischargeMilli, HeightMm, TerrainKind};

fn edge(a: (u32, u32), b: (u32, u32), width: u32) -> ChannelEdge {
    ChannelEdge {
        from: GlobalCell { x: a.0, y: a.1 },
        to: GlobalCell { x: b.0, y: b.1 },
        from_width_dm: width,
        to_width_dm: width,
        discharge: DischargeMilli::new(40_000),
    }
}
fn objects() -> AreaObjects {
    let mut o = AreaObjects::empty();
    o.channel_edges = vec![
        edge((511, 510), (511, 511), 500),
        edge((511, 511), (512, 512), 500),
        edge((512, 511), (512, 512), 500),
        edge((512, 512), (513, 513), 500),
    ];
    o
}
#[test]
fn braided_edges_gain_two_weaving_threads() {
    let at = AreaCoord::new(0, 0);
    let mut o = AreaObjects::empty();
    o.channel_edges = (100..130)
        .map(|x| edge((x, 200), (x + 1, 200), 200))
        .collect();
    let build = |braided: bool| {
        let mut c = OverviewChannelContext::new(at, 1, 1).unwrap();
        c.add_area(at, &o).unwrap();
        if braided {
            c.add_braided((100..120).map(|x| (GlobalCell { x, y: 200 }, 3_000)))
                .unwrap();
        }
        let c = c.finish().unwrap();
        ChannelTile::new(&c, at, [0, 512, 0, 512], [512, 512], ChannelStyle::Formed).unwrap()
    };
    let (plain, braided) = (build(false), build(true));
    // 20 braided edges, two threads each.
    assert_eq!(braided.shapes.len(), plain.shapes.len() + 40);
    // Threads leave the channel axis (y = 200.5 cells) by up to 45% of
    // the 300 m belt, i.e. 1.35 cells.
    let spread = braided.shapes[plain.shapes.len()..]
        .iter()
        .flat_map(|s| s.polygon.iter().map(|p| (p.1 - (200 * Q + Q / 2)).abs()))
        .max()
        .unwrap();
    assert!(spread > Q / 2 && spread < 2 * Q, "{spread}");
    // Without stored forms nothing changes.
    let c = OverviewChannelContext::new(at, 1, 1).unwrap();
    assert!(c.braided.is_empty());
}

fn context(at: AreaCoord, o: &AreaObjects) -> OverviewChannelContext {
    let mut c = OverviewChannelContext::new(at, 2, 2).unwrap();
    for y in 0..2 {
        for x in 0..2 {
            c.add_area(AreaCoord::new(x, y), o).unwrap();
        }
    }
    c.finish().unwrap()
}
fn area(kind: TerrainKind) -> arda_core::AreaCells {
    arda_core::AreaCells::flat(Cell {
        height: HeightMm::new(200_000),
        terrain: kind,
        ..Cell::default()
    })
}
fn terrain(cells: &arda_core::AreaCells) -> AtlasTerrain {
    let mut halo = AtlasHalo::new();
    for d in [
        AtlasNeighbor::North,
        AtlasNeighbor::NorthEast,
        AtlasNeighbor::East,
        AtlasNeighbor::SouthEast,
        AtlasNeighbor::South,
        AtlasNeighbor::SouthWest,
        AtlasNeighbor::West,
        AtlasNeighbor::NorthWest,
    ] {
        halo.mark_world_edge(d).unwrap();
    }
    AtlasTerrain::new(cells, halo).unwrap()
}
fn decode(png: &[u8]) -> Vec<u8> {
    let mut reader = png::Decoder::new(png).read_info().unwrap();
    let mut pixels = vec![0; reader.output_buffer_size()];
    reader.next_frame(&mut pixels).unwrap();
    pixels
}
#[test]
fn rectangular_both_axis_seams_confluence_and_band_stream_match_buffered() {
    let cells = area(TerrainKind::Land);
    let o = objects();
    let mut buffered = OverviewRaster::new_exact(2, 2, 513, 517).unwrap();
    for y in (0..2).rev() {
        for x in (0..2).rev() {
            let at = AreaCoord::new(x, y);
            buffered
                .push_atlas_with_channels(at, &cells, &terrain(&cells), &context(at, &o))
                .unwrap();
        }
    }
    let expected = decode(&buffered.finish().unwrap());
    let mut streamed = Vec::new();
    crate::write_atlas_overview_png_with_channels(2, 2, 513, 517, &mut streamed, |at| {
        Ok::<_, RenderError>((cells.clone(), terrain(&cells), context(at, &o)))
    })
    .unwrap();
    assert_eq!(decode(&streamed), expected);
    // The connected geometry changes pixels in both unequal-width areas around the crossing.
    let mut plain = OverviewRaster::new_exact(2, 2, 513, 517).unwrap();
    for y in 0..2 {
        for x in 0..2 {
            let at = AreaCoord::new(x, y);
            let empty = AreaObjects::empty();
            plain
                .push_atlas_with_channels(at, &cells, &terrain(&cells), &context(at, &empty))
                .unwrap();
        }
    }
    let base = decode(&plain.finish().unwrap());
    assert_ne!(expected, base);
    assert!((253..260).any(|x| (251..266)
        .any(|y| expected[(y * 513 + x) * 3..(y * 513 + x) * 3 + 3]
            != base[(y * 513 + x) * 3..(y * 513 + x) * 3 + 3])));

    let mut fine_stream = Vec::new();
    crate::write_atlas_overview_png_with_channels_recipe4(2, 2, 513, 517, &mut fine_stream, |at| {
        Ok::<_, RenderError>((cells.clone(), terrain(&cells), context(at, &o)))
    })
    .unwrap();
    let fine = decode(&fine_stream);
    let changed = |image: &[u8]| {
        image
            .as_chunks::<3>()
            .0
            .iter()
            .zip(base.as_chunks::<3>().0.iter())
            .filter(|(pixel, ground)| pixel != ground)
            .count()
    };
    assert!(changed(&fine) > 0);
    assert!(changed(&fine) < changed(&expected));
    let colour_shift = |image: &[u8]| {
        image
            .iter()
            .zip(&base)
            .map(|(colour, ground)| colour.abs_diff(*ground) as u64)
            .sum::<u64>()
    };
    assert!(colour_shift(&fine) < colour_shift(&expected));
}
#[test]
fn sea_and_lake_own_the_pixel_even_under_a_saved_strip() {
    let o = objects();
    for kind in [TerrainKind::Sea, TerrainKind::Lake] {
        let cells = area(kind);
        let mut with = OverviewRaster::new_exact(2, 2, 513, 517).unwrap();
        let mut without = OverviewRaster::new_exact(2, 2, 513, 517).unwrap();
        for y in 0..2 {
            for x in 0..2 {
                let at = AreaCoord::new(x, y);
                with.push_atlas_with_channels(at, &cells, &terrain(&cells), &context(at, &o))
                    .unwrap();
                without
                    .push_atlas_with_channels(
                        at,
                        &cells,
                        &terrain(&cells),
                        &context(at, &AreaObjects::empty()),
                    )
                    .unwrap();
            }
        }
        assert_eq!(
            decode(&with.finish().unwrap()),
            decode(&without.finish().unwrap())
        );
    }
}
#[test]
fn context_edge_resource_cap_refuses_instead_of_omitting() {
    let mut c = OverviewChannelContext::new(AreaCoord::new(0, 0), 1, 1).unwrap();
    let repeated = edge((1, 1), (2, 1), 100);
    for _ in 0..MAX_CONTEXT_EDGES {
        c.push(repeated).unwrap();
    }
    assert!(matches!(
        c.push(repeated),
        Err(RenderError::ChannelGeometry { .. })
    ));
}

#[test]
fn cap_extremes_missing_neighbor_and_wrong_rectangle_are_typed() {
    assert_eq!(
        display_width(u32::MAX, ChannelStyle::Legacy).unwrap(),
        10_000
    );
    assert_eq!(display_width(904, ChannelStyle::Legacy).unwrap(), 7_232);
    assert_eq!(display_width(u32::MAX, ChannelStyle::Fine).unwrap(), 4_000);
    assert_eq!(display_width(904, ChannelStyle::Fine).unwrap(), 2_712);
    assert!(display_width(0, ChannelStyle::Legacy).is_err());
    assert!(display_width(0, ChannelStyle::Fine).is_err());
    let mut c = OverviewChannelContext::new(AreaCoord::new(0, 0), 2, 1).unwrap();
    c.add_area(AreaCoord::new(0, 0), &AreaObjects::empty())
        .unwrap();
    assert!(matches!(c.finish(), Err(RenderError::AtlasContext { .. })));
    let mut c = OverviewChannelContext::new(AreaCoord::new(0, 0), 1, 1).unwrap();
    c.add_area(AreaCoord::new(0, 0), &AreaObjects::empty())
        .unwrap();
    let c = c.finish().unwrap();
    assert!(matches!(
        ChannelTile::new(
            &c,
            AreaCoord::new(0, 0),
            [0, 2, 0, 1],
            [1, 1],
            ChannelStyle::Legacy,
        ),
        Err(RenderError::AtlasContext { .. })
    ));
    let mut c = OverviewChannelContext::new(AreaCoord::new(0, 0), 1, 1).unwrap();
    assert!(matches!(
        c.add_area(AreaCoord::new(i32::MIN, 0), &AreaObjects::empty()),
        Err(RenderError::AtlasContext { .. })
    ));
}
