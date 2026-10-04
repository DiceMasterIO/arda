//! Which asset, rotation and mirror a placement draws.
//!
//! An [`AssetRef::Id`] names a variant *family*: the asset and its extra
//! takes `<id>.alt1`, `<id>.alt2`, … ([`Library::family`]). One member is
//! drawn per placement, picked by a hash of the seed and the placement's
//! position, so a map full of barrels does not repeat one barrel. An id
//! that names a take (`prop.barrel.alt2`) pins exactly that asset.
//!
//! A [`AssetRef::Query`] first picks among the families of its matches by
//! the seed and placement index (as it always has), then among the chosen
//! family's matching takes by position.
//!
//! Assets carrying the free tag [`ROT_FREE`] are also turned and mirrored
//! by hash when the placement leaves rotation 0 and no mirror.
//!
//! Vegetation on a square footprint anchored at its centre is also given a
//! hashed *pose* when the placement leaves rotation 0 and no mirror: a
//! scale between [`SCALE_MIN_PCT`] and [`SCALE_MAX_PCT`] percent in steps of
//! [`SCALE_STEP_PCT`] and, half
//! the time, a transpose (a mirror across the top-left to bottom-right
//! diagonal, which keeps a top-left highlight in the top-left). The free tag
//! [`FIXED_POSE`] opts an asset out. The pose is purely visual: the
//! footprint, and so blocking and cover, stay as catalogued.
//!
//! The position is the placement's anchor in world squares (`origin` plus
//! the local coordinates) when the layout has an origin, so neighbouring
//! windows that share a placement draw the same take and turn; without an
//! origin the placement index is hashed in too.

use crate::catalog::{Asset, AssetClass};
use crate::layout::{AssetRef, Placement, TacticalLayout};
use crate::library::{family_base, Library};
use crate::noise::hash2;

/// Free tag that lets the compositor turn and mirror an asset by hash:
/// for round or radial things (barrels, crates, trees, bushes, rocks)
/// whose look and meaning do not depend on which way they face.
pub const ROT_FREE: &str = "rot_free";

/// Free tag that keeps a vegetation asset at its catalogued size and
/// orientation (no hashed scale or transpose).
pub const FIXED_POSE: &str = "fixed_pose";

/// Smallest hashed vegetation scale, percent.
pub const SCALE_MIN_PCT: u8 = 85;
/// Largest hashed vegetation scale, percent.
pub const SCALE_MAX_PCT: u8 = 115;
/// Step between hashed vegetation scales, percent: coarse enough that each
/// asset has few distinct sprites to cache (7 scales × 2 transposes).
pub const SCALE_STEP_PCT: u8 = 5;

/// Sub-square steps per square when hashing a placement's position.
const POS_STEPS: f32 = 64.0;

/// What one placement draws.
#[derive(Debug, Clone, Copy)]
pub struct Resolved<'a> {
    /// The asset.
    pub asset: &'a Asset,
    /// Clockwise quarter turns.
    pub turns: u8,
    /// Mirrored before turning.
    pub mirror: bool,
    /// Drawn size in percent of the footprint (100: as catalogued).
    pub scale_pct: u8,
    /// Mirrored across the top-left to bottom-right diagonal, before the
    /// mirror and turns.
    pub transpose: bool,
}

/// Whether `a` may be drawn at `rotation` degrees, mirrored or not.
#[must_use]
pub fn fits(a: &Asset, rotation: u16, mirror: bool) -> bool {
    a.rotations.contains(&rotation) && (!mirror || a.mirror)
}

/// Every asset an [`AssetRef`] could resolve to: an id's whole variant
/// family, or every match of a query.
#[must_use]
pub fn candidates<'a>(lib: &'a Library, r: &AssetRef) -> Vec<&'a Asset> {
    match r {
        AssetRef::Id(id) => lib.family(id),
        AssetRef::Query { class, tags } => lib.query(*class, tags),
    }
}

/// The hash key of placement `index` of `layout`: its world position, plus
/// its index when the layout has no origin.
#[allow(clippy::cast_possible_truncation)] // rounded map positions
fn key(layout: &TacticalLayout, p: &Placement, index: usize, seed: u64) -> u64 {
    let step = |v: f32| (v * POS_STEPS).round() as i64;
    let (ox, oy) = layout.world_origin();
    let steps = POS_STEPS as i64;
    let (x, y) = (ox * steps + step(p.x), oy * steps + step(p.y));
    let index = match layout.origin {
        Some(_) => 0,
        None => i64::try_from(index).unwrap_or(0),
    };
    hash2(hash2(seed ^ 0x7A57, x, y), index, 0)
}

