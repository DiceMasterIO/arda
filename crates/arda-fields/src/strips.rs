//! Open-field strips (`04-settlements-roads.md` step 4): each furlong near
//! a village is cut into long strips a few rods wide, ploughed lengthwise,
//! with a grass balk between neighbours and a reverse-S curve from the
//! turning plough team.

use crate::fields::Crop;
use crate::geom::{h2, h3, u01};
use crate::partition::Site;

/// Amplitude of the reverse-S curve, in squares.
const CURVE: f64 = 8.0;
/// Half-length over which the curve develops, in squares.
const CURVE_HALF: f64 = 60.0;

/// Where a square falls in its furlong.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripAt {
    /// Strip index across the furlong.
    pub index: i64,
    /// Whether the square is the grass balk on the strip's edge.
    pub balk: bool,
    /// Unit vector along the strip (the furrow direction).
    pub along: [f64; 2],
    /// Strip width in squares.
    pub width: f64,
}

/// The furlong's strip direction and width.
#[must_use]
pub fn furlong(seed: u64, site: &Site) -> ([f64; 2], f64) {
    let (k0, k1) = site.hkey();
    let h = h2(seed, 0x57A1, k0, k1);
    // The partition already turns neighbouring furlongs to their own
    // directions (along the furlong, across it, or down the slope).
    #[allow(clippy::cast_precision_loss)] // 0..5
    let width = 7.0 + (h >> 20) as f64 % 5.0;
    (site.along, width.floor())
}

/// Locates a square centre `p` within the furlong of `site`.
#[must_use]
pub fn locate(seed: u64, site: &Site, p: [f64; 2]) -> StripAt {
    let (along, width) = furlong(seed, site);
    let d = [p[0] - site.p[0], p[1] - site.p[1]];
    let u = d[0] * along[0] + d[1] * along[1];
    let v = -d[0] * along[1] + d[1] * along[0];
    let t = (u / CURVE_HALF).clamp(-1.0, 1.0);
    let v = v - CURVE * (t * t * t - t);
    let (k0, k1) = site.hkey();
    let off = u01(h2(seed, 0x57A2, k0, k1)) * width;
    let q = v + off;
    #[allow(clippy::cast_possible_truncation)] // strip counts are small
    let index = (q / width).floor() as i64;
    let rem = q - (index as f64) * width;
    StripAt {
        index,
        balk: rem < 1.0,
        along,
        width,
    }
}

/// The crop of one strip: mostly the furlong's, sometimes its own.
#[must_use]
pub fn crop(seed: u64, site: &Site, furlong_crop: Crop, index: i64) -> Crop {
    let (k0, k1) = site.hkey();
    let h = h3(seed, 0x57A3, k0, k1, index);
    if u01(h) < 0.72 {
        furlong_crop
    } else {
        Crop::arable(u01(h >> 17))
    }
}
