//! Worked land on relief tiles (logic/17 §land): field parcels with
//! hedgerows, pasture, orchards and woodland, roads and lanes, and the
//! settlements' buildings and streets, drawn from the same society data
//! and the same geometry the tactical layer composes (arda-blocks), so the
//! map keeps its parcels, roads and buildings in place across the
//! relief → tactical hand-off, as it keeps its water (§water).
//!
//! Detail follows the pixel (metres per pixel):
//!
//! | pixel | drawn |
//! |---|---|
//! | > 45 m | land use as a soft tone from the 100 m raster |
//! | ≤ 45 m | parcels (fading in to full at 20 m), hedgerows, roads |
//! | ≤ 20 m | settlements from their plans |
//! | ≤ 6 m | strips, furrows, orchard rows, tree crowns, roof pitches, shadows |
//!
//! A world without `society/` has no [`Landscape`] and renders exactly as
//! before.

pub mod fields;
pub mod roads;
pub mod tone;
pub mod town;

use crate::fixed::{smooth, ONE};
use crate::lowrelief::LowRelief;
use crate::pyramid::Pyramid;
use crate::source::Terrain as _;
use crate::world::ReliefWorld;
use crate::{MidzoomError, SurfacePoint};
use arda_blocks::society::{Bbox, SocietyOverlays};
use arda_core::Cover;
use arda_fields::geom::{Sq, SQUARE_M};
use arda_fields::{Settlement, Tier};
use arda_people::terrain::Surroundings;
use fields::Parcels;
use roads::{PixelGrid, RoadRaster};
use std::sync::Arc;
use tone::Tone;
use town::Towns;

/// Pixels at or below this size show parcels, metres.
pub const PARCEL_MAX_M: f64 = 45.0;
/// Pixels at or below this size show settlements from their plans, metres.
pub const TOWN_MAX_M: f64 = 20.0;
/// Strength (Q12) of the 100 m land-use tone on pixels too coarse for
/// parcels: the parcels' own strength there.
pub const CELL_TONE_Q12: i64 = 1_720;
/// Pixels at or below this size show road networks, metres.
pub const ROAD_MAX_M: f64 = 128.0;

/// A world's society data as relief tiles read it.
#[derive(Clone)]
pub struct Landscape {
    overlays: Arc<SocietyOverlays>,
}

impl std::fmt::Debug for Landscape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Landscape").finish_non_exhaustive()
    }
}

impl Landscape {
    /// The landscape of the tactical overlays (shared with the server's
    /// tactical blocks, so both read one land-use raster and road list).
    #[must_use]
    pub const fn new(overlays: Arc<SocietyOverlays>) -> Self {
        Self { overlays }
    }

    /// Opens `<dir>/society/`; `None` when the world has none.
    ///
    /// # Errors
    /// Unreadable or malformed society files.
    pub fn open(dir: &std::path::Path) -> Result<Option<Self>, MidzoomError> {
        if !dir.join("society").join("settlements.json").exists() {
            return Ok(None);
        }
        let e = |x: String| MidzoomError::Society(x);
        let people = arda_people::World::open(dir).map_err(|x| e(x.to_string()))?;
        let overlays = SocietyOverlays::open(Arc::new(people)).map_err(|x| e(x.to_string()))?;
        Ok(Some(Self::new(Arc::new(overlays))))
    }

    /// The settlements, plans and files.
    #[must_use]
    pub fn people(&self) -> &arda_people::World {
        self.overlays.world()
    }
}

/// How strongly parcels tint the ground (Q12): full once fields are a few
/// dozen pixels across (3 m), easing through 0.7 at 8 m and 0.55 at 15 m
/// to 0.42 from 30 m out, so the patchwork reads as tone from afar and
/// never as confetti. Coarser pixels show the 100 m raster at the same
/// strength ([`CELL_TONE_Q12`]).
#[must_use]
pub fn parcel_strength(pixel_m: f64) -> i64 {
    const KNOTS: [(f64, f64); 5] = [
        (3.0, 1.0),
        (8.0, 0.7),
        (15.0, 0.55),
        (30.0, 0.42),
        (64.0, 0.42),
    ];
    let mut v = KNOTS[KNOTS.len() - 1].1;
    if pixel_m <= KNOTS[0].0 {
        v = 1.0;
    } else {
        for w in KNOTS.windows(2) {
            let ((x0, y0), (x1, y1)) = (w[0], w[1]);
            if pixel_m <= x1 {
                v = y0 + (y1 - y0) * (pixel_m - x0) / (x1 - x0);
                break;
            }
        }
    }
    #[allow(clippy::cast_possible_truncation)] // within [0, ONE]
    let q = (v * ONE as f64).round() as i64;
    q
}

