//! Biome signals read from one cell's saved fields: how marshy, arid,
//! snowbound, alpine, craggy or coastal the cell is, each in `0..=1`.
//!
//! Worlds often leave `cover` coarse (a whole continent of `grass`), so the
//! ground and scatter rules lean on the climate and landform fields that
//! every world carries: wetness, rainfall against warmth, temperature,
//! slope and height. Each signal is a pure function of one cell, so cell
//! interpolation of it is the same from every block (seam-stable).

use crate::noise::smoothstep;
use arda::{Cell, Cover, TerrainKind};
use arda_core::ShoreClass;

/// Wetness (`0..=1`) where standing-water ground starts to become marsh,
/// and where it is marsh outright. A wetness of 1.0 is a wetland.
pub const MARSH_WETNESS: (f64, f64) = (0.5, 0.7);

/// Mean annual temperature (°C) at which ground starts to lie under snow,
/// and at which it is snowbound (the refiner's snow prior).
pub const SNOW_C: (f64, f64) = (1.5, -2.5);

/// Mean annual temperature (°C) where tree growth gives out: above it a
/// cell is below the tree line, below the second value fully alpine.
pub const TREELINE_C: (f64, f64) = (3.5, -1.0);

/// Land height (m) under which a land cell counts as lying at sea level,
/// and the height by which it no longer does.
pub const SEA_LEVEL_M: (f64, f64) = (0.02, 0.6);

fn land(c: &Cell) -> bool {
    c.terrain == TerrainKind::Land
}

fn temp_c(c: &Cell) -> f64 {
    f64::from(c.temperature.raw()) / 100.0
}

/// How much of the cell is marsh: marsh cover, or standing-water wetness
/// near saturation.
#[must_use]
pub fn marshy(c: &Cell) -> f64 {
    if !land(c) {
        return 0.0;
    }
    let wet = f64::from(c.wetness) / 255.0;
    let flat = 1.0 - smoothstep(4.0, 12.0, f64::from(c.slope_milli_deg) / 1000.0);
    let bog = smoothstep(MARSH_WETNESS.0, MARSH_WETNESS.1, wet);
    // Marsh cover is marsh outright, but on level ground only.
    if c.cover == Cover::Marsh {
        flat
    } else {
        bog * flat
    }
}

/// Aridity from rainfall against warmth: `0` where rain covers what the
/// warmth evaporates, `1` on dry steppe. The water need is a simple
/// temperature proxy (300 mm plus 45 mm per °C above freezing); a world
/// without a rainfall layer (all zeros) reads as unknown, not desert.
#[must_use]
pub fn arid(c: &Cell) -> f64 {
    let rain = f64::from(c.rainfall.raw());
    if !land(c) || rain <= 0.0 {
        return 0.0;
    }
    let need = 300.0 + 45.0 * temp_c(c).max(0.0);
    smoothstep(0.75, 0.35, rain / need)
}

/// How snowbound the cell is, from its mean annual temperature.
#[must_use]
pub fn snowy(c: &Cell) -> f64 {
    smoothstep(SNOW_C.0, SNOW_C.1, temp_c(c))
}

/// How far above the tree line the cell lies.
#[must_use]
pub fn alpine(c: &Cell) -> f64 {
    if !land(c) {
        return 0.0;
    }
    smoothstep(TREELINE_C.0, TREELINE_C.1, temp_c(c))
}

/// How craggy the cell is: its mean slope from steep to mountain wall.
#[must_use]
pub fn craggy(c: &Cell) -> f64 {
    if !land(c) {
        return 0.0;
    }
    smoothstep(18.0, 34.0, f64::from(c.slope_milli_deg) / 1000.0)
}

/// Whether a land cell lies at sea level (the coast strip): `1` at 0 m,
/// fading out by [`SEA_LEVEL_M`]. Water cells answer `0`.
#[must_use]
pub fn coastal(c: &Cell) -> f64 {
    if !land(c) {
        return 0.0;
    }
    let h = f64::from(c.height.raw()) / 1000.0;
    1.0 - smoothstep(SEA_LEVEL_M.0, SEA_LEVEL_M.1, h)
}

/// How far from the sea (cells, Chebyshev) a land cell still knows it.
pub const COAST_REACH: i64 = 4;

/// What a cell's neighbourhood says about the sea: the distance to it and
/// the saved shore class. Both are pure functions of the cell's position, so
/// interpolating them is the same from every block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coast {
    /// Chebyshev distance in cells to the nearest sea cell: `0` for a sea
    /// cell, `1` for land touching the sea (diagonals included), and
    /// `COAST_REACH + 1` for land farther away.
    pub dist: u8,
    /// The saved shore class of the cell ([`ShoreClass::None`] off the shore).
    pub shore: ShoreClass,
    /// Whether the world has a shore layer at all (recipe 5 and later).
    pub layer: bool,
}

