//! Shore classes and the island census of the formed lattice (logic/02
//! §fine-formation shore classes, goals 15-17).
//!
//! Runs last, on the final lattice, and classifies every 100 m shore cell
//! of the prepared grid (land next to open sea, or open sea next to land)
//! from four physical drivers:
//! - **relief**: the rise of the land within 200 m of the shore;
//! - **wave exposure**: open-water fetch ([`super::coastal`]);
//! - **sediment supply**: river mouths nearby plus eroding weak rock;
//! - **rock strength**: the formation rock field.
//!
//! Bay heads below modest hillsides are beaches; high ground meeting exposed
//! water is cliff; low, sediment-fed straight shores are beaches (shingle
//! below hard rock on high-energy coasts, sand elsewhere), and low headlands
//! only on built sand bodies or at a river mouth; sheltered, low, sediment-rich margins are salt
//! marsh on land and tidal flat in the water; enclosed water at a river
//! mouth is estuary; everything else is rocky shore. Islands are the land
//! components other than the largest, labelled with the process that made
//! them.

use arda_core::{Island, IslandCause, ShoreClass, ShoreLayer};

use super::coastal::{self, CoastSetting, Setting};
use super::drainage::{open_sea_flags_d8, FIXED};
use super::lattice::{alloc, blur_into, Lattice};
use super::margin::Volcano;
use super::water::Delta;
use super::FormationError;

/// Cliff: land rising at least this much within 200 m, metres.
pub const CLIFF_RISE_M: i32 = 18;
/// Beach: land rising less than this within 200 m, metres.
pub const BEACH_RISE_M: i32 = 10;
/// Embayed: open-sea share within 1 km below this, per mille.
pub const BAY_SHARE: i32 = 450;
/// Headland: open-sea share within 1 km above this, per mille.
pub const HEADLAND_SHARE: i32 = 550;
/// Bay-head beaches form below hillsides rising less than this, metres.
pub const BAY_BEACH_RISE_M: i32 = 30;
/// A river mouth this large makes an estuary, km².
pub const ESTUARY_KM2: u64 = 20;
/// Delta shores below this wave exposure are marsh, not beach.
pub const DELTA_MARSH_EXPOSURE: u8 = 100;
/// A river mouth this large within 1.5 km feeds a beach on a headland, km².
pub const HEADLAND_MOUTH_KM2: u64 = 20;

/// What the formation built on the coast, for the census.
#[derive(Debug, Clone, Default)]
pub struct Builders<'a> {
    /// Volcanic cones raised on the macro lattice.
    pub volcanoes: &'a [Volcano],
    /// Delta fans.
    pub deltas: &'a [Delta],
    /// Fine cells of barrier and spit sand.
    pub barrier_cells: &'a [u32],
}

/// Inputs of one shore cell's class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShoreCell {
    /// Land (true) or open sea (false).
    pub land: bool,
    /// Height, millimetres.
    pub z_mm: i32,
    /// Highest land within 200 m minus sea level, metres.
    pub rise_m: i32,
    /// Open-sea share within 1 km, per mille.
    pub sea_share: i32,
    /// Largest river mouth within 1.5 km, km².
    pub mouth_km2: u64,
    /// On a sand body that waves or a river built: barrier or spit sand, or
    /// a delta fan.
    pub sand_body: bool,
    /// Inside a delta fan.
    pub delta: bool,
    /// Coast setting.
    pub setting: Setting,
}