/// The global square containing world metre coordinate `m`.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // world squares fit i64
pub fn square_of(m: f64) -> i64 {
    (m / SQUARE_M).floor() as i64
}

fn fields_tier(t: arda_settle::model::Tier) -> Tier {
    use arda_settle::model::Tier as T;
    match t {
        T::Hamlet => Tier::Hamlet,
        T::Village => Tier::Village,
        T::Town => Tier::Town,
        T::City => Tier::City,
    }
}

/// Everything one window's land layer reads.
pub struct LandWindow {
    grid: PixelGrid,
    seed: u64,
    low: LowRelief,
    parcels: Option<Parcels>,
    parcel_strength: i64,
    cells: Option<CellTones>,
    forest: CellField,
    roads: Option<RoadRaster>,
    towns: Option<Towns>,
}

impl std::fmt::Debug for LandWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LandWindow")
            .field("grid", &self.grid)
            .finish_non_exhaustive()
    }
}

/// A per-100 m-cell value (Q12) over a window, read bilinearly between
/// cell centres so it never shows the cells' edges.
#[derive(Debug, Clone)]
struct CellField {
    c0: (i64, i64),
    w: i64,
    h: i64,
    v: Vec<[i64; 3]>,
}

impl CellField {
    fn build(
        c0: (i64, i64),
        c1: (i64, i64),
        f: impl Fn(i64, i64) -> Result<[i64; 3], MidzoomError>,
    ) -> Result<Self, MidzoomError> {
        let (w, h) = (c1.0 - c0.0 + 1, c1.1 - c0.1 + 1);
        let mut v = Vec::with_capacity(usize::try_from(w * h).unwrap_or(0));
        for gy in c0.1..=c1.1 {
            for gx in c0.0..=c1.0 {
                v.push(f(gx, gy)?);
            }
        }
        Ok(Self { c0, w, h, v })
    }

    /// Bilinear value at world metres.
    #[allow(clippy::cast_possible_truncation)]
    fn at(&self, x: f64, y: f64) -> [i64; 3] {
        let (u, v) = (x / 100.0 - 0.5, y / 100.0 - 0.5);
        let (i, j) = (u.floor() as i64, v.floor() as i64);
        let (fx, fy) = (
            ((u - u.floor()) * ONE as f64) as i64,
            ((v - v.floor()) * ONE as f64) as i64,
        );
        let get = |di: i64, dj: i64| {
            let x = (i + di - self.c0.0).clamp(0, self.w - 1);
            let y = (j + dj - self.c0.1).clamp(0, self.h - 1);
            self.v[usize::try_from(y * self.w + x).unwrap_or(0)]
        };
        let (a, b, c, d) = (get(0, 0), get(1, 0), get(0, 1), get(1, 1));
        std::array::from_fn(|k| {
            let top = a[k] + (b[k] - a[k]) * fx / ONE;
            let bot = c[k] + (d[k] - c[k]) * fx / ONE;
            top + (bot - top) * fy / ONE
        })
    }
}

/// Land-use tones per cell for coarse pixels.
type CellTones = CellField;

/// The average look of a `landuse.bin` code from afar (Q12 multipliers).
fn code_multiplier(code: u8) -> [i64; 3] {
    use arda_ids::LandUse as L;
    let t = match L::from_code(code) {
        Some(L::Field | L::Fallow) => {
            let (a, b) = (Tone::Ploughed.multiplier(), Tone::Stubble.multiplier());
            return std::array::from_fn(|k| (a[k] + b[k]) / 2);
        }
        Some(L::Pasture | L::Farmstead) => Tone::Grazed,
        Some(L::Orchard) => Tone::Orchard,
        Some(L::Woodland) => Tone::Woodland,
        Some(L::Meadow | L::Mill) => Tone::Hay,
        Some(L::Built) => return [150 * ONE / 111, 128 * ONE / 119, 100 * ONE / 62],
        Some(L::None | L::Mine) | None => return [ONE; 3],
    };
    t.multiplier()
}