fn nth<'a>(all: &[&'a Asset], h: u64) -> Option<&'a Asset> {
    let n = all.len() as u64;
    if n == 0 {
        return None;
    }
    all.get(usize::try_from(h % n).ok()?).copied()
}

/// Resolves placement `index` of `layout` to an asset, rotation and mirror.
#[must_use]
pub fn resolve<'a>(
    lib: &'a Library,
    layout: &TacticalLayout,
    index: usize,
    seed: u64,
) -> Option<Resolved<'a>> {
    let p = layout.placements.get(index)?;
    let h = key(layout, p, index, seed);
    let all = candidates(lib, &p.asset);
    let members: Vec<&Asset> = match &p.asset {
        AssetRef::Id(_) => all
            .iter()
            .copied()
            .filter(|a| fits(a, p.rotation, p.mirror))
            .collect(),
        AssetRef::Query { .. } => {
            let family = pick_family(&all, seed, index)?;
            all.into_iter()
                .filter(|a| family_base(&a.id) == family)
                .collect()
        }
    };
    let asset = nth(&members, h)?;
    let (rotation, mirror) = turn(asset, p, h);
    let (scale_pct, transpose) = pose(asset, p, h);
    Some(Resolved {
        asset,
        turns: u8::try_from(rotation / 90).unwrap_or(0),
        mirror,
        scale_pct,
        transpose,
    })
}

/// The hashed scale and transpose of a vegetation placement left at
/// rotation 0 unmirrored, or `(100, false)`. Only square footprints
/// anchored at their centre qualify, so the sprite stays centred on the
/// placement and a transpose keeps the footprint; [`FIXED_POSE`] opts out.
/// `h` is the take pick's key, so overlapping windows agree.
fn pose(a: &Asset, p: &Placement, h: u64) -> (u8, bool) {
    let eligible = a.class == AssetClass::Vegetation
        && a.footprint.w == a.footprint.h
        && a.anchor_or_centre() == a.footprint_centre()
        && p.rotation == 0
        && !p.mirror
        && !a.tags.free.iter().any(|t| t == FIXED_POSE);
    if !eligible {
        return (100, false);
    }
    let h = hash2(h, 0x5CA1E, 0);
    let steps = u64::from((SCALE_MAX_PCT - SCALE_MIN_PCT) / SCALE_STEP_PCT) + 1;
    let pct = SCALE_MIN_PCT + SCALE_STEP_PCT * u8::try_from(h % steps).unwrap_or(0);
    (pct, (h >> 32) & 1 == 1)
}

/// The family a query placement draws from, picked by seed and index
/// among its matches' families in order of first appearance. Without
/// takes this is the pick among the matches themselves.
fn pick_family<'a>(all: &[&'a Asset], seed: u64, index: usize) -> Option<&'a str> {
    let mut families: Vec<&str> = Vec::new();
    for a in all {
        let f = family_base(&a.id);
        if !families.contains(&f) {
            families.push(f);
        }
    }
    let n = families.len() as u64;
    if n == 0 {
        return None;
    }
    let pick = hash2(seed ^ 0x9E50, i64::try_from(index).unwrap_or(0), 0) % n;
    families.get(usize::try_from(pick).ok()?).copied()
}

/// The placement's own rotation and mirror, or for a [`ROT_FREE`] asset
/// left at rotation 0 unmirrored, a hashed one among those the asset
/// allows. Only assets anchored at their centre are turned, so the
/// footprint stays where the layout put it; non-square ones only by half
/// turns, so it keeps its shape.
fn turn(a: &Asset, p: &Placement, h: u64) -> (u16, bool) {
    let free = a.tags.free.iter().any(|t| t == ROT_FREE);
    let centred = a.anchor_or_centre() == a.footprint_centre();
    if !free || !centred || p.rotation != 0 || p.mirror {
        return (p.rotation, p.mirror);
    }
    let square = a.footprint.w == a.footprint.h;
    let turns: Vec<u16> = a
        .rotations
        .iter()
        .copied()
        .filter(|r| square || r % 180 == 0)
        .collect();
    let h = hash2(h, 0x207, 0);
    let rotation = turns
        .get(usize::try_from(h % turns.len().max(1) as u64).unwrap_or(0))
        .copied()
        .unwrap_or(0);
    (rotation, a.mirror && (h >> 32) & 1 == 1)
}