/// The shore class of one cell (logic/02 §fine-formation shore classes).
#[must_use]
pub fn classify(c: &ShoreCell) -> ShoreClass {
    let s = &c.setting;
    let bay = c.sea_share < BAY_SHARE;
    if !c.land {
        return if c.mouth_km2 >= ESTUARY_KM2 && c.sea_share < 550 {
            ShoreClass::Estuary
        } else if c.z_mm > -2_000 && s.exposure < 80 && s.sediment >= 60 {
            ShoreClass::TidalFlat
        } else {
            ShoreClass::None
        };
    }
    let headland = c.sea_share > HEADLAND_SHARE;
    let sandy = || {
        if s.exposure >= 140 && s.rock >= 136 && s.sediment < 150 {
            ShoreClass::ShingleBeach
        } else {
            ShoreClass::SandBeach
        }
    };
    if (c.mouth_km2 >= ESTUARY_KM2 && bay && c.z_mm < 3_000)
        || (s.exposure < 70 && s.sediment >= 60 && c.rise_m < 4)
        || (c.delta && s.exposure < DELTA_MARSH_EXPOSURE && c.rise_m < BEACH_RISE_M)
    {
        // Delta distributaries and interdistributary bays are fringed by
        // marsh on their levees; only the wave-exposed delta front keeps a
        // beach.
        ShoreClass::Marsh
    } else if bay && c.rise_m < BAY_BEACH_RISE_M {
        // Bay-head (pocket) beach: waves diverge in the bay and drop sand.
        sandy()
    } else if c.rise_m >= CLIFF_RISE_M && s.exposure >= 30 {
        ShoreClass::Cliff
    } else if c.rise_m < BEACH_RISE_M
        && (c.sand_body || c.mouth_km2 >= HEADLAND_MOUTH_KM2 || (!headland && s.sediment >= 60))
    {
        // Convex shores focus wave energy and longshore drift strips their
        // sand into the neighbouring bays: a headland keeps a beach only on
        // a sand body waves or a river built, or where a river delivers
        // sand at the point itself (goal 16).
        sandy()
    } else {
        ShoreClass::RockyShore
    }
}

