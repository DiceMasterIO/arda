//! Field parcels on relief tiles (logic/17 §land-fields): the tactical
//! fields layer's own partition (`arda_fields::partition`), so every parcel,
//! hedgerow and furlong sits where the tactical map puts it.
//!
//! A point's parcel is the leaf of the global land-block hierarchy holding
//! it (`Partition::locate_edge`), with its use and crop decided there once
//! for both layers. The hierarchy's cuts are lines, kinked lines and road
//! centrelines, so each pixel knows its exact distance to the parcel's
//! edge and the parcel across it: tones and hedgerows are anti-aliased
//! without supersampling.

use super::tone::{self, Tone};
use crate::fixed::{cos_sin_q14, organic_q12, smooth, ONE, TRIG_ONE, TURN};
use arda_blocks::society::Bbox;
use arda_fields::fields::{Crop, FieldKind};
use arda_fields::geom::SQUARE_M;
use arda_fields::partition::{Partition, PartitionInputs};
use arda_fields::TerrainSample;
use arda_people::terrain::Surroundings;

/// Slope above which mixed country walls its fields in stone
/// (`arda_fields::boundary`).
const STONY_SLOPE: f64 = 0.12;
/// Hedgerow width (hedge plus its canopy), metres.
const HEDGE_W_M: f64 = 2.6;
/// Drystone wall width, metres.
const WALL_W_M: f64 = 1.0;
/// Headland of grass around a furlong, metres.
const HEADLAND_M: f64 = 4.5;

/// A parcel's boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Wall {
    None,
    Hedge,
    Stone,
}

/// How one site's parcel looks.
#[derive(Debug, Clone, Copy)]
struct Look {
    kind: FieldKind,
    crop: Crop,
    wall: Wall,
    mul: [i64; 3],
}

/// The parcels of a window.
pub struct Parcels {
    seed: u64,
    partition: Partition,
    looks: Vec<Look>,
}

/// Which parcel a point lies in and how far inside it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParcelHit {
    /// Site index.
    pub site: usize,
    /// The parcel across the nearest edge, if any.
    pub other: Option<usize>,
    /// Distance to that edge, metres (positive inside).
    pub edge_m: f64,
}

fn tone_of(kind: FieldKind, crop: Crop) -> Option<Tone> {
    match kind {
        FieldKind::Wild => None,
        FieldKind::Woodland => Some(Tone::Woodland),
        FieldKind::Orchard => Some(Tone::Orchard),
        FieldKind::Common => Some(Tone::Grazed),
        _ => Some(match crop {
            Crop::Ploughed => Tone::Ploughed,
            Crop::Stubble => Tone::Stubble,
            Crop::Fallow => Tone::Fallow,
            Crop::Hay => Tone::Hay,
            Crop::Mown => Tone::Mown,
            Crop::Grazed | Crop::None => Tone::Grazed,
        }),
    }
}

impl Parcels {
    /// The partition over the square rectangle `rect` (x0, y0, x1, y1),
    /// exactly as the fields layer builds it for any window inside.
    #[must_use]
    pub fn build(inputs: &PartitionInputs<'_>, seed: u64, rect: (i64, i64, i64, i64)) -> Self {
        let partition = Partition::new(inputs, seed, rect);
        let looks = partition
            .sites
            .iter()
            .map(|st| {
                let (kind, crop) = (st.kind, st.crop);
                let wall = if !kind.enclosed() {
                    Wall::None
                } else if st.slope > STONY_SLOPE {
                    Wall::Stone
                } else {
                    Wall::Hedge
                };
                // Worked parcels differ a little from their neighbours;
                // woods and commons run on without seams.
                let jitter = kind.enclosed() || matches!(kind, FieldKind::Strips);
                let mul = tone_of(kind, crop).map_or([ONE; 3], |t| {
                    if jitter {
                        tone::compose(t.multiplier(), tone::parcel_jitter(seed, st.hkey()))
                    } else {
                        t.multiplier()
                    }
                });
                Look {
                    kind,
                    crop,
                    wall,
                    mul,
                }
            })
            .collect();
        Self {
            seed,
            partition,
            looks,
        }
    }