impl Default for Coast {
    fn default() -> Self {
        Self {
            dist: u8::try_from(COAST_REACH + 1).unwrap_or(u8::MAX),
            shore: ShoreClass::None,
            layer: false,
        }
    }
}

/// The coast facts of a cell from a probe of the cells around it:
/// `probe(dx, dy)` answers the terrain and saved shore class at that offset.
/// The distance counts Chebyshev rings out to [`COAST_REACH`]; the shore
/// class is the first non-`None` class of a land cell, scanning the cell's
/// own ring first and then outwards (row-major within a ring), so an inland
/// cell takes the class of the shore it lies behind.
#[must_use]
pub fn coast_facts(
    mut probe: impl FnMut(i64, i64) -> (TerrainKind, ShoreClass),
    layer: bool,
) -> Coast {
    let mut out = Coast {
        layer,
        ..Coast::default()
    };
    let (mut found_sea, mut found_shore) = (false, false);
    for ring in 0..=COAST_REACH {
        for dy in -ring..=ring {
            for dx in -ring..=ring {
                if dx.abs().max(dy.abs()) != ring {
                    continue;
                }
                let (terrain, class) = probe(dx, dy);
                if !found_sea && terrain == TerrainKind::Sea {
                    out.dist = u8::try_from(ring).unwrap_or(u8::MAX);
                    found_sea = true;
                }
                if !found_shore && terrain == TerrainKind::Land && class != ShoreClass::None {
                    out.shore = class;
                    found_shore = true;
                }
            }
        }
        if found_sea && found_shore {
            break;
        }
    }
    out
}

/// What the sea's edge gives a cell: the width of its strip and the weight
/// of each shore ground in it, each `0..=1`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Shoreline {
    /// How much the cell belongs to the shore strip.
    pub strip: f64,
    /// Sand (beach).
    pub sand: f64,
    /// Shingle (gravel).
    pub shingle: f64,
    /// Bare shore rock.
    pub rock: f64,
    /// Mud and marsh (salt marsh, tidal margin).
    pub mud: f64,
    /// How far turf is suppressed (beach and strip ground replace it).
    pub turf: f64,
    /// How far woods and scrub are suppressed (open sand, shingle, marsh).
    pub open: f64,
}

/// Land height (m) under which a beach can lie, and the height by which the
/// ground has risen into dunes or bluff and no longer does.
pub const BEACH_HEIGHT_M: (f64, f64) = (10.0, 25.0);

/// Mean cell slope (degrees) under which a beach can lie, and the slope by
/// which it no longer does.
pub const BEACH_SLOPE_DEG: (f64, f64) = (10.0, 22.0);

/// Strip reach per saved shore class: the distance (cells) up to which the
/// cell is fully in the strip, and the distance at which the strip ends.
/// The strip counts as present above 0.5, so these are widths of 3 cells for
/// sand (up to 4 on firm ground), 2 for shingle, salt marsh, rocky shore
/// and cliff.
pub const REACH_SAND: (f64, f64) = (3.0, 6.0);
/// Strip reach of a shingle beach, see [`REACH_SAND`].
pub const REACH_SHINGLE: (f64, f64) = (1.0, 3.5);
/// Strip reach of a salt marsh, see [`REACH_SAND`].
pub const REACH_MARSH: (f64, f64) = (1.0, 3.5);
/// Strip reach of rock: a rocky shore or a cliff, see [`REACH_SAND`].
pub const REACH_ROCK: (f64, f64) = (1.0, 2.5);

fn fade(dist: u8, (full, end): (f64, f64)) -> f64 {
    ((end - f64::from(dist)) / (end - full)).clamp(0.0, 1.0)
}

