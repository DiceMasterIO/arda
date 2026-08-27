//! Cartographic area rendering (`logic/04`).
//!
//! Everything drawn here comes from the 100 m cell tier. The continent
//! tier stores a gated river list in `continent/objects.bin` — four
//! rivers on the DEFAULT continent, each validated to reach the sea —
//! and using it for the overview's trunk line work is the obvious move,
//! since those courses are guaranteed continuous where the cell tier's
//! channels are severed at tile seams. It was tried and reverted: the
//! two tiers do not agree about where the water is. Within 0.5 km of
//! river 2's stored course the cell tier's median peak discharge is
//! 0.06 m³/s, against the 239 m³/s that river carries at its mouth, and
//! plotting the course over the cell network shows a straight 45°
//! diagonal running *across* the drainage, perpendicular to the real
//! channels and over the divides between them. Drawing it would paint
//! rivers through country the detailed data says is dry. The
//! disagreement is recorded in `open-items.md`; until it is resolved the
//! map draws only what the cell tier actually holds.

use crate::RenderError;
use arda_core::{AreaCells, CellCoord, TerrainKind, AREA_CELLS};

/// Hypsometric palette: elevation in millimetres to RGB.
///
/// Stops follow the convention of physical atlases — lowland green, upland
/// tan, montane brown, then rock and snow. A single linear ramp was tried
/// first and is useless: it saturated to white above 2,040 m and showed no
/// variation at all below that, which hid the fact that the whole continent
/// was a 118 m plateau.
#[must_use]
pub fn land_colour(height_mm: i32) -> [u8; 3] {
    const STOPS: [(i32, [u8; 3]); 7] = [
        (0, [86, 125, 70]),           // coastal plain
        (200_000, [122, 148, 78]),    // lowland
        (500_000, [163, 165, 92]),    // upland
        (900_000, [173, 141, 88]),    // hill
        (1_400_000, [150, 112, 78]),  // montane
        (2_000_000, [140, 130, 128]), // bare rock
        (2_800_000, [242, 242, 245]), // snow
    ];
    let h = height_mm.max(0);
    let mut i = 0;
    while i + 1 < STOPS.len() && h >= STOPS[i + 1].0 {
        i += 1;
    }
    if i + 1 >= STOPS.len() {
        return STOPS[STOPS.len() - 1].1;
    }
    let (lo, c0) = STOPS[i];
    let (hi, c1) = STOPS[i + 1];
    let span = (hi - lo).max(1);
    let t = i64::from((h - lo).clamp(0, span));
    let mix = |a: u8, b: u8| {
        let v = i64::from(a) + (i64::from(b) - i64::from(a)) * t / i64::from(span);
        u8::try_from(v.clamp(0, 255)).unwrap_or(255)
    };
    [mix(c0[0], c1[0]), mix(c0[1], c1[1]), mix(c0[2], c1[2])]
}

/// Sea colour by depth: shelf is lighter than abyss.
#[must_use]
pub fn sea_colour(height_mm: i32) -> [u8; 3] {
    let depth = (-height_mm).clamp(0, 3_000_000);
    let t = u8::try_from(depth / 14_000).unwrap_or(214);
    [
        26u8.saturating_sub(t / 8),
        58u8.saturating_sub(t / 5),
        110u8.saturating_sub(t / 3),
    ]
}

/// Overview sea fill, flat: the depth ramp is an area-map affordance and
/// only adds noise at 1 km per pixel.
const OVERVIEW_SEA: [u8; 3] = [10, 30, 78];

/// Lake fill.
///
/// Deliberately much lighter than every river band. The previous value
/// `[58, 110, 190]` sat at ΔE00 2.02 from the mid river band `[60, 105,
/// 185]` — below the ~2.3 just-noticeable difference, so a mid-size river
/// and a lake were literally the same colour on the page.
const LAKE_FILL: [u8; 3] = [132, 176, 205];