/// Classifies the shore of `g` on a `spacing_um` grid and takes the
/// island census.
///
/// # Errors
/// Allocation failure.
pub fn survey(
    g: &Lattice,
    spacing_um: i64,
    base_seed: u64,
    built: &Builders<'_>,
) -> Result<ShoreLayer, FormationError> {
    let s = i128::from(spacing_um);
    let span_x = (g.width as i128 - 1) * i128::from(g.spacing_um);
    let span_y = (g.height as i128 - 1) * i128::from(g.spacing_um);
    let (w, h) = ((span_x / s) as usize + 1, (span_y / s) as usize + 1);
    let n = w * h;
    let z: Vec<i32> = (0..n)
        .map(|i| super::sampled::sample(g, (i % w) as i128 * s, (i / w) as i128 * s))
        .collect();
    // The published sea is the hydrology's eight-connected one; a 4-connected
    // survey left diagonal inlets and fjord channels unclassified.
    let mut open: Vec<u8> = alloc(n)?;
    open_sea_flags_d8(&z, w, h, &mut open);
    let sea = |i: usize| open[i] & FIXED != 0 && z[i] <= 0;
    let fine_of = |i: usize| {
        let d = i128::from(g.spacing_um);
        let fx = (((i % w) as i128 * s + d / 2) / d).min(g.width as i128 - 1) as usize;
        let fy = (((i / w) as i128 * s + d / 2) / d).min(g.height as i128 - 1) as usize;
        fy * g.width + fx
    };
    let coarse_of = |f: usize| {
        let d = i128::from(g.spacing_um);
        let (x, y) = ((f % g.width) as i128 * d / s, (f / g.width) as i128 * d / s);
        (y.min(h as i128 - 1) as usize) * w + x.min(w as i128 - 1) as usize
    };
    // Drivers.
    let mouths = coastal::river_mouths(g, &super::flow::route(g)?);
    let setting: CoastSetting = coastal::compute(g, base_seed, &mouths)?;
    let cells_per_km = (1_000_000_000 / spacing_um).max(1) as usize;
    let mut mouth_km2 = vec![0_u64; n];
    let reach = (cells_per_km * 3 / 2) as i64;
    for &(f, km2) in &mouths {
        let c = coarse_of(f);
        let (mx, my) = ((c % w) as i64, (c / w) as i64);
        for oy in -reach..=reach {
            for ox in -reach..=reach {
                let (x, y) = (mx + ox, my + oy);
                if x >= 0
                    && y >= 0
                    && x < w as i64
                    && y < h as i64
                    && ox * ox + oy * oy <= reach * reach
                {
                    let k = y as usize * w + x as usize;
                    mouth_km2[k] = mouth_km2[k].max(km2);
                }
            }
        }
    }
    let share: Vec<i32> = {
        let mask: Vec<i32> = (0..n).map(|i| if sea(i) { 1_000 } else { 0 }).collect();
        let (mut tmp, mut out) = (alloc(n)?, alloc(n)?);
        blur_into(&mask, w, h, cells_per_km, &mut tmp, &mut out);
        out
    };
    let near = |i: usize, want_sea: bool| {
        let (x, y) = (i % w, i / w);
        (y.saturating_sub(1)..(y + 2).min(h))
            .any(|yy| (x.saturating_sub(1)..(x + 2).min(w)).any(|xx| sea(yy * w + xx) == want_sea))
    };
    let rise_m = |i: usize| {
        let (x, y) = (i % w, i / w);
        let mut top = 0;
        for yy in y.saturating_sub(2)..(y + 3).min(h) {
            for xx in x.saturating_sub(2)..(x + 3).min(w) {
                top = top.max(z[yy * w + xx]);
            }
        }
        top / 1_000
    };
    let mark = provenance(w, h, spacing_um, built, &coarse_of)?;
    // Barrier sand is marked cell by cell; allow one cell of misregistration
    // between the fine sand strip and the 100 m survey.
    let sand_body = |i: usize| {
        let (x, y) = (i % w, i / w);
        mark[i] & 2 != 0
            || (y.saturating_sub(1)..(y + 2).min(h)).any(|yy| {
                (x.saturating_sub(1)..(x + 2).min(w)).any(|xx| mark[yy * w + xx] & 1 != 0)
            })
    };
    let mut classes = vec![ShoreClass::None; n];
    for (i, class) in classes.iter_mut().enumerate() {
        let land = z[i] > 0;
        let shore = if land {
            near(i, true)
        } else {
            sea(i) && near(i, false)
        };
        if !shore {
            continue;
        }
        let Some(setting) = setting.at_fine(fine_of(i)) else {
            continue;
        };
        *class = classify(&ShoreCell {
            land,
            z_mm: z[i],
            rise_m: rise_m(i),
            sea_share: share[i],
            mouth_km2: mouth_km2[i],
            sand_body: sand_body(i),
            delta: mark[i] & 2 != 0,
            setting,
        });
    }
    // Open-sea shore cells left unclassified take the class of their
    // highest land neighbour, so every waterline has material on both
    // sides and renders along the contour rather than the cell grid.
    let land_class = classes.clone();
    for (i, class) in classes.iter_mut().enumerate() {
        if *class != ShoreClass::None || z[i] > 0 || !sea(i) {
            continue;
        }
        let (x, y) = (i % w, i / w);
        let mut best: Option<(i32, ShoreClass)> = None;
        for yy in y.saturating_sub(1)..(y + 2).min(h) {
            for xx in x.saturating_sub(1)..(x + 2).min(w) {
                let k = yy * w + xx;
                if z[k] > 0 && land_class[k] != ShoreClass::None && best.is_none_or(|b| z[k] > b.0)
                {
                    best = Some((z[k], land_class[k]));
                }
            }
        }
        if let Some((_, c)) = best {
            *class = c;
        }
    }
    let islands = census(&z, w, h, spacing_um, &mark)?;
    Ok(ShoreLayer {
        spacing_um: u32::try_from(spacing_um).map_err(|_| FormationError::ArithmeticOverflow)?,
        width: u32::try_from(w).map_err(|_| FormationError::ArithmeticOverflow)?,
        height: u32::try_from(h).map_err(|_| FormationError::ArithmeticOverflow)?,
        classes,
        islands,
        landforms: Vec::new(),
    })
}