    /// The parcels of a relief window (`bbox` in world metres), from the
    /// inputs the tactical fields layer reads (arda-blocks): the land-use
    /// raster with crofts as arable, heights and rivers of the
    /// surroundings, settle's roads and the settlements within the
    /// partition's reach, and the fields layer's culture and wealth.
    #[must_use]
    pub fn for_window(
        land: &super::Landscape,
        s: &Surroundings,
        bbox: [f64; 4],
        seed: u64,
    ) -> Self {
        let terrain = |x: f64, y: f64| TerrainSample {
            height_m: s.height_m(x, y),
            water_depth_m: 0.0,
        };
        let far = arda_fields::partition::INPUT_REACH_M + 100.0;
        let near = super::settlements_near(land.people(), bbox, far);
        let window = Bbox {
            x0: bbox[0],
            y0: bbox[1],
            x1: bbox[2],
            y1: bbox[3],
        };
        let roads: Vec<arda_fields::Road> = land
            .overlays
            .field_roads()
            .iter()
            .filter(|(b, _)| b.near(&window, far))
            .map(|(_, r)| r.clone())
            .collect();
        let rivers = arda_blocks::society::fields_rivers(s);
        #[allow(clippy::cast_possible_truncation)] // world squares fit i64
        let sq = |m: f64| (m / SQUARE_M).floor() as i64;
        let rect = (sq(bbox[0]), sq(bbox[1]), sq(bbox[2]) + 1, sq(bbox[3]) + 1);
        let inputs = PartitionInputs {
            landuse: land.overlays.landuse(),
            terrain: &terrain,
            settlements: &near,
            roads: &roads,
            rivers: &rivers,
            culture: arda_blocks::society::FIELDS_CULTURE,
            wealth: arda_blocks::society::FIELDS_WEALTH,
        };
        Self::build(&inputs, seed, rect)
    }

    /// The parcel at `q` (square units).
    #[must_use]
    pub fn at(&self, q: [f64; 2]) -> Option<ParcelHit> {
        self.partition.locate_edge(q).map(|h| ParcelHit {
            site: h.site,
            other: h.other,
            edge_m: h.edge * SQUARE_M,
        })
    }

    /// The global key of a site (block and path), the same in every
    /// window.
    #[must_use]
    pub fn site_key(&self, site: usize) -> Option<(i64, i64, u32)> {
        self.partition.sites.get(site).map(|s| s.key)
    }

    /// Whether a site's parcel is walled in (hedge or drystone).
    #[must_use]
    pub fn walled(&self, site: usize) -> bool {
        self.looks.get(site).is_some_and(|l| l.wall != Wall::None)
    }

    /// Whether a site's parcel is worked land (anything but wild).
    #[must_use]
    pub fn worked(&self, site: usize) -> bool {
        self.looks
            .get(site)
            .is_some_and(|l| l.kind != FieldKind::Wild)
    }

    /// Whether a site's parcel is woodland.
    #[must_use]
    pub fn woodland(&self, site: usize) -> bool {
        self.looks
            .get(site)
            .is_some_and(|l| l.kind == FieldKind::Woodland)
    }