impl LandWindow {
    /// Gathers the land layer of the pixel window at `origin` of `size` at
    /// level `z`.
    ///
    /// # Errors
    /// A world or society read failed, or a plan could not be drawn.
    pub fn gather(
        rw: &ReliefWorld,
        land: &Landscape,
        pyramid: &Pyramid,
        z: u32,
        origin: (i64, i64),
        size: (u32, u32),
    ) -> Result<Self, MidzoomError> {
        #[allow(clippy::cast_precision_loss)] // exact enough for metres
        let pixel_m = pyramid.longest_um() as f64 / Pyramid::pixels(z) as f64 / 1e6;
        let grid = PixelGrid {
            origin,
            size: (size.0 as usize, size.1 as usize),
            pixel_m,
        };
        let (w, h) = (i64::from(size.0), i64::from(size.1));
        let bbox = [
            grid.centre_m(origin.0) - pixel_m,
            grid.centre_m(origin.1) - pixel_m,
            grid.centre_m(origin.0 + w - 1) + pixel_m,
            grid.centre_m(origin.1 + h - 1) + pixel_m,
        ];
        let people = land.people();
        let seed = people.seed();
        let fine = |m: f64| {
            #[allow(clippy::cast_possible_truncation)]
            let um = (m * 1e6) as i64;
            um - arda_core::FINE_FRAME_OFFSET_UM
        };
        let low = LowRelief::gather(
            rw.terrain(),
            (fine(bbox[0]), fine(bbox[1]), fine(bbox[2]), fine(bbox[3])),
        )?;
        #[allow(clippy::cast_possible_truncation)]
        let cell = |m: f64| (m / 100.0).floor() as i64;
        let (c0, c1) = (
            (cell(bbox[0]) - 1, cell(bbox[1]) - 1),
            (cell(bbox[2]) + 1, cell(bbox[3]) + 1),
        );
        let terrain = rw.terrain();
        let forest = CellField::build(c0, c1, |gx, gy| {
            let f = if terrain.cell(gx, gy)?.cover == Cover::Forest {
                ONE
            } else {
                0
            };
            Ok([f; 3])
        })?;
        let landuse = land.overlays.landuse();
        let cells = (pixel_m > PARCEL_MAX_M)
            .then(|| {
                CellField::build(c0, c1, |gx, gy| {
                    Ok(code_multiplier(landuse.code_at(gx, gy).unwrap_or(0)))
                })
            })
            .transpose()?;
        // Roads plan within 600 m; the field partition reads every land
        // block meeting the window (logic/17 §land-fields).
        let reach = arda_fields::partition::PARTITION_REACH_M + 250.0;
        let centre = ((bbox[0] + bbox[2]) / 2.0, (bbox[1] + bbox[3]) / 2.0);
        let radius = (bbox[2] - bbox[0]).hypot(bbox[3] - bbox[1]) / 2.0 + reach;
        let needs_heights = pixel_m <= PARCEL_MAX_M || pixel_m <= ROAD_MAX_M;
        let heights = if needs_heights {
            Some(
                Surroundings::around(&people.src, centre, radius)
                    .map_err(|e| MidzoomError::Society(e.to_string()))?,
            )
        } else {
            None
        };
        let parcels = match (&heights, pixel_m <= PARCEL_MAX_M) {
            (Some(s), true) => Some(Parcels::for_window(land, s, bbox, seed)),
            _ => None,
        };
        let parcel_strength = parcel_strength(pixel_m);
        let towns = (pixel_m <= TOWN_MAX_M)
            .then(|| Towns::gather(people, bbox, 0.0))
            .transpose()?;
        let roads = match &heights {
            Some(s) if pixel_m <= ROAD_MAX_M => {
                let near: Vec<arda_ways::Road> = land
                    .overlays
                    .ways_roads()
                    .iter()
                    .filter(|(b, _)| {
                        b.near(
                            &Bbox {
                                x0: bbox[0],
                                y0: bbox[1],
                                x1: bbox[2],
                                y1: bbox[3],
                            },
                            600.0,
                        )
                    })
                    .map(|(_, r)| r.clone())
                    .collect();
                let skip =
                    |gx: i64, gy: i64| towns.as_ref().is_some_and(|t| t.replaces_road(gx, gy));
                Some(RoadRaster::build(grid, &near, s, seed, &skip))
            }
            _ => None,
        };
        Ok(Self {
            grid,
            seed,
            low,
            parcels,
            parcel_strength,
            cells,
            forest,
            roads,
            towns,
        })
    }

    /// World metres of window pixel `(px, py)`.
    fn centre(&self, px: usize, py: usize) -> [f64; 2] {
        let i = |p: usize, o: i64| o + i64::try_from(p).unwrap_or(0);
        [
            self.grid.centre_m(i(px, self.grid.origin.0)),
            self.grid.centre_m(i(py, self.grid.origin.1)),
        ]
    }