/// Discharge cuts that place a watercourse in a render band, in
/// thousandth-cumecs (`DischargeMilli`, so 4_000 = 4 m³/s).
///
/// Selection moved off Strahler order here, and that is the substance of
/// this retouch rather than a tuning change. Order answers "how deep in
/// the branching hierarchy is this?", which is scale-free: a first-order
/// headwater in a 50,000 km² basin and a first-order rill on a coastal
/// hillside score alike, and order >= 3 admits *both*. Measured on the
/// DEFAULT continent (500x1000 km, seed 42): order >= 3 selects 217,636
/// of 24,551,366 land cells, which the overview's block classification
/// then inflates to 11.50% of the drawn landmass — and it leaves 471
/// separate watercourses touching the sea, one river mouth per 3.8 km of
/// the 1,789 km coastline. Earth averages one per 50-150 km.
///
/// Discharge is absolute, so a cut means the same size of river anywhere
/// on the map. Block-max coverage of the drawn landmass, same world, by
/// cut in m³/s: 2 gives 4.64%, 4 gives 2.31%, 5 gives 1.90%, 10 gives
/// 0.88%, 20 gives 0.33%. Physical atlases carry 1-2% blue line work,
/// and the trunk overlay below adds ~0.35% on top, so the floor sits at
/// 4 m³/s.
const RIVER_Q_MIN: u32 = 4_000;
/// Mid band floor — see [`RIVER_Q_MIN`] for the calibration.
const RIVER_Q_MID: u32 = 20_000;
/// Dark band floor — see [`RIVER_Q_MIN`] for the calibration.
const RIVER_Q_MAX: u32 = 80_000;

/// A watercourse's render band, lightest to darkest.
///
/// Declaration order matters: the derived `Ord` is what makes `Dark` win
/// when one block spans several bands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RiverBand {
    /// `RIVER_Q_MIN..RIVER_Q_MID` — a stream.
    Light,
    /// `RIVER_Q_MID..RIVER_Q_MAX` — a river.
    Mid,
    /// `>= RIVER_Q_MAX` — a trunk.
    Dark,
}

/// Maps a discharge to its render band, or `None` below the floor.
#[must_use]
fn river_band(discharge_milli: u32) -> Option<RiverBand> {
    if discharge_milli >= RIVER_Q_MAX {
        Some(RiverBand::Dark)
    } else if discharge_milli >= RIVER_Q_MID {
        Some(RiverBand::Mid)
    } else if discharge_milli >= RIVER_Q_MIN {
        Some(RiverBand::Light)
    } else {
        None
    }
}

/// The three river band colours, shared by the overview and the area map.
///
/// All three sit well clear of [`LAKE_FILL`] in lightness so line work
/// never reads as a water body.
const fn river_band_colour(band: RiverBand) -> [u8; 3] {
    match band {
        RiverBand::Light => [86, 130, 190],
        RiverBand::Mid => [46, 92, 170],
        RiverBand::Dark => [16, 56, 138],
    }
}

/// Per-cell channel colour for the area map, shaded by discharge.
///
/// Unlike the overview, the area map is one pixel per 100 m cell, so a
/// headwater stream is not confetti here — it is correctly one real
/// pixel. Channels below the overview's render floor still get a colour,
/// just the palest one, so the full network stays visible at area scale
/// even where the overview hides it.
#[must_use]
fn area_channel_colour(discharge_milli: u32) -> [u8; 3] {
    river_band(discharge_milli).map_or([140, 172, 210], river_band_colour)
}

/// Renders one area tile, one pixel per 100 m cell, hypsometrically tinted.
///
/// # Errors
/// [`RenderError::Png`] when encoding fails.
pub fn render_area_png(cells: &AreaCells) -> Result<Vec<u8>, RenderError> {
    let side = u32::from(AREA_CELLS);
    let mut rgb = vec![0u8; usize::try_from(side * side * 3).map_err(|_| RenderError::Png)?];

    for y in 0..AREA_CELLS {
        for x in 0..AREA_CELLS {
            let at = CellCoord::new(x, y).ok_or(RenderError::Png)?;
            let cell = cells.get(at);
            let colour = if cell.terrain == TerrainKind::Lake {
                LAKE_FILL
            } else if cell.watercourse_order > 0 {
                area_channel_colour(cell.discharge.raw())
            } else if cell.terrain == TerrainKind::Land {
                land_colour(cell.height.raw())
            } else {
                sea_colour(cell.height.raw())
            };
            let i = usize::try_from((u32::from(y) * side + u32::from(x)) * 3)
                .map_err(|_| RenderError::Png)?;
            rgb[i..i + 3].copy_from_slice(&colour);
        }
    }
    crate::encode_png(side, side, &rgb)
}

