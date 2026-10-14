//! Kitchen-garden beds (1.0 task 17): the rows of dug beds in a croft
//! kitchen parcel. A pure function of the plan seed and a global square, so
//! blocks and chunks join without seams and both fills (rules and WFC) lay
//! the same garden.

use crate::plan::croft::{local, parcel};
use crate::rng::{hash_i, mix};

/// Beds sit in parcel columns and rows `1..=LAST`; the rest is a bare path
/// round the garden.
const FIRST: i64 = 1;
/// Last bed column along a row.
const LAST_ALONG: i64 = 15;
/// Last bed row across the rows.
const LAST_ACROSS: i64 = 10;

/// The bed standing on global square `(gx, gy)` of a kitchen parcel, if any.
///
/// Rows run along the parcel's long side or across it (by parcel); every
/// third row is a path, a few squares lie fallow, and each row grows one
/// crop: mostly vegetables, now and then herbs or bean poles.
#[must_use]
pub fn bed(seed: u64, gx: i64, gy: i64) -> Option<&'static str> {
    let p = parcel(seed, gx, gy);
    let (lx, ly) = local(seed, gx, gy);
    let (row, along, last_row, last_along) = if p & 2 == 0 {
        (ly, lx, LAST_ACROSS, LAST_ALONG)
    } else {
        (lx, ly, LAST_ALONG, LAST_ACROSS)
    };
    if !(FIRST..=last_row).contains(&row)
        || !(FIRST..=last_along).contains(&along)
        || (row - FIRST) % 3 == 2
    {
        return None;
    }
    if hash_i(seed ^ 0x0BED_5EED, gx, gy) % 100 < 6 {
        return None;
    }
    Some(
        match mix(p ^ 0x0C20_9A55 ^ u64::try_from(row).unwrap_or(0)) % 10 {
            0..=6 => "veg.vegetable_bed",
            7 | 8 => "veg.herb_bed",
            _ => "veg.bean_poles",
        },
    )
}