/// The shoreline of a cell from its fields and coast facts.
///
/// A world with a shore layer reads the distance to the sea and the saved
/// class of the shore the cell lies behind: sand and shingle beaches reach
/// inland only over low, flat ground; cliffs and rocky shores give bare rock
/// and no sand; a salt marsh gives mud. Cells with no classified shore in
/// reach, and every cell of a world without the layer, keep the height rule
/// of [`coastal`].
#[must_use]
pub fn shoreline(c: &Cell, coast: Coast) -> Shoreline {
    if !land(c) {
        return Shoreline::default();
    }
    // The height rule alone, with the weights it always had: a world with no
    // shore layer, or sea-level land whose shore has no class.
    let legacy = || {
        let strip = coastal(c);
        Shoreline {
            strip,
            sand: 0.45 * strip,
            shingle: 0.1875 * strip,
            turf: 0.5 * strip,
            ..Shoreline::default()
        }
    };
    if !coast.layer {
        return legacy();
    }
    // With a layer, sea-level land far from the sea is a coastal plain or a
    // fen, not a beach.
    if i64::from(coast.dist) > COAST_REACH {
        return Shoreline::default();
    }
    if coast.shore == ShoreClass::None {
        return legacy();
    }
    let h = f64::from(c.height.raw()) / 1000.0;
    let slope = f64::from(c.slope_milli_deg) / 1000.0;
    let firm = (1.0 - smoothstep(BEACH_HEIGHT_M.0, BEACH_HEIGHT_M.1, h))
        * (1.0 - smoothstep(BEACH_SLOPE_DEG.0, BEACH_SLOPE_DEG.1, slope));
    match coast.shore {
        ShoreClass::SandBeach => {
            let strip = fade(coast.dist, REACH_SAND) * firm;
            Shoreline {
                strip,
                sand: strip,
                shingle: 0.2 * strip,
                turf: 0.8 * strip,
                open: strip,
                ..Shoreline::default()
            }
        }
        ShoreClass::ShingleBeach => {
            let strip = fade(coast.dist, REACH_SHINGLE) * firm;
            Shoreline {
                strip,
                sand: 0.3 * strip,
                shingle: strip,
                turf: 0.8 * strip,
                open: strip,
                ..Shoreline::default()
            }
        }
        ShoreClass::Cliff => {
            let strip = fade(coast.dist, REACH_ROCK);
            Shoreline {
                strip,
                rock: strip,
                turf: 0.8 * strip,
                ..Shoreline::default()
            }
        }
        ShoreClass::RockyShore => {
            let strip = fade(coast.dist, REACH_ROCK);
            Shoreline {
                strip,
                shingle: 0.3 * strip,
                rock: strip,
                turf: 0.8 * strip,
                ..Shoreline::default()
            }
        }
        ShoreClass::Marsh | ShoreClass::TidalFlat | ShoreClass::Estuary => {
            let strip = fade(coast.dist, REACH_MARSH);
            Shoreline {
                strip,
                sand: 0.1 * strip,
                mud: strip,
                turf: 0.8 * strip,
                open: strip,
                ..Shoreline::default()
            }
        }
        ShoreClass::None => Shoreline::default(),
    }
}