    /// The tone multipliers (Q12) of a site's parcel at `q` (square units),
    /// with strips, furrows and orchard rows once pixels resolve them.
    fn multiplier(&self, site: usize, q: [f64; 2], pixel_m: f64, edge_m: f64) -> [i64; 3] {
        let Some(look) = self.looks.get(site) else {
            return [ONE; 3];
        };
        let st = &self.partition.sites[site];
        let mut mul = look.mul;
        let mut along = st.along;
        if look.kind == FieldKind::Strips && pixel_m <= 6.0 {
            let at = arda_fields::strips::locate(self.seed, st, q);
            along = at.along;
            let crop = arda_fields::strips::crop(self.seed, st, look.crop, at.index);
            if let Some(t) = tone_of(FieldKind::Arable, crop) {
                let (k0, k1) = st.hkey();
                let key = (k0, k1 * 64 + at.index);
                mul = tone::compose(t.multiplier(), tone::parcel_jitter(self.seed, key));
            }
            // logic/17 §land-fields: balks are one square of grass, and a
            // headland of grass runs round the furlong.
            if at.balk && pixel_m <= 3.2 || edge_m < HEADLAND_M {
                mul = std::array::from_fn(|k| tone::BALK[k] * ONE / tone::REFERENCE[k]);
            }
        }
        if matches!(look.crop, Crop::Ploughed | Crop::Stubble) {
            mul = furrows(mul, along, q, pixel_m);
        }
        mul
    }

    /// Paints a worked parcel's tone over `rgb` at `q` (square units),
    /// blended with its neighbour's across the edge. `strength` (Q12)
    /// fades the layer in with zoom.
    #[must_use]
    pub fn paint_tone(
        &self,
        rgb: [u8; 3],
        hit: ParcelHit,
        q: [f64; 2],
        pixel_m: f64,
        strength_q12: i64,
    ) -> [u8; 3] {
        let own = self.multiplier(hit.site, q, pixel_m, hit.edge_m);
        // Box-filtered share of the pixel on the parcel's own side.
        let share = (0.5 + hit.edge_m / pixel_m).clamp(0.0, 1.0);
        let mul = match hit.other {
            Some(o) if share < 1.0 && self.worked(o) => {
                let theirs = self.multiplier(o, q, pixel_m, 0.0);
                let w = q12(share);
                std::array::from_fn(|k| (own[k] * w + theirs[k] * (ONE - w)) / ONE)
            }
            Some(_) if share < 1.0 => {
                // Against wild land the tone fades out across the edge.
                let w = q12(share);
                own.map(|m| ONE + (m - ONE) * w / ONE)
            }
            _ => own,
        };
        // Soil and growth vary gently within a field (rotated noise at
        // 55 m, ±3.5 %).
        let (x, y) = um(q);
        let n = organic_q12(self.seed, 0x736f_696c, x, y, 55_000_000) - ONE / 2;
        let k = ONE + n * 7 / 100;
        let mul = mul.map(|m| m * k / ONE);
        tone::tint(rgb, tone::soften(mul, strength_q12))
    }

    /// Paints the hedgerow or drystone wall on the parcel's edge, when
    /// either parcel is enclosed (the hedge wins where kits differ).
    #[must_use]
    pub fn paint_wall(
        &self,
        rgb: [u8; 3],
        hit: ParcelHit,
        q: [f64; 2],
        pixel_m: f64,
        strength_q12: i64,
    ) -> [u8; 3] {
        let Some(o) = hit.other else {
            return rgb;
        };
        let walls = [self.looks[hit.site].wall, self.looks[o].wall];
        // Walls read as lines from close up and as a faint grain from afar.
        #[allow(clippy::cast_possible_truncation)]
        let far = smooth(4_000, 20_000, (pixel_m * 1_000.0) as i64);
        let (width, colour, alpha) = if walls.contains(&Wall::Hedge) {
            (HEDGE_W_M, tone::HEDGE, ONE * 85 / 100 - far * 40 / 100)
        } else if walls.contains(&Wall::Stone) {
            (WALL_W_M, tone::DRYSTONE, ONE * 70 / 100 - far * 30 / 100)
        } else {
            return rgb;
        };
        // A living hedge swells, thins and gaps along its length (rotated
        // noise at 16 m); walls only weather.
        let (x, y) = um(q);
        let n = organic_q12(self.seed, 0x6865_6467, x, y, 16_000_000);
        let (width, alpha) = if colour == tone::HEDGE {
            let f = 0.45 + 1.1 * n as f64 / ONE as f64;
            (width * f, alpha * (ONE * 3 / 4 + n / 2) / ONE)
        } else {
            (width, alpha * (ONE * 9 / 10 + n / 5) / ONE)
        };
        let cover = line_cover(hit.edge_m, width, pixel_m);
        if cover <= 0.0 {
            return rgb;
        }
        tone::mix(
            rgb,
            colour,
            q12(cover) * alpha.min(ONE) / ONE * strength_q12 / ONE,
        )
    }