/// Provenance marks on the survey grid: 1 barrier or spit sand, 2 delta
/// fan, 4 volcanic cone.
fn provenance(
    w: usize,
    h: usize,
    spacing_um: i64,
    built: &Builders<'_>,
    coarse_of: &dyn Fn(usize) -> usize,
) -> Result<Vec<u8>, FormationError> {
    let cell_m = spacing_um / 1_000_000;
    let mut mark: Vec<u8> = alloc(w * h)?;
    for &f in built.barrier_cells {
        mark[coarse_of(f as usize)] |= 1;
    }
    let splat = |mark: &mut [u8], cx: i64, cy: i64, r: i64, bit: u8| {
        for oy in -r..=r {
            for ox in -r..=r {
                let (x, y) = (cx + ox, cy + oy);
                if x >= 0 && y >= 0 && x < w as i64 && y < h as i64 && ox * ox + oy * oy <= r * r {
                    mark[y as usize * w + x as usize] |= bit;
                }
            }
        }
    };
    for d in built.deltas {
        let (cx, cy) = (d.apex_um.0 / spacing_um, d.apex_um.1 / spacing_um);
        splat(&mut mark, cx, cy, d.radius_m / cell_m.max(1) + 1, 2);
    }
    for v in built.volcanoes {
        let (cx, cy) = (v.x_um / spacing_um, v.y_um / spacing_um);
        splat(&mut mark, cx, cy, v.radius_m / 2 / cell_m.max(1), 4);
    }
    Ok(mark)
}