    /// The ground of a land pixel before rivers: low-relief drainage, land
    /// use and canopy. `(lx, ly)` are fine-lattice micrometres and `c` the
    /// refined surface there.
    #[must_use]
    pub fn ground(
        &self,
        rgb: [u8; 3],
        (px, py): (usize, usize),
        (lx, ly): (i64, i64),
        c: SurfacePoint,
    ) -> [u8; 3] {
        let (low, floor) = self.low.shade(lx, ly, c);
        let mut out = tone::tint(rgb, low);
        let m = self.centre(px, py);
        #[allow(clippy::cast_possible_truncation)] // pixel sizes are small
        let pixel_um = (self.grid.pixel_m * 1e6) as i64;
        let mut canopy = self.forest.at(m[0], m[1])[0];
        if let Some(cells) = &self.cells {
            #[allow(clippy::cast_possible_truncation)]
            let s = CELL_TONE_Q12 * (ONE - smooth(70_000, 160_000, pixel_um / 1_000)) / ONE;
            out = tone::tint(out, tone::soften(cells.at(m[0], m[1]), s));
        }
        if let Some(parcels) = &self.parcels {
            let q = [m[0] / SQUARE_M, m[1] / SQUARE_M];
            let sq = Sq::containing(q);
            // At about one pixel per square, decide at the square's centre
            // exactly as the tactical layer does.
            let at = if self.grid.pixel_m <= 2.0 * SQUARE_M {
                sq.centre()
            } else {
                q
            };
            match parcels.at(at) {
                Some(hit) if parcels.worked(hit.site) => {
                    let px_m = self.grid.pixel_m;
                    out = parcels.paint_tone(out, hit, at, px_m, self.parcel_strength);
                    out = parcels.paint_wall(out, hit, at, px_m, self.parcel_strength);
                    canopy = if parcels.woodland(hit.site) { ONE } else { 0 };
                    let tree = parcels.orchard_tree(hit.site, at, self.grid.pixel_m);
                    if tree > 0 {
                        out = tone::mix(out, [62, 84, 40], tree * 85 / 100);
                    }
                }
                hit => {
                    // Unworked valley floors: wet floodplain meadow.
                    if floor > 0 && canopy < ONE / 2 {
                        let t = Tone::Floodplain.multiplier();
                        let s = floor * self.parcel_strength / ONE * 35 / 100;
                        out = tone::tint(out, tone::soften(t, s));
                    }
                    if let Some(h) = hit {
                        out =
                            parcels.paint_wall(out, h, at, self.grid.pixel_m, self.parcel_strength);
                    }
                }
            }
        }
        let k = tone::canopy(self.seed, lx, ly, pixel_um, canopy);
        if k != ONE {
            out = tone::tint(out, [k; 3]);
        }
        out
    }

    /// Roads and settlements over pixel `(px, py)`, after rivers.
    #[must_use]
    pub fn works(&self, rgb: [u8; 3], (px, py): (usize, usize)) -> [u8; 3] {
        let mut out = rgb;
        if let Some(r) = &self.roads {
            out = r.paint(out, px, py);
        }
        if let Some(t) = &self.towns {
            let road = |_: i64, _: i64| {
                self.roads
                    .as_ref()
                    .is_some_and(|r| r.class_at(px, py).is_some())
            };
            out = t.paint(out, self.centre(px, py), self.grid.pixel_m, &road);
        }
        out
    }

    /// The road class drawn over at least half of pixel `(px, py)`.
    #[must_use]
    pub fn road_at(&self, px: usize, py: usize) -> Option<arda_ways::RoadClass> {
        self.roads.as_ref()?.class_at(px, py)
    }

    /// Whether pixel `(px, py)`'s centre square is a building.
    #[must_use]
    pub fn building_at(&self, px: usize, py: usize) -> bool {
        let m = self.centre(px, py);
        let claim = self
            .towns
            .as_ref()
            .and_then(|t| t.claim(square_of(m[0]), square_of(m[1])));
        town::is_building(claim)
    }

    /// Whether a field wall (hedge or drystone) runs within half a square
    /// of pixel `(px, py)`'s centre.
    #[must_use]
    pub fn boundary_at(&self, px: usize, py: usize) -> bool {
        let Some(parcels) = &self.parcels else {
            return false;
        };
        let m = self.centre(px, py);
        let q = [m[0] / SQUARE_M, m[1] / SQUARE_M];
        let sq = Sq::containing(q);
        parcels.at(sq.centre()).is_some_and(|h| {
            h.other
                .is_some_and(|o| parcels.walled(h.site) || parcels.walled(o))
                && h.edge_m.abs() <= SQUARE_M
        })
    }
}

/// Settlements within `margin` (per axis) of the box, as the fields layer
/// reads them.
fn settlements_near(people: &arda_people::World, b: [f64; 4], margin: f64) -> Vec<Settlement> {
    people
        .files
        .settlements
        .settlements
        .iter()
        .filter_map(|s| {
            #[allow(clippy::cast_precision_loss)] // world metres
            let (x, y) = (s.x_m as f64, s.y_m as f64);
            let near = x >= b[0] - margin
                && x <= b[2] + margin
                && y >= b[1] - margin
                && y <= b[3] + margin;
            near.then(|| Settlement {
                x_m: x,
                y_m: y,
                tier: fields_tier(s.tier),
                population: s.population,
            })
        })
        .collect()
}