/// The coast strip: how much the cell belongs to the shore, `0..=1`.
#[must_use]
pub fn coast_strip(c: &Cell, coast: Coast) -> f64 {
    shoreline(c, coast).strip
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::{HeightMm, RainfallMm, TempCentiC};

    fn cell(f: impl FnOnce(&mut Cell)) -> Cell {
        let mut c = Cell {
            terrain: TerrainKind::Land,
            cover: Cover::Grass,
            temperature: TempCentiC::new(1000),
            rainfall: RainfallMm::new(900),
            height: HeightMm::new(50_000),
            ..Cell::default()
        };
        f(&mut c);
        c
    }

    #[test]
    fn saturated_ground_is_marsh_and_damp_ground_is_not() {
        assert!(marshy(&cell(|c| c.wetness = 255)) > 0.99);
        assert!(marshy(&cell(|c| c.wetness = 100)) < 0.01);
        assert!((marshy(&cell(|c| c.cover = Cover::Marsh)) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn steppe_rainfall_is_arid_and_a_missing_layer_is_not() {
        let steppe = cell(|c| {
            c.rainfall = RainfallMm::new(400);
            c.temperature = TempCentiC::new(1280);
        });
        assert!(arid(&steppe) > 0.75, "{}", arid(&steppe));
        let wet = cell(|c| c.rainfall = RainfallMm::new(1200));
        assert!(arid(&wet) < 1e-9);
        assert!(arid(&cell(|c| c.rainfall = RainfallMm::new(0))) < 1e-9);
        // Cold country evaporates little: 380 mm at -7 °C is not steppe.
        let tundra = cell(|c| {
            c.rainfall = RainfallMm::new(380);
            c.temperature = TempCentiC::new(-700);
        });
        assert!(arid(&tundra) < 1e-9);
    }

    #[test]
    fn peaks_are_snowbound_and_alpine() {
        let peak = cell(|c| c.temperature = TempCentiC::new(-686));
        assert!(snowy(&peak) > 0.99 && alpine(&peak) > 0.99);
        let valley = cell(|_| {});
        assert!(snowy(&valley) < 1e-9 && alpine(&valley) < 1e-9);
    }

    #[test]
    fn sea_level_land_is_coastal() {
        assert!(coastal(&cell(|c| c.height = HeightMm::new(1))) > 0.99);
        assert!(coastal(&cell(|_| {})) < 1e-9);
        assert!(coastal(&cell(|c| c.terrain = TerrainKind::Sea)) < 1e-9);
    }

    fn shore_coast(dist: u8, shore: ShoreClass) -> Coast {
        Coast {
            dist,
            shore,
            layer: true,
        }
    }

    fn low_coast() -> Cell {
        cell(|c| {
            c.height = HeightMm::new(2_000);
            c.slope_milli_deg = 1_000;
        })
    }

    #[test]
    fn a_cliff_cell_with_a_sea_neighbour_is_not_sandy() {
        // Low and flat, but the saved class is a cliff: no beach.
        let s = shoreline(&low_coast(), shore_coast(1, ShoreClass::Cliff));
        assert!(s.sand < 1e-9 && s.shingle < 1e-9, "{s:?}");
        assert!(s.rock > 0.5, "{s:?}");
        // Behind the cliff the strip is gone within two cells.
        assert!(shoreline(&low_coast(), shore_coast(3, ShoreClass::Cliff)).strip < 0.5);
    }

    #[test]
    fn low_coast_with_a_sand_beach_is_sandy_two_to_four_cells_deep() {
        for d in 1..=3 {
            let s = shoreline(&low_coast(), shore_coast(d, ShoreClass::SandBeach));
            assert!(s.strip > 0.5 && s.sand > 0.5, "d {d}: {s:?}");
        }
        assert!(shoreline(&low_coast(), shore_coast(5, ShoreClass::SandBeach)).strip < 1e-9);
        assert!(shoreline(&low_coast(), shore_coast(4, ShoreClass::SandBeach)).strip > 0.5);
        // A dune-backed height is not beach.
        let high = cell(|c| c.height = HeightMm::new(30_000));
        assert!(shoreline(&high, shore_coast(1, ShoreClass::SandBeach)).sand < 0.05);
    }

    #[test]
    fn shingle_and_salt_marsh_take_their_own_ground() {
        let sh = shoreline(&low_coast(), shore_coast(1, ShoreClass::ShingleBeach));
        assert!(sh.shingle > sh.sand && sh.shingle > 0.5, "{sh:?}");
        let m = shoreline(&low_coast(), shore_coast(1, ShoreClass::Marsh));
        assert!(m.mud > 0.5 && m.sand < 0.2, "{m:?}");
    }

    #[test]
    fn a_world_without_a_shore_layer_keeps_the_height_rule() {
        let legacy = Coast::default();
        let s = shoreline(&cell(|c| c.height = HeightMm::new(1)), legacy);
        assert!(s.strip > 0.99 && (s.sand - 0.45).abs() < 1e-9);
        assert!(shoreline(&cell(|_| {}), legacy).strip < 1e-9);
    }

    #[test]
    fn sea_level_land_far_from_a_sea_is_not_a_beach() {
        // A low coastal plain or fen five cells and more from the sea.
        let fen = cell(|c| c.height = HeightMm::new(200));
        let far = shore_coast(5, ShoreClass::None);
        assert_eq!(shoreline(&fen, far), Shoreline::default());
        // Beside the sea but with no saved class: the height rule applies.
        let near = shore_coast(2, ShoreClass::None);
        assert!(shoreline(&fen, near).strip > 0.5);
    }

    #[test]
    fn a_saturated_flat_cell_is_marsh_and_the_same_cell_on_a_slope_is_not() {
        let flat = cell(|c| {
            c.wetness = 255;
            c.slope_milli_deg = 1_000;
        });
        let steep = cell(|c| {
            c.wetness = 255;
            c.slope_milli_deg = 20_000;
        });
        assert!(marshy(&flat) > 0.99);
        assert!(marshy(&steep) < 1e-9);
        // Marsh cover obeys the same slope limit.
        let cover = cell(|c| {
            c.cover = Cover::Marsh;
            c.slope_milli_deg = 20_000;
        });
        assert!(marshy(&cover) < 1e-9);
    }

    #[test]
    fn marsh_starts_at_moderately_wet_ground() {
        // Calibrated against the measured wetness of lowland (logic/09).
        assert!(marshy(&cell(|c| c.wetness = 140)) < 0.5);
        assert!(marshy(&cell(|c| c.wetness = 175)) > 0.5);
    }
}
