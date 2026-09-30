use super::*;
use arda_core::{HeightMm, TerrainFileWriter};
use std::io::Cursor;

#[test]
fn adjacent_area_windows_share_exact_physical_samples_and_clip_only_to_file_nodes() {
    // Two horizontally adjacent areas; the source extends past their last
    // saved 100 m centers but its end does not coincide with an Atlas pixel.
    let width = 2622_u32;
    let height = 1313_u32;
    let spacing = 39_062_500_u32;
    let mut writer = TerrainFileWriter::new(
        Cursor::new(Vec::new()),
        TerrainPoint { x_um: 0, y_um: 0 },
        spacing,
        width,
        height,
    )
    .unwrap();
    for y in 0..height {
        let row: Vec<_> = (0..width)
            .map(|x| {
                HeightMm::new(
                    1000 + 100 * i32::try_from(x).unwrap() + 200 * i32::try_from(y).unwrap(),
                )
            })
            .collect();
        writer.write_row(&row).unwrap();
    }
    let mut reader = TerrainFileReader::open(writer.finish().unwrap(), 1 << 20).unwrap();
    let (left_bounds, left) = fine_window(&mut reader, AreaCoord::new(0, 0), 150_000_000).unwrap();
    let (right_bounds, right) =
        fine_window(&mut reader, AreaCoord::new(1, 0), 150_000_000).unwrap();
    assert_eq!(left_bounds, right_bounds);
    assert_eq!(left_bounds.min, TerrainPoint { x_um: 0, y_um: 0 });
    assert_eq!(
        left_bounds.max.x_um,
        i64::from(width - 1) * i64::from(spacing)
    );
    assert_eq!(left.origin(), TerrainPoint { x_um: 0, y_um: 0 });
    assert_eq!(
        right.origin().x_um + i64::from(right.width() - 1) * i64::from(spacing),
        left_bounds.max.x_um
    );

    // The physical boundary is not a fine lattice point; both windows must
    // interpolate the same canonical height there rather than restart an
    // area-local lattice. y=25 km is fine index 640 exactly.
    let seam = TerrainPoint {
        x_um: 51_200_000_000,
        y_um: 25_000_000_000,
    };
    assert_eq!(left.sample(seam), Some(HeightMm::new(260_072)));
    assert_eq!(right.sample(seam), left.sample(seam));
    assert_eq!(reader.sample(seam).unwrap(), left.sample(seam));

    // The last saved-cell center is still covered; the absolute source end
    // remains a real fine node rather than an extrapolated area edge.
    let last = TerrainPoint {
        x_um: 102_300_000_000,
        y_um: 25_000_000_000,
    };
    assert_eq!(right.sample(last), reader.sample(last).unwrap());
    let edge_node = TerrainPoint {
        x_um: left_bounds.max.x_um,
        y_um: 25_000_000_000,
    };
    assert_eq!(right.sample(edge_node), reader.sample(edge_node).unwrap());

    let left_sky = sky_context(&mut reader, AreaCoord::new(0, 0)).unwrap();
    let right_sky = sky_context(&mut reader, AreaCoord::new(1, 0)).unwrap();
    assert!(left_sky.width() <= 72 && left_sky.height() <= 72);
    assert!(right_sky.width() <= 72 && right_sky.height() <= 72);
    for x_um in [51_000_000_000, 51_200_000_000, 52_000_000_000] {
        let point = TerrainPoint {
            x_um,
            y_um: 25_000_000_000,
        };
        assert_eq!(left_sky.sample(point), right_sky.sample(point));
    }
}