/// What a downsampled block shows, in increasing order of prominence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Feature {
    Sea,
    Land,
    River(RiverBand),
    Lake,
}

/// Renders every area tile into one overview image.
///
/// Each area becomes a `px`-square block, classified from the 100 m cells
/// beneath it. Classification takes the most prominent feature in the
/// block rather than its centre cell: a channel is one cell wide out of
/// 512 and would vanish under nearest-neighbour sampling, so water wins
/// over land and a lake wins over a river.
///
/// `areas` holds `(area_x, area_y, cells)`; missing tiles render as ocean.
///
/// # Errors
/// [`RenderError::Png`] when encoding fails.
pub fn render_overview_png(
    areas: &[(i32, i32, &AreaCells)],
    areas_wide: i32,
    areas_high: i32,
    px: u32,
) -> Result<Vec<u8>, RenderError> {
    let width = u32::try_from(areas_wide).map_err(|_| RenderError::Png)? * px;
    let height = u32::try_from(areas_high).map_err(|_| RenderError::Png)? * px;
    if width == 0 || height == 0 {
        return Err(RenderError::Png);
    }
    let mut rgb = vec![0u8; usize::try_from(width * height * 3).map_err(|_| RenderError::Png)?];
    for chunk in rgb.chunks_exact_mut(3) {
        chunk.copy_from_slice(&OVERVIEW_SEA);
    }
    // Parallel classification grid, so the trunk pass can tell land and
    // sea apart from lake without re-deriving it from RGB bytes.
    let mut features =
        vec![Feature::Sea; usize::try_from(width * height).map_err(|_| RenderError::Png)?];

    let side = u32::from(AREA_CELLS);

    for &(ax, ay, cells) in areas {
        let (Ok(ox), Ok(oy)) = (u32::try_from(ax), u32::try_from(ay)) else {
            continue;
        };
        for py in 0..px {
            for pxi in 0..px {
                // Half-open cell bounds, derived per pixel so the blocks
                // tile the whole 512 exactly. The previous `block = side /
                // px` was 512/48 = 10, and `pxi * block + cx` therefore
                // topped out at 479: cells 480..=511 of every tile — a
                // 3.2 km strip down the right edge and along the bottom of
                // all 171 tiles — were never read, which truncated courses
                // at tile edges. Uneven blocks (here 10 and 11 cells) are
                // the correct answer when px does not divide 512.
                let x0 = pxi * side / px;
                let x1 = ((pxi + 1) * side / px).max(x0 + 1);
                let y0 = py * side / px;
                let y1 = ((py + 1) * side / px).max(y0 + 1);

                let mut best = Feature::Sea;
                let mut height_sum: i64 = 0;
                let mut land_count: i64 = 0;

                for sy in y0..y1 {
                    for sx in x0..x1 {
                        let (Ok(sxu), Ok(syu)) = (u16::try_from(sx), u16::try_from(sy)) else {
                            continue;
                        };
                        let Some(at) = CellCoord::new(sxu, syu) else {
                            continue;
                        };
                        let cell = cells.get(at);
                        let f = match cell.terrain {
                            TerrainKind::Lake => Feature::Lake,
                            TerrainKind::Sea => Feature::Sea,
                            TerrainKind::Land => {
                                height_sum += i64::from(cell.height.raw());
                                land_count += 1;
                                match river_band(cell.discharge.raw()) {
                                    Some(band) => Feature::River(band),
                                    None => Feature::Land,
                                }
                            }
                        };
                        best = best.max(f);
                    }
                }

                let colour = match best {
                    Feature::Sea => OVERVIEW_SEA,
                    // Mean over every land cell in the block, including
                    // the channel cells: excluding them made the tint jump
                    // wherever a river crossed a block.
                    Feature::Land | Feature::River(_) => {
                        let mean = height_sum / land_count.max(1);
                        let base = land_colour(i32::try_from(mean).unwrap_or(0));
                        match best {
                            Feature::River(band) => river_band_colour(band),
                            _ => base,
                        }
                    }
                    Feature::Lake => LAKE_FILL,
                };

                let x = ox * px + pxi;
                let y = oy * px + py;
                let Ok(pixel) = usize::try_from(y * width + x) else {
                    continue;
                };
                rgb[pixel * 3..pixel * 3 + 3].copy_from_slice(&colour);
                features[pixel] = best;
            }
        }
    }

    // The Dark band widens by one pixel so the few real trunks carry
    // visible weight against the streams. Guarded both ways: never over a
    // lake, and never over another river pixel, so the pass cannot change
    // a classification the block pass already made.
    let mut widened = vec![false; features.len()];
    for y in 0..height {
        for x in 0..width {
            let Ok(src) = usize::try_from(y * width + x) else {
                continue;
            };
            if features.get(src) != Some(&Feature::River(RiverBand::Dark)) {
                continue;
            }
            for (dx, dy) in [(1u32, 0u32), (0, 1)] {
                let (tx, ty) = (x + dx, y + dy);
                if tx >= width || ty >= height {
                    continue;
                }
                let Ok(dst) = usize::try_from(ty * width + tx) else {
                    continue;
                };
                if !matches!(features.get(dst), Some(Feature::Land | Feature::Sea)) {
                    continue;
                }
                if widened.get(dst) == Some(&true) {
                    continue;
                }
                if let Some(slot) = widened.get_mut(dst) {
                    *slot = true;
                }
                if let Some(slot) = rgb.get_mut(dst * 3..dst * 3 + 3) {
                    slot.copy_from_slice(&river_band_colour(RiverBand::Dark));
                }
            }
        }
    }

    crate::encode_png(width, height, &rgb)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::{Cell, DischargeMilli};

    fn cell(discharge_milli: u32, terrain: TerrainKind) -> Cell {
        Cell {
            terrain,
            discharge: DischargeMilli::new(discharge_milli),
            watercourse_order: u8::from(discharge_milli > 0),
            ..Cell::default()
        }
    }

    #[test]
    fn river_bands_split_on_the_calibrated_discharge_cuts() {
        assert_eq!(river_band(RIVER_Q_MIN - 1), None);
        assert_eq!(river_band(RIVER_Q_MIN), Some(RiverBand::Light));
        assert_eq!(river_band(RIVER_Q_MID - 1), Some(RiverBand::Light));
        assert_eq!(river_band(RIVER_Q_MID), Some(RiverBand::Mid));
        assert_eq!(river_band(RIVER_Q_MAX - 1), Some(RiverBand::Mid));
        assert_eq!(river_band(RIVER_Q_MAX), Some(RiverBand::Dark));
    }

    #[test]
    fn a_dry_cell_is_never_a_river_however_high_its_order() {
        // The whole point of the retouch: selection is by how much water
        // a channel carries, not by where it sits in the hierarchy.
        let mut c = cell(0, TerrainKind::Land);
        c.watercourse_order = 9;
        assert_eq!(river_band(c.discharge.raw()), None);
    }

    #[test]
    fn every_water_colour_is_distinguishable_from_every_other() {
        // Guards the defect this retouch fixed: the old lake fill
        // [58,110,190] and mid river band [60,105,185] differed by 2
        // units of blue and were the same colour on the page. Cheap
        // proxy for a perceptual metric — a generous Manhattan floor in
        // sRGB, which the old pair (ΔE00 2.02, Manhattan 12) fails.
        let water = [
            ("lake", LAKE_FILL),
            ("light", river_band_colour(RiverBand::Light)),
            ("mid", river_band_colour(RiverBand::Mid)),
            ("dark", river_band_colour(RiverBand::Dark)),
            ("sea", OVERVIEW_SEA),
        ];
        for (i, (an, a)) in water.iter().enumerate() {
            for (bn, b) in water.iter().skip(i + 1) {
                let d: i32 = (0..3)
                    .map(|k| (i32::from(a[k]) - i32::from(b[k])).abs())
                    .sum();
                assert!(d >= 40, "{an} and {bn} are too close (Manhattan {d})");
            }
        }
    }

    #[test]
    fn blocks_cover_every_cell_of_the_tile() {
        // The shipped renderer used `block = 512 / px`, so at the default
        // px = 48 it read cells 0..=479 and silently dropped 480..=511 of
        // every tile. Walk the same bounds the render loop derives and
        // assert they tile 0..512 with no gap and no overlap.
        for px in [1u32, 2, 3, 7, 16, 48, 64, 512] {
            let side = u32::from(AREA_CELLS);
            let mut covered = 0u32;
            let mut prev_end = 0u32;
            for i in 0..px {
                let x0 = i * side / px;
                let x1 = ((i + 1) * side / px).max(x0 + 1);
                assert_eq!(x0, prev_end, "gap or overlap at px={px} block={i}");
                covered += x1 - x0;
                prev_end = x1;
            }
            assert_eq!(prev_end, side, "px={px} stops short of the tile");
            assert_eq!(covered, side, "px={px} does not cover the tile exactly");
        }
    }

    fn uniform_tile(discharge_milli: u32) -> AreaCells {
        AreaCells::flat(cell(discharge_milli, TerrainKind::Land))
    }

    fn pixel_at(png_bytes: &[u8], x: usize, y: usize) -> [u8; 3] {
        let Ok(mut reader) = png::Decoder::new(png_bytes).read_info() else {
            panic!("invalid PNG header");
        };
        let mut buf = vec![0u8; reader.output_buffer_size()];
        let Ok(info) = reader.next_frame(&mut buf) else {
            panic!("invalid PNG frame");
        };
        let row = y * info.line_size;
        let col = x * 3;
        [buf[row + col], buf[row + col + 1], buf[row + col + 2]]
    }

    #[test]
    fn the_strongest_band_in_a_block_wins_it() {
        let dark = uniform_tile(RIVER_Q_MAX);
        let light = uniform_tile(RIVER_Q_MIN);
        let dry = uniform_tile(0);
        let areas = [(0, 0, &dark), (1, 0, &light), (0, 1, &dry)];
        let Ok(png) = render_overview_png(&areas, 2, 2, 1) else {
            panic!("render_overview_png failed");
        };
        assert_eq!(pixel_at(&png, 0, 0), river_band_colour(RiverBand::Dark));
        assert_eq!(pixel_at(&png, 1, 0), river_band_colour(RiverBand::Light));
        assert_ne!(pixel_at(&png, 0, 1), river_band_colour(RiverBand::Light));
    }

    #[test]
    fn rendering_is_byte_identical_on_repeat() {
        // logic/04: re-export must produce the same bytes.
        let t = uniform_tile(RIVER_Q_MID);
        let areas = [(0, 0, &t)];
        let (Ok(a), Ok(b)) = (
            render_overview_png(&areas, 1, 1, 8),
            render_overview_png(&areas, 1, 1, 8),
        ) else {
            panic!("render failed");
        };
        assert_eq!(a, b);
    }

    #[test]
    fn the_dark_band_widens_and_the_light_band_does_not() {
        // Hierarchy has to come from the data now that no continent
        // course is drawn: a trunk gets an extra pixel, a stream does not.
        for (q, widens) in [(RIVER_Q_MAX, true), (RIVER_Q_MIN, false)] {
            let wet = uniform_tile(q);
            let dry = uniform_tile(0);
            let areas = [(0, 0, &wet), (1, 0, &dry), (0, 1, &dry), (1, 1, &dry)];
            let Ok(png) = render_overview_png(&areas, 2, 2, 1) else {
                panic!("render failed");
            };
            let spread = pixel_at(&png, 1, 0) == river_band_colour(RiverBand::Dark);
            assert_eq!(spread, widens, "band at discharge {q} widened: {spread}");
        }
    }

    #[test]
    fn a_lake_outranks_a_river_in_the_same_block() {
        let mut cells = AreaCells::flat(cell(RIVER_Q_MAX, TerrainKind::Land));
        let Some(at) = CellCoord::new(0, 0) else {
            panic!("0,0 is in range");
        };
        cells.set(at, cell(0, TerrainKind::Lake));
        let areas = [(0, 0, &cells)];
        let Ok(png) = render_overview_png(&areas, 1, 1, 1) else {
            panic!("render_overview_png failed");
        };
        assert_eq!(pixel_at(&png, 0, 0), LAKE_FILL);
    }


}
