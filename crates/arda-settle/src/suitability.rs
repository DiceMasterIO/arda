//! Per-cell suitability as a place to live (spec step 1; artifact "Where
//! people settle").
//!
//! Every component is a per-mille score; the weighted sum is per-mille too.
//! Water within a short walk counts most, arable land within a kilometre
//! nearly as much, then flat ground, warmth, a view, a sheltered coast and
//! the natural nodes of travel. Refused cells score zero.

use crate::error::SettleError;
use crate::field::{falloff, rise, Integral};
use crate::grid::{filled, Grid};
use crate::tags::{self, Sites};

/// Radius, in cells, of the "land a village can reach before breakfast" box
/// (17 × 17 cells ≈ the area of a 1 km disc).
pub const ARABLE_R: i64 = 8;

/// Component weights, summing to 100.
const W_WATER: i64 = 30;
const W_ARABLE: i64 = 26;
const W_FLAT: i64 = 10;
const W_WARM: i64 = 10;
const W_VIEW: i64 = 8;
const W_COAST: i64 = 6;
const W_NODE: i64 = 10;

/// Base suitability plus the arable share around each cell.
#[derive(Debug, Clone)]
pub struct Suitability {
    /// Base score, per-mille; 0 on refused cells.
    pub score: Vec<u16>,
    /// Arable share of the surrounding kilometre, per-mille.
    pub arable_pm: Vec<u16>,
}

/// Scores every cell.
///
/// # Errors
/// [`SettleError::Reserve`] when a raster cannot be allocated.
pub fn compute(g: &Grid, sites: &Sites) -> Result<Suitability, SettleError> {
    let arable = Integral::new(g.width, g.height, |i| {
        i64::from(sites.tags[i] & tags::ARABLE != 0)
    })?;
    let mut score = filled(g.len(), 0_u16, "suitability")?;
    let mut arable_pm = filled(g.len(), 0_u16, "arable share")?;
    for i in 0..g.len() {
        if !g.is_land(i) {
            continue;
        }
        let (x, y) = g.xy(i);
        let share = arable.permille(x, y, ARABLE_R);
        arable_pm[i] = u16::try_from(share).unwrap_or(0);
        let t = sites.tags[i];
        if t & tags::REFUSED != 0 {
            continue;
        }
        let water = falloff(i64::from(sites.water_m[i]), 150, 750);
        let farm = rise(share, 0, 600);
        let flat = falloff(i64::from(g.slope_md[i]), 2000, 14_000);
        let warm = rise(i64::from(g.temp_cc[i]), 300, 1200);
        let view = rise(i64::from(sites.prominence_m[i]), 0, 30);
        let coast = if t & tags::HARBOUR != 0 {
            1000
        } else if t & tags::ESTUARY != 0 {
            700
        } else if t & (tags::COAST | tags::LAKE) != 0 {
            250
        } else {
            0
        };
        let node = if t & tags::CONFLUENCE != 0 {
            1000
        } else if t & (tags::FORD | tags::BRIDGE) != 0 {
            800
        } else {
            0
        };
        let total = W_WATER * water
            + W_ARABLE * farm
            + W_FLAT * flat
            + W_WARM * warm
            + W_VIEW * view
            + W_COAST * coast
            + W_NODE * node;
        score[i] = u16::try_from((total / 100).clamp(0, 1000)).unwrap_or(0);
    }
    Ok(Suitability { score, arable_pm })
}