    /// Orchard trees in rows along the parcel (Q12 coverage) at `q`.
    #[must_use]
    pub fn orchard_tree(&self, site: usize, q: [f64; 2], pixel_m: f64) -> i64 {
        #[allow(clippy::cast_possible_truncation)] // pixel sizes are small
        let keep = ONE - smooth(3_000, 8_000, (pixel_m * 1_000.0) as i64);
        if keep == 0
            || self
                .looks
                .get(site)
                .is_none_or(|l| l.kind != FieldKind::Orchard)
        {
            return 0;
        }
        let st = &self.partition.sites[site];
        let a = st.along;
        let d = [q[0] - st.p[0], q[1] - st.p[1]];
        // Trees every 5 squares along the rows, rows 4 squares apart.
        let u = (d[0] * a[0] + d[1] * a[1]) / 5.0;
        let v = (-d[0] * a[1] + d[1] * a[0]) / 4.0;
        let du = (u - u.round()) * 5.0 * SQUARE_M;
        let dv = (v - v.round()) * 4.0 * SQUARE_M;
        let r = du.hypot(dv);
        let cover = ((2.4 - r) / pixel_m.max(0.5) + 0.5).clamp(0.0, 1.0);
        q12(cover) * keep / ONE
    }
}

/// World micrometres of a point in square units.
#[allow(clippy::cast_possible_truncation)] // world extents fit i64
fn um(q: [f64; 2]) -> (i64, i64) {
    let s = SQUARE_M * 1e6;
    ((q[0] * s) as i64, (q[1] * s) as i64)
}

/// Box-filtered coverage of a line of `width` centred at signed distance
/// `d` from a pixel of `pixel` (all metres).
#[must_use]
pub fn line_cover(d: f64, width: f64, pixel: f64) -> f64 {
    let (lo, hi) = (d.abs() - pixel / 2.0, d.abs() + pixel / 2.0);
    let overlap = hi.min(width / 2.0) - lo.max(-width / 2.0);
    (overlap / pixel).clamp(0.0, 1.0)
}

/// Q12 of a share in `[0, 1]`.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // within [0, ONE]
pub fn q12(v: f64) -> i64 {
    (v.clamp(0.0, 1.0) * ONE as f64).round() as i64
}

/// Plough furrows (about 2.6 m apart) across a ploughed or stubble field,
/// faded before they alias.
fn furrows(mul: [i64; 3], along: [f64; 2], q: [f64; 2], pixel_m: f64) -> [i64; 3] {
    const PERIOD_M: f64 = 2.6;
    #[allow(clippy::cast_possible_truncation)]
    let keep = ONE - smooth(1_100, 2_000, (pixel_m / PERIOD_M * 1_000.0) as i64);
    if keep == 0 {
        return mul;
    }
    let v = (-q[0] * along[1] + q[1] * along[0]) * SQUARE_M / PERIOD_M;
    #[allow(clippy::cast_possible_truncation)]
    let phase = (v.rem_euclid(1.0) * TURN as f64) as i64;
    let (c, _) = cos_sin_q14(phase);
    let m = ONE + c * ONE / TRIG_ONE * 7 / 100 * keep / ONE;
    mul.map(|x| x * m / ONE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_cover_integrates_the_box() {
        assert!((line_cover(0.0, 2.0, 1.0) - 1.0).abs() < 1e-12);
        assert!((line_cover(0.0, 1.0, 4.0) - 0.25).abs() < 1e-12);
        assert!(line_cover(3.0, 2.0, 1.0) == 0.0);
        assert!((line_cover(1.0, 2.0, 1.0) - 0.5).abs() < 1e-12);
    }
}