/// Land components other than the largest, with their causes.
fn census(
    z: &[i32],
    w: usize,
    h: usize,
    spacing_um: i64,
    mark: &[u8],
) -> Result<Vec<Island>, FormationError> {
    let n = w * h;
    let mut label: Vec<u32> = alloc(n)?;
    let mut comps: Vec<(u32, i64, i64, i32, [u32; 3])> = Vec::new();
    let mut stack = Vec::new();
    for s0 in 0..n {
        if z[s0] <= 0 || label[s0] != 0 {
            continue;
        }
        let id = u32::try_from(comps.len() + 1).map_err(|_| FormationError::ArithmeticOverflow)?;
        label[s0] = id;
        stack.push(s0);
        let (mut cells, mut sx, mut sy, mut top, mut marks) = (0_u32, 0_i64, 0_i64, 0, [0_u32; 3]);
        while let Some(c) = stack.pop() {
            cells += 1;
            sx += (c % w) as i64;
            sy += (c / w) as i64;
            top = top.max(z[c]);
            for (b, m) in marks.iter_mut().enumerate() {
                *m += u32::from(mark[c] >> b & 1);
            }
            let (x, y) = (c % w, c / w);
            for yy in y.saturating_sub(1)..(y + 2).min(h) {
                for xx in x.saturating_sub(1)..(x + 2).min(w) {
                    let k = yy * w + xx;
                    if z[k] > 0 && label[k] == 0 {
                        label[k] = id;
                        stack.push(k);
                    }
                }
            }
        }
        comps.push((cells, sx, sy, top, marks));
    }
    let main = comps
        .iter()
        .enumerate()
        .max_by_key(|(k, c)| (c.0, std::cmp::Reverse(*k)))
        .map(|(k, _)| k);
    let mut islands: Vec<Island> = comps
        .iter()
        .enumerate()
        .filter(|&(k, _)| Some(k) != main)
        .map(|(_, &(cells, sx, sy, top, marks))| {
            let cause = if marks[2] > 0 {
                IslandCause::Volcanic
            } else if marks[1] * 2 >= cells {
                IslandCause::Delta
            } else if marks[0] * 10 >= cells * 3 {
                IslandCause::Barrier
            } else {
                IslandCause::Continental
            };
            let c = i64::from(cells.max(1));
            Island {
                x_um: sx * spacing_um / c,
                y_um: sy * spacing_um / c,
                cells,
                top_m: i16::try_from(top / 1_000).unwrap_or(i16::MAX),
                cause,
            }
        })
        .collect();
    islands.sort_by_key(|i| (i.y_um, i.x_um));
    Ok(islands)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setting(exposure: u8, sediment: u8, rock: u8) -> Setting {
        Setting {
            exposure,
            sediment,
            hinterland_m: 0,
            nearshore_depth_mm: 5_000,
            rock,
            dist_m: 0,
        }
    }

    fn cell(land: bool, rise_m: i32, sea_share: i32, s: Setting) -> ShoreCell {
        ShoreCell {
            land,
            z_mm: if land { 2_000 } else { -1_000 },
            rise_m,
            sea_share,
            mouth_km2: 0,
            sand_body: false,
            delta: false,
            setting: s,
        }
    }

    #[test]
    fn high_ground_meeting_exposed_sea_is_cliff() {
        assert_eq!(
            classify(&cell(true, 40, 500, setting(150, 30, 150))),
            ShoreClass::Cliff
        );
    }

    #[test]
    fn low_embayed_shores_are_beaches_and_exposed_hard_rock_gives_shingle() {
        assert_eq!(
            classify(&cell(true, 3, 350, setting(90, 20, 100))),
            ShoreClass::SandBeach,
            "a bay without a river still collects sand"
        );
        assert_eq!(
            classify(&cell(true, 6, 500, setting(200, 80, 160))),
            ShoreClass::ShingleBeach
        );
        assert_eq!(
            classify(&cell(true, 12, 600, setting(120, 20, 150))),
            ShoreClass::RockyShore,
            "a moderate headland without sediment is rocky"
        );
    }

    #[test]
    fn sheltered_muddy_margins_are_marsh_and_flats_and_mouths_estuaries() {
        let quiet = setting(30, 120, 120);
        assert_eq!(classify(&cell(true, 2, 400, quiet)), ShoreClass::Marsh);
        assert_eq!(classify(&cell(false, 0, 400, quiet)), ShoreClass::TidalFlat);
        let mut mouth = cell(false, 0, 400, setting(120, 200, 120));
        mouth.mouth_km2 = 300;
        assert_eq!(classify(&mouth), ShoreClass::Estuary);
    }

    #[test]
    fn survey_finds_a_cliffed_coast_and_a_barrier_island() {
        // Mainland plateau (40 m) in the north; a barrier strip offshore.
        let (w, h) = (512, 512);
        let mut g = Lattice::new(w, h, 39_062_500).unwrap();
        let mut barrier = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                g.z[i] = if y < 200 {
                    40_000
                } else if (260..266).contains(&y) && (100..400).contains(&x) {
                    barrier.push(i as u32);
                    2_000
                } else {
                    -8_000
                };
            }
        }
        let built = Builders {
            barrier_cells: &barrier,
            ..Builders::default()
        };
        let layer = survey(&g, 100_000_000, 3, &built).unwrap();
        assert_eq!(layer.islands.len(), 1);
        assert_eq!(layer.islands[0].cause, IslandCause::Barrier);
        let cliffs = layer
            .classes
            .iter()
            .filter(|&&c| c == ShoreClass::Cliff)
            .count();
        assert!(cliffs > 50, "the plateau edge is cliffed: {cliffs}");
    }

    #[test]
    fn low_headlands_keep_beaches_only_on_sand_bodies_or_at_mouths() {
        // Regression (seed-42 full size: 148‰ beach in bays, 200‰ on
        // headlands): any sediment within 6 km made a low headland a beach.
        let fed = setting(120, 200, 120);
        let point = cell(true, 3, 700, fed);
        assert_eq!(classify(&point), ShoreClass::RockyShore);
        let mouth = ShoreCell {
            mouth_km2: 40,
            ..point
        };
        assert_eq!(classify(&mouth), ShoreClass::SandBeach);
        let barrier = ShoreCell {
            sand_body: true,
            ..point
        };
        assert_eq!(classify(&barrier), ShoreClass::SandBeach);
        // A straight low shore with sediment keeps its beach.
        assert_eq!(classify(&cell(true, 3, 500, fed)), ShoreClass::SandBeach);
        // Sheltered delta shores are marsh; the exposed front is sand.
        let delta = ShoreCell {
            delta: true,
            sand_body: true,
            ..cell(true, 2, 400, setting(60, 200, 120))
        };
        assert_eq!(classify(&delta), ShoreClass::Marsh);
        let front = ShoreCell {
            delta: true,
            sand_body: true,
            ..cell(true, 2, 700, setting(160, 200, 120))
        };
        assert_eq!(classify(&front), ShoreClass::SandBeach);
    }
}
